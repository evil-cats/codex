//! Интеграционные проверки начального `stdin` инструмента `exec_command`.
//!
//! Сценарии проверяют точность UTF-8-текста и EOF в интерактивном и управляемом
//! одноразовом путях, продолжение TTY, удалённую возможность и отказ
//! перехваченного `apply_patch` до изменения файлов.

use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use codex_config::test_support::CloudConfigBundleFixture;
use codex_protocol::models::PermissionProfile;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::skip_if_no_network;
use core_test_support::skip_if_sandbox;
use core_test_support::skip_if_target_windows;
use core_test_support::test_codex::TestCodexHarness;
use core_test_support::test_codex::test_codex;
use futures::SinkExt;
use futures::StreamExt;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

/// Создаёт базовую конфигурацию с unified exec, которую одноразовый сценарий сужает управляемым требованием.
fn unified_exec_builder() -> core_test_support::test_codex::TestCodexBuilder {
    test_codex()
}

/// Монтирует один вызов `exec_command` и завершающий ответ модели.
async fn mount_exec_command(
    harness: &TestCodexHarness,
    call_id: &str,
    arguments: serde_json::Value,
) -> Result<()> {
    mount_sse_sequence(
        harness.server(),
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_function_call(call_id, "exec_command", &serde_json::to_string(&arguments)?),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;
    Ok(())
}

/// Читает следующее текстовое или бинарное JSON-RPC-сообщение тестового exec-server.
async fn read_exec_server_json(websocket: &mut WebSocketStream<TcpStream>) -> Value {
    loop {
        match timeout(Duration::from_secs(5), websocket.next())
            .await
            .expect("websocket read should not time out")
            .expect("websocket should stay open")
            .expect("websocket frame should read")
        {
            Message::Text(text) => {
                return serde_json::from_str(text.as_ref()).expect("valid JSON-RPC message");
            }
            Message::Binary(bytes) => {
                return serde_json::from_slice(bytes.as_ref()).expect("valid JSON-RPC message");
            }
            Message::Ping(_) | Message::Pong(_) => {}
            other => panic!("expected JSON-RPC message, got {other:?}"),
        }
    }
}

/// Отправляет JSON-RPC-сообщение через тестовое WebSocket-соединение exec-server.
async fn send_exec_server_json(websocket: &mut WebSocketStream<TcpStream>, message: Value) {
    websocket
        .send(Message::Text(message.to_string().into()))
        .await
        .expect("exec-server message should send");
}

/// Обслуживает минимальный exec-server и возвращает полученный `process/start`.
///
/// При отключённой возможности `initial_stdin` функция допускает только запросы подготовки и
/// завершается ошибкой теста, если клиент всё же пытается запустить процесс.
async fn serve_initial_stdin_exec_server(
    listener: TcpListener,
    supports_initial_stdin: bool,
    process_started: Arc<AtomicBool>,
) -> Value {
    let (stream, _) = listener.accept().await.expect("connection");
    let mut websocket = accept_async(stream).await.expect("websocket handshake");

    let initialize = read_exec_server_json(&mut websocket).await;
    assert_eq!(initialize["method"], "initialize");
    send_exec_server_json(
        &mut websocket,
        json!({
            "id": initialize["id"],
            "result": { "sessionId": "initial-stdin-test" }
        }),
    )
    .await;
    let initialized = read_exec_server_json(&mut websocket).await;
    assert_eq!(initialized["method"], "initialized");

    loop {
        let request = read_exec_server_json(&mut websocket).await;
        match request["method"].as_str() {
            Some("environment/info") => {
                send_exec_server_json(
                    &mut websocket,
                    json!({
                        "id": request["id"],
                        "result": {
                            "shell": { "name": "sh", "path": "/bin/sh" },
                            "capabilities": {
                                "initialStdin": supports_initial_stdin,
                            }
                        }
                    }),
                )
                .await;
            }
            Some("fs/getMetadata") => {
                send_exec_server_json(
                    &mut websocket,
                    json!({
                        "id": request["id"],
                        "error": { "code": -32004, "message": "not found" }
                    }),
                )
                .await;
            }
            Some("fs/canonicalize") => {
                send_exec_server_json(
                    &mut websocket,
                    json!({
                        "id": request["id"],
                        "result": { "path": request["params"]["path"] }
                    }),
                )
                .await;
            }
            Some("fs/walk") => {
                send_exec_server_json(
                    &mut websocket,
                    json!({
                        "id": request["id"],
                        "result": { "entries": [], "errors": [], "truncated": false }
                    }),
                )
                .await;
            }
            Some("process/start") => {
                process_started.store(true, Ordering::Release);
                assert!(
                    supports_initial_stdin,
                    "legacy exec-server must not receive process/start"
                );
                let process_id = request["params"]["processId"]
                    .as_str()
                    .expect("processId")
                    .to_string();
                send_exec_server_json(
                    &mut websocket,
                    json!({
                        "id": request["id"],
                        "result": { "processId": &process_id }
                    }),
                )
                .await;
                send_exec_server_json(
                    &mut websocket,
                    json!({
                        "method": "process/output",
                        "params": {
                            "processId": &process_id,
                            "seq": 1,
                            "stream": "stdout",
                            "chunk": BASE64_STANDARD.encode("remote stdin accepted\n"),
                        }
                    }),
                )
                .await;
                send_exec_server_json(
                    &mut websocket,
                    json!({
                        "method": "process/exited",
                        "params": {
                            "processId": &process_id,
                            "seq": 2,
                            "exitCode": 0,
                            "sandboxDenied": false,
                        }
                    }),
                )
                .await;
                send_exec_server_json(
                    &mut websocket,
                    json!({
                        "method": "process/closed",
                        "params": { "processId": &process_id, "seq": 3 }
                    }),
                )
                .await;
                return request;
            }
            method => panic!("unexpected exec-server request: {method:?}"),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_command_stdin_passes_multiline_utf8_and_eof() -> Result<()> {
    skip_if_target_windows!(Ok(()), "uses POSIX cat");
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let harness = TestCodexHarness::with_auto_env_builder(unified_exec_builder()).await?;
    let call_id = "exec-command-stdin-yaml";
    let stdin = concat!(
        "qn: \"QUALIFIED NAME\"\n",
        "add:\n",
        "  - \"ADDED CHILD QN\"\n",
        "del:\n",
        "  - \"REMOVED CHILD QN\"\n",
    );
    mount_exec_command(
        &harness,
        call_id,
        json!({
            "cmd": "cat",
            "stdin": stdin,
            "yield_time_ms": 1_000,
        }),
    )
    .await?;

    harness
        .submit_with_permission_profile("send multiline stdin", PermissionProfile::Disabled)
        .await?;

    let output = harness.function_call_stdout(call_id).await;
    assert!(output.contains(stdin), "unexpected output: {output:?}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_command_stdin_does_not_add_a_newline() -> Result<()> {
    skip_if_target_windows!(Ok(()), "uses POSIX wc");
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let harness = TestCodexHarness::with_auto_env_builder(unified_exec_builder()).await?;
    let call_id = "exec-command-stdin-byte-count";
    mount_exec_command(
        &harness,
        call_id,
        json!({
            "cmd": "wc -c",
            "stdin": "hello",
            "yield_time_ms": 1_000,
        }),
    )
    .await?;

    harness
        .submit_with_permission_profile("count stdin bytes", PermissionProfile::Disabled)
        .await?;

    let output = harness.function_call_stdout(call_id).await;
    assert!(output.lines().any(|line| line.trim() == "5"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_command_stdin_preserves_an_explicit_newline() -> Result<()> {
    skip_if_target_windows!(Ok(()), "uses POSIX wc");
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let harness = TestCodexHarness::with_auto_env_builder(unified_exec_builder()).await?;
    let call_id = "exec-command-stdin-newline-byte-count";
    mount_exec_command(
        &harness,
        call_id,
        json!({
            "cmd": "wc -c",
            "stdin": "hello\n",
            "yield_time_ms": 1_000,
        }),
    )
    .await?;

    harness
        .submit_with_permission_profile(
            "count stdin bytes with newline",
            PermissionProfile::Disabled,
        )
        .await?;

    let output = harness.function_call_stdout(call_id).await;
    assert!(output.lines().any(|line| line.trim() == "6"));
    Ok(())
}

/// Проверяет, что управляемый одноразовый запуск передаёт начальный текст и завершает поток EOF.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_command_stdin_reaches_managed_one_shot_and_eof() -> Result<()> {
    skip_if_target_windows!(Ok(()), "uses POSIX wc");
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let builder = unified_exec_builder().with_cloud_config_bundle(
        CloudConfigBundleFixture::loader_with_enterprise_requirement(
            r#"
[features]
unified_exec = false
shell_tool = true
"#,
        ),
    );
    let harness = TestCodexHarness::with_auto_env_builder(builder).await?;
    let call_id = "exec-command-stdin-one-shot";
    mount_exec_command(
        &harness,
        call_id,
        json!({
            "cmd": "wc -c",
            "stdin": "hello",
            "timeout_ms": 1_000,
        }),
    )
    .await?;

    harness
        .submit_with_permission_profile("count one-shot stdin bytes", PermissionProfile::Disabled)
        .await?;

    let output = harness.function_call_stdout(call_id).await;
    assert!(output.lines().any(|line| line.trim() == "5"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_command_stdin_omission_preserves_closed_non_tty_stdin() -> Result<()> {
    skip_if_target_windows!(Ok(()), "uses POSIX shell read");
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let harness = TestCodexHarness::with_auto_env_builder(unified_exec_builder()).await?;
    let call_id = "exec-command-stdin-omitted";
    mount_exec_command(
        &harness,
        call_id,
        json!({
            "cmd": "if IFS= read -r line; then printf 'read:%s\\n' \"$line\"; else printf 'eof\\n'; fi",
            "yield_time_ms": 1_000,
        }),
    )
    .await?;

    harness
        .submit_with_permission_profile("observe omitted stdin", PermissionProfile::Disabled)
        .await?;

    let output = harness.function_call_stdout(call_id).await;
    assert!(output.lines().any(|line| line.trim() == "eof"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_command_stdin_keeps_tty_open_for_write_stdin() -> Result<()> {
    skip_if_target_windows!(Ok(()), "uses POSIX shell and PTY semantics");
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let harness = TestCodexHarness::with_auto_env_builder(unified_exec_builder()).await?;
    let start_call_id = "exec-command-stdin-tty-start";
    let write_call_id = "exec-command-stdin-tty-write";
    mount_sse_sequence(
        harness.server(),
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_function_call(
                    start_call_id,
                    "exec_command",
                    &serde_json::to_string(&json!({
                        "cmd": "IFS= read -r first; printf 'first:%s\\n' \"$first\"; IFS= read -r second; printf 'second:%s\\n' \"$second\"",
                        "stdin": "one\n",
                        "tty": true,
                        "yield_time_ms": 200,
                    }))?,
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_function_call(
                    write_call_id,
                    "write_stdin",
                    &serde_json::to_string(&json!({
                        "session_id": 1000,
                        "chars": "two\n",
                        "yield_time_ms": 1_000,
                    }))?,
                ),
                ev_completed("resp-2"),
            ]),
            sse(vec![
                ev_response_created("resp-3"),
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-3"),
            ]),
        ],
    )
    .await;

    harness
        .submit_with_permission_profile("continue initial tty stdin", PermissionProfile::Disabled)
        .await?;

    let start_output = harness.function_call_stdout(start_call_id).await;
    let write_output = harness.function_call_stdout(write_call_id).await;
    assert!(
        start_output.contains("first:one"),
        "unexpected initial output: {start_output:?}"
    );
    assert!(
        write_output.contains("second:two"),
        "unexpected continued output: {write_output:?}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_command_stdin_rejects_intercepted_apply_patch_before_mutation() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));
    skip_if_target_windows!(Ok(()), "uses a POSIX apply_patch heredoc");

    let harness = TestCodexHarness::with_builder(unified_exec_builder()).await?;
    let call_id = "exec-command-stdin-apply-patch";
    let patch =
        "*** Begin Patch\n*** Add File: stdin-should-not-exist.txt\n+unexpected\n*** End Patch";
    mount_exec_command(
        &harness,
        call_id,
        json!({
            "cmd": format!("apply_patch <<'EOF'\n{patch}\nEOF\n"),
            "stdin": "must not be ignored",
        }),
    )
    .await?;

    harness
        .submit_with_permission_profile("reject ambiguous patch input", PermissionProfile::Disabled)
        .await?;

    let output = harness.function_call_stdout(call_id).await;
    assert!(
        output.contains("cannot combine intercepted apply_patch with `stdin`"),
        "unexpected output: {output:?}"
    );
    assert!(!harness.path_exists("stdin-should-not-exist.txt").await?);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_command_stdin_remote_start_carries_initial_text() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let exec_server_url = format!("ws://{}", listener.local_addr()?);
    let process_started = Arc::new(AtomicBool::new(false));
    let exec_server = tokio::spawn(serve_initial_stdin_exec_server(
        listener,
        /*supports_initial_stdin*/ true,
        Arc::clone(&process_started),
    ));
    let harness = TestCodexHarness::with_builder(
        unified_exec_builder().with_exec_server_url(exec_server_url),
    )
    .await?;
    let call_id = "exec-command-stdin-remote";
    let stdin = "first\nsecond\n";
    mount_exec_command(
        &harness,
        call_id,
        json!({
            "cmd": "cat",
            "stdin": stdin,
            "yield_time_ms": 1_000,
        }),
    )
    .await?;

    harness
        .submit_with_permission_profile("send remote stdin", PermissionProfile::Disabled)
        .await?;

    let process_start = timeout(Duration::from_secs(5), exec_server)
        .await
        .expect("fake exec-server should receive process/start")
        .expect("fake exec-server task should succeed");
    assert!(process_started.load(Ordering::Acquire));
    assert_eq!(process_start["params"]["initialStdin"], stdin);
    assert_eq!(process_start["params"]["pipeStdin"], false);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_command_stdin_rejects_legacy_remote_before_process_start() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_sandbox!(Ok(()));

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let exec_server_url = format!("ws://{}", listener.local_addr()?);
    let process_started = Arc::new(AtomicBool::new(false));
    let exec_server = tokio::spawn(serve_initial_stdin_exec_server(
        listener,
        /*supports_initial_stdin*/ false,
        Arc::clone(&process_started),
    ));
    let harness = TestCodexHarness::with_builder(
        unified_exec_builder().with_exec_server_url(exec_server_url),
    )
    .await?;
    let call_id = "exec-command-stdin-legacy-remote";
    mount_exec_command(
        &harness,
        call_id,
        json!({
            "cmd": "cat",
            "stdin": "must not be discarded",
        }),
    )
    .await?;

    harness
        .submit_with_permission_profile("reject legacy remote", PermissionProfile::Disabled)
        .await?;

    let output = harness.function_call_stdout(call_id).await;
    assert!(
        output.contains("selected exec-server does not support initial stdin"),
        "unexpected output: {output:?}"
    );
    assert!(!process_started.load(Ordering::Acquire));
    exec_server.abort();
    Ok(())
}
