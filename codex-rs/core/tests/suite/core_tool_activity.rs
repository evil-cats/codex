//! Интеграционные проверки событий пользовательской activity для core function tools.

use std::fs;

use anyhow::Context;
use anyhow::Result;
use codex_core::TurnInputRequest;
use codex_protocol::items::CoreToolActivityItem;
use codex_protocol::items::CoreToolActivityKind;
use codex_protocol::items::CoreToolActivityStatus;
use codex_protocol::items::TurnItem;
use codex_protocol::models::PermissionProfile;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::ThreadSettingsOverrides;
use codex_protocol::user_input::UserInput;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::skip_if_sandbox;
use core_test_support::test_codex::local_selections;
use core_test_support::test_codex::test_codex;
use core_test_support::test_codex::turn_permission_fields;
use core_test_support::wait_for_event;
use core_test_support::wait_for_event_match;
use pretty_assertions::assert_eq;
use serde_json::json;

/// Проверяет, что реальный успешный `read_file` создаёт согласованную пару
/// `ItemStarted`/`ItemCompleted` для одной core tool activity.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn read_file_core_tool_activity_emits_paired_lifecycle_items() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex().with_config(|config| {
        config.read_file_content_max_tokens = 10_000;
    });
    let fixture = builder.build(&server).await?;
    let fixture_path = fixture.workspace_path("core-tool-activity.txt");
    fs::write(&fixture_path, "activity lifecycle\n").context("write core tool activity fixture")?;

    let call_id = "read-file-core-tool-activity";
    let arguments = json!({ "path": "core-tool-activity.txt" });
    let _responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_function_call(call_id, "read_file", &serde_json::to_string(&arguments)?),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let (sandbox_policy, permission_profile) =
        turn_permission_fields(PermissionProfile::read_only(), fixture.config.cwd.as_path());
    fixture
        .codex
        .start_or_steer_turn(
            TurnInputRequest::user_input(vec![UserInput::Text {
                text: "read the activity fixture".to_string(),
                text_elements: Vec::new(),
            }])
            .with_thread_settings(ThreadSettingsOverrides {
                environments: Some(local_selections(fixture.config.cwd.clone())),
                approval_policy: Some(AskForApproval::Never),
                sandbox_policy: Some(sandbox_policy),
                permission_profile,
                ..Default::default()
            }),
        )
        .await?;

    let started = wait_for_event_match(&fixture.codex, |event| match event {
        EventMsg::ItemStarted(event) => match &event.item {
            TurnItem::CoreToolActivity(item) if item.id == call_id => Some(item.clone()),
            _ => None,
        },
        _ => None,
    })
    .await;
    let completed = wait_for_event_match(&fixture.codex, |event| match event {
        EventMsg::ItemCompleted(event) => match &event.item {
            TurnItem::CoreToolActivity(item) if item.id == call_id => Some(item.clone()),
            _ => None,
        },
        _ => None,
    })
    .await;

    let expected_started = CoreToolActivityItem {
        id: call_id.to_string(),
        tool_name: "read_file".to_string(),
        kind: CoreToolActivityKind::File,
        detail: "core-tool-activity.txt".to_string(),
        arguments,
        status: CoreToolActivityStatus::InProgress,
        error: None,
        duration: None,
    };
    assert_eq!(started, expected_started);
    assert!(completed.duration.is_some());
    let expected_completed = CoreToolActivityItem {
        status: CoreToolActivityStatus::Completed,
        duration: completed.duration,
        ..expected_started
    };
    assert_eq!(completed, expected_completed);

    wait_for_event(&fixture.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    Ok(())
}
