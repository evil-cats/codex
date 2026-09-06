//! Регрессионные проверки накопительных и интервальных токенов видимого хода `ChatWidget`.

use super::*;
use codex_app_server_protocol::ModelTokenUsageSnapshot;
use codex_app_server_protocol::RawResponseCompletedNotification;
use codex_app_server_protocol::TokenUsageBreakdown;
use codex_app_server_protocol::TokenUsageSnapshot;
use codex_config::CreditRatesState;
use codex_config::parse_credit_rates;
use pretty_assertions::assert_eq;

use crate::token_usage::SeparatorTokenUsageSnapshot;

fn credit_rates() -> CreditRatesState {
    let loaded = parse_credit_rates(
        r#"
        {
          "schema_version": 1,
          "unit": "credits_per_million_tokens",
          "models": {
            "gpt-5.6-sol": { "input": 100, "cached_input": 10, "output": 500 },
            "gpt-5.6-luna": { "input": 5, "cached_input": 0.5, "output": 30 }
          }
        }
        "#,
    )
    .expect("valid test credit rates");
    assert_eq!(loaded.invalid_model_entries, Vec::<String>::new());
    CreditRatesState::Loaded(loaded.rates)
}

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
/// В оба снимка разделителя входят только уникальные завершения активного хода.
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

    let expected = TokenUsageSnapshot {
        models: vec![ModelTokenUsageSnapshot {
            model: Some("gpt-5.6-luna".to_string()),
            usage: Some(usage(100, 40, 20, 5)),
            incomplete: false,
        }],
    };
    assert_eq!(
        interval.take_snapshot(),
        Some(SeparatorTokenUsageSnapshot {
            cumulative: expected,
            delta: None,
        })
    );
    interval.record_response(&first);
    assert_eq!(interval.take_snapshot(), None);
}

#[test]
/// Разделитель очищает только дельту, а новый ход — накопление и дедупликацию.
fn separator_usage_keeps_turn_total_and_resets_at_the_next_turn() {
    let mut accumulator = SeparatorTokenUsage::default();
    accumulator.start_turn("turn-1".to_string());
    accumulator.record_response(&completed(
        "turn-1",
        "response-1",
        "gpt-5.6-sol",
        Some(usage(100, 40, 20, 5)),
    ));
    let first = accumulator.take_snapshot().expect("first divider snapshot");
    assert_eq!(first.delta, None);

    accumulator.record_response(&completed(
        "turn-1",
        "response-2",
        "gpt-5.6-sol",
        Some(usage(200, 100, 30, 10)),
    ));
    let second = accumulator
        .take_snapshot()
        .expect("second divider snapshot");
    assert_eq!(
        second,
        SeparatorTokenUsageSnapshot {
            cumulative: TokenUsageSnapshot {
                models: vec![ModelTokenUsageSnapshot {
                    model: Some("gpt-5.6-sol".to_string()),
                    usage: Some(usage(300, 140, 50, 15)),
                    incomplete: false,
                }],
            },
            delta: Some(TokenUsageSnapshot {
                models: vec![ModelTokenUsageSnapshot {
                    model: Some("gpt-5.6-sol".to_string()),
                    usage: Some(usage(200, 100, 30, 10)),
                    incomplete: false,
                }],
            }),
        }
    );

    accumulator.record_response(&completed("turn-1", "response-3", "gpt-5.6-sol", None));
    let unknown_delta = accumulator.take_snapshot().expect("unknown divider delta");
    assert_eq!(
        crate::token_usage::format_separator_token_usage_snapshot(&unknown_delta, &credit_rates(),),
        "[gpt-5.6-sol] 0.04Ƶ+ (+?Ƶ), 160 (+?) in, 140 (+?) cached, 35 (+?) / 50 (+?) out, partial"
    );

    accumulator.start_turn("turn-2".to_string());
    accumulator.record_response(&completed(
        "turn-2",
        "response-1",
        "gpt-5.6-luna",
        Some(usage(50, 20, 4, 1)),
    ));
    let next_turn = accumulator
        .take_snapshot()
        .expect("next turn divider snapshot");
    assert_eq!(next_turn.delta, None);
    assert_eq!(
        next_turn.cumulative,
        TokenUsageSnapshot {
            models: vec![ModelTokenUsageSnapshot {
                model: Some("gpt-5.6-luna".to_string()),
                usage: Some(usage(50, 20, 4, 1)),
                incomplete: false,
            }],
        }
    );
}

#[test]
/// Отсутствующий `usage` остаётся неизвестной дельтой и не превращается в ноль.
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
        crate::token_usage::format_separator_token_usage_snapshot(&snapshot, &credit_rates()),
        "[gpt-5.6-sol] 0.03Ƶ+, 120 in, 80 cached, 23 / 40 out, partial; [gpt-5.6-luna] ?Ƶ, unavailable"
    );
}

#[test]
/// Пустой первый разделитель всё равно создаёт границу для следующей дельты.
fn separator_without_usage_marks_the_next_interval_as_delta() {
    let mut accumulator = SeparatorTokenUsage::default();
    accumulator.start_turn("turn".to_string());
    assert_eq!(accumulator.take_snapshot(), None);

    accumulator.record_response(&completed(
        "turn",
        "response",
        "gpt-5.6-sol",
        Some(usage(100, 40, 20, 5)),
    ));
    let snapshot = accumulator.take_snapshot().expect("interval snapshot");
    assert_eq!(snapshot.delta, Some(snapshot.cumulative.clone()));
}

#[test]
/// Модели сортируются по числовой версии и известному уровню, а не по поступлению.
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
        crate::token_usage::format_token_usage_snapshot(&snapshot, &CreditRatesState::Disabled),
        "[gpt-6] 1 in, 0 cached, 1 / 1 out; [gpt-5.6-sol] 1 in, 0 cached, 1 / 1 out; [gpt-5.6-terra] 1 in, 0 cached, 1 / 1 out; [gpt-5.6-luna] 1 in, 0 cached, 1 / 1 out; [gpt-5.5] 1 in, 0 cached, 1 / 1 out; [other-model] 1 in, 0 cached, 1 / 1 out"
    );
}

#[test]
/// Кредитные строки сохраняют `partial` и неизвестные цены, не скрывая токены.
fn credit_formatting_marks_partial_unknown_and_subcent_costs() {
    let rates = credit_rates();
    let snapshot = TokenUsageSnapshot {
        models: vec![
            ModelTokenUsageSnapshot {
                model: Some("gpt-5.6-sol".to_string()),
                usage: Some(usage(200, 80, 40, 17)),
                incomplete: true,
            },
            ModelTokenUsageSnapshot {
                model: Some("gpt-5.6-luna".to_string()),
                usage: Some(usage(1, 0, 0, 0)),
                incomplete: false,
            },
            ModelTokenUsageSnapshot {
                model: Some("gpt-5.5".to_string()),
                usage: Some(usage(100, 0, 20, 0)),
                incomplete: false,
            },
        ],
    };

    assert_eq!(
        crate::token_usage::format_token_usage_snapshot(&snapshot, &rates),
        "[gpt-5.6-sol] 0.03Ƶ+, 120 in, 80 cached, 23 / 40 out, partial; [gpt-5.6-luna] <0.01Ƶ, 1 in, 0 cached, 0 / 0 out; [gpt-5.5] ?Ƶ, 100 in, 0 cached, 20 / 20 out"
    );
}

#[test]
/// `Total` складывает точные межмодельные цены до округления и не повторяет сумму одной модели.
fn total_credit_formatting_combines_only_multiple_models() {
    let rates = credit_rates();
    let sol = ModelTokenUsageSnapshot {
        model: Some("gpt-5.6-sol".to_string()),
        usage: Some(usage(467_322, 463_872, 2_573, 1_791)),
        incomplete: false,
    };
    let luna = ModelTokenUsageSnapshot {
        model: Some("gpt-5.6-luna".to_string()),
        usage: Some(usage(200_833, 156_672, 546, 138)),
        incomplete: false,
    };

    assert_eq!(
        crate::token_usage::format_total_token_usage_snapshot(
            &TokenUsageSnapshot {
                models: vec![sol.clone(), luna.clone()],
            },
            &rates,
        ),
        "6.59Ƶ; [gpt-5.6-sol] 6.27Ƶ, 3,450 in, 463,872 cached, 782 / 2,573 out; [gpt-5.6-luna] 0.32Ƶ, 44,161 in, 156,672 cached, 408 / 546 out"
    );
    let mut partial_luna = luna;
    partial_luna.incomplete = true;
    assert_eq!(
        crate::token_usage::format_total_token_usage_snapshot(
            &TokenUsageSnapshot {
                models: vec![sol.clone(), partial_luna],
            },
            &rates,
        ),
        "6.59Ƶ+; [gpt-5.6-sol] 6.27Ƶ, 3,450 in, 463,872 cached, 782 / 2,573 out; [gpt-5.6-luna] 0.32Ƶ+, 44,161 in, 156,672 cached, 408 / 546 out, partial"
    );
    assert_eq!(
        crate::token_usage::format_total_token_usage_snapshot(
            &TokenUsageSnapshot {
                models: vec![
                    sol.clone(),
                    ModelTokenUsageSnapshot {
                        model: Some("gpt-5.5".to_string()),
                        usage: Some(usage(100, 0, 20, 0)),
                        incomplete: false,
                    },
                ],
            },
            &rates,
        ),
        "?Ƶ; [gpt-5.6-sol] 6.27Ƶ, 3,450 in, 463,872 cached, 782 / 2,573 out; [gpt-5.5] ?Ƶ, 100 in, 0 cached, 20 / 20 out"
    );
    assert_eq!(
        crate::token_usage::format_total_token_usage_snapshot(
            &TokenUsageSnapshot { models: vec![sol] },
            &rates,
        ),
        "[gpt-5.6-sol] 6.27Ƶ, 3,450 in, 463,872 cached, 782 / 2,573 out"
    );
}

#[test]
/// Недоступная настроенная таблица показывает неизвестную оценку, не выключая кредиты.
fn unavailable_credit_table_keeps_enabled_unknown_marker() {
    let snapshot = TokenUsageSnapshot {
        models: vec![ModelTokenUsageSnapshot {
            model: Some("gpt-5.6-sol".to_string()),
            usage: Some(usage(10, 0, 1, 0)),
            incomplete: false,
        }],
    };

    assert_eq!(
        crate::token_usage::format_token_usage_snapshot(&snapshot, &CreditRatesState::Unavailable),
        "[gpt-5.6-sol] ?Ƶ, 10 in, 0 cached, 1 / 1 out"
    );
}
