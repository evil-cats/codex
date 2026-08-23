//! Проверяет восстановление транспорта MCP stdio без сохранения диагностики.

use std::collections::HashMap;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use codex_config::types::McpServerAuth;
use codex_config::types::McpServerConfig;
use codex_config::types::McpServerTransportConfig;
use codex_core::TurnInputRequest;
use codex_core::config::Config;
use codex_protocol::mcp::CallToolResult;
use codex_protocol::models::PermissionProfile;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::ThreadSettingsOverrides;
use codex_protocol::user_input::UserInput;
use core_test_support::responses;
use core_test_support::skip_if_no_network;
use core_test_support::skip_if_wine_exec;
use core_test_support::test_codex::TestCodex;
use core_test_support::test_codex::test_codex;
use core_test_support::test_codex::turn_permission_fields;
use core_test_support::wait_for_event;
use core_test_support::wait_for_mcp_server;
use pretty_assertions::assert_eq;
use serde_json::json;
use serial_test::serial;
use wiremock::MockServer;

use super::rmcp_client::remote_aware_environment_id;
use super::rmcp_client::remote_aware_stdio_server_bin;

const RECOVERY_CLOSE_COUNT_ENV: &str = "MCP_TEST_RECOVERY_CLOSE_COUNT";
const RECOVERY_CLOSE_STATE_FILE_ENV: &str = "MCP_TEST_RECOVERY_CLOSE_STATE_FILE";
const RECOVERY_EXIT_STATE_FILE_ENV: &str = "MCP_TEST_RECOVERY_EXIT_STATE_FILE";

fn insert_mcp_server(
    config: &mut Config,
    server_name: &str,
    command: String,
    env: HashMap<String, String>,
) {
    let mut servers = config.mcp_servers.get().clone();
    servers.insert(
        server_name.to_string(),
        McpServerConfig {
            transport: McpServerTransportConfig::Stdio {
                command,
                args: Vec::new(),
                env: Some(env),
                env_vars: Vec::new(),
                cwd: None,
            },
            auth: McpServerAuth::default(),
            environment_id: remote_aware_environment_id(),
            enabled: true,
            required: false,
            supports_parallel_tool_calls: false,
            omit_tools_from: None,
            disabled_reason: None,
            startup_timeout_sec: Some(Duration::from_secs(10)),
            tool_timeout_sec: None,
            default_tools_approval_mode: None,
            enabled_tools: None,
            disabled_tools: None,
            scopes: None,
            oauth: None,
            oauth_resource: None,
            tools: HashMap::new(),
        },
    );
    config
        .mcp_servers
        .set(servers)
        .expect("test mcp servers should accept any configuration");
}

fn read_only_user_turn(fixture: &TestCodex, text: impl Into<String>) -> TurnInputRequest {
    let cwd = fixture.config.cwd.clone();
    let (sandbox_policy, permission_profile) =
        turn_permission_fields(PermissionProfile::read_only(), cwd.as_path());
    TurnInputRequest::user_input(vec![UserInput::Text {
        text: text.into(),
        text_elements: Vec::new(),
    }])
    .with_thread_settings(ThreadSettingsOverrides {
        approval_policy: Some(AskForApproval::Never),
        sandbox_policy: Some(sandbox_policy),
        permission_profile,
        collaboration_mode: None,
        ..Default::default()
    })
}

fn unique_state_file(label: &str) -> String {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();
    std::env::temp_dir()
        .join(format!(
            "codex-mcp-recovery-{label}-{}-{unique_suffix}",
            std::process::id()
        ))
        .to_string_lossy()
        .into_owned()
}

async fn build_fixture(
    server_name: &'static str,
    env: HashMap<String, String>,
) -> anyhow::Result<(MockServer, TestCodex)> {
    let server = responses::start_mock_server().await;
    let command = remote_aware_stdio_server_bin()?;
    let fixture = test_codex()
        .with_config(move |config| {
            insert_mcp_server(config, server_name, command, env);
        })
        .build_with_auto_env(&server)
        .await?;
    wait_for_mcp_server(&fixture.codex, server_name).await?;
    Ok((server, fixture))
}

async fn call_recovery_probe(
    server: &MockServer,
    fixture: &TestCodex,
    server_name: &str,
    call_id: &str,
    prompt: &str,
) -> anyhow::Result<Result<CallToolResult, String>> {
    let namespace = format!("mcp__{server_name}");
    responses::mount_sse_once(
        server,
        responses::sse(vec![
            responses::ev_response_created(&format!("resp-tool-{call_id}")),
            responses::ev_function_call_with_namespace(call_id, &namespace, "recovery_probe", "{}"),
            responses::ev_completed(&format!("resp-tool-{call_id}")),
        ]),
    )
    .await;
    responses::mount_sse_once(
        server,
        responses::sse(vec![
            responses::ev_assistant_message(&format!("msg-final-{call_id}"), "done"),
            responses::ev_completed(&format!("resp-final-{call_id}")),
        ]),
    )
    .await;

    fixture
        .codex
        .start_or_steer_turn(read_only_user_turn(fixture, prompt))
        .await?;

    let end_event = wait_for_event(&fixture.codex, |event| {
        matches!(event, EventMsg::McpToolCallEnd(_))
    })
    .await;
    let EventMsg::McpToolCallEnd(end) = end_event else {
        unreachable!("event guard guarantees McpToolCallEnd");
    };
    assert_eq!(end.call_id, call_id);
    wait_for_event(&fixture.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    Ok(end.result)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial(mcp_transport_recovery)]
async fn mcp_transport_close_reinitializes_and_retries_once() -> anyhow::Result<()> {
    skip_if_wine_exec!(
        Ok(()),
        "requires a Windows test_stdio_server in the Wine-exec environment"
    );
    skip_if_no_network!(Ok(()));

    let server_name = "rmcp_recovery_close_once";
    let state_file = unique_state_file("close-once");
    let (server, fixture) = build_fixture(
        server_name,
        HashMap::from([
            (RECOVERY_CLOSE_COUNT_ENV.to_string(), "1".to_string()),
            (RECOVERY_CLOSE_STATE_FILE_ENV.to_string(), state_file),
        ]),
    )
    .await?;

    let result = call_recovery_probe(
        &server,
        &fixture,
        server_name,
        "mcp-recovery-close-once",
        "recover one closed transport",
    )
    .await?
    .expect("one transport close should recover");
    assert_eq!(
        result.structured_content,
        Some(json!({ "result": "recovered" }))
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial(mcp_transport_recovery)]
async fn mcp_transport_second_close_is_not_retried_again() -> anyhow::Result<()> {
    skip_if_wine_exec!(
        Ok(()),
        "requires a Windows test_stdio_server in the Wine-exec environment"
    );
    skip_if_no_network!(Ok(()));

    let server_name = "rmcp_recovery_second_close";
    let state_file = unique_state_file("close-twice");
    let (server, fixture) = build_fixture(
        server_name,
        HashMap::from([
            (RECOVERY_CLOSE_COUNT_ENV.to_string(), "2".to_string()),
            (RECOVERY_CLOSE_STATE_FILE_ENV.to_string(), state_file),
        ]),
    )
    .await?;

    // Третий запуск вернул бы успех, поэтому ошибка доказывает отсутствие
    // второго цикла восстановления после единственного повтора.
    let result = call_recovery_probe(
        &server,
        &fixture,
        server_name,
        "mcp-recovery-second-close",
        "do not recover a second closed transport",
    )
    .await?;
    assert!(result.is_err(), "second transport close should escape");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial(mcp_transport_recovery)]
async fn mcp_idle_process_exit_recovers_before_next_call() -> anyhow::Result<()> {
    skip_if_wine_exec!(
        Ok(()),
        "requires a Windows test_stdio_server in the Wine-exec environment"
    );
    skip_if_no_network!(Ok(()));

    let server_name = "rmcp_recovery_idle_exit";
    let state_file = unique_state_file("idle-exit");
    let (server, fixture) = build_fixture(
        server_name,
        HashMap::from([(RECOVERY_EXIT_STATE_FILE_ENV.to_string(), state_file)]),
    )
    .await?;

    let scheduled = call_recovery_probe(
        &server,
        &fixture,
        server_name,
        "mcp-recovery-schedule-exit",
        "schedule the MCP process exit",
    )
    .await?
    .expect("the scheduling call should succeed");
    assert_eq!(
        scheduled.structured_content,
        Some(json!({ "result": "exit_scheduled" }))
    );
    tokio::time::sleep(Duration::from_millis(200)).await;

    let recovered = call_recovery_probe(
        &server,
        &fixture,
        server_name,
        "mcp-recovery-after-idle-exit",
        "recover the exited MCP process",
    )
    .await?
    .expect("the next call should recover the exited process");
    assert_eq!(
        recovered.structured_content,
        Some(json!({ "result": "recovered" }))
    );
    Ok(())
}
