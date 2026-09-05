//! TUI token usage models and display formatting.

use std::fmt;

use codex_app_server_protocol::ModelTokenUsageSnapshot;
use codex_app_server_protocol::TokenUsageSnapshot;
use codex_protocol::num_format::format_with_separators;
use codex_protocol::protocol::compare_model_slugs;
use serde::Deserialize;
use serde::Serialize;

const BASELINE_TOKENS: i64 = 12000;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_output_tokens: i64,
    pub total_tokens: i64,
}

impl TokenUsage {
    pub fn is_zero(&self) -> bool {
        self.total_tokens == 0
    }

    pub(crate) fn cached_input(&self) -> i64 {
        self.cached_input_tokens.max(0)
    }

    pub(crate) fn non_cached_input(&self) -> i64 {
        (self.input_tokens - self.cached_input()).max(0)
    }

    pub(crate) fn blended_total(&self) -> i64 {
        (self.non_cached_input() + self.output_tokens.max(0)).max(0)
    }

    /// Returns the raw `total_tokens` value. For `last_token_usage`, this is the latest active
    /// context size; for `total_token_usage`, this is the accumulated session total.
    pub(crate) fn tokens_in_context_window(&self) -> i64 {
        self.total_tokens
    }

    pub(crate) fn percent_of_context_window_remaining(&self, context_window: i64) -> i64 {
        if context_window <= BASELINE_TOKENS {
            return 0;
        }
        let effective_window = context_window - BASELINE_TOKENS;
        let used = (self.tokens_in_context_window() - BASELINE_TOKENS).max(0);
        let remaining = (effective_window - used).max(0);
        ((remaining as f64 / effective_window as f64) * 100.0)
            .clamp(0.0, 100.0)
            .round() as i64
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TokenUsageInfo {
    pub(crate) total_token_usage: TokenUsage,
    pub(crate) last_token_usage: TokenUsage,
    pub(crate) model_context_window: Option<i64>,
}

impl fmt::Display for TokenUsage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Token usage: total={} input={}{} output={}{}",
            format_with_separators(self.blended_total()),
            format_with_separators(self.non_cached_input()),
            if self.cached_input() > 0 {
                format!(
                    " (+ {} cached)",
                    format_with_separators(self.cached_input())
                )
            } else {
                String::new()
            },
            format_with_separators(self.output_tokens),
            if self.reasoning_output_tokens > 0 {
                format!(
                    " (reasoning {})",
                    format_with_separators(self.reasoning_output_tokens)
                )
            } else {
                String::new()
            }
        )
    }
}

/// Formats an immutable model map without ever adding counters across different model slugs.
pub(crate) fn format_token_usage_snapshot(snapshot: &TokenUsageSnapshot) -> String {
    if snapshot.models.is_empty() {
        return "unavailable".to_string();
    }
    let mut models = snapshot.models.iter().collect::<Vec<_>>();
    models.sort_by(|left, right| match (&left.model, &right.model) {
        (Some(left), Some(right)) => compare_model_slugs(left, right),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    models
        .into_iter()
        .map(format_model_token_usage)
        .collect::<Vec<_>>()
        .join("; ")
}

fn format_model_token_usage(model_usage: &ModelTokenUsageSnapshot) -> String {
    let Some(model) = model_usage.model.as_deref() else {
        return "unavailable".to_string();
    };
    let Some(usage) = &model_usage.usage else {
        return format!("[{model}] unavailable");
    };
    let cached = usage.cached_input_tokens.max(0);
    let non_cached = usage.input_tokens.saturating_sub(cached).max(0);
    let output = usage.output_tokens.max(0);
    let reasoning = usage.reasoning_output_tokens.max(0);
    let partial = if model_usage.incomplete {
        ", partial"
    } else {
        ""
    };
    format!(
        "[{model}] {} in, {} cached, {} ({}) out{partial}",
        format_with_separators(non_cached),
        format_with_separators(cached),
        format_with_separators(output),
        format_with_separators(reasoning),
    )
}
