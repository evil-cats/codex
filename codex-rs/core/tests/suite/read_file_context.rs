//! Интеграционные тесты контекстной дедупликации `read_file`.
//!
//! Сценарии проходят через mocked Responses flow и проверяют фактическую
//! model-visible history, source identity и fallback после изменения истории.

use std::fs;

use anyhow::Context;
use anyhow::Result;
use codex_protocol::models::PermissionProfile;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::skip_if_sandbox;
use core_test_support::test_codex::local;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn read_file_context_dedup_uses_one_complete_content_output() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex().with_config(|config| {
        config.read_file_content_max_tokens = 10_000;
    });
    let fixture = builder.build_with_auto_env(&server).await?;
    fs::write(
        fixture.workspace_path("read-file-context-a.txt"),
        "alpha\nbeta\ngamma\ndelta\n",
    )
    .context("write first read_file context fixture")?;
    fs::write(
        fixture.workspace_path("read-file-context-b.txt"),
        "one\ntwo\nthree\nfour\n",
    )
    .context("write second read_file context fixture")?;

    let calls = [
        (
            "read-file-context-full",
            json!({ "path": "read-file-context-a.txt" }),
        ),
        (
            "read-file-context-same",
            json!({ "path": "read-file-context-a.txt" }),
        ),
        (
            "read-file-context-nested",
            json!({
                "path": "read-file-context-a.txt",
                "start_line": 2,
                "end_line": 3,
            }),
        ),
        (
            "read-file-context-range",
            json!({
                "path": "read-file-context-b.txt",
                "start_line": 1,
                "end_line": 3,
            }),
        ),
        (
            "read-file-context-overlap",
            json!({
                "path": "read-file-context-b.txt",
                "start_line": 3,
                "end_line": 4,
            }),
        ),
        (
            "read-file-context-full-after-ranges",
            json!({ "path": "read-file-context-b.txt" }),
        ),
        (
            "read-file-context-raw",
            json!({
                "path": "read-file-context-b.txt",
                "start_line": 1,
                "end_line": 3,
                "line_numbers": false,
            }),
        ),
    ];
    let mut responses = calls
        .iter()
        .enumerate()
        .map(|(index, (call_id, args))| {
            sse(vec![
                ev_response_created(&format!("resp-{}", index + 1)),
                ev_function_call(call_id, "read_file", &args.to_string()),
                ev_completed(&format!("resp-{}", index + 1)),
            ])
        })
        .collect::<Vec<_>>();
    responses.push(sse(vec![
        ev_assistant_message("msg-read-file-context", "done"),
        ev_completed("resp-read-file-context-done"),
    ]));
    let mock = mount_sse_sequence(&server, responses).await;

    fixture
        .submit_turn_with_permission_profile(
            "exercise repeated read_file calls",
            PermissionProfile::read_only(),
        )
        .await?;

    assert_eq!(
        mock.function_call_output_text("read-file-context-full")
            .context("initial full read_file output present")?,
        "ReadFile: read-file-context-a.txt\nLines: total=4 requested=1-4 returned=1-4 complete=yes\nLineNumbers: yes\n\n1 | alpha\n2 | beta\n3 | gamma\n4 | delta\n"
    );
    assert_eq!(
        mock.function_call_output_text("read-file-context-same")
            .context("same read_file reference output present")?,
        "ReadFile: read-file-context-a.txt\nStatus: already_in_context\nCoverage: requested=1-4 available=1-4 complete=yes\nLineNumbers: yes\nCoveredBy: read-file-context-full\n"
    );
    assert_eq!(
        mock.function_call_output_text("read-file-context-nested")
            .context("nested read_file reference output present")?,
        "ReadFile: read-file-context-a.txt\nStatus: already_in_context\nCoverage: requested=2-3 available=1-4 complete=yes\nLineNumbers: yes\nCoveredBy: read-file-context-full\n"
    );
    assert_eq!(
        mock.function_call_output_text("read-file-context-overlap")
            .context("partially overlapping read_file output present")?,
        "ReadFile: read-file-context-b.txt\nLines: total=4 requested=3-4 returned=3-4 complete=yes\nLineNumbers: yes\n\n3 | three\n4 | four\n"
    );
    assert_eq!(
        mock.function_call_output_text("read-file-context-full-after-ranges")
            .context("full read_file output after ranges present")?,
        "ReadFile: read-file-context-b.txt\nLines: total=4 requested=1-4 returned=1-4 complete=yes\nLineNumbers: yes\n\n1 | one\n2 | two\n3 | three\n4 | four\n"
    );
    assert_eq!(
        mock.function_call_output_text("read-file-context-raw")
            .context("raw read_file output present")?,
        "ReadFile: read-file-context-b.txt\nLines: total=4 requested=1-3 returned=1-3 complete=yes\nLineNumbers: no\n\none\ntwo\nthree\n"
    );
    let requests = mock.requests();
    let same_reference_request = requests
        .iter()
        .find(|request| {
            request
                .function_call_output_text("read-file-context-same")
                .is_some()
        })
        .context("request carrying the same-range reference")?;
    assert_eq!(
        same_reference_request.function_call_output_text("read-file-context-full"),
        mock.function_call_output_text("read-file-context-full"),
        "CoveredBy content must be present in the same model-visible request"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn read_file_context_dedup_rehydrates_changed_content() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex().with_config(|config| {
        config.read_file_content_max_tokens = 10_000;
    });
    let fixture = builder.build_with_auto_env(&server).await?;
    let path = fixture.workspace_path("read-file-context-changed.txt");
    fs::write(&path, "before\nstable\n").context("write initial read_file fixture")?;

    let first_call_id = "read-file-context-before-change";
    let second_call_id = "read-file-context-after-change";
    let args = json!({ "path": "read-file-context-changed.txt" });
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-before-change"),
                ev_function_call(first_call_id, "read_file", &args.to_string()),
                ev_completed("resp-before-change"),
            ]),
            sse(vec![
                ev_assistant_message("msg-before-change", "done"),
                ev_completed("resp-before-change-done"),
            ]),
            sse(vec![
                ev_response_created("resp-after-change"),
                ev_function_call(second_call_id, "read_file", &args.to_string()),
                ev_completed("resp-after-change"),
            ]),
            sse(vec![
                ev_assistant_message("msg-after-change", "done"),
                ev_completed("resp-after-change-done"),
            ]),
        ],
    )
    .await;

    fixture
        .submit_turn_with_permission_profile(
            "read the original fixture",
            PermissionProfile::read_only(),
        )
        .await?;
    fs::write(&path, "after\nstable\n").context("replace read_file fixture content")?;
    fixture
        .submit_turn_with_permission_profile(
            "read the changed fixture",
            PermissionProfile::read_only(),
        )
        .await?;

    assert_eq!(
        mock.function_call_output_text(second_call_id)
            .context("changed read_file output present")?,
        "ReadFile: read-file-context-changed.txt\nLines: total=2 requested=1-2 returned=1-2 complete=yes\nLineNumbers: yes\n\n1 | after\n2 | stable\n"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn read_file_context_dedup_rehydrates_when_primary_cwd_changes() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = start_mock_server().await;
    let fixture = test_codex().build(&server).await?;
    let first_cwd = fixture.config.cwd.join("read-file-context-first-cwd");
    let second_cwd = fixture.config.cwd.join("read-file-context-second-cwd");
    fs::create_dir_all(&first_cwd).context("create first read_file context cwd")?;
    fs::create_dir_all(&second_cwd).context("create second read_file context cwd")?;
    fs::write(first_cwd.join("same.txt"), "same content\n")
        .context("write first same-path read_file fixture")?;
    fs::write(second_cwd.join("same.txt"), "same content\n")
        .context("write second same-path read_file fixture")?;

    let first_call_id = "read-file-context-first-cwd";
    let second_call_id = "read-file-context-second-cwd";
    let args = json!({ "path": "same.txt" });
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-first-cwd"),
                ev_function_call(first_call_id, "read_file", &args.to_string()),
                ev_completed("resp-first-cwd"),
            ]),
            sse(vec![
                ev_assistant_message("msg-first-cwd", "done"),
                ev_completed("resp-first-cwd-done"),
            ]),
            sse(vec![
                ev_response_created("resp-second-cwd"),
                ev_function_call(second_call_id, "read_file", &args.to_string()),
                ev_completed("resp-second-cwd"),
            ]),
            sse(vec![
                ev_assistant_message("msg-second-cwd", "done"),
                ev_completed("resp-second-cwd-done"),
            ]),
        ],
    )
    .await;

    fixture
        .submit_turn_with_environments(
            "read from the first primary cwd",
            Some(vec![local(first_cwd)]),
        )
        .await?;
    fixture
        .submit_turn_with_environments(
            "read from the second primary cwd",
            Some(vec![local(second_cwd)]),
        )
        .await?;

    assert_eq!(
        mock.function_call_output_text(second_call_id)
            .context("second-cwd read_file output present")?,
        "ReadFile: same.txt\nLines: total=1 requested=1-1 returned=1-1 complete=yes\nLineNumbers: yes\n\n1 | same content\n"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn read_file_context_dedup_rehydrates_after_rollback() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_codex().with_config(|config| {
        config.read_file_content_max_tokens = 10_000;
    });
    let fixture = builder.build_with_auto_env(&server).await?;
    fs::write(
        fixture.workspace_path("read-file-context-rollback.txt"),
        "alpha\nbeta\n",
    )
    .context("write rolled-back read_file fixture")?;

    let first_call_id = "read-file-context-before-rollback";
    let second_call_id = "read-file-context-after-rollback";
    let args = json!({ "path": "read-file-context-rollback.txt" });
    let mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-before-rollback"),
                ev_function_call(first_call_id, "read_file", &args.to_string()),
                ev_completed("resp-before-rollback"),
            ]),
            sse(vec![
                ev_assistant_message("msg-before-rollback", "done"),
                ev_completed("resp-before-rollback-done"),
            ]),
            sse(vec![
                ev_response_created("resp-after-rollback"),
                ev_function_call(second_call_id, "read_file", &args.to_string()),
                ev_completed("resp-after-rollback"),
            ]),
            sse(vec![
                ev_assistant_message("msg-after-rollback", "done"),
                ev_completed("resp-after-rollback-done"),
            ]),
        ],
    )
    .await;

    fixture
        .submit_turn_with_permission_profile("read before rollback", PermissionProfile::read_only())
        .await?;
    fixture
        .codex
        .submit(Op::ThreadRollback { num_turns: 1 })
        .await?;
    wait_for_event(&fixture.codex, |event| {
        matches!(event, EventMsg::ThreadRolledBack(_))
    })
    .await;
    fixture
        .submit_turn_with_permission_profile("read after rollback", PermissionProfile::read_only())
        .await?;

    assert_eq!(
        mock.function_call_output_text(second_call_id)
            .context("post-rollback read_file output present")?,
        "ReadFile: read-file-context-rollback.txt\nLines: total=2 requested=1-2 returned=1-2 complete=yes\nLineNumbers: yes\n\n1 | alpha\n2 | beta\n"
    );
    let requests = mock.requests();
    assert!(
        !requests[2].has_function_call(first_call_id),
        "post-rollback prompt should not retain the original read_file call"
    );

    Ok(())
}
