//! Поиск уже доступного модели содержимого для повторного `read_file`.
//!
//! Модуль анализирует только нормализованную историю следующего inference и
//! связывает typed `read_file` calls с успешными outputs по `call_id`. Он не
//! хранит cache: исчезновение исходного output из истории автоматически
//! отключает дедупликацию.

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::PoisonError;

use codex_protocol::models::ResponseItem;
use codex_utils_path_uri::PathUri;

use super::LineRange;
use super::ReadFileArgs;
use super::normalize_range;
use super::render_lines_content;
use super::split_lines_preserving_endings;
use super::yes_no;
use crate::tools::handlers::read_file_spec::READ_FILE_TOOL_NAME;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ReadFileSource {
    environment_id: String,
    path: PathUri,
}

impl ReadFileSource {
    pub(super) fn new(environment_id: String, path: PathUri) -> Self {
        Self {
            environment_id,
            path,
        }
    }
}

pub(super) struct ReadFileContextRequest<'a> {
    pub(super) args: &'a ReadFileArgs,
    pub(super) source: ReadFileSource,
    pub(super) lines: &'a [&'a str],
    pub(super) requested_range: LineRange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ReadFileContextCoverage {
    call_id: String,
    available_range: LineRange,
}

/// Хранит только resolved identity успешно обработанных вызовов в живом thread.
///
/// Это не cache содержимого и не доказательство доступности текста: matcher
/// всегда дополнительно требует исходный output в активной model-visible
/// history. Индекс очищается по этой истории при каждом повторном чтении.
#[derive(Debug, Default)]
pub(super) struct ReadFileContextIndex {
    sources: Mutex<HashMap<String, ReadFileSource>>,
}

impl ReadFileContextIndex {
    pub(super) fn record(&self, call_id: String, source: ReadFileSource) {
        self.sources().insert(call_id, source);
    }

    /// Удаляет provenance вызовов, которых больше нет в активной истории.
    pub(super) fn synchronize(&self, history: &[ResponseItem]) {
        let active_call_ids = history
            .iter()
            .filter_map(|item| match item {
                ResponseItem::FunctionCall { name, call_id, .. } if name == READ_FILE_TOOL_NAME => {
                    Some(call_id.as_str())
                }
                _ => None,
            })
            .collect::<HashSet<_>>();
        self.sources()
            .retain(|call_id, _| active_call_ids.contains(call_id.as_str()));
    }

    fn source(&self, call_id: &str) -> Option<ReadFileSource> {
        self.sources().get(call_id).cloned()
    }

    fn sources(&self) -> std::sync::MutexGuard<'_, HashMap<String, ReadFileSource>> {
        self.sources.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ParsedContentOutput<'a> {
    returned_range: LineRange,
    body: &'a str,
}

/// Ищет самый свежий одиночный output, который сам полностью покрывает запрос.
pub(super) fn find_context_coverage(
    history: &[ResponseItem],
    request: &ReadFileContextRequest<'_>,
    context_index: &ReadFileContextIndex,
) -> Option<ReadFileContextCoverage> {
    let calls = history
        .iter()
        .enumerate()
        .filter_map(|(index, item)| match item {
            ResponseItem::FunctionCall {
                name,
                arguments,
                call_id,
                ..
            } if name == READ_FILE_TOOL_NAME => {
                Some((call_id.as_str(), (index, arguments.as_str())))
            }
            _ => None,
        })
        .collect::<HashMap<_, _>>();

    history
        .iter()
        .enumerate()
        .rev()
        .find_map(|(output_index, item)| {
            let ResponseItem::FunctionCallOutput {
                call_id, output, ..
            } = item
            else {
                return None;
            };
            if output.success != Some(true) {
                return None;
            }
            let (call_index, arguments) = calls.get(call_id.as_str())?;
            if *call_index >= output_index {
                return None;
            }
            let candidate_args = serde_json::from_str::<ReadFileArgs>(arguments).ok()?;
            if candidate_args.line_numbers != request.args.line_numbers
                || context_index.source(call_id)? != request.source
            {
                return None;
            }
            let output_text = output.text_content()?;
            let parsed = parse_content_output(output_text, &candidate_args)?;
            if !parsed.returned_range.contains(request.requested_range)
                || parsed.returned_range.end > request.lines.len()
                || !output_matches_requested_lines(&parsed, request)
            {
                return None;
            }

            Some(ReadFileContextCoverage {
                call_id: call_id.clone(),
                available_range: parsed.returned_range,
            })
        })
}

/// Сравнивает только запрошенный поддиапазон с соответствующим местом прежнего
/// output. Изменение строк вне нового запроса не делает сохраненный текст
/// запрошенных строк недостоверным.
fn output_matches_requested_lines(
    parsed: &ParsedContentOutput<'_>,
    request: &ReadFileContextRequest<'_>,
) -> bool {
    let output_lines = split_lines_preserving_endings(parsed.body);
    let available_len = parsed
        .returned_range
        .end
        .saturating_sub(parsed.returned_range.start)
        .saturating_add(1);
    if output_lines.len() != available_len {
        return false;
    }

    let relative_start = request
        .requested_range
        .start
        .saturating_sub(parsed.returned_range.start);
    let requested_len = request
        .requested_range
        .end
        .saturating_sub(request.requested_range.start)
        .saturating_add(1);
    let Some(relative_end) = relative_start.checked_add(requested_len) else {
        return false;
    };
    let Some(previous_requested_lines) = output_lines.get(relative_start..relative_end) else {
        return false;
    };
    previous_requested_lines.concat()
        == render_lines_content(
            request.lines,
            request.requested_range,
            request.args.line_numbers,
        )
}

/// Формирует короткую ссылку на один проверенный content-bearing output.
pub(super) fn render_already_in_context(
    args: &ReadFileArgs,
    requested_range: LineRange,
    coverage: &ReadFileContextCoverage,
) -> String {
    format!(
        "ReadFile: {}\nStatus: already_in_context\nCoverage: requested={} available={} complete=yes\nLineNumbers: {}\nCoveredBy: {}\n",
        args.path,
        super::format_range(Some(requested_range)),
        super::format_range(Some(coverage.available_range)),
        yes_no(args.line_numbers),
        coverage.call_id,
    )
}

/// Разбирает только обычный content-bearing `ReadFile` output и отвергает
/// ошибки, ссылки `already_in_context` и поврежденные metadata.
fn parse_content_output<'a>(
    output: &'a str,
    args: &ReadFileArgs,
) -> Option<ParsedContentOutput<'a>> {
    let mut sections = output.splitn(4, '\n');
    if sections.next()? != format!("ReadFile: {}", args.path) {
        return None;
    }
    let lines_metadata = parse_lines_metadata(sections.next()?)?;
    let line_numbers = sections.next()?.strip_prefix("LineNumbers: ")?;
    if line_numbers != yes_no(args.line_numbers) {
        return None;
    }
    let body = sections.next()?.strip_prefix('\n')?;

    let expected_requested = normalize_range(args, lines_metadata.total_lines).ok()?;
    if expected_requested != lines_metadata.requested_range {
        return None;
    }
    let requested_range = lines_metadata.requested_range?;
    let returned_range = lines_metadata.returned_range?;
    if returned_range.start != requested_range.start
        || !requested_range.contains(returned_range)
        || lines_metadata.complete != (returned_range == requested_range)
    {
        return None;
    }

    Some(ParsedContentOutput {
        returned_range,
        body,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LinesMetadata {
    total_lines: usize,
    requested_range: Option<LineRange>,
    returned_range: Option<LineRange>,
    complete: bool,
}

/// Разбирает фиксированную строку `Lines:` без попытки угадать иной формат.
fn parse_lines_metadata(line: &str) -> Option<LinesMetadata> {
    let mut fields = line.strip_prefix("Lines: ")?.split_ascii_whitespace();
    let total_lines = fields.next()?.strip_prefix("total=")?.parse().ok()?;
    let requested_range = parse_range(fields.next()?.strip_prefix("requested=")?)?;
    let returned_range = parse_range(fields.next()?.strip_prefix("returned=")?)?;
    let complete = match fields.next()?.strip_prefix("complete=")? {
        "yes" => true,
        "no" => false,
        _ => return None,
    };
    if fields.next().is_some() {
        return None;
    }

    Some(LinesMetadata {
        total_lines,
        requested_range,
        returned_range,
        complete,
    })
}

fn parse_range(value: &str) -> Option<Option<LineRange>> {
    if value == "empty" {
        return Some(None);
    }
    let (start, end) = value.split_once('-')?;
    let range = LineRange {
        start: start.parse().ok()?,
        end: end.parse().ok()?,
    };
    (range.start >= 1 && range.end >= range.start).then_some(Some(range))
}

impl LineRange {
    fn contains(self, other: Self) -> bool {
        self.start <= other.start && self.end >= other.end
    }
}

#[cfg(test)]
#[path = "read_file_context_tests.rs"]
mod tests;
