use anyhow::Result;
use app_test_support::TestAppServer;
use app_test_support::create_mock_responses_server_repeating_assistant;
use app_test_support::to_response;
use codex_app_server_protocol::JSONRPCResponse;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ThreadLoadedListParams;
use codex_app_server_protocol::ThreadLoadedListResponse;
use codex_app_server_protocol::ThreadResumeParams;
use codex_app_server_protocol::ThreadResumeResponse;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadStartResponse;
use codex_app_server_protocol::ThreadUnloadParams;
use codex_app_server_protocol::ThreadUnloadResponse;
use codex_app_server_protocol::ThreadUnloadStatus;
use codex_app_server_protocol::TurnStartParams;
use codex_app_server_protocol::TurnStartResponse;
use codex_app_server_protocol::UserInput;
use codex_core::find_archived_thread_path_by_id_str;
use codex_core::find_thread_path_by_id_str;
use pretty_assertions::assert_eq;
use std::path::Path;
use tempfile::TempDir;
use tokio::time::timeout;

const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[tokio::test]
async fn thread_unload_shuts_down_loaded_runtime_without_deleting_session() -> Result<()> {
    let server = create_mock_responses_server_repeating_assistant("Done").await;
    let codex_home = TempDir::new()?;
    create_config_toml(codex_home.path(), &server.uri())?;

    let mut mcp = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .build()
        .await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let thread_id = start_materialized_thread(&mut mcp, codex_home.path()).await?;
    assert_eq!(loaded_thread_ids(&mut mcp).await?, vec![thread_id.clone()]);

    let unload_id = mcp
        .send_thread_unload_request(ThreadUnloadParams {
            thread_id: thread_id.clone(),
        })
        .await?;
    let unload_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(unload_id)),
    )
    .await??;
    let unload = to_response::<ThreadUnloadResponse>(unload_resp)?;
    assert_eq!(unload.status, ThreadUnloadStatus::Unloaded);

    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("thread/closed"),
    )
    .await??;
    assert_eq!(loaded_thread_ids(&mut mcp).await?, Vec::<String>::new());

    let second_unload_id = mcp
        .send_thread_unload_request(ThreadUnloadParams {
            thread_id: thread_id.clone(),
        })
        .await?;
    let second_unload_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(second_unload_id)),
    )
    .await??;
    let second_unload = to_response::<ThreadUnloadResponse>(second_unload_resp)?;
    assert_eq!(second_unload.status, ThreadUnloadStatus::NotLoaded);

    assert!(
        find_thread_path_by_id_str(codex_home.path(), &thread_id, /*state_db_ctx*/ None)
            .await?
            .is_some(),
        "thread/unload must leave the active rollout resumable"
    );
    assert!(
        find_archived_thread_path_by_id_str(
            codex_home.path(),
            &thread_id,
            /*state_db_ctx*/ None
        )
        .await?
        .is_none(),
        "thread/unload must not archive the rollout"
    );

    let resume_id = mcp
        .send_thread_resume_request(ThreadResumeParams {
            thread_id: thread_id.clone(),
            cwd: Some(codex_home.path().to_string_lossy().to_string()),
            ..Default::default()
        })
        .await?;
    let resume_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(resume_id)),
    )
    .await??;
    let resumed = to_response::<ThreadResumeResponse>(resume_resp)?;
    assert_eq!(resumed.thread.id, thread_id);
    assert_eq!(loaded_thread_ids(&mut mcp).await?, vec![thread_id]);

    Ok(())
}

async fn start_materialized_thread(mcp: &mut TestAppServer, cwd: &Path) -> Result<String> {
    let start_id = mcp
        .send_thread_start_request_with_auto_env(ThreadStartParams {
            model: Some("mock-model".to_string()),
            ..Default::default()
        })
        .await?;
    let start_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_id)),
    )
    .await??;
    let ThreadStartResponse { thread, .. } = to_response::<ThreadStartResponse>(start_resp)?;

    let turn_id = mcp
        .send_turn_start_request(TurnStartParams {
            thread_id: thread.id.clone(),
            client_user_message_id: None,
            input: vec![UserInput::Text {
                text: "materialize".to_string(),
                text_elements: Vec::new(),
            }],
            cwd: Some(cwd.to_path_buf()),
            ..Default::default()
        })
        .await?;
    let turn_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(turn_id)),
    )
    .await??;
    let _: TurnStartResponse = to_response::<TurnStartResponse>(turn_resp)?;
    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("turn/completed"),
    )
    .await??;

    Ok(thread.id)
}

async fn loaded_thread_ids(mcp: &mut TestAppServer) -> Result<Vec<String>> {
    let list_id = mcp
        .send_thread_loaded_list_request(ThreadLoadedListParams::default())
        .await?;
    let list_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(list_id)),
    )
    .await??;
    let ThreadLoadedListResponse { mut data, .. } =
        to_response::<ThreadLoadedListResponse>(list_resp)?;
    data.sort();
    Ok(data)
}

fn create_config_toml(codex_home: &Path, server_uri: &str) -> std::io::Result<()> {
    let config_toml = codex_home.join("config.toml");
    std::fs::write(
        config_toml,
        format!(
            r#"
model = "mock-model"
approval_policy = "never"
sandbox_mode = "read-only"

model_provider = "mock_provider"

[model_providers.mock_provider]
name = "Mock provider for test"
base_url = "{server_uri}/v1"
wire_api = "responses"
request_max_retries = 0
stream_max_retries = 0
"#
        ),
    )
}
