//! Накопительные и интервальные токены одного видимого хода `ChatWidget`.
//!
//! Аккумулятор намеренно принимает только уведомления `rawResponse/completed` своего потока.
//! Создание снимка очищает только дельту разделителя; ответы других агентов остаются в итоговом
//! снимке корневого хода, который передаёт core.

use crate::token_usage::SeparatorTokenUsageSnapshot;

use codex_app_server_protocol::ModelTokenUsageSnapshot;
use codex_app_server_protocol::RawResponseCompletedNotification;
use codex_app_server_protocol::TokenUsageBreakdown;
use codex_app_server_protocol::TokenUsageSnapshot;
use codex_protocol::protocol::compare_model_slugs;
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Default)]
pub(super) struct SeparatorTokenUsage {
    turn_id: Option<String>,
    since_turn_start: HashMap<Option<String>, ModelAggregate>,
    since_separator: HashMap<Option<String>, ModelAggregate>,
    seen_responses: HashSet<String>,
    has_previous_separator: bool,
}

#[derive(Default)]
struct ModelAggregate {
    usage: Option<TokenUsageBreakdown>,
    incomplete: bool,
}

impl SeparatorTokenUsage {
    /// Открывает новый пользовательский ход и очищает обе области предыдущего хода.
    pub(super) fn start_turn(&mut self, turn_id: String) {
        self.turn_id = Some(turn_id);
        self.since_turn_start.clear();
        self.since_separator.clear();
        self.seen_responses.clear();
        self.has_previous_separator = false;
    }

    /// Учитывает ответ активного хода, если его ещё не доставляли этому аккумулятору.
    pub(super) fn record_response(&mut self, notification: &RawResponseCompletedNotification) {
        if self.turn_id.as_deref() != Some(notification.turn_id.as_str())
            || !self.seen_responses.insert(notification.response_id.clone())
        {
            return;
        }
        let model = (!notification.model.is_empty()).then(|| notification.model.clone());
        self.since_turn_start
            .entry(model.clone())
            .or_default()
            .record(notification.usage.as_ref());
        self.since_separator
            .entry(model)
            .or_default()
            .record(notification.usage.as_ref());
    }

    /// Замораживает итог хода и существующий интервал, затем отмечает новую границу.
    pub(super) fn take_snapshot(&mut self) -> Option<SeparatorTokenUsageSnapshot> {
        let has_previous_separator = std::mem::replace(&mut self.has_previous_separator, true);
        if self.since_separator.is_empty() {
            return None;
        }
        let since_separator = std::mem::take(&mut self.since_separator);
        Some(SeparatorTokenUsageSnapshot {
            cumulative: snapshot(&self.since_turn_start),
            delta: has_previous_separator.then(|| snapshot(&since_separator)),
        })
    }
}

/// Строит детерминированно отсортированный снимок текущих модельных агрегатов.
fn snapshot(by_model: &HashMap<Option<String>, ModelAggregate>) -> TokenUsageSnapshot {
    let mut models = by_model
        .iter()
        .map(|(model, aggregate)| ModelTokenUsageSnapshot {
            model: model.clone(),
            usage: aggregate.usage.clone(),
            incomplete: aggregate.incomplete,
        })
        .collect::<Vec<_>>();
    models.sort_by(|left, right| match (&left.model, &right.model) {
        (Some(left), Some(right)) => compare_model_slugs(left, right),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    TokenUsageSnapshot { models }
}

impl ModelAggregate {
    fn record(&mut self, usage: Option<&TokenUsageBreakdown>) {
        let Some(usage) = usage else {
            self.incomplete = true;
            return;
        };
        let usage = normalized_usage(usage);
        match &mut self.usage {
            Some(total) => saturating_add_usage(total, &usage),
            None => self.usage = Some(usage),
        }
    }
}

fn normalized_usage(usage: &TokenUsageBreakdown) -> TokenUsageBreakdown {
    TokenUsageBreakdown {
        total_tokens: usage.total_tokens.max(0),
        input_tokens: usage.input_tokens.max(0),
        cached_input_tokens: usage.cached_input_tokens.max(0),
        cache_write_input_tokens: usage.cache_write_input_tokens.max(0),
        output_tokens: usage.output_tokens.max(0),
        reasoning_output_tokens: usage.reasoning_output_tokens.max(0),
    }
}

fn saturating_add_usage(total: &mut TokenUsageBreakdown, usage: &TokenUsageBreakdown) {
    total.total_tokens = total.total_tokens.saturating_add(usage.total_tokens);
    total.input_tokens = total.input_tokens.saturating_add(usage.input_tokens);
    total.cached_input_tokens = total
        .cached_input_tokens
        .saturating_add(usage.cached_input_tokens);
    total.cache_write_input_tokens = total
        .cache_write_input_tokens
        .saturating_add(usage.cache_write_input_tokens);
    total.output_tokens = total.output_tokens.saturating_add(usage.output_tokens);
    total.reasoning_output_tokens = total
        .reasoning_output_tokens
        .saturating_add(usage.reasoning_output_tokens);
}

#[cfg(test)]
#[path = "token_usage_tests.rs"]
mod tests;
