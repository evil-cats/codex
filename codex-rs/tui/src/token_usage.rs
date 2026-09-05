//! Модели токенов TUI и форматирование токенов с расчётной кредитной стоимостью.

use std::fmt;

use codex_app_server_protocol::ModelTokenUsageSnapshot;
use codex_app_server_protocol::TokenUsageSnapshot;
use codex_config::CreditAmount;
use codex_config::CreditRatesState;
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

/// Неизменяемое накопление видимого хода и необязательная дельта после прошлой границы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SeparatorTokenUsageSnapshot {
    pub(crate) cumulative: TokenUsageSnapshot,
    pub(crate) delta: Option<TokenUsageSnapshot>,
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

/// Форматирует неизменяемую карту моделей, не складывая токены разных slug.
pub(crate) fn format_token_usage_snapshot(
    snapshot: &TokenUsageSnapshot,
    credit_rates: &CreditRatesState,
) -> String {
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
        .map(|model_usage| format_model_token_usage(model_usage, credit_rates))
        .collect::<Vec<_>>()
        .join("; ")
}

/// Форматирует накопление каждой модели и существующую дельту последнего разделителя.
pub(crate) fn format_separator_token_usage_snapshot(
    snapshot: &SeparatorTokenUsageSnapshot,
    credit_rates: &CreditRatesState,
) -> String {
    if snapshot.cumulative.models.is_empty() {
        return "unavailable".to_string();
    }
    let mut models = snapshot.cumulative.models.iter().collect::<Vec<_>>();
    models.sort_by(|left, right| match (&left.model, &right.model) {
        (Some(left), Some(right)) => compare_model_slugs(left, right),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    models
        .into_iter()
        .map(|model_usage| match &snapshot.delta {
            Some(delta) => {
                let delta = delta
                    .models
                    .iter()
                    .find(|delta| delta.model.as_deref() == model_usage.model.as_deref());
                format_model_token_usage_with_delta(model_usage, delta, credit_rates)
            }
            None => format_model_token_usage(model_usage, credit_rates),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// Форматирует `Total` и добавляет общую кредитную сумму только при нескольких моделях.
pub(crate) fn format_total_token_usage_snapshot(
    snapshot: &TokenUsageSnapshot,
    credit_rates: &CreditRatesState,
) -> String {
    let models = format_token_usage_snapshot(snapshot, credit_rates);
    if !credit_rates.is_enabled() || snapshot.models.len() < 2 {
        return models;
    }

    let mut total = CreditAmount::default();
    let mut incomplete = false;
    for model_usage in &snapshot.models {
        let Some(cost) = model_credit_cost(model_usage, credit_rates) else {
            return format!("?Ƶ; {models}");
        };
        total = total.saturating_add(cost);
        incomplete |= model_usage.incomplete;
    }
    format!("{}; {models}", format_credit_amount(total, incomplete))
}

fn format_model_token_usage(
    model_usage: &ModelTokenUsageSnapshot,
    credit_rates: &CreditRatesState,
) -> String {
    let Some(model) = model_usage.model.as_deref() else {
        return "unavailable".to_string();
    };
    let Some(usage) = &model_usage.usage else {
        return if credit_rates.is_enabled() {
            format!("[{model}] ?Ƶ, unavailable")
        } else {
            format!("[{model}] unavailable")
        };
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
    let credit = if credit_rates.is_enabled() {
        let formatted = model_credit_cost(model_usage, credit_rates)
            .map(|cost| format_credit_amount(cost, model_usage.incomplete))
            .unwrap_or_else(|| "?Ƶ".to_string());
        format!("{formatted}, ")
    } else {
        String::new()
    };
    format!(
        "[{model}] {credit}{} in, {} cached, {} / {} out{partial}",
        format_with_separators(non_cached),
        format_with_separators(cached),
        format_with_separators(output),
        format_with_separators(reasoning),
    )
}

#[derive(Clone, Copy, Default)]
struct DisplayTokenCounts {
    non_cached: i64,
    cached: i64,
    output: i64,
    reasoning: i64,
}

/// Форматирует модель обычного разделителя, не подменяя отсутствующий `usage` нулевой дельтой.
fn format_model_token_usage_with_delta(
    model_usage: &ModelTokenUsageSnapshot,
    delta: Option<&ModelTokenUsageSnapshot>,
    credit_rates: &CreditRatesState,
) -> String {
    let Some(model) = model_usage.model.as_deref() else {
        return "unavailable".to_string();
    };
    let delta_counts = match delta {
        Some(delta) => delta.usage.as_ref().map(display_token_counts),
        None => Some(DisplayTokenCounts::default()),
    };
    let incomplete = model_usage.incomplete || delta.is_some_and(|delta| delta.incomplete);
    let Some(counts) = model_usage.usage.as_ref().map(display_token_counts) else {
        return if credit_rates.is_enabled() {
            format!(
                "[{model}] ?Ƶ ({}), unavailable",
                format_delta_credit(delta, credit_rates)
            )
        } else {
            format!("[{model}] unavailable")
        };
    };
    let partial = if incomplete { ", partial" } else { "" };
    let credit = if credit_rates.is_enabled() {
        let formatted = model_credit_cost(model_usage, credit_rates)
            .map(|cost| format_credit_amount(cost, incomplete))
            .unwrap_or_else(|| "?Ƶ".to_string());
        format!(
            "{formatted} ({}), ",
            format_delta_credit(delta, credit_rates)
        )
    } else {
        String::new()
    };
    format!(
        "[{model}] {credit}{} {} in, {} {} cached, {} {} / {} {} out{partial}",
        format_with_separators(counts.non_cached),
        format_delta_count(delta_counts.map(|counts| counts.non_cached)),
        format_with_separators(counts.cached),
        format_delta_count(delta_counts.map(|counts| counts.cached)),
        format_with_separators(counts.output),
        format_delta_count(delta_counts.map(|counts| counts.output)),
        format_with_separators(counts.reasoning),
        format_delta_count(delta_counts.map(|counts| counts.reasoning)),
    )
}

fn display_token_counts(
    usage: &codex_app_server_protocol::TokenUsageBreakdown,
) -> DisplayTokenCounts {
    let cached = usage.cached_input_tokens.max(0);
    DisplayTokenCounts {
        non_cached: usage.input_tokens.saturating_sub(cached).max(0),
        cached,
        output: usage.output_tokens.max(0),
        reasoning: usage.reasoning_output_tokens.max(0),
    }
}

fn format_delta_count(value: Option<i64>) -> String {
    value
        .map(|value| format!("(+{})", format_with_separators(value)))
        .unwrap_or_else(|| "(+?)".to_string())
}

fn format_delta_credit(
    delta: Option<&ModelTokenUsageSnapshot>,
    credit_rates: &CreditRatesState,
) -> String {
    let Some(delta) = delta else {
        return "+0Ƶ".to_string();
    };
    model_credit_cost(delta, credit_rates)
        .map(|cost| format!("+{}", format_credit_amount(cost, delta.incomplete)))
        .unwrap_or_else(|| "+?Ƶ".to_string())
}

fn model_credit_cost(
    model_usage: &ModelTokenUsageSnapshot,
    credit_rates: &CreditRatesState,
) -> Option<CreditAmount> {
    let model = model_usage.model.as_deref()?;
    let usage = model_usage.usage.as_ref()?;
    let cached = usage.cached_input_tokens.max(0);
    let non_cached = usage.input_tokens.saturating_sub(cached).max(0);
    credit_rates.cost_for_model(
        model,
        u64::try_from(non_cached).ok()?,
        u64::try_from(cached).ok()?,
        u64::try_from(usage.output_tokens.max(0)).ok()?,
    )
}

fn format_credit_amount(amount: CreditAmount, incomplete: bool) -> String {
    let value = if amount.is_positive_below_one_cent() {
        "<0.01".to_string()
    } else {
        let hundredths = amount.rounded_hundredths();
        let whole = hundredths / 100;
        match hundredths % 100 {
            0 => whole.to_string(),
            fractional if fractional % 10 == 0 => format!("{whole}.{}", fractional / 10),
            fractional => format!("{whole}.{fractional:02}"),
        }
    };
    if incomplete {
        format!("{value}Ƶ+")
    } else {
        format!("{value}Ƶ")
    }
}
