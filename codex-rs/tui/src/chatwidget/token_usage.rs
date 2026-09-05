//! Per-separator token interval for one visible `ChatWidget` turn.
//!
//! This accumulator deliberately sees only `rawResponse/completed` notifications routed to its
//! own thread. Taking a snapshot is the divider boundary; unrelated agent responses remain in the
//! root-turn snapshot supplied by core.

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
    by_model: HashMap<Option<String>, ModelAggregate>,
    seen_responses: HashSet<String>,
}

#[derive(Default)]
struct ModelAggregate {
    usage: Option<TokenUsageBreakdown>,
    incomplete: bool,
}

impl SeparatorTokenUsage {
    /// Starts a fresh user-visible turn and discards any interval left by the previous one.
    pub(super) fn start_turn(&mut self, turn_id: String) {
        self.turn_id = Some(turn_id);
        self.by_model.clear();
        self.seen_responses.clear();
    }

    /// Adds one response when it belongs to the active turn and has not already been delivered.
    pub(super) fn record_response(&mut self, notification: &RawResponseCompletedNotification) {
        if self.turn_id.as_deref() != Some(notification.turn_id.as_str())
            || !self.seen_responses.insert(notification.response_id.clone())
        {
            return;
        }
        let model = (!notification.model.is_empty()).then(|| notification.model.clone());
        self.by_model
            .entry(model)
            .or_default()
            .record(notification.usage.as_ref());
    }

    /// Freezes and clears exactly the interval since the preceding emitted divider.
    pub(super) fn take_snapshot(&mut self) -> Option<TokenUsageSnapshot> {
        if self.by_model.is_empty() {
            return None;
        }
        let mut models = std::mem::take(&mut self.by_model)
            .into_iter()
            .map(|(model, aggregate)| ModelTokenUsageSnapshot {
                model,
                usage: aggregate.usage,
                incomplete: aggregate.incomplete,
            })
            .collect::<Vec<_>>();
        models.sort_by(|left, right| match (&left.model, &right.model) {
            (Some(left), Some(right)) => compare_model_slugs(left, right),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });
        Some(TokenUsageSnapshot { models })
    }
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
