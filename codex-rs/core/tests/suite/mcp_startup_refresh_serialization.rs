//! Проверяет замену обычного MCP-сервера после отказа общего startup.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::Context;
use codex_config::types::McpServerAuth;
use codex_config::types::McpServerConfig;
use codex_config::types::McpServerTransportConfig;
use codex_core::TurnInputRequest;
use codex_core::config::Config;
use codex_file_system::WriteFileOptions;
use codex_protocol::models::PermissionProfile;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::McpStartupCompleteEvent;
use codex_protocol::protocol::McpStartupStatus;
use codex_protocol::protocol::ThreadSettingsOverrides;
use codex_protocol::user_input::UserInput;
use codex_utils_path_uri::PathUri;
use core_test_support::responses;
use core_test_support::skip_if_no_network;
use core_test_support::skip_if_wine_exec;
use core_test_support::test_codex::TestCodex;
use core_test_support::test_codex::test_codex;
use core_test_support::test_codex::turn_permission_fields;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use wiremock::MockServer;

use super::rmcp_client::remote_aware_environment_id;
use super::rmcp_client::remote_aware_stdio_server_bin;

const BARRIER_FILE: &str = "allow-startup-refresh-initialize";
const MCP_SERVER_NAME: &str = "startup_refresh";
const OBSERVATION_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Default, PartialEq, Eq)]
struct ObservedStartupEvents {
    starts_after_initial: usize,
    summaries: Vec<ObservedStartupSummary>,
}

impl ObservedStartupEvents {
    /// Сохраняет только события startup, различающие повторный запуск и переиспользование.
    fn record(&mut self, event: &EventMsg) {
        match event {
            EventMsg::McpStartupUpdate(update)
                if update.server == MCP_SERVER_NAME
                    && matches!(update.status, McpStartupStatus::Starting) =>
            {
                self.starts_after_initial += 1;
            }
            EventMsg::McpStartupComplete(summary) => {
                self.summaries.push(ObservedStartupSummary::from(summary));
            }
            _ => {}
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ObservedStartupSummary {
    ready: Vec<String>,
    failed: Vec<String>,
    cancelled: Vec<String>,
}

impl From<&McpStartupCompleteEvent> for ObservedStartupSummary {
    fn from(summary: &McpStartupCompleteEvent) -> Self {
        Self {
            ready: summary.ready.clone(),
            failed: summary
                .failed
                .iter()
                .map(|failure| failure.server.clone())
                .collect(),
            cancelled: summary.cancelled.clone(),
        }
    }
}

/// Хранит thread, макет модели и путь к barrier-файлу в среде executor.
struct GatedStartupFixture {
    server: MockServer,
    test: TestCodex,
    barrier: PathUri,
}

/// Добавляет необязательный stdio server с управляемым через barrier вызовом initialize.
fn insert_gated_mcp_server(
    config: &mut Config,
    command: String,
    environment_id: String,
    startup_timeout: Duration,
) {
    let barrier_file = config.cwd.join(BARRIER_FILE).to_string_lossy().into_owned();
    let mut servers = config.mcp_servers.get().clone();
    servers.insert(
        MCP_SERVER_NAME.to_string(),
        McpServerConfig {
            transport: McpServerTransportConfig::Stdio {
                command,
                args: Vec::new(),
                env: Some(HashMap::from([(
                    "MCP_TEST_INITIALIZE_BARRIER_FILE".to_string(),
                    barrier_file,
                )])),
                env_vars: Vec::new(),
                cwd: None,
            },
            auth: McpServerAuth::default(),
            environment_id,
            enabled: true,
            required: false,
            supports_parallel_tool_calls: false,
            omit_tools_from: None,
            disabled_reason: None,
            startup_timeout_sec: Some(startup_timeout),
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
        .expect("test MCP servers should accept any configuration");
}

/// Запускает thread и останавливает первое рукопожатие MCP до проверки refresh.
async fn build_gated_startup_fixture(
    startup_timeout: Duration,
) -> anyhow::Result<GatedStartupFixture> {
    let server = responses::start_mock_server().await;
    let command = remote_aware_stdio_server_bin()?;
    let environment_id = remote_aware_environment_id();
    let mut builder = test_codex().with_config(move |config| {
        insert_gated_mcp_server(
            config,
            command.clone(),
            environment_id.clone(),
            startup_timeout,
        );
    });
    let test = builder.build_with_auto_env(&server).await?;
    let barrier = PathUri::from_host_native_path(test.config.cwd.join(BARRIER_FILE))?;

    wait_for_event(&test.codex, |event| {
        matches!(
            event,
            EventMsg::McpStartupUpdate(update)
                if update.server == MCP_SERVER_NAME
                    && matches!(update.status, McpStartupStatus::Starting)
        )
    })
    .await;

    Ok(GatedStartupFixture {
        server,
        test,
        barrier,
    })
}

/// Формирует короткий turn с разрешениями только на чтение для проверки грязного MCP runtime.
fn read_only_user_turn(fixture: &TestCodex, text: impl Into<String>) -> TurnInputRequest {
    let (sandbox_policy, permission_profile) =
        turn_permission_fields(PermissionProfile::read_only(), fixture.config.cwd.as_path());
    TurnInputRequest::user_input(vec![UserInput::Text {
        text: text.into(),
        text_elements: Vec::new(),
    }])
    .with_thread_settings(ThreadSettingsOverrides {
        approval_policy: Some(AskForApproval::Never),
        sandbox_policy: Some(sandbox_policy),
        permission_profile,
        ..Default::default()
    })
}

/// Выполняет turn до бездействия и сохраняет события startup перед `TurnComplete`.
async fn run_turn_and_observe_startup(
    fixture: &GatedStartupFixture,
    label: &str,
    observed: &mut ObservedStartupEvents,
) -> anyhow::Result<()> {
    let response_id = format!("resp-{label}");
    let response = responses::mount_sse_once(
        &fixture.server,
        responses::sse(vec![
            responses::ev_response_created(&response_id),
            responses::ev_assistant_message(&format!("msg-{label}"), "done"),
            responses::ev_completed(&response_id),
        ]),
    )
    .await;
    fixture
        .test
        .codex
        .start_or_steer_turn(read_only_user_turn(&fixture.test, label))
        .await?;

    tokio::time::timeout(OBSERVATION_TIMEOUT, async {
        loop {
            let event = fixture.test.codex.next_event().await?;
            let turn_complete = matches!(event.msg, EventMsg::TurnComplete(_));
            observed.record(&event.msg);
            if turn_complete {
                break;
            }
        }
        Ok::<(), anyhow::Error>(())
    })
    .await
    .context("turn should complete while optional MCP startup remains gated")??;
    response.single_request();
    Ok(())
}

/// Собирает события startup до заданного состояния без синхронизации через `sleep`.
async fn observe_startup_until(
    fixture: &GatedStartupFixture,
    observed: &mut ObservedStartupEvents,
    condition: impl Fn(&ObservedStartupEvents) -> bool,
    timeout_message: &'static str,
) -> anyhow::Result<()> {
    let result = tokio::time::timeout(OBSERVATION_TIMEOUT, async {
        while !condition(observed) {
            let event = fixture.test.codex.next_event().await?;
            observed.record(&event.msg);
        }
        Ok::<(), anyhow::Error>(())
    })
    .await;
    result.with_context(|| format!("{timeout_message}; observed {observed:?}"))??;
    Ok(())
}

/// Создаёт barrier-файл через файловую систему executor и завершает рукопожатие.
async fn release_startup(fixture: &GatedStartupFixture) -> anyhow::Result<()> {
    fixture
        .test
        .fs()
        .write_file(
            &fixture.barrier,
            b"ready".to_vec(),
            WriteFileOptions::default(),
            /*sandbox*/ None,
        )
        .await?;
    Ok(())
}

fn ready_summary() -> ObservedStartupSummary {
    ObservedStartupSummary {
        ready: vec![MCP_SERVER_NAME.to_string()],
        failed: Vec::new(),
        cancelled: Vec::new(),
    }
}

fn failed_summary() -> ObservedStartupSummary {
    ObservedStartupSummary {
        ready: Vec::new(),
        failed: vec![MCP_SERVER_NAME.to_string()],
        cancelled: Vec::new(),
    }
}

/// Исход `Failed` у общего startup запускает одну замену обычного MCP-сервера.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mcp_refresh_continues_after_initial_startup_failure() -> anyhow::Result<()> {
    skip_if_wine_exec!(
        Ok(()),
        "requires a Windows test_stdio_server in the Wine-exec environment"
    );
    skip_if_no_network!(Ok(()));

    let fixture = build_gated_startup_fixture(Duration::from_secs(2)).await?;
    fixture.test.thread_manager.invalidate_mcp_runtimes().await;
    let mut observed = ObservedStartupEvents::default();

    run_turn_and_observe_startup(&fixture, "failed-startup", &mut observed).await?;
    observe_startup_until(
        &fixture,
        &mut observed,
        |events| events.summaries.len() >= 2 && events.starts_after_initial == 1,
        "failed shared startup should publish its views and start one replacement",
    )
    .await?;
    let failed_summary = failed_summary();
    let failed_view_count = observed.summaries.len();
    assert!(failed_view_count >= 2);
    assert_eq!(
        observed,
        ObservedStartupEvents {
            starts_after_initial: 1,
            summaries: vec![failed_summary.clone(); failed_view_count],
        }
    );

    release_startup(&fixture).await?;
    let ready_summary = ready_summary();
    observe_startup_until(
        &fixture,
        &mut observed,
        |events| events.summaries.last() == Some(&ready_summary),
        "replacement startup should complete after the barrier is released",
    )
    .await?;
    let mut expected_summaries = vec![failed_summary; failed_view_count];
    expected_summaries.push(ready_summary);
    assert_eq!(
        observed,
        ObservedStartupEvents {
            starts_after_initial: 1,
            summaries: expected_summaries,
        }
    );

    fixture.test.codex.shutdown_and_wait().await?;
    Ok(())
}
