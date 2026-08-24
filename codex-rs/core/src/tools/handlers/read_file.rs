//! Обработчик встроенного tool `read_file`.
//!
//! Он выбирает `environment` текущего шага, разрешает переданный `path` через
//! `PathUri` выбранного окружения и читает обычный текстовый UTF-8 файл через его
//! файловую систему с действующими sandbox-ограничениями. Формирование диапазона,
//! метаданных полноты и усечение по целым строкам остаются контрактом этого
//! обработчика, а окончания строк сохраняет общая утилита инструментов. Перед
//! выдачей содержимого обработчик также проверяет, остался ли один полностью
//! покрывающий результат в текущем окне контекста.

#[path = "read_file_context.rs"]
mod read_file_context;

use std::collections::HashSet;

use crate::function_tool::FunctionCallError;
use crate::session::session::Session;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::context::boxed_tool_output;
use crate::tools::handlers::parse_arguments;
use crate::tools::handlers::read_file_spec::READ_FILE_TOOL_NAME;
use crate::tools::handlers::read_file_spec::create_read_file_tool;
use crate::tools::handlers::resolve_tool_environment;
use crate::tools::line_utils::split_lines_preserving_endings;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::ToolExecutor;
use codex_exec_server::GetMetadataOptions;
use codex_exec_server::ReadFileOptions;
use codex_protocol::models::ResponseItem;
use codex_tools::ToolExposure;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use codex_utils_output_truncation::approx_token_count;
use read_file_context::ReadFileContextIndex;
use read_file_context::ReadFileContextRequest;
use read_file_context::ReadFileSource;
use read_file_context::find_context_coverage;
use read_file_context::render_already_in_context;
use serde::Deserialize;

pub struct ReadFileHandler {
    include_environment_id: bool,
}

impl ReadFileHandler {
    pub(crate) fn new(include_environment_id: bool) -> Self {
        Self {
            include_environment_id,
        }
    }
}

/// Восстанавливает оконный provenance `read_file` из replay текущего thread.
///
/// Вызовы, принесённые replacement-history последней compaction, исключаются;
/// допустимы только прямые вызовы, записанные после этой границы.
pub(crate) fn restore_read_file_context_index<'a>(
    session: &Session,
    window_id: String,
    history: impl IntoIterator<Item = &'a ResponseItem>,
    replacement_history_call_ids: &HashSet<String>,
) {
    session
        .services
        .thread_extension_data
        .get_or_init(ReadFileContextIndex::default)
        .restore(window_id, history, replacement_history_call_ids);
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadFileArgs {
    path: String,
    #[serde(default)]
    start_line: Option<usize>,
    #[serde(default)]
    end_line: Option<usize>,
    #[serde(default = "default_true")]
    line_numbers: bool,
    #[serde(default)]
    environment_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LineRange {
    start: usize,
    end: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ReturnedRange {
    range: Option<LineRange>,
    complete: bool,
}

impl ToolExecutor<ToolInvocation> for ReadFileHandler {
    fn tool_name(&self) -> ToolName {
        ToolName::plain(READ_FILE_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        create_read_file_tool(self.include_environment_id)
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::DirectModelOnly
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        true
    }

    fn handle(&self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'_> {
        Box::pin(self.handle_call(invocation))
    }
}

impl ReadFileHandler {
    /// Читает файл через filesystem выбранного environment и формирует model-visible output.
    ///
    /// Metadata и содержимое читаются с `GetMetadataOptions::default()`,
    /// `ReadFileOptions::default()` и действующим sandbox. После нормализации
    /// диапазона результат либо ссылается на один проверенный output текущего
    /// окна, либо возвращает свежий текст.
    async fn handle_call(
        &self,
        invocation: ToolInvocation,
    ) -> Result<Box<dyn crate::tools::context::ToolOutput>, FunctionCallError> {
        let ToolInvocation {
            session,
            turn,
            step_context,
            call_id,
            payload,
            ..
        } = invocation;
        let arguments = match payload {
            ToolPayload::Function { arguments } => arguments,
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "read_file handler received unsupported payload".to_string(),
                ));
            }
        };

        let args: ReadFileArgs = parse_arguments(&arguments)?;
        if args.path.trim().is_empty() {
            return Err(FunctionCallError::RespondToModel(
                "read_file path must be a non-empty string".to_string(),
            ));
        }
        let Some(turn_environment) =
            resolve_tool_environment(&step_context.environments, args.environment_id.as_deref())?
        else {
            return Err(FunctionCallError::RespondToModel(
                "read_file is unavailable in this session".to_string(),
            ));
        };

        let path_uri = turn_environment.cwd().join(&args.path).map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "unable to resolve file path `{}` against environment cwd `{}`: {err}",
                args.path,
                turn_environment.cwd(),
            ))
        })?;
        let model_visible_path = path_uri.inferred_native_path_string();
        let sandbox = turn
            .file_system_sandbox_context(/*additional_permissions*/ None, turn_environment);
        let fs = turn_environment.environment.get_filesystem();

        let metadata = fs
            .get_metadata(&path_uri, GetMetadataOptions::default(), Some(&sandbox))
            .await
            .map_err(|error| {
                FunctionCallError::RespondToModel(format!(
                    "unable to locate file at `{model_visible_path}`: {error}"
                ))
            })?;

        if !metadata.is_file {
            return Err(FunctionCallError::RespondToModel(format!(
                "read_file path `{model_visible_path}` is not a regular file"
            )));
        }

        let content = fs
            .read_file_text(&path_uri, ReadFileOptions::default(), Some(&sandbox))
            .await
            .map_err(|error| {
                FunctionCallError::RespondToModel(format!(
                    "unable to read UTF-8 text file at `{model_visible_path}`: {error}"
                ))
            })?;
        let lines = split_lines_preserving_endings(&content);
        let requested_range = normalize_range(&args, lines.len())?;
        let current_source = ReadFileSource::new(
            turn_environment.selection.environment_id.clone(),
            path_uri.clone(),
        );
        let context_index = session
            .services
            .thread_extension_data
            .get_or_init(ReadFileContextIndex::default);
        let window_id = session.current_window_id().await;
        let history = session
            .clone_history()
            .await
            .for_prompt(&turn.model_info.input_modalities);
        context_index.synchronize(&window_id, &history);
        let contextual_output = if let Some(requested_range) = requested_range {
            let request = ReadFileContextRequest {
                args: &args,
                source: current_source.clone(),
                lines: &lines,
                requested_range,
            };
            let coverage = find_context_coverage(&history, &request, &context_index);
            coverage.map(|coverage| render_already_in_context(&args, requested_range, &coverage))
        } else {
            None
        };
        let output = contextual_output.map_or_else(
            || {
                fresh_read_file_output(
                    &args,
                    &lines,
                    requested_range,
                    turn.config.read_file_content_max_tokens,
                )
            },
            Ok,
        )?;
        context_index.record(&window_id, call_id, current_source);

        Ok(boxed_tool_output(FunctionToolOutput::from_text(
            output,
            Some(true),
        )))
    }
}

impl CoreToolRuntime for ReadFileHandler {}

fn default_true() -> bool {
    true
}

fn read_file_output(
    args: &ReadFileArgs,
    content: &str,
    content_max_tokens: usize,
) -> Result<String, FunctionCallError> {
    let lines = split_lines_preserving_endings(content);
    let requested_range = normalize_range(args, lines.len())?;
    fresh_read_file_output(args, &lines, requested_range, content_max_tokens)
}

/// Строит обычный content-bearing output после нормализации диапазона.
fn fresh_read_file_output(
    args: &ReadFileArgs,
    lines: &[&str],
    requested_range: Option<LineRange>,
    content_max_tokens: usize,
) -> Result<String, FunctionCallError> {
    let returned = select_returned_range(lines, requested_range, content_max_tokens)?;
    Ok(render_output(args, lines, requested_range, returned))
}

fn normalize_range(
    args: &ReadFileArgs,
    total_lines: usize,
) -> Result<Option<LineRange>, FunctionCallError> {
    match (args.start_line, args.end_line) {
        (None, None) if total_lines == 0 => Ok(None),
        (None, None) => Ok(Some(LineRange {
            start: 1,
            end: total_lines,
        })),
        (Some(_), None) | (None, Some(_)) => Err(FunctionCallError::RespondToModel(
            "start_line and end_line must be provided together".to_string(),
        )),
        (Some(start), Some(end)) => {
            if start < 1 {
                return Err(FunctionCallError::RespondToModel(
                    "start_line must be greater than or equal to 1".to_string(),
                ));
            }
            if end < start {
                return Err(FunctionCallError::RespondToModel(format!(
                    "end_line ({end}) must be greater than or equal to start_line ({start})"
                )));
            }
            if total_lines == 0 {
                return Err(FunctionCallError::RespondToModel(
                    "cannot request a line range from an empty file".to_string(),
                ));
            }
            if start > total_lines {
                return Err(FunctionCallError::RespondToModel(format!(
                    "start_line ({start}) exceeds total lines ({total_lines})"
                )));
            }

            Ok(Some(LineRange {
                start,
                end: end.min(total_lines),
            }))
        }
    }
}

fn select_returned_range(
    lines: &[&str],
    requested_range: Option<LineRange>,
    content_max_tokens: usize,
) -> Result<ReturnedRange, FunctionCallError> {
    let Some(requested_range) = requested_range else {
        return Ok(ReturnedRange {
            range: None,
            complete: true,
        });
    };

    let mut returned_end = requested_range.end;
    while returned_end >= requested_range.start {
        let candidate = raw_lines_content(lines, requested_range.start, returned_end);
        if approx_token_count(&candidate) <= content_max_tokens {
            return Ok(ReturnedRange {
                range: Some(LineRange {
                    start: requested_range.start,
                    end: returned_end,
                }),
                complete: returned_end == requested_range.end,
            });
        }
        if returned_end == requested_range.start {
            break;
        }
        returned_end -= 1;
    }

    Ok(ReturnedRange {
        range: None,
        complete: false,
    })
}

fn render_output(
    args: &ReadFileArgs,
    lines: &[&str],
    requested_range: Option<LineRange>,
    returned: ReturnedRange,
) -> String {
    let requested = format_range(requested_range);
    let returned_range = format_range(returned.range);
    let complete = yes_no(returned.complete);
    let mut output = format!(
        "ReadFile: {}\nLines: total={} requested={requested} returned={returned_range} complete={complete}\n",
        args.path,
        lines.len(),
    );

    if returned.range.is_none()
        && !returned.complete
        && let Some(range) = requested_range
    {
        output.push_str(&format!(
            "Error: line {} exceeds ReadFile content token limit\n",
            range.start
        ));
        return output;
    }

    let line_numbers = yes_no(args.line_numbers);
    output.push_str(&format!("LineNumbers: {line_numbers}\n\n"));

    let Some(range) = returned.range else {
        return output;
    };

    output.push_str(&render_lines_content(lines, range, args.line_numbers));

    output
}

/// Рендерит только содержимое диапазона в точности так, как его видит модель.
fn render_lines_content(lines: &[&str], range: LineRange, line_numbers: bool) -> String {
    if !line_numbers {
        return raw_lines_content(lines, range.start, range.end);
    }

    let mut output = String::new();
    for line_number in range.start..=range.end {
        output.push_str(&format!("{} | {}", line_number, lines[line_number - 1]));
    }
    output
}

fn raw_lines_content(lines: &[&str], start: usize, end: usize) -> String {
    lines[start - 1..end].concat()
}

fn format_range(range: Option<LineRange>) -> String {
    range.map_or_else(
        || "empty".to_string(),
        |range| format!("{}-{}", range.start, range.end),
    )
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(test)]
#[path = "read_file_tests.rs"]
mod tests;
