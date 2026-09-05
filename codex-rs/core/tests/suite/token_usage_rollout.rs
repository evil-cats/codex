//! Проверяет устойчивую запись наблюдаемого Responses API usage в историю rollout.

use anyhow::Result;
use codex_config::CreditRatesState;
use codex_config::parse_credit_rates;
use codex_history::RolloutItem;
use codex_history::RolloutLine;
use codex_protocol::SessionId;
use codex_protocol::protocol::TokenUsageRecord;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed_with_tokens;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use pretty_assertions::assert_eq;
use serde_json::json;

fn token_usage_records(path: &std::path::Path) -> Vec<TokenUsageRecord> {
    std::fs::read_to_string(path)
        .expect("read rollout")
        .lines()
        .filter_map(|line| serde_json::from_str::<RolloutLine>(line).ok())
        .filter_map(|line| match line.item {
            RolloutItem::TokenUsageRecord(record) => Some(record),
            _ => None,
        })
        .collect()
}

fn credit_rates(input_rate: u64) -> CreditRatesState {
    let loaded = parse_credit_rates(&format!(
        r#"{{
          "schema_version": 1,
          "unit": "credits_per_million_tokens",
          "models": {{
            "gpt-5.6-sol": {{
              "input": {input_rate},
              "cached_input": 0,
              "output": 0
            }}
          }}
        }}"#
    ))
    .expect("valid credit rates");
    assert_eq!(loaded.invalid_model_entries, Vec::<String>::new());
    CreditRatesState::Loaded(loaded.rates)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn observed_response_usage_accumulates_per_turn_and_thread() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let plan_args = json!({
        "plan": [{
            "step": "keep sampling",
            "status": "in_progress"
        }]
    })
    .to_string();
    mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("response-a"),
                ev_function_call("call-a", "update_plan", &plan_args),
                ev_completed_with_tokens("response-a", /*total_tokens*/ 120),
            ]),
            sse(vec![
                ev_response_created("response-b"),
                ev_assistant_message("message-b", "done"),
                ev_completed_with_tokens("response-b", /*total_tokens*/ 80),
            ]),
            sse(vec![
                ev_response_created("response-c"),
                ev_assistant_message("message-c", "next"),
                ev_completed_with_tokens("response-c", /*total_tokens*/ 30),
            ]),
            sse(vec![
                ev_response_created("response-without-usage"),
                ev_assistant_message("message-d", "no usage"),
                json!({
                    "type": "response.completed",
                    "response": {
                        "id": "response-without-usage"
                    }
                }),
            ]),
        ],
    )
    .await;
    let test = test_codex()
        .with_model("gpt-5.6-sol")
        .with_config(|config| config.credit_rates = credit_rates(100))
        .build_with_auto_env(&server)
        .await?;
    let rollout_path = test.codex.rollout_path().expect("rollout path");
    let home = test.home.clone();

    test.submit_turn("first").await?;
    test.codex.shutdown_and_wait().await?;

    let resumed = test_codex()
        .with_model("gpt-5.6-sol")
        .with_config(|config| config.credit_rates = credit_rates(200))
        .resume(&server, home, rollout_path.clone())
        .await?;
    for prompt in ["second", "third"] {
        resumed.submit_turn(prompt).await?;
    }
    resumed.codex.shutdown_and_wait().await?;

    let records = token_usage_records(&rollout_path);
    assert_eq!(records.len(), 3);
    assert_eq!(
        records
            .iter()
            .map(|record| {
                (
                    record.response_id.as_str(),
                    record.model.as_deref(),
                    record.credit_cost_picocredits.as_deref(),
                    record.turn_token_usage.total_tokens,
                    record.thread_token_usage.total_tokens,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (
                "response-a",
                Some("gpt-5.6-sol"),
                Some("12000000000"),
                120,
                120,
            ),
            (
                "response-b",
                Some("gpt-5.6-sol"),
                Some("8000000000"),
                200,
                200,
            ),
            (
                "response-c",
                Some("gpt-5.6-sol"),
                Some("6000000000"),
                30,
                230,
            ),
        ]
    );
    assert_eq!(records[0].turn_id, records[1].turn_id);
    assert_ne!(records[1].turn_id, records[2].turn_id);
    assert!(records.iter().all(|record| {
        record.session_id == SessionId::from(record.thread_id)
            && record.root_turn_id == record.turn_id
    }));

    Ok(())
}
