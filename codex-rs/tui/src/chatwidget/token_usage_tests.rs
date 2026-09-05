//! Regression coverage for the token interval owned by one visible `ChatWidget` turn.

use super::*;
use codex_app_server_protocol::ModelTokenUsageSnapshot;
use codex_app_server_protocol::RawResponseCompletedNotification;
use codex_app_server_protocol::TokenUsageBreakdown;
use codex_app_server_protocol::TokenUsageSnapshot;
use pretty_assertions::assert_eq;

fn usage(input: i64, cached: i64, output: i64, reasoning: i64) -> TokenUsageBreakdown {
    TokenUsageBreakdown {
        total_tokens: input.saturating_add(output),
        input_tokens: input,
        cached_input_tokens: cached,
        cache_write_input_tokens: 0,
        output_tokens: output,
        reasoning_output_tokens: reasoning,
    }
}

fn completed(
    turn_id: &str,
    response_id: &str,
    model: &str,
    usage: Option<TokenUsageBreakdown>,
) -> RawResponseCompletedNotification {
    RawResponseCompletedNotification {
        thread_id: "thread".to_string(),
        turn_id: turn_id.to_string(),
        response_id: response_id.to_string(),
        model: model.to_string(),
        usage,
        usage_metadata: None,
    }
}

#[test]
fn separator_interval_accepts_only_the_active_turn_and_deduplicates_responses() {
    let mut interval = SeparatorTokenUsage::default();
    interval.start_turn("turn-1".to_string());
    let first = completed(
        "turn-1",
        "response-1",
        "gpt-5.6-luna",
        Some(usage(100, 40, 20, 5)),
    );
    interval.record_response(&first);
    interval.record_response(&first);
    interval.record_response(&completed(
        "turn-2",
        "wrong-turn",
        "gpt-6",
        Some(usage(999, 0, 999, 0)),
    ));

    assert_eq!(
        interval.take_snapshot(),
        Some(TokenUsageSnapshot {
            models: vec![ModelTokenUsageSnapshot {
                model: Some("gpt-5.6-luna".to_string()),
                usage: Some(usage(100, 40, 20, 5)),
                incomplete: false,
            }],
        })
    );
    interval.record_response(&first);
    assert_eq!(interval.take_snapshot(), None);
}

#[test]
fn separator_interval_preserves_missing_usage_as_partial_or_unavailable() {
    let mut interval = SeparatorTokenUsage::default();
    interval.start_turn("turn".to_string());
    interval.record_response(&completed(
        "turn",
        "known",
        "gpt-5.6-sol",
        Some(usage(200, 80, 40, 17)),
    ));
    interval.record_response(&completed(
        "turn",
        "missing-same-model",
        "gpt-5.6-sol",
        None,
    ));
    interval.record_response(&completed(
        "turn",
        "missing-other-model",
        "gpt-5.6-luna",
        None,
    ));

    let snapshot = interval.take_snapshot().expect("interval snapshot");
    assert_eq!(
        crate::token_usage::format_token_usage_snapshot(&snapshot),
        "[gpt-5.6-sol] 120 in, 80 cached, 40 (17) out, partial; [gpt-5.6-luna] unavailable"
    );
}

#[test]
fn model_sorting_uses_numeric_version_then_known_tier() {
    let snapshot = TokenUsageSnapshot {
        models: [
            "other-model",
            "gpt-5.6-luna",
            "gpt-5.5",
            "gpt-5.6-terra",
            "gpt-6",
            "gpt-5.6-sol",
        ]
        .into_iter()
        .map(|model| ModelTokenUsageSnapshot {
            model: Some(model.to_string()),
            usage: Some(usage(1, 0, 1, 0)),
            incomplete: false,
        })
        .collect(),
    };

    assert_eq!(
        crate::token_usage::format_token_usage_snapshot(&snapshot),
        "[gpt-6] 1 in, 0 cached, 1 (0) out; [gpt-5.6-sol] 1 in, 0 cached, 1 (0) out; [gpt-5.6-terra] 1 in, 0 cached, 1 (0) out; [gpt-5.6-luna] 1 in, 0 cached, 1 (0) out; [gpt-5.5] 1 in, 0 cached, 1 (0) out; [other-model] 1 in, 0 cached, 1 (0) out"
    );
}
