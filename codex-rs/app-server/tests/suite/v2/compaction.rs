//! Интеграционные тесты потоков compaction через app-server.
//!
//! Этапы:
//! 1) Подготовить mock endpoints `responses`/`compact` и конфигурацию.
//! 2) Запустить thread и отправить turns, вызывающие auto-compaction.
//! 3) Проверить lifecycle-уведомления и контекст после compaction.

use anyhow::Result;
use app_test_support::ChatGptAuthFixture;
use app_test_support::MockResponsesConfig;
use app_test_support::TestAppServer;
use app_test_support::write_chatgpt_auth;
use codex_app_server_protocol::ItemCompletedNotification;
use codex_app_server_protocol::ItemStartedNotification;
use codex_app_server_protocol::JSONRPCError;
use codex_app_server_protocol::RawResponseCompletedNotification;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ResponseUsageMetadata;
use codex_app_server_protocol::ThreadCompactStartParams;
use codex_app_server_protocol::ThreadCompactStartResponse;
use codex_app_server_protocol::ThreadGoalClearedNotification;
use codex_app_server_protocol::ThreadGoalSetResponse;
use codex_app_server_protocol::ThreadHistoryMode;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::ThreadResumeParams;
use codex_app_server_protocol::ThreadResumeResponse;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadStartResponse;
use codex_app_server_protocol::TokenUsageBreakdown;
use codex_app_server_protocol::TurnCompletedNotification;
use codex_app_server_protocol::TurnStartParams;
use codex_app_server_protocol::TurnStartResponse;
use codex_app_server_protocol::UserInput as V2UserInput;
use codex_config::types::AuthCredentialsStoreMode;
use codex_features::Feature;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use core_test_support::responses;
use core_test_support::skip_if_no_network;
use pretty_assertions::assert_eq;
use tempfile::TempDir;
use tokio::time::timeout;

// macOS and Windows Bazel CI can spend tens of seconds starting app-server
// subprocesses or processing test RPCs under load.
#[cfg(any(target_os = "macos", windows))]
const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
#[cfg(not(any(target_os = "macos", windows)))]
const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
const AUTO_COMPACT_LIMIT: i64 = 1_000;
const COMPACT_PROMPT: &str = "Summarize the conversation.";
const INVALID_REQUEST_ERROR_CODE: i64 = -32600;
const ACTIVE_GOAL_OBJECTIVE: &str = "Finish <alpha> & verify compaction";
const ESCAPED_ACTIVE_GOAL_OBJECTIVE: &str = "Finish &lt;alpha&gt; &amp; verify compaction";
const ACTIVE_GOAL_USER_MESSAGE: &str = "create the durable mid-turn goal";

#[derive(Clone, Copy, Debug)]
enum TerminalGoalAction {
    Complete,
    Blocked,
    Cancel,
}

impl TerminalGoalAction {
    fn status(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Blocked => "blocked",
            Self::Cancel => "cancel",
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_compaction_local_emits_started_and_completed_items() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let sse1 = responses::sse(vec![
        responses::ev_assistant_message("m1", "FIRST_REPLY"),
        responses::ev_completed_with_tokens("r1", /*total_tokens*/ 70_000),
    ]);
    let sse2 = responses::sse(vec![
        responses::ev_assistant_message("m2", "SECOND_REPLY"),
        responses::ev_completed_with_tokens("r2", /*total_tokens*/ 330_000),
    ]);
    let sse3 = responses::sse(vec![
        responses::ev_assistant_message("m3", "LOCAL_SUMMARY"),
        responses::ev_completed_with_tokens("r3", /*total_tokens*/ 200),
    ]);
    let sse4 = responses::sse(vec![
        responses::ev_assistant_message("m4", "FINAL_REPLY"),
        responses::ev_completed_with_tokens("r4", /*total_tokens*/ 120),
    ]);
    responses::mount_sse_sequence(&server, vec![sse1, sse2, sse3, sse4]).await;

    let codex_home = TempDir::new()?;
    compaction_config(&server.uri(), AUTO_COMPACT_LIMIT).write(codex_home.path())?;

    let mut mcp = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;

    let thread_id = start_thread(&mut mcp).await?;
    for message in ["first", "second", "third"] {
        send_turn_and_wait(&mut mcp, &thread_id, message).await?;
    }

    let started = wait_for_context_compaction_started(&mut mcp).await?;
    let completed = wait_for_context_compaction_completed(&mut mcp).await?;

    let ThreadItem::ContextCompaction { id: started_id } = started.item else {
        unreachable!("started item should be context compaction");
    };
    let ThreadItem::ContextCompaction { id: completed_id } = completed.item else {
        unreachable!("completed item should be context compaction");
    };

    assert_eq!(started.thread_id, thread_id);
    assert_eq!(completed.thread_id, thread_id);
    assert_eq!(started_id, completed_id);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_goal_world_state_survives_mid_turn_local_compaction() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("create-goal"),
            responses::ev_function_call(
                "call-create-goal",
                "create_goal",
                &serde_json::json!({ "objective": ACTIVE_GOAL_OBJECTIVE }).to_string(),
            ),
            responses::ev_completed_with_tokens("create-goal", /*total_tokens*/ 330_000),
        ]),
    )
    .await;
    responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("compact"),
            responses::ev_assistant_message(
                "compact-summary",
                "A deliberately goal-free compact summary.",
            ),
            responses::ev_completed_with_tokens("compact", /*total_tokens*/ 200),
        ]),
    )
    .await;
    let post_compact = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("complete-goal"),
            responses::ev_function_call(
                "call-complete-goal",
                "update_goal",
                r#"{"status":"complete"}"#,
            ),
            responses::ev_completed_with_tokens("complete-goal", /*total_tokens*/ 120),
        ]),
    )
    .await;
    let completed_goal_follow_up = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("final"),
            responses::ev_assistant_message("final-message", "done"),
            responses::ev_completed_with_tokens("final", /*total_tokens*/ 120),
        ]),
    )
    .await;

    let codex_home = TempDir::new()?;
    compaction_config(&server.uri(), AUTO_COMPACT_LIMIT)
        .enable_feature(Feature::Goals)
        .write(codex_home.path())?;

    let mut mcp = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;

    let thread_id = start_thread(&mut mcp).await?;
    send_turn_and_wait(&mut mcp, &thread_id, ACTIVE_GOAL_USER_MESSAGE).await?;

    let post_compact_request = post_compact.single_request();
    let post_compact_body = post_compact_request.body_json().to_string();
    assert!(post_compact_body.contains("<thread_goal_context>"));
    assert!(post_compact_body.contains(ESCAPED_ACTIVE_GOAL_OBJECTIVE));
    assert_eq!(
        post_compact_body
            .matches(ESCAPED_ACTIVE_GOAL_OBJECTIVE)
            .count(),
        1,
        "the exact objective should be injected once after local compaction"
    );
    assert!(
        post_compact_body.contains("A deliberately goal-free compact summary."),
        "the regression must prove the goal comes from WorldState rather than the summary"
    );
    assert!(
        completed_goal_follow_up
            .single_request()
            .body_json()
            .to_string()
            .contains("previously active thread goal is no longer active"),
        "completing the goal should clear its active WorldState"
    );

    Ok(())
}

/// Проверяет, что Remote V2 не сохраняет старый goal fragment рядом со свежим `WorldState`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_goal_world_state_survives_mid_turn_remote_v2_compaction_once() -> Result<()> {
    skip_if_no_network!(Ok(()));
    const REMOTE_V2_AUTO_COMPACT_LIMIT: i64 = 200_000;

    let server = responses::start_mock_server().await;
    let responses_log = responses::mount_sse_sequence(
        &server,
        vec![
            responses::sse(vec![
                responses::ev_response_created("trigger-compaction"),
                responses::ev_function_call(
                    "call-update-plan",
                    "update_plan",
                    &serde_json::json!({
                        "plan": [{
                            "step": "Verify Remote V2 goal context",
                            "status": "in_progress",
                        }],
                    })
                    .to_string(),
                ),
                responses::ev_completed_with_tokens(
                    "trigger-compaction",
                    /*total_tokens*/ 330_000,
                ),
            ]),
            responses::sse(vec![
                serde_json::json!({
                    "type": "response.output_item.done",
                    "item": {
                        "type": "compaction",
                        "encrypted_content": "REMOTE_V2_GOAL_FREE_COMPACTION",
                    },
                }),
                responses::ev_completed("remote-v2-compaction"),
            ]),
            responses::sse(vec![
                responses::ev_response_created("post-compaction"),
                responses::ev_assistant_message("final-message", "done"),
                responses::ev_completed_with_tokens("post-compaction", /*total_tokens*/ 120),
            ]),
        ],
    )
    .await;

    let codex_home = TempDir::new()?;
    compaction_config(&server.uri(), REMOTE_V2_AUTO_COMPACT_LIMIT)
        .enable_feature(Feature::Goals)
        .enable_feature(Feature::RemoteCompactionV2)
        .with_provider_name("OpenAI")
        .with_provider_config("requires_openai_auth = true")
        .write(codex_home.path())?;
    write_chatgpt_auth(
        codex_home.path(),
        ChatGptAuthFixture::new("access-chatgpt").plan_type("pro"),
        AuthCredentialsStoreMode::File,
    )?;

    let mut mcp = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .with_env_overrides(&[("OPENAI_API_KEY", None)])
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;
    let thread_id = start_thread(&mut mcp).await?;
    let goal_request_id = mcp
        .send_raw_request(
            "thread/goal/set",
            Some(serde_json::json!({
                "threadId": thread_id.clone(),
                "objective": ACTIVE_GOAL_OBJECTIVE,
                "status": "active",
            })),
        )
        .await?;
    let _: ThreadGoalSetResponse =
        timeout(DEFAULT_READ_TIMEOUT, mcp.read_response(goal_request_id)).await??;
    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("thread/goal/updated"),
    )
    .await??;

    send_turn_and_wait(
        &mut mcp,
        &thread_id,
        "trigger Remote V2 compaction while the goal is active",
    )
    .await?;

    let requests = responses_log.requests();
    assert_eq!(requests.len(), 3);
    let compact_body = requests[1].body_json().to_string();
    assert_eq!(
        compact_body.matches(ESCAPED_ACTIVE_GOAL_OBJECTIVE).count(),
        1,
        "the Remote V2 input should contain the active objective once"
    );
    let post_compact_body = requests[2].body_json().to_string();
    assert!(post_compact_body.contains("<thread_goal_context>"));
    assert_eq!(
        post_compact_body
            .matches(ESCAPED_ACTIVE_GOAL_OBJECTIVE)
            .count(),
        1,
        "Remote V2 replacement should contain only the fresh WorldState objective"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_goal_world_state_survives_mid_turn_token_budget_compaction() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("create-goal"),
            responses::ev_function_call(
                "call-create-goal",
                "create_goal",
                &serde_json::json!({ "objective": ACTIVE_GOAL_OBJECTIVE }).to_string(),
            ),
            responses::ev_completed_with_tokens("create-goal", /*total_tokens*/ 330_000),
        ]),
    )
    .await;
    let post_compact = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("complete-goal"),
            responses::ev_function_call(
                "call-complete-goal",
                "update_goal",
                r#"{"status":"complete"}"#,
            ),
            responses::ev_completed_with_tokens("complete-goal", /*total_tokens*/ 120),
        ]),
    )
    .await;
    responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("final"),
            responses::ev_assistant_message("final-message", "done"),
            responses::ev_completed_with_tokens("final", /*total_tokens*/ 120),
        ]),
    )
    .await;

    let codex_home = TempDir::new()?;
    compaction_config(&server.uri(), AUTO_COMPACT_LIMIT)
        .enable_feature(Feature::Goals)
        .enable_feature(Feature::TokenBudget)
        .write(codex_home.path())?;

    let mut mcp = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;

    let thread_id = start_thread(&mut mcp).await?;
    send_turn_and_wait(&mut mcp, &thread_id, ACTIVE_GOAL_USER_MESSAGE).await?;

    let post_compact_request = post_compact.single_request();
    let post_compact_body = post_compact_request.body_json().to_string();
    assert!(post_compact_body.contains("<thread_goal_context>"));
    assert!(post_compact_body.contains(ESCAPED_ACTIVE_GOAL_OBJECTIVE));
    assert_eq!(
        post_compact_body
            .matches(ESCAPED_ACTIVE_GOAL_OBJECTIVE)
            .count(),
        1,
        "the exact objective should be injected once after TokenBudget compaction"
    );
    assert!(
        !post_compact_body.contains(ACTIVE_GOAL_USER_MESSAGE),
        "a fresh TokenBudget window should drop the original user message"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_goal_world_state_complete_clearing_is_one_shot_across_compaction() -> Result<()> {
    assert_terminal_goal_context_is_one_shot(TerminalGoalAction::Complete).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_goal_world_state_blocked_clearing_is_one_shot_across_compaction() -> Result<()> {
    assert_terminal_goal_context_is_one_shot(TerminalGoalAction::Blocked).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_goal_world_state_cancel_clearing_is_one_shot_across_compaction() -> Result<()> {
    assert_terminal_goal_context_is_one_shot(TerminalGoalAction::Cancel).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn never_active_goal_world_state_stays_absent_after_ordinary_tool_follow_up() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("update-plan"),
            responses::ev_function_call(
                "call-update-plan",
                "update_plan",
                &serde_json::json!({
                    "plan": [{
                        "step": "Finish the ordinary tool follow-up regression test",
                        "status": "in_progress",
                    }],
                })
                .to_string(),
            ),
            responses::ev_completed_with_tokens("update-plan", /*total_tokens*/ 100),
        ]),
    )
    .await;
    let post_tool = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("final"),
            responses::ev_assistant_message("final-message", "done"),
            responses::ev_completed_with_tokens("final", /*total_tokens*/ 120),
        ]),
    )
    .await;

    let codex_home = TempDir::new()?;
    compaction_config(&server.uri(), AUTO_COMPACT_LIMIT)
        .enable_feature(Feature::Goals)
        .write(codex_home.path())?;

    let mut mcp = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;

    let thread_id = start_thread(&mut mcp).await?;
    const USER_MESSAGE: &str = "update the plan without creating a goal";
    send_turn_and_wait(&mut mcp, &thread_id, USER_MESSAGE).await?;

    let post_tool_body = post_tool.single_request().body_json().to_string();
    assert_no_thread_goal_context(&post_tool_body, "ordinary tool follow-up");
    assert!(
        post_tool_body.contains(USER_MESSAGE),
        "the regression must inspect a follow-up in the original context window"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn never_active_goal_world_state_stays_absent_after_mid_turn_local_compaction() -> Result<()>
{
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let initial = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("update-plan"),
            responses::ev_function_call(
                "call-update-plan",
                "update_plan",
                &serde_json::json!({
                    "plan": [{
                        "step": "Finish the compaction regression test",
                        "status": "in_progress",
                    }],
                })
                .to_string(),
            ),
            responses::ev_completed_with_tokens("update-plan", /*total_tokens*/ 330_000),
        ]),
    )
    .await;
    responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("compact"),
            responses::ev_assistant_message(
                "compact-summary",
                "A deliberately goal-free compact summary.",
            ),
            responses::ev_completed_with_tokens("compact", /*total_tokens*/ 200),
        ]),
    )
    .await;
    let post_compact = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("final"),
            responses::ev_assistant_message("final-message", "done"),
            responses::ev_completed_with_tokens("final", /*total_tokens*/ 120),
        ]),
    )
    .await;

    let codex_home = TempDir::new()?;
    compaction_config(&server.uri(), AUTO_COMPACT_LIMIT)
        .enable_feature(Feature::Goals)
        .write(codex_home.path())?;

    let mut mcp = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;

    let thread_id = start_thread(&mut mcp).await?;
    send_turn_and_wait(
        &mut mcp,
        &thread_id,
        "update the plan without creating a goal",
    )
    .await?;

    assert_no_thread_goal_context(
        &initial.single_request().body_json().to_string(),
        "initial sampling",
    );
    let post_compact_body = post_compact.single_request().body_json().to_string();
    assert_no_thread_goal_context(&post_compact_body, "local compaction");
    assert!(
        post_compact_body.contains("A deliberately goal-free compact summary."),
        "the regression must inspect the request after local compaction"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn never_active_goal_world_state_stays_absent_after_mid_turn_token_budget_compaction()
-> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("update-plan"),
            responses::ev_function_call(
                "call-update-plan",
                "update_plan",
                &serde_json::json!({
                    "plan": [{
                        "step": "Finish the compaction regression test",
                        "status": "in_progress",
                    }],
                })
                .to_string(),
            ),
            responses::ev_completed_with_tokens("update-plan", /*total_tokens*/ 330_000),
        ]),
    )
    .await;
    let post_compact = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("final"),
            responses::ev_assistant_message("final-message", "done"),
            responses::ev_completed_with_tokens("final", /*total_tokens*/ 120),
        ]),
    )
    .await;

    let codex_home = TempDir::new()?;
    compaction_config(&server.uri(), AUTO_COMPACT_LIMIT)
        .enable_feature(Feature::Goals)
        .enable_feature(Feature::TokenBudget)
        .write(codex_home.path())?;

    let mut mcp = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;

    let thread_id = start_thread(&mut mcp).await?;
    const USER_MESSAGE: &str = "update the plan without creating a goal";
    send_turn_and_wait(&mut mcp, &thread_id, USER_MESSAGE).await?;

    let post_compact_body = post_compact.single_request().body_json().to_string();
    assert_no_thread_goal_context(&post_compact_body, "TokenBudget compaction");
    assert!(
        !post_compact_body.contains(USER_MESSAGE),
        "the regression must inspect the fresh TokenBudget context window"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_compaction_remote_emits_started_and_completed_items() -> Result<()> {
    skip_if_no_network!(Ok(()));
    const REMOTE_AUTO_COMPACT_LIMIT: i64 = 200_000;

    let server = responses::start_mock_server().await;
    let sse1 = responses::sse(vec![
        responses::ev_assistant_message("m1", "FIRST_REPLY"),
        responses::ev_completed_with_tokens("r1", /*total_tokens*/ 70_000),
    ]);
    let sse2 = responses::sse(vec![
        responses::ev_assistant_message("m2", "SECOND_REPLY"),
        responses::ev_completed_with_tokens("r2", /*total_tokens*/ 330_000),
    ]);
    let sse3 = responses::sse(vec![
        responses::ev_assistant_message("m3", "FINAL_REPLY"),
        responses::ev_completed_with_tokens("r3", /*total_tokens*/ 120),
    ]);
    let responses_log = responses::mount_sse_sequence(&server, vec![sse1, sse2, sse3]).await;

    let compacted_history = vec![
        ResponseItem::Message {
            id: None,
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText {
                text: "REMOTE_COMPACT_SUMMARY".to_string(),
            }],
            phase: None,
            internal_chat_message_metadata_passthrough: None,
        },
        ResponseItem::Compaction {
            id: None,
            encrypted_content: "ENCRYPTED_COMPACTION_SUMMARY".to_string(),
            internal_chat_message_metadata_passthrough: None,
        },
    ];
    let compact_mock = responses::mount_compact_json_once(
        &server,
        serde_json::json!({ "output": compacted_history }),
    )
    .await;

    let codex_home = TempDir::new()?;
    compaction_config(&server.uri(), REMOTE_AUTO_COMPACT_LIMIT)
        .disable_feature(Feature::RemoteCompactionV2)
        .with_provider_name("OpenAI")
        .with_provider_config("requires_openai_auth = true")
        .write(codex_home.path())?;
    write_chatgpt_auth(
        codex_home.path(),
        ChatGptAuthFixture::new("access-chatgpt").plan_type("pro"),
        AuthCredentialsStoreMode::File,
    )?;

    let mut mcp = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .with_env_overrides(&[("OPENAI_API_KEY", None)])
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;

    let thread_id = start_thread(&mut mcp).await?;
    for message in ["first", "second", "third"] {
        send_turn_and_wait(&mut mcp, &thread_id, message).await?;
    }

    let started = wait_for_context_compaction_started(&mut mcp).await?;
    let completed = wait_for_context_compaction_completed(&mut mcp).await?;

    let ThreadItem::ContextCompaction { id: started_id } = started.item else {
        unreachable!("started item should be context compaction");
    };
    let ThreadItem::ContextCompaction { id: completed_id } = completed.item else {
        unreachable!("completed item should be context compaction");
    };

    assert_eq!(started.thread_id, thread_id);
    assert_eq!(completed.thread_id, thread_id);
    assert_eq!(started_id, completed_id);

    let compact_requests = compact_mock.requests();
    assert_eq!(compact_requests.len(), 1);
    assert_eq!(compact_requests[0].path(), "/v1/responses/compact");

    let response_requests = responses_log.requests();
    assert_eq!(response_requests.len(), 3);
    let turn_metadata = response_requests
        .iter()
        .map(|request| {
            request
                .header("x-codex-turn-metadata")
                .as_deref()
                .map(parse_json_header)
                .expect("turn request should include turn metadata")
        })
        .collect::<Vec<_>>();
    for (request, metadata) in response_requests.iter().zip(&turn_metadata) {
        assert_eq!(metadata["request_kind"].as_str(), Some("turn"));
        assert!(
            metadata["turn_id"]
                .as_str()
                .is_some_and(|turn_id| !turn_id.is_empty()),
            "turn request should carry a non-empty turn id"
        );
        assert_eq!(
            metadata["window_id"].as_str(),
            request.header("x-codex-window-id").as_deref()
        );
        assert!(metadata.get("compaction").is_none());
    }

    let compact_metadata = compact_requests[0]
        .header("x-codex-turn-metadata")
        .as_deref()
        .map(parse_json_header)
        .expect("compact request should include turn metadata");
    assert_eq!(
        compact_metadata["request_kind"].as_str(),
        Some("compaction")
    );
    assert_eq!(
        compact_metadata["compaction"],
        serde_json::json!({
            "trigger": "auto",
            "reason": "context_limit",
            "implementation": "responses_compact",
            "phase": "pre_turn",
            "strategy": "memento",
        })
    );
    assert_eq!(
        compact_metadata["turn_id"], turn_metadata[2]["turn_id"],
        "pre-turn compaction should carry the current turn id"
    );
    assert_eq!(
        compact_metadata["window_id"].as_str(),
        compact_requests[0].header("x-codex-window-id").as_deref()
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn thread_compact_start_triggers_compaction_and_returns_empty_response() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let seed = responses::sse(vec![
        responses::ev_assistant_message("seed", "FIRST_REPLY"),
        responses::ev_completed_with_tokens("seed", /*total_tokens*/ 120),
    ]);
    let mut completed = responses::ev_completed_with_tokens("r1", /*total_tokens*/ 200);
    completed["response"]["usage_metadata"] = serde_json::json!({ "amount": "0.125" });
    completed["response"]["usage"]["extra"] = serde_json::json!({ "label": "example" });
    let expected_metadata = completed["response"]["usage"].clone();
    let sse = responses::sse(vec![
        responses::ev_assistant_message("m1", "MANUAL_COMPACT_SUMMARY"),
        completed,
    ]);
    let followup = responses::sse(vec![
        responses::ev_assistant_message("followup", "FINAL_REPLY"),
        responses::ev_completed_with_tokens("followup", /*total_tokens*/ 120),
    ]);
    let _responses = responses::mount_sse_sequence(&server, vec![seed, sse, followup]).await;

    let codex_home = TempDir::new()?;
    let initial_cwd = TempDir::new()?;
    let updated_cwd = TempDir::new()?;
    compaction_config(&server.uri(), /*auto_compact_limit*/ 1_000_000).write(codex_home.path())?;

    // Top-level cwd restoration uses host-native paths, not a foreign executor's paths.
    let mut mcp = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .without_auto_env()
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;

    let thread_req = mcp
        .send_thread_start_request(ThreadStartParams {
            model: Some("mock-model".to_string()),
            cwd: Some(initial_cwd.path().to_string_lossy().into_owned()),
            history_mode: Some(ThreadHistoryMode::Paginated),
            experimental_raw_events: true,
            ..Default::default()
        })
        .await?;
    let ThreadStartResponse { thread, .. } =
        timeout(DEFAULT_READ_TIMEOUT, mcp.read_response(thread_req)).await??;
    let thread_id = thread.id;
    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.start_turn_and_wait_for_completion(TurnStartParams {
            thread_id: thread_id.clone(),
            cwd: Some(updated_cwd.path().to_path_buf()),
            input: vec![V2UserInput::Text {
                text: "seed history".to_string(),
                text_elements: Vec::new(),
            }],
            ..Default::default()
        }),
    )
    .await??;
    mcp.clear_message_buffer();

    let compact_id = mcp
        .send_thread_compact_start_request(ThreadCompactStartParams {
            thread_id: thread_id.clone(),
        })
        .await?;
    let _: ThreadCompactStartResponse =
        timeout(DEFAULT_READ_TIMEOUT, mcp.read_response(compact_id)).await??;

    let started = wait_for_context_compaction_started(&mut mcp).await?;
    let raw_completed: RawResponseCompletedNotification = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_notification("rawResponse/completed"),
    )
    .await??;
    let completed = wait_for_context_compaction_completed(&mut mcp).await?;
    wait_for_turn_completed(&mut mcp, &started.turn_id).await?;

    let ThreadItem::ContextCompaction { id: started_id } = started.item else {
        unreachable!("started item should be context compaction");
    };
    let ThreadItem::ContextCompaction { id: completed_id } = completed.item else {
        unreachable!("completed item should be context compaction");
    };

    assert_eq!(started.thread_id, thread_id);
    assert_eq!(completed.thread_id, thread_id);
    assert_eq!(started_id, completed_id);
    assert_eq!(
        raw_completed,
        RawResponseCompletedNotification {
            thread_id: thread_id.clone(),
            turn_id: started.turn_id,
            response_id: "r1".to_string(),
            usage_metadata: Some(ResponseUsageMetadata {
                amount: Some("0.125".to_string()),
                metadata: Some(expected_metadata),
            }),
            usage: Some(TokenUsageBreakdown {
                total_tokens: 200,
                input_tokens: 200,
                cached_input_tokens: 0,
                cache_write_input_tokens: 0,
                output_tokens: 0,
                reasoning_output_tokens: 0,
            }),
        }
    );

    // A completed turn after compaction permits bounded replay. Neither this turn nor
    // resume resends settings, so restoring the updated cwd depends on the checkpoint.
    send_turn_and_wait(&mut mcp, &thread_id, "continue").await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.shutdown_gracefully()).await??;

    let mut mcp = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .without_auto_env()
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;
    let resume_id = mcp
        .send_thread_resume_request(ThreadResumeParams {
            thread_id,
            exclude_turns: true,
            ..Default::default()
        })
        .await?;
    let ThreadResumeResponse { cwd, .. } =
        timeout(DEFAULT_READ_TIMEOUT, mcp.read_response(resume_id)).await??;
    assert_eq!(cwd.as_path(), updated_cwd.path());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn thread_compact_start_rejects_invalid_thread_id() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let codex_home = TempDir::new()?;
    compaction_config(&server.uri(), AUTO_COMPACT_LIMIT).write(codex_home.path())?;

    let mut mcp = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;

    let request_id = mcp
        .send_thread_compact_start_request(ThreadCompactStartParams {
            thread_id: "not-a-thread-id".to_string(),
        })
        .await?;
    let error: JSONRPCError = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;

    assert_eq!(error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert!(error.error.message.contains("invalid thread id"));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn thread_compact_start_rejects_unknown_thread_id() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let codex_home = TempDir::new()?;
    compaction_config(&server.uri(), AUTO_COMPACT_LIMIT).write(codex_home.path())?;

    let mut mcp = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;

    let request_id = mcp
        .send_thread_compact_start_request(ThreadCompactStartParams {
            thread_id: "67e55044-10b1-426f-9247-bb680e5fe0c8".to_string(),
        })
        .await?;
    let error: JSONRPCError = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;

    assert_eq!(error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert!(error.error.message.contains("thread not found"));

    Ok(())
}

/// Переводит thread goal выбранным действием в финальное состояние и проверяет
/// одноразовую очистку model context.
async fn assert_terminal_goal_context_is_one_shot(action: TerminalGoalAction) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("create-goal"),
            responses::ev_function_call(
                "call-create-goal",
                "create_goal",
                &serde_json::json!({ "objective": ACTIVE_GOAL_OBJECTIVE }).to_string(),
            ),
            responses::ev_completed_with_tokens("create-goal", /*total_tokens*/ 100),
        ]),
    )
    .await;
    let terminal = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("terminal-goal"),
            responses::ev_function_call(
                "call-terminal-goal",
                "update_goal",
                &serde_json::json!({ "status": action.status() }).to_string(),
            ),
            responses::ev_completed_with_tokens("terminal-goal", /*total_tokens*/ 330_000),
        ]),
    )
    .await;
    let compact = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("compact-terminal-goal"),
            responses::ev_assistant_message(
                "compact-terminal-summary",
                "A deliberately terminal-goal-free compact summary.",
            ),
            responses::ev_completed_with_tokens("compact-terminal-goal", /*total_tokens*/ 200),
        ]),
    )
    .await;
    let clearing = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("clearing-follow-up"),
            responses::ev_function_call(
                "call-update-plan",
                "update_plan",
                &serde_json::json!({
                    "plan": [{
                        "step": "Verify terminal goal context cleanup",
                        "status": "in_progress",
                    }],
                })
                .to_string(),
            ),
            responses::ev_completed_with_tokens("clearing-follow-up", /*total_tokens*/ 100),
        ]),
    )
    .await;
    let post_compact = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("post-terminal-compact"),
            responses::ev_assistant_message("final-message", "done"),
            responses::ev_completed_with_tokens("post-terminal-compact", /*total_tokens*/ 120),
        ]),
    )
    .await;

    let codex_home = TempDir::new()?;
    compaction_config(&server.uri(), AUTO_COMPACT_LIMIT)
        .enable_feature(Feature::Goals)
        .write(codex_home.path())?;

    let mut mcp = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;

    let thread_id = start_thread(&mut mcp).await?;
    send_turn_and_wait(&mut mcp, &thread_id, ACTIVE_GOAL_USER_MESSAGE).await?;

    let terminal_body = terminal.single_request().body_json().to_string();
    assert!(terminal_body.contains("<thread_goal_context>"));
    assert!(terminal_body.contains(ESCAPED_ACTIVE_GOAL_OBJECTIVE));
    assert!(
        terminal_body.contains("Cancellation is immediate and does not require the blocked audit.")
    );
    assert_eq!(
        terminal_body.matches("No-progress check:").count(),
        1,
        "persistent goal invariants should come from active WorldState without continuation duplication"
    );

    let clearing_body = clearing.single_request().body_json().to_string();
    assert_eq!(
        clearing_body
            .matches("previously active thread goal is no longer active")
            .count(),
        1,
        "terminal transition should be delivered to exactly one sampling request"
    );
    let compact_body = compact.single_request().body_json().to_string();
    assert!(
        !compact_body.contains("previously active thread goal is no longer active"),
        "the clearing fragment must not be persisted into compaction input"
    );
    let post_compact_body = post_compact.single_request().body_json().to_string();
    assert_no_thread_goal_context(&post_compact_body, "terminal goal compaction replacement");
    assert!(post_compact_body.contains("A deliberately terminal-goal-free compact summary."));

    if matches!(action, TerminalGoalAction::Cancel) {
        let cleared: ThreadGoalClearedNotification = timeout(
            DEFAULT_READ_TIMEOUT,
            mcp.read_notification("thread/goal/cleared"),
        )
        .await??;
        assert_eq!(cleared.thread_id, thread_id);
    }

    Ok(())
}

async fn start_thread(mcp: &mut TestAppServer) -> Result<String> {
    let thread_id = mcp
        .send_thread_start_request_with_auto_env(ThreadStartParams {
            model: Some("mock-model".to_string()),
            ..Default::default()
        })
        .await?;
    let ThreadStartResponse { thread, .. } =
        timeout(DEFAULT_READ_TIMEOUT, mcp.read_response(thread_id)).await??;
    Ok(thread.id)
}

async fn send_turn_and_wait(
    mcp: &mut TestAppServer,
    thread_id: &str,
    text: &str,
) -> Result<String> {
    let turn_id = mcp
        .send_turn_start_request(TurnStartParams {
            thread_id: thread_id.to_string(),
            client_user_message_id: None,
            input: vec![V2UserInput::Text {
                text: text.to_string(),
                text_elements: Vec::new(),
            }],
            ..Default::default()
        })
        .await?;
    let TurnStartResponse { turn } =
        timeout(DEFAULT_READ_TIMEOUT, mcp.read_response(turn_id)).await??;
    wait_for_turn_completed(mcp, &turn.id).await?;
    Ok(turn.id)
}

async fn wait_for_turn_completed(mcp: &mut TestAppServer, turn_id: &str) -> Result<()> {
    loop {
        let completed: TurnCompletedNotification = timeout(
            DEFAULT_READ_TIMEOUT,
            mcp.read_notification("turn/completed"),
        )
        .await??;
        if completed.turn.id == turn_id {
            return Ok(());
        }
    }
}

async fn wait_for_context_compaction_started(
    mcp: &mut TestAppServer,
) -> Result<ItemStartedNotification> {
    loop {
        let started: ItemStartedNotification =
            timeout(DEFAULT_READ_TIMEOUT, mcp.read_notification("item/started")).await??;
        if let ThreadItem::ContextCompaction { .. } = started.item {
            return Ok(started);
        }
    }
}

async fn wait_for_context_compaction_completed(
    mcp: &mut TestAppServer,
) -> Result<ItemCompletedNotification> {
    loop {
        let completed: ItemCompletedNotification = timeout(
            DEFAULT_READ_TIMEOUT,
            mcp.read_notification("item/completed"),
        )
        .await??;
        if let ThreadItem::ContextCompaction { .. } = completed.item {
            return Ok(completed);
        }
    }
}

fn parse_json_header(value: &str) -> serde_json::Value {
    serde_json::from_str(value).expect("turn metadata should be JSON")
}

fn assert_no_thread_goal_context(body: &str, compaction_path: &str) {
    assert!(
        !body.contains("<thread_goal_context>"),
        "thread goal context must be absent after {compaction_path}"
    );
    assert!(
        !body.contains("previously active thread goal is no longer active"),
        "a clearing marker must be absent after {compaction_path}"
    );
}

fn compaction_config(server_uri: &str, auto_compact_limit: i64) -> MockResponsesConfig {
    MockResponsesConfig::new(server_uri)
        .with_root_config(&format!(
            "compact_prompt = \"{COMPACT_PROMPT}\"\nmodel_auto_compact_token_limit = {auto_compact_limit}"
        ))
        .with_provider_config("supports_websockets = false")
}
