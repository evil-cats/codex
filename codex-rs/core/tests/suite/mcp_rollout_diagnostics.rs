use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command as StdCommand;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Context as _;
use anyhow::ensure;
use codex_config::McpServerConfig;
use codex_config::types::McpServerAuth;
use codex_config::types::McpServerTransportConfig;
use codex_core::config::Config;
use codex_protocol::models::PermissionProfile;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::McpDiagnosticEvent;
use codex_protocol::protocol::McpDiagnosticItem;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::RolloutItem;
use codex_protocol::protocol::RolloutLine;
use codex_protocol::user_input::UserInput;
use codex_utils_path_uri::LegacyAppPathString;
use core_test_support::responses;
use core_test_support::skip_if_no_network;
use core_test_support::skip_if_wine_exec;
use core_test_support::stdio_server_bin;
use core_test_support::test_codex::TestCodex;
use core_test_support::test_codex::test_codex;
use core_test_support::test_codex::turn_permission_fields;
use core_test_support::test_docker_container_name;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use serial_test::serial;
use wiremock::MockServer;

const REMOTE_MCP_ENVIRONMENT: &str = "remote";

fn remote_aware_environment_id() -> String {
    if core_test_support::is_remote_test_environment() {
        REMOTE_MCP_ENVIRONMENT.to_string()
    } else {
        codex_config::DEFAULT_MCP_SERVER_ENVIRONMENT_ID.to_string()
    }
}

fn remote_aware_stdio_server_bin() -> anyhow::Result<String> {
    let bin = stdio_server_bin()?;
    let Some(container_name) = test_docker_container_name() else {
        return Ok(bin);
    };

    copy_binary_to_remote_env(&container_name, Path::new(&bin), "test_stdio_server")
}

fn unique_remote_path(binary_name: &str) -> anyhow::Result<String> {
    let unique_suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(format!(
        "/tmp/codex-remote-env/{binary_name}-{}-{unique_suffix}",
        std::process::id()
    ))
}

fn copy_binary_to_remote_env(
    container_name: &str,
    host_path: &Path,
    binary_name: &str,
) -> anyhow::Result<String> {
    let remote_path = unique_remote_path(binary_name)?;
    let mkdir_output = StdCommand::new("docker")
        .args([
            "exec",
            container_name,
            "mkdir",
            "-p",
            "/tmp/codex-remote-env",
        ])
        .output()
        .context("create remote MCP test binary directory")?;
    ensure!(
        mkdir_output.status.success(),
        "docker mkdir remote MCP test binary directory failed: stdout={} stderr={}",
        String::from_utf8_lossy(&mkdir_output.stdout).trim(),
        String::from_utf8_lossy(&mkdir_output.stderr).trim()
    );

    let container_target = format!("{container_name}:{remote_path}");
    let copy_output = StdCommand::new("docker")
        .arg("cp")
        .arg(host_path)
        .arg(&container_target)
        .output()
        .with_context(|| {
            format!(
                "copy {} to remote MCP test env",
                host_path.to_string_lossy()
            )
        })?;
    ensure!(
        copy_output.status.success(),
        "docker cp {binary_name} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&copy_output.stdout).trim(),
        String::from_utf8_lossy(&copy_output.stderr).trim()
    );

    let chmod_output = StdCommand::new("docker")
        .args(["exec", container_name, "chmod", "+x", remote_path.as_str()])
        .output()
        .with_context(|| format!("mark remote {binary_name} executable"))?;
    ensure!(
        chmod_output.status.success(),
        "docker chmod {binary_name} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&chmod_output.stdout).trim(),
        String::from_utf8_lossy(&chmod_output.stderr).trim()
    );

    Ok(remote_path)
}

fn stdio_transport(
    command: String,
    env: Option<HashMap<String, String>>,
) -> McpServerTransportConfig {
    McpServerTransportConfig::Stdio {
        command,
        args: Vec::new(),
        env,
        env_vars: Vec::new(),
        cwd: None::<PathBuf>.map(|cwd| LegacyAppPathString::from_path(&cwd)),
    }
}

fn insert_mcp_server(config: &mut Config, server_name: &str, transport: McpServerTransportConfig) {
    let mut servers = config.mcp_servers.get().clone();
    servers.insert(
        server_name.to_string(),
        McpServerConfig {
            transport,
            auth: McpServerAuth::default(),
            environment_id: remote_aware_environment_id(),
            enabled: true,
            required: false,
            supports_parallel_tool_calls: false,
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

fn read_only_user_turn(fixture: &TestCodex, text: impl Into<String>) -> Op {
    let cwd = fixture.config.cwd.clone();
    let (sandbox_policy, permission_profile) =
        turn_permission_fields(PermissionProfile::read_only(), cwd.as_path());
    Op::UserInput {
        items: vec![UserInput::Text {
            text: text.into(),
            text_elements: Vec::new(),
        }],
        final_output_json_schema: None,
        responsesapi_client_metadata: None,
        additional_context: Default::default(),
        thread_settings: codex_protocol::protocol::ThreadSettingsOverrides {
            approval_policy: Some(AskForApproval::Never),
            sandbox_policy: Some(sandbox_policy),
            permission_profile,
            collaboration_mode: None,
            ..Default::default()
        },
    }
}

async fn read_mcp_diagnostics(fixture: &TestCodex) -> anyhow::Result<Vec<McpDiagnosticItem>> {
    fixture.codex.ensure_rollout_materialized().await;
    fixture.codex.flush_rollout().await?;
    let rollout_path = fixture.codex.rollout_path().context("rollout path")?;
    let rollout = std::fs::read_to_string(&rollout_path)
        .with_context(|| format!("read rollout file at {}", rollout_path.display()))?;
    Ok(rollout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str::<RolloutLine>)
        .collect::<serde_json::Result<Vec<_>>>()?
        .into_iter()
        .filter_map(|line| match line.item {
            RolloutItem::McpDiagnostic(item) => Some(item),
            _ => None,
        })
        .collect::<Vec<_>>())
}

fn unique_state_file(label: &str) -> String {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();
    format!(
        "/tmp/codex-mcp-{label}-{}-{unique_suffix}",
        std::process::id()
    )
}

async fn call_tool_turn(
    server: &MockServer,
    fixture: &TestCodex,
    server_name: &str,
    tool_name: &str,
    call_id: &str,
    prompt: &str,
) -> anyhow::Result<Value> {
    let namespace = format!("mcp__{server_name}");
    let tool_response_id = format!("resp-tool-{call_id}");
    responses::mount_sse_once(
        server,
        responses::sse(vec![
            responses::ev_response_created(&tool_response_id),
            responses::ev_function_call_with_namespace(call_id, &namespace, tool_name, "{}"),
            responses::ev_completed(&tool_response_id),
        ]),
    )
    .await;
    let final_message_id = format!("msg-final-{call_id}");
    let final_response_id = format!("resp-final-{call_id}");
    let final_mock = responses::mount_sse_once(
        server,
        responses::sse(vec![
            responses::ev_assistant_message(&final_message_id, "done"),
            responses::ev_completed(&final_response_id),
        ]),
    )
    .await;

    fixture
        .codex
        .submit(read_only_user_turn(fixture, prompt))
        .await?;

    let end_event = wait_for_event(&fixture.codex, |ev| {
        matches!(ev, EventMsg::McpToolCallEnd(_))
    })
    .await;
    let EventMsg::McpToolCallEnd(end) = end_event else {
        unreachable!("event guard guarantees McpToolCallEnd");
    };
    assert_eq!(end.call_id, call_id);
    let result = end
        .result
        .as_ref()
        .expect("MCP call should recover and return success");
    assert_eq!(result.is_error, Some(false));

    wait_for_event(&fixture.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;
    let output_item = final_mock.single_request().function_call_output(call_id);
    let output_text = output_item
        .get("output")
        .and_then(Value::as_str)
        .expect("function_call_output output should be a string");
    let (_wall_time, payload) = output_text
        .split_once("Output:\n")
        .expect("wrapped MCP output should contain Output marker");
    Ok(serde_json::from_str(payload)?)
}

fn diagnostic_events(diagnostics: &[McpDiagnosticItem]) -> Vec<McpDiagnosticEvent> {
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.event)
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial(mcp_rollout_diagnostics)]
async fn mcp_transport_closed_persists_diagnostic_and_retries_once() -> anyhow::Result<()> {
    skip_if_wine_exec!(
        Ok(()),
        "requires a Windows test_stdio_server in the Wine-exec environment"
    );
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let server_name = "rmcp_diag_closed";
    let call_id = "mcp-diagnostic-transport-closed";
    let state_file = unique_state_file("transport-closed");
    let rmcp_test_server_bin = remote_aware_stdio_server_bin()?;

    let fixture = test_codex()
        .with_config(move |config| {
            insert_mcp_server(
                config,
                server_name,
                stdio_transport(
                    rmcp_test_server_bin,
                    Some(HashMap::from([(
                        "MCP_TEST_TRANSPORT_CLOSE_ONCE_STATE_FILE".to_string(),
                        state_file,
                    )])),
                ),
            );
        })
        .build_with_auto_env(&server)
        .await?;
    core_test_support::wait_for_mcp_server(&fixture.codex, server_name).await?;

    let output = call_tool_turn(
        &server,
        &fixture,
        server_name,
        "flaky_recovery",
        call_id,
        "call flaky recovery",
    )
    .await?;
    assert_eq!(output, json!({ "result": "recovered" }));

    let diagnostics = read_mcp_diagnostics(&fixture).await?;
    let events = diagnostic_events(&diagnostics);
    assert!(events.contains(&McpDiagnosticEvent::ProcessStarted));
    assert!(events.contains(&McpDiagnosticEvent::RecoveryStarted));
    assert!(events.contains(&McpDiagnosticEvent::RecoverySucceeded));

    let transport_diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            matches!(
                diagnostic.event,
                McpDiagnosticEvent::TransportClosed | McpDiagnosticEvent::TransportBrokenPipe
            )
        })
        .expect("transport failure diagnostic should be persisted");
    assert_eq!(transport_diagnostic.call_id.as_deref(), Some(call_id));
    assert_eq!(
        transport_diagnostic.tool_name.as_deref(),
        Some("flaky_recovery")
    );
    assert_eq!(transport_diagnostic.server_name, server_name);
    assert!(
        transport_diagnostic
            .stderr_tail
            .as_deref()
            .is_some_and(|tail| tail.contains("mcp flaky_recovery forced transport close")),
        "transport diagnostic should include bounded stderr tail: {transport_diagnostic:?}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial(mcp_rollout_diagnostics)]
async fn mcp_transport_closed_replay_failure_is_not_retried_again() -> anyhow::Result<()> {
    skip_if_wine_exec!(
        Ok(()),
        "requires a Windows test_stdio_server in the Wine-exec environment"
    );
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let server_name = "rmcp_diag_replay_closed";
    let call_id = "mcp-diagnostic-replay-closed";
    let rmcp_test_server_bin = remote_aware_stdio_server_bin()?;

    let fixture = test_codex()
        .with_config(move |config| {
            insert_mcp_server(
                config,
                server_name,
                stdio_transport(
                    rmcp_test_server_bin,
                    Some(HashMap::from([(
                        "MCP_TEST_TRANSPORT_CLOSE_ALWAYS".to_string(),
                        "1".to_string(),
                    )])),
                ),
            );
        })
        .build_with_auto_env(&server)
        .await?;
    core_test_support::wait_for_mcp_server(&fixture.codex, server_name).await?;

    let namespace = format!("mcp__{server_name}");
    responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("resp-tool"),
            responses::ev_function_call_with_namespace(call_id, &namespace, "flaky_recovery", "{}"),
            responses::ev_completed("resp-tool"),
        ]),
    )
    .await;
    responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_assistant_message("msg-final", "done"),
            responses::ev_completed("resp-final"),
        ]),
    )
    .await;

    fixture
        .codex
        .submit(read_only_user_turn(
            &fixture,
            "call repeatedly failing recovery",
        ))
        .await?;

    let end_event = wait_for_event(&fixture.codex, |ev| {
        matches!(ev, EventMsg::McpToolCallEnd(_))
    })
    .await;
    let EventMsg::McpToolCallEnd(end) = end_event else {
        unreachable!("event guard guarantees McpToolCallEnd");
    };
    assert_eq!(end.call_id, call_id);
    assert!(end.result.is_err(), "second transport close should escape");
    wait_for_event(&fixture.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let diagnostics = read_mcp_diagnostics(&fixture).await?;
    let recovery_failures = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.event == McpDiagnosticEvent::RecoveryFailed)
        .collect::<Vec<_>>();
    assert_eq!(recovery_failures.len(), 1);
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.event == McpDiagnosticEvent::RecoveryStarted)
            .count(),
        1,
        "replay failure must not start a second recovery loop"
    );
    let recovery_failure = recovery_failures[0];
    assert_eq!(recovery_failure.call_id.as_deref(), Some(call_id));
    assert_eq!(
        recovery_failure.tool_name.as_deref(),
        Some("flaky_recovery")
    );
    assert_eq!(recovery_failure.server_name, server_name);
    assert!(recovery_failure.old_launch_id.is_some());
    assert!(recovery_failure.new_launch_id.is_some());
    assert!(
        recovery_failure
            .stderr_tail
            .as_deref()
            .is_some_and(|tail| tail.contains("mcp flaky_recovery forced transport close")),
        "replay failure diagnostic should include bounded stderr tail: {recovery_failure:?}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial(mcp_rollout_diagnostics)]
async fn mcp_process_exit_marks_launch_dead_and_recovers_on_next_call() -> anyhow::Result<()> {
    skip_if_wine_exec!(
        Ok(()),
        "requires a Windows test_stdio_server in the Wine-exec environment"
    );
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let server_name = "rmcp_diag_exit";
    let schedule_call_id = "mcp-diagnostic-schedule-process-exit";
    let call_id = "mcp-diagnostic-process-exit";
    let state_file = unique_state_file("process-exit");
    let rmcp_test_server_bin = remote_aware_stdio_server_bin()?;

    let fixture = test_codex()
        .with_config(move |config| {
            insert_mcp_server(
                config,
                server_name,
                stdio_transport(
                    rmcp_test_server_bin,
                    Some(HashMap::from([(
                        "MCP_TEST_EXIT_AFTER_CALL_STATE_FILE".to_string(),
                        state_file,
                    )])),
                ),
            );
        })
        .build_with_auto_env(&server)
        .await?;
    core_test_support::wait_for_mcp_server(&fixture.codex, server_name).await?;

    let scheduled = call_tool_turn(
        &server,
        &fixture,
        server_name,
        "flaky_recovery",
        schedule_call_id,
        "schedule process exit after this call",
    )
    .await?;
    assert_eq!(scheduled, json!({ "result": "exit_scheduled" }));
    tokio::time::sleep(Duration::from_millis(200)).await;

    let output = call_tool_turn(
        &server,
        &fixture,
        server_name,
        "flaky_recovery",
        call_id,
        "call echo after process exit",
    )
    .await?;
    assert_eq!(output, json!({ "result": "recovered" }));

    let diagnostics = read_mcp_diagnostics(&fixture).await?;
    let events = diagnostic_events(&diagnostics);
    assert!(
        events.contains(&McpDiagnosticEvent::ProcessExited),
        "expected process exit diagnostic: {events:?}"
    );
    assert!(events.contains(&McpDiagnosticEvent::RecoveryStarted));
    assert!(events.contains(&McpDiagnosticEvent::RecoverySucceeded));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, McpDiagnosticEvent::TransportClosed)),
        "dead launch should recover before sending the next operation: {events:?}"
    );

    let process_exit = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.event == McpDiagnosticEvent::ProcessExited)
        .expect("process exit diagnostic should be persisted");
    assert_eq!(process_exit.call_id.as_deref(), Some(call_id));
    assert_eq!(process_exit.tool_name.as_deref(), Some("flaky_recovery"));
    assert!(
        process_exit
            .stderr_tail
            .as_deref()
            .is_some_and(|tail| tail.contains("mcp flaky_recovery scheduled process exit")),
        "process exit diagnostic should include bounded stderr tail: {process_exit:?}"
    );

    Ok(())
}
