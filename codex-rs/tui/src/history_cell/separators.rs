//! Turn separators and runtime-metrics labels for transcript history.

use super::*;
use codex_app_server_protocol::RootTurnTokenUsageSnapshot;
use codex_config::CreditRatesState;

use crate::token_usage::SeparatorTokenUsageSnapshot;

#[derive(Debug)]
/// A visual divider between turns, optionally showing how long the assistant "worked for".
///
/// This separator is only emitted for turns that performed concrete work (e.g., running commands,
/// applying patches, making MCP tool calls), so purely conversational turns do not show an empty
/// divider.
pub struct FinalMessageSeparator {
    elapsed_seconds: Option<u64>,
    runtime_metrics: Option<RuntimeMetricsSummary>,
    interval_token_usage: Option<SeparatorTokenUsageSnapshot>,
    root_turn_token_usage: Option<RootTurnTokenUsageSnapshot>,
    credit_rates: CreditRatesState,
}
impl FinalMessageSeparator {
    /// Creates a separator; completed turns should pass protocol turn duration when available.
    pub(crate) fn new(
        elapsed_seconds: Option<u64>,
        runtime_metrics: Option<RuntimeMetricsSummary>,
    ) -> Self {
        Self {
            elapsed_seconds,
            runtime_metrics,
            interval_token_usage: None,
            root_turn_token_usage: None,
            credit_rates: CreditRatesState::Disabled,
        }
    }

    /// Присоединяет замороженные итог хода и дельту ответов обычного разделителя.
    pub(crate) fn with_interval_token_usage(
        mut self,
        token_usage: SeparatorTokenUsageSnapshot,
    ) -> Self {
        self.interval_token_usage = Some(token_usage);
        self
    }

    /// Присоединяет замороженный отчёт корневого хода для финального разделителя.
    pub(crate) fn with_root_turn_token_usage(
        mut self,
        token_usage: RootTurnTokenUsageSnapshot,
    ) -> Self {
        self.root_turn_token_usage = Some(token_usage);
        self
    }

    /// Закрепляет загруженные при старте тарифы рядом со снимками этой ячейки.
    pub(crate) fn with_credit_rates(mut self, credit_rates: CreditRatesState) -> Self {
        self.credit_rates = credit_rates;
        self
    }

    /// Собирает метки по порядку и переносит `Total` без агентов в ту же строку.
    fn label_parts(&self) -> Vec<String> {
        let mut label_parts = Vec::new();
        if let Some(token_usage) = &self.interval_token_usage {
            label_parts.push(format!(
                "Tokens: {}",
                crate::token_usage::format_separator_token_usage_snapshot(
                    token_usage,
                    &self.credit_rates,
                )
            ));
        }
        if let Some(elapsed_seconds) = self
            .elapsed_seconds
            .filter(|seconds| *seconds > 60)
            .map(crate::status_indicator_widget::fmt_elapsed_compact)
        {
            label_parts.push(format!("Worked for {elapsed_seconds}"));
        }
        if let Some(snapshot) = self
            .root_turn_token_usage
            .as_ref()
            .filter(|snapshot| snapshot.agent_count == 0)
        {
            label_parts.push(format!(
                "Total: {}",
                crate::token_usage::format_total_token_usage_snapshot(
                    &snapshot.total,
                    &self.credit_rates,
                )
            ));
        }
        if let Some(metrics_label) = self.runtime_metrics.and_then(runtime_metrics_label) {
            label_parts.push(metrics_label);
        }
        label_parts
    }
}
impl HistoryCell for FinalMessageSeparator {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let label_parts = self.label_parts();

        let mut lines = self
            .root_turn_token_usage
            .as_ref()
            .map(|snapshot| root_turn_token_usage_lines(snapshot, &self.credit_rates))
            .unwrap_or_default()
            .into_iter()
            .map(|line| {
                let (line, _suffix, _line_width) = take_prefix_by_width(&line, width as usize);
                Line::from(line).dim()
            })
            .collect::<Vec<_>>();

        if label_parts.is_empty() {
            lines.push(Line::from_iter(["─".repeat(width as usize).dim()]));
        } else {
            let label = format!("─ {} ─", label_parts.join(" • "));
            let (label, _suffix, label_width) = take_prefix_by_width(&label, width as usize);
            lines.push(
                Line::from_iter([
                    label,
                    "─".repeat((width as usize).saturating_sub(label_width)),
                ])
                .dim(),
            );
        }
        lines
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        let label_parts = self.label_parts();
        let mut lines = self
            .root_turn_token_usage
            .as_ref()
            .map(|snapshot| root_turn_token_usage_lines(snapshot, &self.credit_rates))
            .unwrap_or_default()
            .into_iter()
            .map(Line::from)
            .collect::<Vec<_>>();
        if !label_parts.is_empty() {
            lines.push(Line::from(label_parts.join(" • ")));
        }
        lines
    }
}

fn root_turn_token_usage_lines(
    snapshot: &RootTurnTokenUsageSnapshot,
    credit_rates: &CreditRatesState,
) -> Vec<String> {
    let running = running_agents_label(snapshot.running_agent_count);
    if snapshot.agent_count == 0 {
        return Vec::new();
    }
    vec![
        format!(
            "  Turn:   {}",
            crate::token_usage::format_token_usage_snapshot(&snapshot.turn, credit_rates)
        ),
        format!(
            "  Agents: {}{running}",
            crate::token_usage::format_token_usage_snapshot(&snapshot.agents, credit_rates)
        ),
        format!(
            "  Total:  {}{running}",
            crate::token_usage::format_total_token_usage_snapshot(&snapshot.total, credit_rates)
        ),
    ]
}

fn running_agents_label(count: u64) -> String {
    match count {
        0 => String::new(),
        1 => " · 1 agent running".to_string(),
        count => format!(" · {count} agents running"),
    }
}

pub(crate) fn runtime_metrics_label(summary: RuntimeMetricsSummary) -> Option<String> {
    let mut parts = Vec::new();
    if summary.tool_calls.count > 0 {
        let duration = format_duration_ms(summary.tool_calls.duration_ms);
        let calls = pluralize(summary.tool_calls.count, "call", "calls");
        parts.push(format!(
            "Local tools: {} {calls} ({duration})",
            summary.tool_calls.count
        ));
    }
    if summary.api_calls.count > 0 {
        let duration = format_duration_ms(summary.api_calls.duration_ms);
        let calls = pluralize(summary.api_calls.count, "call", "calls");
        parts.push(format!(
            "Inference: {} {calls} ({duration})",
            summary.api_calls.count
        ));
    }
    if summary.websocket_calls.count > 0 {
        let duration = format_duration_ms(summary.websocket_calls.duration_ms);
        parts.push(format!(
            "WebSocket: {} events send ({duration})",
            summary.websocket_calls.count
        ));
    }
    if summary.streaming_events.count > 0 {
        let duration = format_duration_ms(summary.streaming_events.duration_ms);
        let stream_label = pluralize(summary.streaming_events.count, "Stream", "Streams");
        let events = pluralize(summary.streaming_events.count, "event", "events");
        parts.push(format!(
            "{stream_label}: {} {events} ({duration})",
            summary.streaming_events.count
        ));
    }
    if summary.websocket_events.count > 0 {
        let duration = format_duration_ms(summary.websocket_events.duration_ms);
        parts.push(format!(
            "{} events received ({duration})",
            summary.websocket_events.count
        ));
    }
    if summary.responses_api_overhead_ms > 0 {
        let duration = format_duration_ms(summary.responses_api_overhead_ms);
        parts.push(format!("Responses API overhead: {duration}"));
    }
    if summary.responses_api_inference_time_ms > 0 {
        let duration = format_duration_ms(summary.responses_api_inference_time_ms);
        parts.push(format!("Responses API inference: {duration}"));
    }
    if summary.responses_api_engine_iapi_ttft_ms > 0
        || summary.responses_api_engine_service_ttft_ms > 0
    {
        let mut ttft_parts = Vec::new();
        if summary.responses_api_engine_iapi_ttft_ms > 0 {
            let duration = format_duration_ms(summary.responses_api_engine_iapi_ttft_ms);
            ttft_parts.push(format!("{duration} (iapi)"));
        }
        if summary.responses_api_engine_service_ttft_ms > 0 {
            let duration = format_duration_ms(summary.responses_api_engine_service_ttft_ms);
            ttft_parts.push(format!("{duration} (service)"));
        }
        parts.push(format!("TTFT: {}", ttft_parts.join(" ")));
    }
    if summary.responses_api_engine_iapi_tbt_ms > 0.0
        || summary.responses_api_engine_service_tbt_ms > 0.0
    {
        let mut tbt_parts = Vec::new();
        if summary.responses_api_engine_iapi_tbt_ms > 0.0 {
            let duration =
                format_duration_ms(summary.responses_api_engine_iapi_tbt_ms.round() as u64);
            tbt_parts.push(format!("{duration} (iapi)"));
        }
        if summary.responses_api_engine_service_tbt_ms > 0.0 {
            let duration =
                format_duration_ms(summary.responses_api_engine_service_tbt_ms.round() as u64);
            tbt_parts.push(format!("{duration} (service)"));
        }
        parts.push(format!("TBT: {}", tbt_parts.join(" ")));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" • "))
    }
}

fn format_duration_ms(duration_ms: u64) -> String {
    if duration_ms >= 1_000 {
        let seconds = duration_ms as f64 / 1_000.0;
        format!("{seconds:.1}s")
    } else {
        format!("{duration_ms}ms")
    }
}

fn pluralize(count: u64, singular: &'static str, plural: &'static str) -> &'static str {
    if count == 1 { singular } else { plural }
}
