//! Интеграционные проверки контракта `get_system_time`, видимого модели.
//!
//! Тесты проводят вызов через Responses API и настоящий core-обработчик, а затем
//! проверяют структурированный `function_call_output` следующего запроса.

use anyhow::Result;
use core_test_support::responses::ResponseMock;
use core_test_support::responses::ResponsesRequest;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_once;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;

const GET_SYSTEM_TIME_TOOL_NAME: &str = "get_system_time";

/// Монтирует полный Responses-цикл с одним вызовом настоящего `get_system_time`.
async fn mount_system_time_turn(
    server: &wiremock::MockServer,
    call_id: &str,
    arguments: Value,
) -> ResponseMock {
    let call_response_id = format!("resp-{call_id}-call");
    mount_sse_once(
        server,
        sse(vec![
            ev_response_created(&call_response_id),
            ev_function_call(call_id, GET_SYSTEM_TIME_TOOL_NAME, &arguments.to_string()),
            ev_completed(&call_response_id),
        ]),
    )
    .await;

    let completion_response_id = format!("resp-{call_id}-complete");
    mount_sse_once(
        server,
        sse(vec![
            ev_assistant_message(&format!("msg-{call_id}"), "готово"),
            ev_completed(&completion_response_id),
        ]),
    )
    .await
}

/// Извлекает текст и флаг `success`, одновременно проверяя тип и `call_id` элемента ответа.
fn system_time_output(request: &ResponsesRequest, call_id: &str) -> (String, Option<bool>) {
    let raw = request.function_call_output(call_id);
    assert_eq!(
        (
            raw.get("type").and_then(Value::as_str),
            raw.get("call_id").and_then(Value::as_str),
        ),
        (Some("function_call_output"), Some(call_id)),
        "function_call_output должен сохранять тип и call_id"
    );
    let (content, success) = request
        .function_call_output_content_and_success(call_id)
        .expect("следующий запрос должен содержать function_call_output");
    (
        content.expect("function_call_output должен содержать текст"),
        success,
    )
}

/// Проверяет детерминированный формат без динамических директив через весь Responses API.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn system_time_handler_returns_structured_output_through_responses_api() -> Result<()> {
    skip_if_no_network!(Ok(()));

    const CALL_ID: &str = "system-time-literal-format";
    let server = start_mock_server().await;
    let output_mock = mount_system_time_turn(
        &server,
        CALL_ID,
        json!({
            "format": "literal-system-time",
            "offset": "utc",
        }),
    )
    .await;
    let mut builder = test_codex();
    let test = builder.build_with_auto_env(&server).await?;

    test.submit_turn("получи системное время").await?;

    let request = output_mock.single_request();
    let (content, success) = system_time_output(&request, CALL_ID);
    assert_eq!(
        (serde_json::from_str::<Value>(&content)?, success),
        (json!({"formatted": "literal-system-time"}), None)
    );
    Ok(())
}

/// Проверяет, что неизвестное JSON-поле доходит до модели как ошибка обработчика, а не игнорируется.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn system_time_handler_rejects_unknown_fields_for_model() -> Result<()> {
    skip_if_no_network!(Ok(()));

    const CALL_ID: &str = "system-time-unknown-field";
    let server = start_mock_server().await;
    let output_mock = mount_system_time_turn(&server, CALL_ID, json!({"unexpected": true})).await;
    let mut builder = test_codex();
    let test = builder.build_with_auto_env(&server).await?;

    test.submit_turn("вызови системное время с неизвестным полем")
        .await?;

    let request = output_mock.single_request();
    let (content, success) = system_time_output(&request, CALL_ID);
    assert_eq!(success, None);
    assert!(
        content.contains("failed to parse function arguments: unknown field `unexpected`"),
        "видимая модели ошибка должна называть неизвестное поле: {content}"
    );
    Ok(())
}
