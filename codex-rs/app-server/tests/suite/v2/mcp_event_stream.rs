use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use app_test_support::ChatGptAuthFixture;
use app_test_support::MockResponsesConfig;
use app_test_support::TestAppServer;
use app_test_support::write_chatgpt_auth;
use axum::Json;
use axum::Router;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::sse::Event;
use axum::response::sse::Sse;
use axum::routing::post;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::McpServerEventNotification;
use codex_app_server_protocol::McpServerEventStreamNotification;
use codex_app_server_protocol::McpServerEventStreamStartResponse;
use codex_app_server_protocol::McpServerEventStreamStopParams;
use codex_app_server_protocol::McpServerEventStreamStopResponse;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadStartResponse;
use codex_app_server_protocol::ThreadUnloadParams;
use codex_app_server_protocol::ThreadUnloadResponse;
use codex_app_server_protocol::ThreadUnloadStatus;
use codex_config::types::AuthCredentialsStoreMode;
use codex_features::Feature;
use core_test_support::responses;
use futures::StreamExt as _;
use futures::stream;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::timeout;

/// Передаёт тесту запрос потока событий и наблюдаемый сигнал закрытия HTTP-потока.
struct StartedEventStream {
    request: Value,
    closed: oneshot::Receiver<()>,
}

/// Отправляет сигнал, когда app-server освобождает удерживаемый HTTP-поток.
struct EventStreamLifetime(Option<oneshot::Sender<()>>);

impl Drop for EventStreamLifetime {
    fn drop(&mut self) {
        if let Some(closed) = self.0.take() {
            let _ = closed.send(());
        }
    }
}

/// Предоставляет тесту endpoint и средства наблюдения жизненного цикла потока.
struct EventStreamServer {
    apps_url: String,
    started: mpsc::UnboundedReceiver<StartedEventStream>,
    allow_activation: Arc<Notify>,
    task: tokio::task::JoinHandle<std::io::Result<()>>,
}

/// Запускает endpoint `hosted Apps` MCP для наблюдения запросов и отмены потока событий.
async fn start_event_stream_server() -> Result<EventStreamServer> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let apps_url = format!("http://{}", listener.local_addr()?);
    let (stream_started_tx, stream_started_rx) = mpsc::unbounded_channel();
    let allow_activation = Arc::new(Notify::new());
    let activation_gate = Arc::clone(&allow_activation);
    let router = Router::new().route(
        "/api/codex/ps/mcp",
        post(move |Json(message): Json<Value>| {
            let stream_started_tx = stream_started_tx.clone();
            let allow_activation = Arc::clone(&activation_gate);

            async move {
                match message["method"].as_str() {
                    Some("initialize") => (
                        [("mcp-session-id", "event-test-session")],
                        Json(json!({
                            "jsonrpc": "2.0",
                            "id": message["id"],
                            "result": {
                                "protocolVersion": "2025-06-18",
                                "capabilities": {},
                                "serverInfo": {
                                    "name": "event-test-server",
                                    "version": "1.0.0",
                                },
                            },
                        })),
                    )
                        .into_response(),
                    Some("notifications/initialized") => StatusCode::ACCEPTED.into_response(),
                    Some("tools/list") => Json(json!({
                        "jsonrpc": "2.0",
                        "id": message["id"],
                        "result": { "tools": [] },
                    }))
                    .into_response(),
                    Some("events/stream") => {
                        let (stream_lifetime, stream_closed) = oneshot::channel();
                        stream_started_tx
                            .send(StartedEventStream {
                                request: message.clone(),
                                closed: stream_closed,
                            })
                            .expect("stream-start receiver must remain open");
                        let metadata = json!({
                            "io.modelcontextprotocol/subscriptionId": message["id"],
                            "provider": "event-test-server",
                        });
                        let active = json!({
                            "jsonrpc": "2.0",
                            "method": "notifications/events/active",
                            "params": { "_meta": metadata, "status": "active" },
                        });
                        let event = json!({
                            "jsonrpc": "2.0",
                            "method": "notifications/events/event",
                            "params": {
                                "_meta": metadata,
                                "name": "issue.updated",
                                "data": { "issue": 42 },
                            },
                        });
                        let stream_lifetime = EventStreamLifetime(Some(stream_lifetime));
                        let events = stream::once(async move {
                            allow_activation.notified().await;
                            Ok::<_, Infallible>(
                                Event::default().event("message").data(active.to_string()),
                            )
                        })
                        .chain(stream::once(async move {
                            Ok::<_, Infallible>(
                                Event::default().event("message").data(event.to_string()),
                            )
                        }))
                        .chain(stream::pending().map(move |event| {
                            let _ = &stream_lifetime;
                            event
                        }));

                        Sse::new(events).into_response()
                    }
                    method => panic!("unexpected MCP request: {method:?}"),
                }
            }
        }),
    );
    let task = tokio::spawn(async move { axum::serve(listener, router).await });
    Ok(EventStreamServer {
        apps_url,
        started: stream_started_rx,
        allow_activation,
        task,
    })
}

/// Проверяет активацию, передачу уведомлений, запрет дубликата и явную остановку потока.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mcp_event_stream_waits_for_activation_forwards_events_and_cancels() -> Result<()> {
    let responses_server = responses::start_mock_server().await;
    let mut mcp_server = start_event_stream_server().await?;

    let codex_home = TempDir::new()?;
    let apps_url = &mcp_server.apps_url;
    MockResponsesConfig::new(&responses_server.uri())
        .with_root_config(&format!("chatgpt_base_url = \"{apps_url}\""))
        .enable_feature(Feature::Apps)
        .write(codex_home.path())?;
    write_chatgpt_auth(
        codex_home.path(),
        ChatGptAuthFixture::new("chatgpt-token")
            .account_id("account-123")
            .chatgpt_user_id("user-123")
            .chatgpt_account_id("account-123"),
        AuthCredentialsStoreMode::File,
    )?;
    let mut app_server = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .build_initialized()
        .await?;
    let ThreadStartResponse { thread, .. } = app_server
        .start_thread(ThreadStartParams::default())
        .await?;
    let start_params = json!({
        "threadId": thread.id,
        "server": "codex_apps",
        "subscriptionId": "subscription-1",
        "name": "issue.updated",
        "arguments": { "project": "codex" },
        "_meta": { "source": "desktop" },
    });
    let start_request_id = app_server
        .send_raw_request("mcpServer/event/stream/start", Some(start_params.clone()))
        .await?;
    let started_stream = timeout(Duration::from_secs(5), mcp_server.started.recv())
        .await?
        .context("MCP event stream was not requested")?;
    let stream_request = started_stream.request;

    assert_eq!(
        stream_request["params"],
        json!({
            "name": "issue.updated",
            "arguments": { "project": "codex" },
            "_meta": {
                "progressToken": stream_request["params"]["_meta"]["progressToken"],
                "source": "desktop",
            },
        })
    );
    let duplicate_request_id = app_server
        .send_raw_request("mcpServer/event/stream/start", Some(start_params))
        .await?;
    let duplicate_error = timeout(
        Duration::from_secs(5),
        app_server.read_stream_until_error_message(RequestId::Integer(duplicate_request_id)),
    )
    .await??;
    assert!(duplicate_error.error.message.contains("already exists"));
    assert!(
        timeout(
            Duration::from_millis(100),
            app_server.read_stream_until_response_message(RequestId::Integer(start_request_id)),
        )
        .await
        .is_err(),
        "subscription start must not complete before the active notification"
    );

    mcp_server.allow_activation.notify_one();
    let _: McpServerEventStreamStartResponse = timeout(
        Duration::from_secs(5),
        app_server.read_response(start_request_id),
    )
    .await??;
    let metadata = json!({
        "io.modelcontextprotocol/subscriptionId": stream_request["id"],
        "provider": "event-test-server",
    });
    let active: McpServerEventStreamNotification = timeout(
        Duration::from_secs(5),
        app_server.read_notification("mcpServer/event/stream/notification"),
    )
    .await??;
    assert_eq!(
        active,
        McpServerEventStreamNotification {
            subscription_id: "subscription-1".to_string(),
            notification: McpServerEventNotification {
                method: "notifications/events/active".to_string(),
                params: json!({ "_meta": metadata, "status": "active" }),
            },
        }
    );
    let event: McpServerEventStreamNotification = timeout(
        Duration::from_secs(5),
        app_server.read_notification("mcpServer/event/stream/notification"),
    )
    .await??;
    assert_eq!(
        event,
        McpServerEventStreamNotification {
            subscription_id: "subscription-1".to_string(),
            notification: McpServerEventNotification {
                method: "notifications/events/event".to_string(),
                params: json!({
                    "_meta": metadata,
                    "name": "issue.updated",
                    "data": { "issue": 42 },
                }),
            },
        }
    );

    let _: McpServerEventStreamStopResponse = app_server
        .request(|request_id| ClientRequest::McpServerEventStreamStop {
            request_id,
            params: McpServerEventStreamStopParams {
                subscription_id: "subscription-1".to_string(),
            },
        })
        .await?;

    mcp_server.task.abort();
    let _ = mcp_server.task.await;
    Ok(())
}

/// Проверяет, что явная выгрузка thread завершает поток событий MCP текущего подключения.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn thread_unload_stops_mcp_event_stream_for_connection() -> Result<()> {
    let responses_server = responses::start_mock_server().await;
    let mut mcp_server = start_event_stream_server().await?;
    let codex_home = TempDir::new()?;
    let apps_url = &mcp_server.apps_url;
    MockResponsesConfig::new(&responses_server.uri())
        .with_root_config(&format!("chatgpt_base_url = \"{apps_url}\""))
        .enable_feature(Feature::Apps)
        .write(codex_home.path())?;
    write_chatgpt_auth(
        codex_home.path(),
        ChatGptAuthFixture::new("chatgpt-token")
            .account_id("account-123")
            .chatgpt_user_id("user-123")
            .chatgpt_account_id("account-123"),
        AuthCredentialsStoreMode::File,
    )?;
    let mut app_server = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .build_initialized()
        .await?;
    let ThreadStartResponse { thread, .. } = app_server
        .start_thread(ThreadStartParams::default())
        .await?;
    let start_request_id = app_server
        .send_raw_request(
            "mcpServer/event/stream/start",
            Some(json!({
                "threadId": thread.id,
                "server": "codex_apps",
                "subscriptionId": "subscription-unload",
                "name": "issue.updated",
                "arguments": { "project": "codex" },
            })),
        )
        .await?;
    let started_stream = timeout(Duration::from_secs(5), mcp_server.started.recv())
        .await?
        .context("MCP event stream was not requested")?;

    mcp_server.allow_activation.notify_one();
    let _: McpServerEventStreamStartResponse = timeout(
        Duration::from_secs(5),
        app_server.read_response(start_request_id),
    )
    .await??;

    let unload_id = app_server
        .send_thread_unload_request(ThreadUnloadParams {
            thread_id: thread.id,
        })
        .await?;
    let unload: ThreadUnloadResponse =
        timeout(Duration::from_secs(5), app_server.read_response(unload_id)).await??;
    assert_eq!(
        unload,
        ThreadUnloadResponse {
            status: ThreadUnloadStatus::Unloaded,
        }
    );
    timeout(Duration::from_secs(5), started_stream.closed).await??;

    mcp_server.task.abort();
    let _ = mcp_server.task.await;
    Ok(())
}
