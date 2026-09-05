//! Regression coverage for root-turn token accounting across an agent tree.

use super::*;
use codex_protocol::AgentPath;
use codex_protocol::ThreadId;
use codex_protocol::protocol::ModelTokenUsageSnapshot;
use codex_protocol::protocol::TokenUsage;
use pretty_assertions::assert_eq;

fn thread_id(suffix: u128) -> ThreadId {
    ThreadId::from_string(&format!("00000000-0000-0000-0000-{suffix:012x}"))
        .expect("test thread ID")
}

fn usage(input: i64, cached: i64, output: i64, reasoning: i64) -> TokenUsage {
    TokenUsage {
        input_tokens: input,
        cached_input_tokens: cached,
        cache_write_input_tokens: 0,
        output_tokens: output,
        reasoning_output_tokens: reasoning,
        total_tokens: input.saturating_add(output),
        codex_rollout_budget_units: None,
    }
}

fn model_usage(
    model: &str,
    usage: Option<TokenUsage>,
    incomplete: bool,
) -> ModelTokenUsageSnapshot {
    ModelTokenUsageSnapshot {
        model: Some(model.to_string()),
        usage,
        incomplete,
    }
}

#[test]
fn root_snapshot_deduplicates_responses_and_keeps_models_separate() {
    let root_thread_id = thread_id(1);
    let mut ledger = RootTurnUsageLedger::default();

    ledger.begin_model_call(
        "root-turn",
        root_thread_id,
        root_thread_id,
        "root-turn",
        "gpt-5.5",
    );
    ledger.record_response(
        "root-turn",
        root_thread_id,
        root_thread_id,
        "root-turn",
        "response-1",
        "gpt-5.5",
        Some(&usage(100, 20, 30, 10)),
    );
    ledger.record_response(
        "root-turn",
        root_thread_id,
        root_thread_id,
        "root-turn",
        "response-1",
        "gpt-5.5",
        Some(&usage(100, 20, 30, 10)),
    );
    ledger.begin_model_call(
        "root-turn",
        root_thread_id,
        root_thread_id,
        "root-turn",
        "gpt-5.6-sol",
    );
    ledger.record_response(
        "root-turn",
        root_thread_id,
        root_thread_id,
        "root-turn",
        "response-2",
        "gpt-5.6-sol",
        Some(&usage(200, 80, 50, 25)),
    );

    let snapshot = ledger.complete_root_turn("root-turn", root_thread_id, "root-turn");

    assert_eq!(
        snapshot.turn.models,
        vec![
            model_usage("gpt-5.6-sol", Some(usage(200, 80, 50, 25)), false),
            model_usage("gpt-5.5", Some(usage(100, 20, 30, 10)), false),
        ]
    );
    assert_eq!(snapshot.total, snapshot.turn);
    assert_eq!(snapshot.agent_count, 0);
    assert_eq!(snapshot.running_agent_count, 0);
}

#[test]
fn repeated_agent_turns_accumulate_within_one_root_turn_and_reset_in_the_next() {
    let root_thread_id = thread_id(1);
    let agent_thread_id = thread_id(2);
    let agent_path = AgentPath::root().join("worker").expect("agent path");
    let mut ledger = RootTurnUsageLedger::default();

    ledger.register_agent(
        "root-turn-1",
        root_thread_id,
        agent_thread_id,
        agent_path.clone(),
        Some("gpt-5.6-luna".to_string()),
    );
    ledger.begin_model_call(
        "root-turn-1",
        root_thread_id,
        agent_thread_id,
        "agent-turn-1",
        "gpt-5.6-luna",
    );
    ledger.record_response(
        "root-turn-1",
        root_thread_id,
        agent_thread_id,
        "agent-turn-1",
        "agent-response-1",
        "gpt-5.6-luna",
        Some(&usage(100, 40, 20, 5)),
    );
    let first = ledger
        .complete_agent_turn(
            "root-turn-1",
            root_thread_id,
            agent_thread_id,
            agent_path.clone(),
            "agent-turn-1",
            Some("gpt-5.6-luna".to_string()),
        )
        .expect("first terminal snapshot");
    assert_eq!(
        first.models,
        vec![model_usage(
            "gpt-5.6-luna",
            Some(usage(100, 40, 20, 5)),
            false,
        )]
    );

    ledger.register_agent(
        "root-turn-1",
        root_thread_id,
        agent_thread_id,
        agent_path.clone(),
        Some("gpt-5.6-luna".to_string()),
    );
    ledger.begin_model_call(
        "root-turn-1",
        root_thread_id,
        agent_thread_id,
        "agent-turn-2",
        "gpt-5.6-luna",
    );
    ledger.record_response(
        "root-turn-1",
        root_thread_id,
        agent_thread_id,
        "agent-turn-2",
        "agent-response-2",
        "gpt-5.6-luna",
        Some(&usage(60, 10, 15, 4)),
    );
    let cumulative = ledger
        .complete_agent_turn(
            "root-turn-1",
            root_thread_id,
            agent_thread_id,
            agent_path.clone(),
            "agent-turn-2",
            Some("gpt-5.6-luna".to_string()),
        )
        .expect("second terminal snapshot");
    assert_eq!(
        cumulative.models,
        vec![model_usage(
            "gpt-5.6-luna",
            Some(usage(160, 50, 35, 9)),
            false,
        )]
    );
    assert!(
        ledger
            .complete_agent_turn(
                "root-turn-1",
                root_thread_id,
                agent_thread_id,
                agent_path.clone(),
                "agent-turn-2",
                Some("gpt-5.6-luna".to_string()),
            )
            .is_none(),
        "one child turn must publish only one terminal snapshot"
    );

    ledger.register_agent(
        "root-turn-2",
        root_thread_id,
        agent_thread_id,
        agent_path.clone(),
        Some("gpt-5.6-luna".to_string()),
    );
    let reset = ledger
        .complete_agent_turn(
            "root-turn-2",
            root_thread_id,
            agent_thread_id,
            agent_path,
            "agent-turn-3",
            Some("gpt-5.6-luna".to_string()),
        )
        .expect("new-root terminal snapshot");
    assert_eq!(reset.models, vec![model_usage("gpt-5.6-luna", None, true)]);
}

#[test]
fn nested_agents_fold_direct_usage_once_per_model() {
    let root_thread_id = thread_id(1);
    let parent_agent_id = thread_id(2);
    let child_agent_id = thread_id(3);
    let mut ledger = RootTurnUsageLedger::default();

    for (thread_id, path, response_id, token_usage) in [
        (
            parent_agent_id,
            "/root/parent",
            "parent-response",
            usage(80, 20, 10, 4),
        ),
        (
            child_agent_id,
            "/root/parent/child",
            "child-response",
            usage(120, 50, 30, 12),
        ),
    ] {
        let path = AgentPath::try_from(path).expect("agent path");
        ledger.register_agent(
            "root-turn",
            root_thread_id,
            thread_id,
            path.clone(),
            Some("gpt-5.6-terra".to_string()),
        );
        ledger.begin_model_call(
            "root-turn",
            root_thread_id,
            thread_id,
            response_id,
            "gpt-5.6-terra",
        );
        ledger.record_response(
            "root-turn",
            root_thread_id,
            thread_id,
            response_id,
            response_id,
            "gpt-5.6-terra",
            Some(&token_usage),
        );
        ledger.complete_agent_turn(
            "root-turn",
            root_thread_id,
            thread_id,
            path,
            response_id,
            Some("gpt-5.6-terra".to_string()),
        );
    }

    let snapshot = ledger.complete_root_turn("root-turn", root_thread_id, "root-turn");

    assert_eq!(snapshot.agent_count, 2);
    assert_eq!(
        snapshot.agents.models,
        vec![model_usage(
            "gpt-5.6-terra",
            Some(usage(200, 70, 40, 16)),
            false,
        )]
    );
    assert_eq!(snapshot.total, snapshot.agents);
}

#[test]
fn unfinished_model_call_is_unavailable_and_keeps_root_snapshot_immutable() {
    let root_thread_id = thread_id(1);
    let agent_thread_id = thread_id(2);
    let agent_path = AgentPath::root().join("worker").expect("agent path");
    let mut ledger = RootTurnUsageLedger::default();

    ledger.register_agent(
        "root-turn",
        root_thread_id,
        agent_thread_id,
        agent_path.clone(),
        Some("gpt-5.6-luna".to_string()),
    );
    ledger.begin_model_call(
        "root-turn",
        root_thread_id,
        agent_thread_id,
        "agent-turn",
        "gpt-5.6-luna",
    );

    let root_snapshot = ledger.complete_root_turn("root-turn", root_thread_id, "root-turn");
    assert_eq!(root_snapshot.running_agent_count, 1);
    assert_eq!(
        root_snapshot.agents.models,
        vec![model_usage("gpt-5.6-luna", None, true)]
    );

    let terminal_snapshot = ledger
        .complete_agent_turn(
            "root-turn",
            root_thread_id,
            agent_thread_id,
            agent_path,
            "agent-turn",
            Some("gpt-5.6-luna".to_string()),
        )
        .expect("late terminal snapshot");
    assert_eq!(
        terminal_snapshot.models,
        vec![model_usage("gpt-5.6-luna", None, true)]
    );
    assert_eq!(root_snapshot.running_agent_count, 1);
}

#[test]
fn pending_followup_marks_existing_agent_model_partial_in_root_snapshot() {
    let root_thread_id = thread_id(1);
    let agent_thread_id = thread_id(2);
    let agent_path = AgentPath::root().join("worker").expect("agent path");
    let mut ledger = RootTurnUsageLedger::default();

    ledger.register_agent(
        "root-turn",
        root_thread_id,
        agent_thread_id,
        agent_path.clone(),
        Some("gpt-5.6-luna".to_string()),
    );
    ledger.begin_model_call(
        "root-turn",
        root_thread_id,
        agent_thread_id,
        "agent-turn-1",
        "gpt-5.6-luna",
    );
    ledger.record_response(
        "root-turn",
        root_thread_id,
        agent_thread_id,
        "agent-turn-1",
        "agent-response-1",
        "gpt-5.6-luna",
        Some(&usage(100, 40, 20, 5)),
    );
    ledger
        .complete_agent_turn(
            "root-turn",
            root_thread_id,
            agent_thread_id,
            agent_path.clone(),
            "agent-turn-1",
            Some("gpt-5.6-luna".to_string()),
        )
        .expect("first terminal snapshot");

    ledger.register_agent(
        "root-turn",
        root_thread_id,
        agent_thread_id,
        agent_path.clone(),
        Some("gpt-5.6-luna".to_string()),
    );

    let snapshot = ledger.complete_root_turn("root-turn", root_thread_id, "root-turn");
    let expected = vec![model_usage(
        "gpt-5.6-luna",
        Some(usage(100, 40, 20, 5)),
        true,
    )];
    assert_eq!(snapshot.agents.models, expected);
    assert_eq!(snapshot.total.models, expected);
    assert_eq!(snapshot.running_agent_count, 1);

    let terminal = ledger
        .complete_agent_turn(
            "root-turn",
            root_thread_id,
            agent_thread_id,
            agent_path,
            "agent-turn-2",
            Some("gpt-5.6-luna".to_string()),
        )
        .expect("pending followup terminal snapshot");
    assert_eq!(terminal.models, expected);
}

#[test]
fn running_agent_between_model_calls_marks_observed_usage_partial() {
    let root_thread_id = thread_id(1);
    let agent_thread_id = thread_id(2);
    let agent_path = AgentPath::root().join("worker").expect("agent path");
    let mut ledger = RootTurnUsageLedger::default();

    ledger.register_agent(
        "root-turn",
        root_thread_id,
        agent_thread_id,
        agent_path,
        Some("gpt-5.6-luna".to_string()),
    );
    ledger.begin_model_call(
        "root-turn",
        root_thread_id,
        agent_thread_id,
        "agent-turn",
        "gpt-5.6-luna",
    );
    ledger.record_response(
        "root-turn",
        root_thread_id,
        agent_thread_id,
        "agent-turn",
        "agent-response",
        "gpt-5.6-luna",
        Some(&usage(100, 40, 20, 5)),
    );

    let snapshot = ledger.complete_root_turn("root-turn", root_thread_id, "root-turn");

    assert_eq!(
        snapshot.agents.models,
        vec![model_usage(
            "gpt-5.6-luna",
            Some(usage(100, 40, 20, 5)),
            true,
        )]
    );
    assert_eq!(snapshot.running_agent_count, 1);
}

#[test]
fn late_registration_does_not_reopen_an_already_terminal_agent_run() {
    let root_thread_id = thread_id(1);
    let agent_thread_id = thread_id(2);
    let agent_path = AgentPath::root().join("worker").expect("agent path");
    let mut ledger = RootTurnUsageLedger::default();

    ledger.begin_model_call(
        "root-turn",
        root_thread_id,
        agent_thread_id,
        "agent-turn",
        "gpt-5.6-luna",
    );
    ledger.record_response(
        "root-turn",
        root_thread_id,
        agent_thread_id,
        "agent-turn",
        "agent-response",
        "gpt-5.6-luna",
        Some(&usage(100, 40, 20, 5)),
    );
    ledger
        .complete_agent_turn(
            "root-turn",
            root_thread_id,
            agent_thread_id,
            agent_path.clone(),
            "agent-turn",
            Some("gpt-5.6-luna".to_string()),
        )
        .expect("terminal snapshot");

    ledger.register_agent(
        "root-turn",
        root_thread_id,
        agent_thread_id,
        agent_path,
        Some("gpt-5.6-luna".to_string()),
    );
    let snapshot = ledger.complete_root_turn("root-turn", root_thread_id, "root-turn");

    assert_eq!(snapshot.running_agent_count, 0);
    assert_eq!(
        snapshot.agents.models,
        vec![model_usage(
            "gpt-5.6-luna",
            Some(usage(100, 40, 20, 5)),
            false,
        )]
    );
}

#[test]
fn retry_preserves_the_unfinished_attempt_as_partial_before_recording_success() {
    let root_thread_id = thread_id(1);
    let agent_thread_id = thread_id(2);
    let agent_path = AgentPath::root().join("worker").expect("agent path");
    let mut ledger = RootTurnUsageLedger::default();

    ledger.register_agent(
        "root-turn",
        root_thread_id,
        agent_thread_id,
        agent_path.clone(),
        Some("gpt-5.6-luna".to_string()),
    );
    ledger.begin_model_call(
        "root-turn",
        root_thread_id,
        agent_thread_id,
        "agent-turn",
        "gpt-5.6-luna",
    );
    ledger.update_model_call(
        "root-turn",
        root_thread_id,
        agent_thread_id,
        "agent-turn",
        "gpt-5.6-terra",
    );
    ledger.begin_model_call(
        "root-turn",
        root_thread_id,
        agent_thread_id,
        "agent-turn",
        "gpt-5.6-luna",
    );
    ledger.record_response(
        "root-turn",
        root_thread_id,
        agent_thread_id,
        "agent-turn",
        "successful-retry",
        "gpt-5.6-luna",
        Some(&usage(80, 30, 20, 7)),
    );

    let snapshot = ledger
        .complete_agent_turn(
            "root-turn",
            root_thread_id,
            agent_thread_id,
            agent_path,
            "agent-turn",
            Some("gpt-5.6-luna".to_string()),
        )
        .expect("terminal snapshot");

    assert_eq!(
        snapshot.models,
        vec![
            model_usage("gpt-5.6-terra", None, true),
            model_usage("gpt-5.6-luna", Some(usage(80, 30, 20, 7)), false),
        ]
    );
}
