//! Сохраняет полный вывод exec и формирует последовательный построчный фрагмент для модели.

use std::io;

use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_output_truncation::TruncationPolicy;
use codex_utils_output_truncation::approx_token_count;
use tokio::io::AsyncWriteExt;

use crate::tools::line_utils::split_lines_preserving_endings;

const EXEC_OUTPUTS_DIR: &str = "exec_outputs";
const FALLBACK_THREAD_ID: &str = "thread";
const FALLBACK_CALL_ID: &str = "call";
const FALLBACK_ARTIFACT_ID: &str = "artifact";
const MAX_ERROR_CHARS: usize = 240;

/// Описывает построчный фрагмент большого результата и судьбу его полного файла.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecCommandOutputSpill {
    pub inline_limit_tokens: usize,
    pub result: ExecCommandOutputSpillResult,
}

/// Фиксирует успешную запись полного результата либо ограниченное описание ошибки.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExecCommandOutputSpillResult {
    Saved { path: AbsolutePathBuf },
    SaveFailed { error: String },
}

/// Содержит готовое spill-сообщение и стоимость только возвращённого фрагмента.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderedOutputSpill {
    pub body: String,
    pub excerpt_token_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LinePrefixSelection {
    returned_lines: usize,
    prefix_bytes: usize,
    token_count: usize,
}

/// Выбирает наименьший из лимитов конфигурации, запроса и модели.
pub(crate) fn effective_inline_output_max_tokens(
    configured_max_tokens: usize,
    requested_max_output_tokens: Option<usize>,
    truncation_policy: TruncationPolicy,
) -> usize {
    configured_max_tokens
        .min(requested_max_output_tokens.unwrap_or(usize::MAX))
        .min(truncation_policy.token_budget())
}

/// Сохраняет весь доступный вывод, только когда он превышает inline-лимит.
///
/// Ошибка записи не проваливает вызов инструмента: результат содержит ограниченное
/// описание ошибки, которое позднее попадёт в ответ для модели.
pub(crate) async fn maybe_spill_exec_command_output(
    codex_home: &AbsolutePathBuf,
    thread_id: &str,
    call_id: &str,
    artifact_id: &str,
    raw_output: &[u8],
    original_token_count: usize,
    inline_limit_tokens: usize,
) -> Option<ExecCommandOutputSpill> {
    if original_token_count <= inline_limit_tokens {
        return None;
    }

    let path = spill_path(codex_home, thread_id, call_id, artifact_id);
    match write_spill_file(&path, raw_output).await {
        Ok(()) => Some(ExecCommandOutputSpill {
            inline_limit_tokens,
            result: ExecCommandOutputSpillResult::Saved { path },
        }),
        Err(error) => {
            tracing::warn!(
                error = %error,
                path = %path.display(),
                "failed to save exec_command output spill file"
            );
            Some(ExecCommandOutputSpill {
                inline_limit_tokens,
                result: ExecCommandOutputSpillResult::SaveFailed {
                    error: bounded_error_message(&error),
                },
            })
        }
    }
}

/// Формирует общее spill-сообщение для модели без разрыва строк и добавления суффикса.
pub(crate) fn render_output_spill(
    raw_output: &[u8],
    spill: &ExecCommandOutputSpill,
) -> RenderedOutputSpill {
    let text = String::from_utf8_lossy(raw_output);
    let lines = split_lines_preserving_endings(&text);
    let prefix = select_line_prefix(&text, &lines, spill.inline_limit_tokens);
    let returned_lines = prefix.returned_lines;
    let remaining_lines = lines.len().saturating_sub(returned_lines);
    let returned = if returned_lines == 0 {
        "none".to_string()
    } else {
        format!("1-{returned_lines}")
    };

    let mut sections = vec![format!(
        "Lines: total={} returned={returned} remaining={remaining_lines} complete=no",
        lines.len()
    )];
    match &spill.result {
        ExecCommandOutputSpillResult::Saved { path } => {
            sections.push(format!("Full output: {}", path.display()));
        }
        ExecCommandOutputSpillResult::SaveFailed { error } => {
            sections.push(format!("Failed to save output: {error}"));
        }
    }

    if returned_lines == 0 {
        sections.push("Error: first line exceeds excerpt token limit".to_string());
    } else {
        sections.push("Output excerpt:\n".to_string());
        sections.push(text[..prefix.prefix_bytes].to_string());
    }

    RenderedOutputSpill {
        body: sections.join("\n"),
        excerpt_token_count: prefix.token_count,
    }
}

/// Находит максимальное число первых целых строк, помещающихся в лимит токенов.
fn select_line_prefix(
    text: &str,
    lines: &[&str],
    inline_limit_tokens: usize,
) -> LinePrefixSelection {
    let mut prefix_bytes = 0_usize;
    let mut returned_lines = 0;
    let mut token_count = 0;
    for line in lines {
        let next_prefix_bytes = prefix_bytes.saturating_add(line.len());
        let next_token_count = approx_token_count(&text[..next_prefix_bytes]);
        if next_token_count > inline_limit_tokens {
            break;
        }
        prefix_bytes = next_prefix_bytes;
        returned_lines += 1;
        token_count = next_token_count;
    }
    LinePrefixSelection {
        returned_lines,
        prefix_bytes,
        token_count,
    }
}

/// Строит абсолютный путь внутри каталога Codex из безопасных компонентов идентификатора.
fn spill_path(
    codex_home: &AbsolutePathBuf,
    thread_id: &str,
    call_id: &str,
    artifact_id: &str,
) -> AbsolutePathBuf {
    let thread_id = sanitize_path_component(thread_id, FALLBACK_THREAD_ID);
    let call_id = sanitize_path_component(call_id, FALLBACK_CALL_ID);
    let artifact_id = sanitize_path_component(artifact_id, FALLBACK_ARTIFACT_ID);
    codex_home
        .join(EXEC_OUTPUTS_DIR)
        .join(thread_id)
        .join(format!("{call_id}-{artifact_id}.log"))
}

/// Заменяет небезопасные символы компонента пути и не допускает пустое имя.
fn sanitize_path_component(value: &str, fallback: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();

    if sanitized.is_empty() {
        fallback.to_string()
    } else {
        sanitized
    }
}

/// Создаёт новый файл с закрытыми правами Unix и не перезаписывает существующий.
async fn write_spill_file(path: &AbsolutePathBuf, raw_output: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent.as_path()).await?;
    }

    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        options.mode(0o600);
    }

    let mut file = options.open(path.as_path()).await?;
    file.write_all(raw_output).await?;
    file.flush().await
}

/// Ограничивает текст ошибки файловой системы перед включением в ответ для модели.
fn bounded_error_message(error: &io::Error) -> String {
    let message = error.to_string();
    if message.chars().count() <= MAX_ERROR_CHARS {
        return message;
    }

    let mut truncated = message.chars().take(MAX_ERROR_CHARS).collect::<String>();
    truncated.push_str("...");
    truncated
}

#[cfg(test)]
#[path = "output_spill_tests.rs"]
mod tests;
