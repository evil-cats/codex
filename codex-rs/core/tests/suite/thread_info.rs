//! Интеграционные проверки контракта `get_thread_info`, видимого модели.
//!
//! Тесты вызывают настоящий core tool через Responses API и проверяют текущие
//! root и subagent sessions, сохранённую metadata и ограничение повреждённых
//! цепочек `parent_thread_id` без чтения полной истории thread.

use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use anyhow::anyhow;
use codex_core::ARCHIVED_SESSIONS_SUBDIR;
use codex_core::config::AgentRoleConfig;
use codex_core::config::ThreadStoreConfig;
use codex_features::Feature;
use codex_protocol::SessionId;
use codex_protocol::ThreadId;
use codex_protocol::models::BaseInstructions;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_protocol::protocol::ThreadMemoryMode;
use codex_thread_store::CreateThreadParams;
use codex_thread_store::InMemoryThreadStore;
use codex_thread_store::ThreadMetadataPatch;
use codex_thread_store::ThreadPersistenceMetadata;
use codex_thread_store::ThreadStore;
use codex_thread_store::UpdateThreadMetadataParams;
use core_test_support::responses::ResponseMock;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::ev_function_call_with_namespace;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_once_match;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use pretty_assertions::assert_eq;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use uuid::Uuid;

const GET_THREAD_INFO_TOOL_NAME: &str = "get_thread_info";
const COLLABORATION_NAMESPACE: &str = "collaboration";

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct ThreadInfoOutput {
    thread_id: String,
    session_id: Option<String>,
    rollout_path: Option<PathBuf>,
    agent_name: Option<String>,
}

/// Удаляет именованный in-memory store даже при досрочном завершении теста.
struct InMemoryThreadStoreGuard(String);

impl Drop for InMemoryThreadStoreGuard {
    fn drop(&mut self) {
        InMemoryThreadStore::remove_id(&self.0);
    }
}

/// Монтирует полный цикл Responses с одним вызовом `get_thread_info`.
async fn mount_thread_info_turn(
    server: &wiremock::MockServer,
    call_id: &str,
    arguments: Value,
) -> ResponseMock {
    let response_id = format!("resp-{call_id}");
    let completion_id = format!("resp-{call_id}-complete");
    mount_sse_sequence(
        server,
        vec![
            sse(vec![
                ev_response_created(&response_id),
                ev_function_call(call_id, GET_THREAD_INFO_TOOL_NAME, &arguments.to_string()),
                ev_completed(&response_id),
            ]),
            sse(vec![
                ev_response_created(&completion_id),
                ev_completed(&completion_id),
            ]),
        ],
    )
    .await
}

/// Извлекает типизированный результат tool из следующего запроса Responses.
fn thread_info_output(mock: &ResponseMock, call_id: &str) -> Result<ThreadInfoOutput> {
    let output = mock
        .requests()
        .into_iter()
        .find_map(|request| request.function_call_output_text(call_id))
        .expect("follow-up request should contain get_thread_info output");
    Ok(serde_json::from_str(&output)?)
}

/// Проверяет наличие текста в JSON-теле запроса Responses для точной маршрутизации mock.
fn request_body_contains(request: &wiremock::Request, text: &str) -> bool {
    serde_json::from_slice::<Value>(&request.body).is_ok_and(|body| body.to_string().contains(text))
}

/// Отличает исходный запрос root от дочернего запроса с унаследованной историей.
fn request_has_input_type(request: &wiremock::Request, input_type: &str) -> bool {
    serde_json::from_slice::<Value>(&request.body)
        .ok()
        .and_then(|body| body.get("input").and_then(Value::as_array).cloned())
        .is_some_and(|items| {
            items
                .iter()
                .any(|item| item.get("type").and_then(Value::as_str) == Some(input_type))
        })
}

/// Создаёт сохранённую metadata без history для проверки обхода parent chain.
async fn seed_stored_thread(
    store: &InMemoryThreadStore,
    thread_id: ThreadId,
    session_id: SessionId,
    parent_thread_id: Option<ThreadId>,
    agent_role: Option<&str>,
    rollout_path: Option<PathBuf>,
) -> Result<()> {
    let source = match parent_thread_id {
        Some(parent_thread_id) => SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id,
            depth: 1,
            agent_path: None,
            agent_nickname: None,
            agent_role: agent_role.map(str::to_owned),
        }),
        None => SessionSource::Exec,
    };
    store
        .create_thread(CreateThreadParams {
            session_id,
            thread_id,
            extra_config: None,
            forked_from_id: None,
            parent_thread_id,
            source,
            thread_source: None,
            originator: "thread-info-test".to_string(),
            base_instructions: BaseInstructions::default(),
            dynamic_tools: Vec::new(),
            selected_capability_roots: Vec::new(),
            multi_agent_version: None,
            history_mode: Default::default(),
            history_base: None,
            subagent_history_start_ordinal: None,
            initial_window_id: Uuid::now_v7().to_string(),
            metadata: ThreadPersistenceMetadata {
                cwd: None,
                model_provider: "test-provider".to_string(),
                memory_mode: ThreadMemoryMode::Disabled,
            },
        })
        .await?;
    store
        .update_thread_metadata(UpdateThreadMetadataParams {
            thread_id,
            patch: ThreadMetadataPatch {
                rollout_path,
                agent_role: agent_role.map(|role| Some(role.to_string())),
                ..Default::default()
            },
            include_archived: true,
        })
        .await?;
    Ok(())
}

/// Возвращает стабильный путь архивного rollout fixture для заданного thread.
fn archived_rollout_path(codex_home: &Path, thread_id: ThreadId) -> PathBuf {
    codex_home
        .join(ARCHIVED_SESSIONS_SUBDIR)
        .join("2024/01/01")
        .join(format!("rollout-2024-01-01T00-00-00-{thread_id}.jsonl"))
}

/// Записывает минимальный архивный rollout с metadata, нужной `get_thread_info`.
fn write_archived_thread(
    codex_home: &Path,
    thread_id: ThreadId,
    session_id: SessionId,
    parent_thread_id: Option<ThreadId>,
    agent_role: Option<&str>,
) -> std::io::Result<PathBuf> {
    let path = archived_rollout_path(codex_home, thread_id);
    let parent = path.parent().expect("archived rollout should have parent");
    std::fs::create_dir_all(parent)?;
    let line = json!({
        "timestamp": "2024-01-01T00:00:00.000Z",
        "type": "session_meta",
        "payload": {
            "session_id": session_id,
            "id": thread_id,
            "timestamp": "2024-01-01T00:00:00Z",
            "cwd": ".",
            "originator": "thread-info-test",
            "cli_version": "test",
            "model_provider": "test-provider",
            "parent_thread_id": parent_thread_id,
            "agent_role": agent_role,
        }
    });
    std::fs::write(&path, format!("{line}\n"))?;
    Ok(path)
}

/// Проверяет materialize rollout и live metadata текущего root thread.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn thread_info_handler_returns_current_root_metadata() -> Result<()> {
    skip_if_no_network!(Ok(()));

    const CALL_ID: &str = "thread-info-current-root";
    let server = start_mock_server().await;
    let mock = mount_thread_info_turn(&server, CALL_ID, json!({})).await;
    let mut builder = test_codex().with_pre_build_hook(|home| {
        std::fs::write(home.join("config.toml"), "name = \"Hermione\"\n")
            .expect("test config should be writable");
    });
    let test = builder.build_with_auto_env(&server).await?;

    test.submit_turn("report the current root thread metadata")
        .await?;

    let rollout_path = test.codex.rollout_path();
    assert!(rollout_path.as_ref().is_some_and(|path| path.is_file()));
    assert_eq!(
        thread_info_output(&mock, CALL_ID)?,
        ThreadInfoOutput {
            thread_id: test.session_configured.thread_id.to_string(),
            session_id: Some(test.session_configured.session_id.to_string()),
            rollout_path,
            agent_name: Some("Hermione".to_string()),
        }
    );
    Ok(())
}

/// Проверяет общий `session_id` дерева и роль текущего subagent thread.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn thread_info_handler_returns_current_subagent_metadata() -> Result<()> {
    skip_if_no_network!(Ok(()));

    const ROOT_PROMPT: &str = "delegate the thread metadata check";
    const CHILD_TASK: &str = "report your current subagent thread metadata";
    const SPAWN_CALL_ID: &str = "spawn-thread-info-worker";
    const THREAD_INFO_CALL_ID: &str = "thread-info-current-subagent";

    let server = start_mock_server().await;
    let spawn_args = json!({
        "message": CHILD_TASK,
        "task_name": "thread_info_worker",
        "agent_type": "Researcher",
    });
    mount_sse_once_match(
        &server,
        |request: &wiremock::Request| {
            request_body_contains(request, ROOT_PROMPT)
                && !request_has_input_type(request, "agent_message")
                && !request_body_contains(request, SPAWN_CALL_ID)
        },
        sse(vec![
            ev_response_created("resp-thread-info-root"),
            ev_function_call_with_namespace(
                SPAWN_CALL_ID,
                COLLABORATION_NAMESPACE,
                "spawn_agent",
                &spawn_args.to_string(),
            ),
            ev_completed("resp-thread-info-root"),
        ]),
    )
    .await;
    mount_sse_once_match(
        &server,
        |request: &wiremock::Request| {
            request_body_contains(request, CHILD_TASK)
                && !request_body_contains(request, SPAWN_CALL_ID)
        },
        sse(vec![
            ev_response_created("resp-thread-info-child"),
            ev_function_call(THREAD_INFO_CALL_ID, GET_THREAD_INFO_TOOL_NAME, "{}"),
            ev_completed("resp-thread-info-child"),
        ]),
    )
    .await;
    let child_follow_up = mount_sse_once_match(
        &server,
        |request: &wiremock::Request| {
            request_body_contains(request, THREAD_INFO_CALL_ID)
                && !request_body_contains(request, SPAWN_CALL_ID)
        },
        sse(vec![
            ev_response_created("resp-thread-info-child-complete"),
            ev_assistant_message("msg-thread-info-child", "metadata reported"),
            ev_completed("resp-thread-info-child-complete"),
        ]),
    )
    .await;
    mount_sse_once_match(
        &server,
        |request: &wiremock::Request| {
            request_body_contains(request, SPAWN_CALL_ID)
                && !request_body_contains(request, THREAD_INFO_CALL_ID)
        },
        sse(vec![
            ev_response_created("resp-thread-info-root-complete"),
            ev_assistant_message("msg-thread-info-root", "worker started"),
            ev_completed("resp-thread-info-root-complete"),
        ]),
    )
    .await;

    let mut builder = test_codex().with_config(|config| {
        config
            .features
            .enable(Feature::Collab)
            .expect("test config should enable collaboration");
        config
            .features
            .enable(Feature::MultiAgentV2)
            .expect("test config should enable multi-agent v2");
        let role_path = config.codex_home.join("researcher-role.toml");
        std::fs::write(&role_path, "name = \"Researcher\"\n")
            .expect("agent role fixture should be writable");
        config.agent_roles.insert(
            "Researcher".to_string(),
            AgentRoleConfig {
                description: Some("Thread metadata researcher".to_string()),
                config_file: Some(role_path.to_path_buf()),
                nickname_candidates: None,
            },
        );
    });
    let test = builder.build_with_auto_env(&server).await?;
    let root_thread_id = test.session_configured.thread_id.to_string();
    let root_session_id = test.session_configured.session_id.to_string();

    test.submit_turn(ROOT_PROMPT).await?;

    let child_request = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(request) = child_follow_up.requests().into_iter().find(|request| {
                request
                    .function_call_output_text(THREAD_INFO_CALL_ID)
                    .is_some()
            }) {
                break request;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| anyhow!("timed out waiting for the subagent get_thread_info output"))?;
    let child_thread_id = child_request
        .header("thread-id")
        .expect("child request should contain thread ID");
    let output = thread_info_output(&child_follow_up, THREAD_INFO_CALL_ID)?;
    let rollout_path = output.rollout_path.clone();
    assert!(rollout_path.as_ref().is_some_and(|path| path.is_file()));
    assert_ne!(child_thread_id, root_thread_id);
    assert_eq!(
        output,
        ThreadInfoOutput {
            thread_id: child_thread_id,
            session_id: Some(root_session_id),
            rollout_path,
            agent_name: Some("Researcher".to_string()),
        }
    );
    Ok(())
}

/// Проверяет архивный сохранённый thread и восстановление root `session_id`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn thread_info_handler_reads_archived_persisted_thread() -> Result<()> {
    skip_if_no_network!(Ok(()));

    const CALL_ID: &str = "thread-info-archived";
    let root_thread_id = ThreadId::new();
    let child_thread_id = ThreadId::new();
    let root_session_id = SessionId::from(root_thread_id);
    let server = start_mock_server().await;
    let mut builder = test_codex().with_pre_build_hook(move |home| {
        write_archived_thread(home, root_thread_id, root_session_id, None, None)
            .expect("archived root fixture should be writable");
        write_archived_thread(
            home,
            child_thread_id,
            root_session_id,
            Some(root_thread_id),
            Some("Researcher"),
        )
        .expect("archived child fixture should be writable");
    });
    let test = builder.build_with_auto_env(&server).await?;
    let expected_rollout_path = archived_rollout_path(test.codex_home_path(), child_thread_id);
    let mock = mount_thread_info_turn(
        &server,
        CALL_ID,
        json!({"thread_id": child_thread_id.to_string()}),
    )
    .await;

    test.submit_turn("report the archived persisted thread metadata")
        .await?;

    assert_eq!(
        thread_info_output(&mock, CALL_ID)?,
        ThreadInfoOutput {
            thread_id: child_thread_id.to_string(),
            session_id: Some(root_session_id.to_string()),
            rollout_path: Some(expected_rollout_path),
            agent_name: Some("Researcher".to_string()),
        }
    );
    Ok(())
}

/// Проверяет ограничение parent traversal при цикле, превышении глубины и нечитаемом родителе.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn thread_info_handler_bounds_corrupt_parent_chains() -> Result<()> {
    skip_if_no_network!(Ok(()));

    const NORMAL_CALL_ID: &str = "thread-info-stored-normal";
    const CYCLE_CALL_ID: &str = "thread-info-stored-cycle";
    const DEEP_CALL_ID: &str = "thread-info-stored-deep";
    const MISSING_PARENT_CALL_ID: &str = "thread-info-stored-missing-parent";

    let store_id = Uuid::new_v4().to_string();
    let _store_guard = InMemoryThreadStoreGuard(store_id.clone());
    let store = InMemoryThreadStore::for_id(store_id.clone());
    let server = start_mock_server().await;
    let store_id_for_config = store_id.clone();
    let mut builder = test_codex().with_config(move |config| {
        config.experimental_thread_store = ThreadStoreConfig::InMemory {
            id: store_id_for_config,
        };
    });
    let test = builder.build_with_auto_env(&server).await?;

    let normal_root = ThreadId::new();
    let normal_child = ThreadId::new();
    let normal_session_id = SessionId::from(normal_root);
    let normal_rollout_path = PathBuf::from("/virtual/thread-info-normal.jsonl");
    seed_stored_thread(&store, normal_root, normal_session_id, None, None, None).await?;
    seed_stored_thread(
        &store,
        normal_child,
        normal_session_id,
        Some(normal_root),
        Some("Researcher"),
        Some(normal_rollout_path.clone()),
    )
    .await?;

    let cycle_a = ThreadId::new();
    let cycle_b = ThreadId::new();
    let cycle_session_id = SessionId::from(cycle_a);
    seed_stored_thread(&store, cycle_a, cycle_session_id, Some(cycle_b), None, None).await?;
    seed_stored_thread(&store, cycle_b, cycle_session_id, Some(cycle_a), None, None).await?;

    let deep_thread_ids = (0..66).map(|_| ThreadId::new()).collect::<Vec<_>>();
    let deep_session_id = SessionId::from(
        *deep_thread_ids
            .last()
            .expect("deep chain should contain a root"),
    );
    for (index, thread_id) in deep_thread_ids.iter().copied().enumerate() {
        seed_stored_thread(
            &store,
            thread_id,
            deep_session_id,
            deep_thread_ids.get(index + 1).copied(),
            None,
            None,
        )
        .await?;
    }

    let missing_parent_child = ThreadId::new();
    let missing_parent = ThreadId::new();
    seed_stored_thread(
        &store,
        missing_parent_child,
        SessionId::from(missing_parent_child),
        Some(missing_parent),
        None,
        None,
    )
    .await?;

    let first_response = sse(vec![
        ev_response_created("resp-thread-info-stored"),
        ev_function_call(
            NORMAL_CALL_ID,
            GET_THREAD_INFO_TOOL_NAME,
            &json!({"thread_id": normal_child.to_string()}).to_string(),
        ),
        ev_function_call(
            CYCLE_CALL_ID,
            GET_THREAD_INFO_TOOL_NAME,
            &json!({"thread_id": cycle_a.to_string()}).to_string(),
        ),
        ev_function_call(
            DEEP_CALL_ID,
            GET_THREAD_INFO_TOOL_NAME,
            &json!({"thread_id": deep_thread_ids[0].to_string()}).to_string(),
        ),
        ev_function_call(
            MISSING_PARENT_CALL_ID,
            GET_THREAD_INFO_TOOL_NAME,
            &json!({"thread_id": missing_parent_child.to_string()}).to_string(),
        ),
        ev_completed("resp-thread-info-stored"),
    ]);
    let mock = mount_sse_sequence(
        &server,
        vec![
            first_response,
            sse(vec![
                ev_response_created("resp-thread-info-stored-complete"),
                ev_completed("resp-thread-info-stored-complete"),
            ]),
        ],
    )
    .await;

    test.submit_turn("report stored thread metadata, including corrupt chains")
        .await?;

    assert_eq!(
        thread_info_output(&mock, NORMAL_CALL_ID)?,
        ThreadInfoOutput {
            thread_id: normal_child.to_string(),
            session_id: Some(normal_session_id.to_string()),
            rollout_path: Some(normal_rollout_path),
            agent_name: Some("Researcher".to_string()),
        }
    );
    for (call_id, thread_id) in [
        (CYCLE_CALL_ID, cycle_a),
        (DEEP_CALL_ID, deep_thread_ids[0]),
        (MISSING_PARENT_CALL_ID, missing_parent_child),
    ] {
        assert_eq!(
            thread_info_output(&mock, call_id)?,
            ThreadInfoOutput {
                thread_id: thread_id.to_string(),
                session_id: None,
                rollout_path: None,
                agent_name: None,
            }
        );
    }
    Ok(())
}
