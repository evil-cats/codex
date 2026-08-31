//! Проверяет восстановление MCP stdio напрямую через публичный `RmcpClient`.
//!
//! Сценарии запускают управляемый `test_stdio_server` с отдельным временным
//! состоянием. Они наблюдают число реальных запусков, инициализаций и вызовов
//! инструмента, поэтому отличают успешное восстановление от лишнего повторного
//! запуска или скрытого повтора исходной операции.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use codex_rmcp_client::ElicitationAction;
use codex_rmcp_client::ElicitationResponse;
use codex_rmcp_client::LocalStdioServerLauncher;
use codex_rmcp_client::RmcpClient;
use futures::FutureExt;
use pretty_assertions::assert_eq;
use rmcp::model::CallToolResult;
use rmcp::model::ClientCapabilities;
use rmcp::model::Implementation;
use rmcp::model::InitializeRequestParams;
use rmcp::model::ProtocolVersion;
use serde_json::json;
use tempfile::TempDir;

const RECOVERY_BROKEN_PIPE_STATE_FILE_ENV: &str = "MCP_TEST_RECOVERY_BROKEN_PIPE_STATE_FILE";
const RECOVERY_CALL_LOG_FILE_ENV: &str = "MCP_TEST_RECOVERY_CALL_LOG_FILE";
const RECOVERY_CLOSE_BARRIER_PARTICIPANTS_ENV: &str =
    "MCP_TEST_RECOVERY_CLOSE_BARRIER_PARTICIPANTS";
const RECOVERY_CLOSE_COUNT_ENV: &str = "MCP_TEST_RECOVERY_CLOSE_COUNT";
const RECOVERY_CLOSE_STATE_FILE_ENV: &str = "MCP_TEST_RECOVERY_CLOSE_STATE_FILE";
const RECOVERY_INITIALIZE_FAIL_AT_ENV: &str = "MCP_TEST_RECOVERY_INITIALIZE_FAIL_AT";
const RECOVERY_INITIALIZE_STATE_FILE_ENV: &str = "MCP_TEST_RECOVERY_INITIALIZE_STATE_FILE";
const RECOVERY_LAUNCH_LOG_FILE_ENV: &str = "MCP_TEST_RECOVERY_LAUNCH_LOG_FILE";
const RECOVERY_REMOVE_CWD_ON_CLOSE_ENV: &str = "MCP_TEST_RECOVERY_REMOVE_CWD_ON_CLOSE";
const TEST_EXIT_FILE_ENV: &str = "MCP_TEST_EXIT_FILE";
const OPERATION_TIMEOUT: Duration = Duration::from_secs(10);

/// Наблюдаемые события полного цикла восстановления.
#[derive(Debug, PartialEq, Eq)]
struct ObservedRecoveryState {
    launches: usize,
    initialize_attempts: u64,
    tool_calls: usize,
}

/// Изолированные каталоги, счётчики и журналы одного тестового сценария.
struct RecoveryPaths {
    _temp_dir: TempDir,
    server_cwd: PathBuf,
    launch_log: PathBuf,
    initialize_state: PathBuf,
    call_log: PathBuf,
    close_state: PathBuf,
    broken_pipe_state: PathBuf,
    exit_file: PathBuf,
}

impl RecoveryPaths {
    /// Создаёт изолированные пути для одного сценария восстановления.
    fn new() -> anyhow::Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let server_cwd = temp_dir.path().join("server-cwd");
        std::fs::create_dir(&server_cwd)?;
        Ok(Self {
            launch_log: temp_dir.path().join("launch.log"),
            initialize_state: temp_dir.path().join("initialize.state"),
            call_log: temp_dir.path().join("calls.log"),
            close_state: temp_dir.path().join("close.state"),
            broken_pipe_state: temp_dir.path().join("broken-pipe.state"),
            exit_file: temp_dir.path().join("exit.marker"),
            _temp_dir: temp_dir,
            server_cwd,
        })
    }

    /// Возвращает базовое окружение с общими журналами и заданным числом закрытий.
    fn environment(&self, close_count: u64) -> HashMap<OsString, OsString> {
        HashMap::from([
            (
                OsString::from(RECOVERY_CLOSE_COUNT_ENV),
                OsString::from(close_count.to_string()),
            ),
            (
                OsString::from(RECOVERY_CLOSE_STATE_FILE_ENV),
                self.close_state.as_os_str().to_owned(),
            ),
            (
                OsString::from(RECOVERY_CALL_LOG_FILE_ENV),
                self.call_log.as_os_str().to_owned(),
            ),
            (
                OsString::from(RECOVERY_INITIALIZE_STATE_FILE_ENV),
                self.initialize_state.as_os_str().to_owned(),
            ),
            (
                OsString::from(RECOVERY_LAUNCH_LOG_FILE_ENV),
                self.launch_log.as_os_str().to_owned(),
            ),
        ])
    }

    /// Считывает наблюдаемое число запусков, инициализаций и вызовов инструмента.
    fn observed_state(&self) -> anyhow::Result<ObservedRecoveryState> {
        Ok(ObservedRecoveryState {
            launches: count_log_lines(&self.launch_log)?,
            initialize_attempts: read_counter(&self.initialize_state)?,
            tool_calls: count_log_lines(&self.call_log)?,
        })
    }
}

/// Создаёт и инициализирует клиент с восстановимым локальным stdio transport.
async fn initialized_recovery_client(
    paths: &RecoveryPaths,
    env: HashMap<OsString, OsString>,
) -> anyhow::Result<RmcpClient> {
    let server = codex_utils_cargo_bin::cargo_bin("test_stdio_server")?;
    let client = RmcpClient::new_stdio_client(
        server.into(),
        Vec::new(),
        Some(env),
        &[],
        Some(paths.server_cwd.to_string_lossy().into_owned()),
        Arc::new(LocalStdioServerLauncher::new(std::env::current_dir()?)),
    )
    .await?;
    client
        .initialize(
            InitializeRequestParams::new(
                ClientCapabilities::default(),
                Implementation::new("stdio-recovery-test", "1.0.0"),
            )
            .with_protocol_version(ProtocolVersion::V_2025_06_18),
            Some(OPERATION_TIMEOUT),
            Box::new(|_, _| {
                async {
                    Ok(ElicitationResponse {
                        action: ElicitationAction::Decline,
                        content: None,
                        meta: None,
                    })
                }
                .boxed()
            }),
        )
        .await?;
    Ok(client)
}

/// Вызывает `recovery_probe` и возвращает полный MCP-результат операции.
async fn call_recovery_probe(client: &RmcpClient) -> anyhow::Result<CallToolResult> {
    client
        .call_tool(
            "recovery_probe".to_string(),
            Some(json!({})),
            /*meta*/ None,
            Some(OPERATION_TIMEOUT),
        )
        .await
}

/// Строит полный ожидаемый результат управляемого `recovery_probe`.
fn expected_probe_result(result: &str) -> CallToolResult {
    let mut response = CallToolResult::success(Vec::new());
    response.result_type = None;
    response.structured_content = Some(json!({ "result": result }));
    response
}

/// Считает отдельные события в дозаписываемом журнале `test_stdio_server`.
fn count_log_lines(path: &Path) -> anyhow::Result<usize> {
    match std::fs::read_to_string(path) {
        Ok(contents) => Ok(contents.lines().count()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error.into()),
    }
}

/// Считывает числовой счётчик `test_stdio_server`, считая отсутствующий файл нулём.
fn read_counter(path: &Path) -> anyhow::Result<u64> {
    match std::fs::read_to_string(path) {
        Ok(contents) => Ok(contents.trim().parse::<u64>()?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error.into()),
    }
}

/// Два вызова на одном закрытом сервисе создают один новый процесс и повторяются независимо.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_transport_failures_share_one_recovery_launch() -> anyhow::Result<()> {
    let paths = RecoveryPaths::new()?;
    let mut env = paths.environment(/*close_count*/ 2);
    env.insert(
        OsString::from(RECOVERY_CLOSE_BARRIER_PARTICIPANTS_ENV),
        OsString::from("2"),
    );
    let client = initialized_recovery_client(&paths, env).await?;

    let (first, second) = tokio::join!(call_recovery_probe(&client), call_recovery_probe(&client));
    assert_eq!(
        (first?, second?),
        (
            expected_probe_result("recovered"),
            expected_probe_result("recovered"),
        )
    );
    assert_eq!(
        paths.observed_state()?,
        ObservedRecoveryState {
            launches: 2,
            initialize_attempts: 2,
            tool_calls: 4,
        }
    );

    client.shutdown().await;
    Ok(())
}

/// Живой stdout исключает EOF, поэтому следующий запрос восстанавливается после `BrokenPipe`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transport_send_broken_pipe_reinitializes_and_retries_once() -> anyhow::Result<()> {
    let paths = RecoveryPaths::new()?;
    let mut env = paths.environment(/*close_count*/ 0);
    env.insert(
        OsString::from(RECOVERY_BROKEN_PIPE_STATE_FILE_ENV),
        paths.broken_pipe_state.as_os_str().to_owned(),
    );
    let client = initialized_recovery_client(&paths, env).await?;

    assert_eq!(
        call_recovery_probe(&client).await?,
        expected_probe_result("broken_pipe_scheduled")
    );
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        call_recovery_probe(&client).await?,
        expected_probe_result("recovered")
    );
    assert_eq!(
        paths.observed_state()?,
        ObservedRecoveryState {
            launches: 2,
            initialize_attempts: 2,
            tool_calls: 2,
        }
    );

    client.shutdown().await;
    Ok(())
}

/// Отправка `events/stream` после смерти stdio в простое использует общую
/// границу ленивого восстановления. Ровно два запуска и две инициализации
/// отличают восстановленный транспорт от ошибки старого сервиса и лишнего цикла.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn event_stream_dispatch_recovers_idle_stdio_process() -> anyhow::Result<()> {
    let paths = RecoveryPaths::new()?;
    let mut env = paths.environment(/*close_count*/ 0);
    env.insert(
        OsString::from(TEST_EXIT_FILE_ENV),
        paths.exit_file.as_os_str().to_owned(),
    );
    let client = initialized_recovery_client(&paths, env).await?;

    std::fs::write(&paths.exit_file, "exit")?;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let request = client
        .send_event_stream_request(Some(json!({"name": "recovery.idle"})))
        .await?;
    drop(request);
    assert_eq!(
        paths.observed_state()?,
        ObservedRecoveryState {
            launches: 2,
            initialize_attempts: 2,
            tool_calls: 0,
        }
    );

    client.shutdown().await;
    Ok(())
}

/// Ошибка создания транспорта не повторяет операцию, а следующий вызов видит старый мёртвый запуск.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transport_creation_failure_preserves_dead_state_for_next_call() -> anyhow::Result<()> {
    let paths = RecoveryPaths::new()?;
    let mut env = paths.environment(/*close_count*/ 1);
    env.insert(
        OsString::from(RECOVERY_REMOVE_CWD_ON_CLOSE_ENV),
        OsString::from("1"),
    );
    let client = initialized_recovery_client(&paths, env).await?;

    let failed = call_recovery_probe(&client).await;
    assert!(failed.is_err(), "missing cwd must fail transport creation");
    assert!(!paths.server_cwd.exists());
    assert_eq!(
        paths.observed_state()?,
        ObservedRecoveryState {
            launches: 1,
            initialize_attempts: 1,
            tool_calls: 1,
        }
    );

    std::fs::create_dir(&paths.server_cwd)?;
    assert_eq!(
        call_recovery_probe(&client).await?,
        expected_probe_result("recovered")
    );
    assert_eq!(
        paths.observed_state()?,
        ObservedRecoveryState {
            launches: 2,
            initialize_attempts: 2,
            tool_calls: 2,
        }
    );

    client.shutdown().await;
    Ok(())
}

/// Ошибка повторного `initialize` не запускает операцию и не заменяет старый закрытый сервис.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reinitialize_failure_preserves_closed_service_for_next_call() -> anyhow::Result<()> {
    let paths = RecoveryPaths::new()?;
    let mut env = paths.environment(/*close_count*/ 1);
    env.insert(
        OsString::from(RECOVERY_INITIALIZE_FAIL_AT_ENV),
        OsString::from("2"),
    );
    let client = initialized_recovery_client(&paths, env).await?;

    let failed = call_recovery_probe(&client).await;
    assert!(failed.is_err(), "second initialize must fail recovery");
    assert_eq!(
        paths.observed_state()?,
        ObservedRecoveryState {
            launches: 2,
            initialize_attempts: 2,
            tool_calls: 1,
        }
    );

    assert_eq!(
        call_recovery_probe(&client).await?,
        expected_probe_result("recovered")
    );
    assert_eq!(
        paths.observed_state()?,
        ObservedRecoveryState {
            launches: 3,
            initialize_attempts: 3,
            tool_calls: 2,
        }
    );

    client.shutdown().await;
    Ok(())
}
