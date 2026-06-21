use std::io;

use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_output_truncation::TruncationPolicy;
use tokio::io::AsyncWriteExt;

use crate::tools::context::ExecCommandOutputSpill;
use crate::tools::context::ExecCommandOutputSpillResult;

const EXEC_OUTPUTS_DIR: &str = "exec_outputs";
const FALLBACK_THREAD_ID: &str = "thread";
const FALLBACK_CALL_ID: &str = "call";
const FALLBACK_CHUNK_ID: &str = "chunk";
const MAX_ERROR_CHARS: usize = 240;

pub(crate) fn effective_inline_output_max_tokens(
    configured_max_tokens: usize,
    requested_max_output_tokens: Option<usize>,
    truncation_policy: TruncationPolicy,
) -> usize {
    configured_max_tokens
        .min(requested_max_output_tokens.unwrap_or(usize::MAX))
        .min(truncation_policy.token_budget())
}

pub(crate) async fn maybe_spill_exec_command_output(
    codex_home: &AbsolutePathBuf,
    thread_id: &str,
    call_id: &str,
    chunk_id: &str,
    raw_output: &[u8],
    original_token_count: usize,
    inline_limit_tokens: usize,
) -> Option<ExecCommandOutputSpill> {
    if original_token_count <= inline_limit_tokens {
        return None;
    }

    let path = spill_path(codex_home, thread_id, call_id, chunk_id);
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

fn spill_path(
    codex_home: &AbsolutePathBuf,
    thread_id: &str,
    call_id: &str,
    chunk_id: &str,
) -> AbsolutePathBuf {
    let thread_id = sanitize_path_component(thread_id, FALLBACK_THREAD_ID);
    let call_id = sanitize_path_component(call_id, FALLBACK_CALL_ID);
    let chunk_id = sanitize_path_component(chunk_id, FALLBACK_CHUNK_ID);
    codex_home
        .join(EXEC_OUTPUTS_DIR)
        .join(thread_id)
        .join(format!("{call_id}-{chunk_id}.log"))
}

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
