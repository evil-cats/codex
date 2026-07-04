use crate::function_tool::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::context::boxed_tool_output;
use crate::tools::handlers::parse_arguments;
use crate::tools::handlers::read_file_spec::READ_FILE_TOOL_NAME;
use crate::tools::handlers::read_file_spec::create_read_file_tool;
use crate::tools::handlers::resolve_tool_environment;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::ToolExecutor;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use codex_utils_output_truncation::approx_token_count;
use codex_utils_path_uri::PathUri;
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

    fn supports_parallel_tool_calls(&self) -> bool {
        true
    }

    fn handle(&self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'_> {
        Box::pin(self.handle_call(invocation))
    }
}

impl ReadFileHandler {
    async fn handle_call(
        &self,
        invocation: ToolInvocation,
    ) -> Result<Box<dyn crate::tools::context::ToolOutput>, FunctionCallError> {
        let ToolInvocation { turn, payload, .. } = invocation;
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
            resolve_tool_environment(turn.as_ref(), args.environment_id.as_deref())?
        else {
            return Err(FunctionCallError::RespondToModel(
                "read_file is unavailable in this session".to_string(),
            ));
        };

        let cwd = turn_environment.cwd().clone();
        let abs_path = cwd.join(&args.path);
        let sandbox = turn.file_system_sandbox_context(
            /*additional_permissions*/ None,
            turn_environment.cwd_uri(),
        );
        let fs = turn_environment.environment.get_filesystem();
        let path_uri = PathUri::from_abs_path(&abs_path);

        let metadata = fs
            .get_metadata(&path_uri, Some(&sandbox))
            .await
            .map_err(|error| {
                FunctionCallError::RespondToModel(format!(
                    "unable to locate file at `{}`: {error}",
                    abs_path.display()
                ))
            })?;

        if !metadata.is_file {
            return Err(FunctionCallError::RespondToModel(format!(
                "read_file path `{}` is not a regular file",
                abs_path.display()
            )));
        }

        let content = fs
            .read_file_text(&path_uri, Some(&sandbox))
            .await
            .map_err(|error| {
                FunctionCallError::RespondToModel(format!(
                    "unable to read UTF-8 text file at `{}`: {error}",
                    abs_path.display()
                ))
            })?;
        let output = read_file_output(&args, &content, turn.config.read_file_content_max_tokens)?;

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
    let returned = select_returned_range(&lines, requested_range, content_max_tokens)?;
    Ok(render_output(args, &lines, requested_range, returned))
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

    if args.line_numbers {
        for line_number in range.start..=range.end {
            output.push_str(&format!("{} | {}", line_number, lines[line_number - 1]));
        }
    } else {
        output.push_str(&raw_lines_content(lines, range.start, range.end));
    }

    output
}

fn split_lines_preserving_endings(content: &str) -> Vec<&str> {
    if content.is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::new();
    let mut start = 0;
    for (idx, ch) in content.char_indices() {
        if ch == '\n' {
            lines.push(&content[start..idx + ch.len_utf8()]);
            start = idx + ch.len_utf8();
        }
    }
    if start < content.len() {
        lines.push(&content[start..]);
    }
    lines
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
