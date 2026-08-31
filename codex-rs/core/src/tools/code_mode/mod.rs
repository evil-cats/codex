//! Оркестрирует сессию Code Mode, вложенные инструменты и итоговый ответ для модели.

mod delegate;
mod execute_handler;
pub(crate) mod execute_spec;
mod output_spill;
mod response_adapter;
mod telemetry;
mod wait_handler;
pub(crate) mod wait_spec;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use codex_code_mode::CellId;
use codex_code_mode::CodeModeNestedToolCall;
use codex_code_mode::CodeModeSession;
use codex_code_mode::CodeModeSessionProvider;
use codex_code_mode::CodeModeToolKind;
use codex_code_mode::RuntimeResponse;
use codex_protocol::models::FunctionCallOutputContentItem;
use futures::future::join_all;
use serde_json::Value as JsonValue;
use tokio::sync::OnceCell;
use tokio_util::sync::CancellationToken;

use crate::config::CodeModeConfig;
use crate::function_tool::FunctionCallError;
use crate::original_image_detail::can_request_original_image_detail;
use crate::original_image_detail::sanitize_original_image_detail as sanitize_image_detail_items;
use crate::session::session::Session;
use crate::session::step_context::StepContext;
use crate::session::turn_context::TurnContext;
use crate::tools::ExecutedToolCallRecorder;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::SharedTurnDiffTracker;
use crate::tools::context::ToolPayload;
use crate::tools::parallel::ToolCallRuntime;
use crate::tools::router::ToolCall;
use crate::tools::router::ToolCallSource;
use crate::unified_exec::resolve_max_tokens;
use codex_protocol::openai_models::ToolMode;
use codex_tools::ToolName;
use codex_utils_audio::estimate_audio_token_count;
use codex_utils_output_truncation::TruncationPolicy;
use codex_utils_output_truncation::formatted_truncate_text_content_items_with_policy;
use codex_utils_output_truncation::truncate_function_output_items_with_policy;

use delegate::CodeModeDispatchBroker;
use delegate::CodeModeDispatchWorker;
pub(crate) use execute_handler::CodeModeExecuteHandler;
use output_spill::RuntimeOutputDelivery;
use output_spill::RuntimeOutputIdentity;
use output_spill::maybe_spill_runtime_output;
use response_adapter::into_function_call_output_content_items;
pub(crate) use wait_handler::CodeModeWaitHandler;

pub(crate) const PUBLIC_TOOL_NAME: &str = codex_code_mode::PUBLIC_TOOL_NAME;
pub(crate) const WAIT_TOOL_NAME: &str = codex_code_mode::WAIT_TOOL_NAME;
pub(crate) const DEFAULT_WAIT_YIELD_TIME_MS: u64 = codex_code_mode::DEFAULT_WAIT_YIELD_TIME_MS;

/// Returns true for the code-mode `exec` tool in the default namespace.
pub(crate) fn is_exec_tool_name(tool_name: &ToolName) -> bool {
    tool_name.is_default_namespace() && tool_name.name == PUBLIC_TOOL_NAME
}

#[derive(Clone)]
pub(crate) struct ExecContext {
    pub(super) session: Arc<Session>,
    pub(super) turn: Arc<TurnContext>,
}

pub(crate) struct CodeModeService {
    session: OnceCell<Arc<dyn CodeModeSession>>,
    session_provider: Arc<dyn CodeModeSessionProvider>,
    availability: Result<(), String>,
    dispatch_broker: Arc<CodeModeDispatchBroker>,
    default_exec_yield_time_ms: u64,
    shutdown_token: CancellationToken,
    unavailable_warning_emitted: AtomicBool,
}

impl CodeModeService {
    pub(crate) fn new(
        session_provider: Arc<dyn CodeModeSessionProvider>,
        config: &CodeModeConfig,
        executed_tool_calls: Option<Arc<ExecutedToolCallRecorder>>,
    ) -> Self {
        let dispatch_broker = Arc::new(CodeModeDispatchBroker::new(executed_tool_calls));
        let availability = session_provider.availability();
        Self {
            session: OnceCell::new(),
            session_provider,
            availability,
            dispatch_broker,
            default_exec_yield_time_ms: config.default_exec_yield_time_ms,
            shutdown_token: CancellationToken::new(),
            unavailable_warning_emitted: AtomicBool::new(false),
        }
    }

    pub(crate) fn is_available(&self) -> bool {
        self.availability.is_ok()
    }

    pub(crate) fn take_unavailable_warning(&self, tool_mode: ToolMode) -> Option<String> {
        let error = self.availability.as_ref().err()?;
        let behavior = match tool_mode {
            ToolMode::Direct => "Falling back to direct tools",
            ToolMode::CodeMode | ToolMode::CodeModeOnly => "Code mode will fail closed",
        };
        (!self
            .unavailable_warning_emitted
            .swap(true, Ordering::Relaxed))
        .then(|| {
            format!(
                "Code Mode is unavailable because {error}. {behavior}; enable `features.code_mode_host` and install `codex-code-mode-host`."
            )
        })
    }

    pub(crate) fn session_provider(&self) -> Arc<dyn CodeModeSessionProvider> {
        Arc::clone(&self.session_provider)
    }

    pub(crate) async fn execute(
        &self,
        mut request: codex_code_mode::ExecuteRequest,
    ) -> Result<codex_code_mode::StartedCell, String> {
        request
            .yield_time_ms
            .get_or_insert(self.default_exec_yield_time_ms);
        self.session().await?.execute(request).await
    }

    pub(crate) async fn wait(
        &self,
        request: codex_code_mode::WaitRequest,
    ) -> Result<codex_code_mode::WaitOutcome, String> {
        self.session().await?.wait(request).await
    }

    pub(crate) async fn terminate(
        &self,
        cell_id: CellId,
    ) -> Result<codex_code_mode::WaitOutcome, String> {
        self.session().await?.terminate(cell_id).await
    }

    pub(crate) async fn interrupt_active_cells(&self) {
        let Some(session) = self.session.get() else {
            return;
        };
        join_all(
            self.dispatch_broker
                .active_cell_ids()
                .into_iter()
                .map(|cell_id| async move {
                    if let Err(error) = session.terminate(cell_id.clone()).await {
                        tracing::warn!(%cell_id, %error, "failed to terminate interrupted code-mode cell");
                    }
                }),
        )
        .await;
    }

    pub(crate) async fn shutdown(&self) -> Result<(), String> {
        self.shutdown_token.cancel();
        // Join any initialization already in progress without initializing an unused service.
        match self
            .session
            .get_or_try_init(|| async {
                Err::<Arc<dyn CodeModeSession>, String>(
                    "code mode session is shutting down".to_string(),
                )
            })
            .await
        {
            Ok(session) => session.shutdown().await,
            Err(_) => Ok(()),
        }
    }

    pub(crate) fn mark_cell_ready_for_dispatch(
        &self,
        cell_id: &codex_code_mode::CellId,
        originating_item_id: Option<codex_protocol::ResponseItemId>,
    ) {
        self.dispatch_broker
            .mark_cell_ready_for_dispatch(cell_id, originating_item_id);
    }

    pub(crate) fn cell_originating_item_id(
        &self,
        cell_id: &codex_code_mode::CellId,
    ) -> Option<codex_protocol::ResponseItemId> {
        self.dispatch_broker.cell_originating_item_id(cell_id)
    }

    pub(crate) fn finish_cell_dispatch(&self, cell_id: &CellId) {
        self.dispatch_broker.close_cell(cell_id);
    }

    pub(crate) fn start_turn_worker(
        &self,
        session: &Arc<Session>,
        step_context: Arc<StepContext>,
        tracker: SharedTurnDiffTracker,
    ) -> Option<CodeModeDispatchWorker> {
        let turn = &step_context.turn;
        if !step_context.tool_router.requires_code_mode_worker() {
            return None;
        }

        let exec = ExecContext {
            session: Arc::clone(session),
            turn: Arc::clone(turn),
        };
        Some(
            self.dispatch_broker
                .start_turn_worker(exec, step_context, tracker),
        )
    }

    pub(crate) async fn session(&self) -> Result<Arc<dyn CodeModeSession>, String> {
        if self.shutdown_token.is_cancelled() {
            return Err("code mode session is shutting down".to_string());
        }
        self.session
            .get_or_try_init(|| async {
                if self.shutdown_token.is_cancelled() {
                    return Err("code mode session is shutting down".to_string());
                }
                let session = tokio::select! {
                    biased;
                    _ = self.shutdown_token.cancelled() => {
                        return Err("code mode session is shutting down".to_string());
                    }
                    session = self
                        .session_provider
                        .create_session(self.dispatch_broker.clone()) => session?,
                };
                if self.shutdown_token.is_cancelled() {
                    let _ = session.shutdown().await;
                    return Err("code mode session is shutting down".to_string());
                }
                Ok(session)
            })
            .await
            .map(Arc::clone)
    }
}

/// Преобразует ответ host-процесса в один внешний результат и применяет spill до усечения.
pub(super) async fn handle_runtime_response(
    exec: &ExecContext,
    call_id: &str,
    response: RuntimeResponse,
    max_output_tokens: Option<usize>,
    started_at: std::time::Instant,
) -> Result<FunctionToolOutput, String> {
    let script_status = format_script_status(&response);
    let cell_id = match &response {
        RuntimeResponse::Yielded { cell_id, .. }
        | RuntimeResponse::Terminated { cell_id, .. }
        | RuntimeResponse::Result { cell_id, .. } => cell_id.to_string(),
    };
    let (content_items, error_text, success) = match response {
        RuntimeResponse::Yielded { content_items, .. }
        | RuntimeResponse::Terminated { content_items, .. } => (content_items, None, true),
        RuntimeResponse::Result {
            content_items,
            error_text,
            ..
        } => {
            let success = error_text.is_none();
            (content_items, error_text, success)
        }
    };

    let mut content_items = into_function_call_output_content_items(content_items);
    sanitize_runtime_image_detail(exec.turn.as_ref(), &mut content_items);
    if let Some(error_text) = error_text {
        content_items.push(FunctionCallOutputContentItem::InputText {
            text: format!("Script error:\n{error_text}"),
        });
    }
    let delivery = maybe_spill_runtime_output(
        exec,
        content_items,
        RuntimeOutputIdentity {
            call_id,
            cell_id: &cell_id,
        },
        max_output_tokens,
    )
    .await;
    content_items = match delivery {
        RuntimeOutputDelivery::Inline(items) => truncate_code_mode_result(items, max_output_tokens),
        RuntimeOutputDelivery::Spilled {
            items,
            excerpt_token_count,
        } => truncate_spilled_code_mode_result(items, max_output_tokens, excerpt_token_count),
    };
    prepend_script_status(&mut content_items, &script_status, started_at.elapsed());
    Ok(FunctionToolOutput::from_content(
        content_items,
        Some(success),
    ))
}

fn sanitize_runtime_image_detail(turn: &TurnContext, items: &mut [FunctionCallOutputContentItem]) {
    sanitize_image_detail_items(can_request_original_image_detail(turn.model_info()), items);
}

fn format_script_status(response: &RuntimeResponse) -> String {
    match response {
        RuntimeResponse::Yielded { cell_id, .. } => {
            format!("Script running with cell ID {cell_id}")
        }
        RuntimeResponse::Terminated { .. } => "Script terminated".to_string(),
        RuntimeResponse::Result { error_text, .. } => {
            if error_text.is_none() {
                "Script completed".to_string()
            } else {
                "Script failed".to_string()
            }
        }
    }
}

fn prepend_script_status(
    content_items: &mut Vec<FunctionCallOutputContentItem>,
    status: &str,
    wall_time: Duration,
) {
    let wall_time_seconds = ((wall_time.as_secs_f32()) * 10.0).round() / 10.0;
    let header = format!("{status}\nWall time {wall_time_seconds:.1} seconds\nOutput:\n");
    content_items.insert(0, FunctionCallOutputContentItem::InputText { text: header });
}

fn truncate_code_mode_result(
    items: Vec<FunctionCallOutputContentItem>,
    max_output_tokens: Option<usize>,
) -> Vec<FunctionCallOutputContentItem> {
    let max_output_tokens = resolve_max_tokens(max_output_tokens);
    let policy = TruncationPolicy::Tokens(max_output_tokens);
    if items
        .iter()
        .all(|item| matches!(item, FunctionCallOutputContentItem::InputText { .. }))
    {
        let (truncated_items, _) =
            formatted_truncate_text_content_items_with_policy(&items, policy);
        return truncated_items;
    }

    truncate_function_output_items_with_policy(&items, policy, estimate_audio_token_count)
}

/// Сохраняет spill-метаданные целиком и расходует внешний лимит только на фрагмент и аудио.
fn truncate_spilled_code_mode_result(
    items: Vec<FunctionCallOutputContentItem>,
    max_output_tokens: Option<usize>,
    excerpt_token_count: usize,
) -> Vec<FunctionCallOutputContentItem> {
    let mut remaining_budget =
        resolve_max_tokens(max_output_tokens).saturating_sub(excerpt_token_count);
    let mut omitted_audio_items = 0;
    let mut output = Vec::with_capacity(items.len());

    for item in items {
        match item {
            FunctionCallOutputContentItem::InputText { .. }
            | FunctionCallOutputContentItem::InputImage { .. }
            | FunctionCallOutputContentItem::EncryptedContent { .. } => output.push(item),
            FunctionCallOutputContentItem::InputAudio { audio_url } => {
                let token_cost = estimate_audio_token_count(&audio_url);
                if token_cost <= remaining_budget {
                    output.push(FunctionCallOutputContentItem::InputAudio { audio_url });
                    remaining_budget = remaining_budget.saturating_sub(token_cost);
                } else {
                    omitted_audio_items += 1;
                }
            }
        }
    }

    if omitted_audio_items > 0 {
        output.push(FunctionCallOutputContentItem::InputText {
            text: format!("[omitted {omitted_audio_items} audio items ...]"),
        });
    }
    output
}

// Submit synchronously so the recorder sees the call before the cell's dispatch gate closes.
fn submit_nested_tool(
    exec: ExecContext,
    tool_runtime: ToolCallRuntime,
    invocation: CodeModeNestedToolCall,
    cancellation_token: CancellationToken,
) -> Result<
    impl std::future::Future<Output = Result<JsonValue, FunctionCallError>> + Send + 'static,
    FunctionCallError,
> {
    let CodeModeNestedToolCall {
        cell_id,
        runtime_tool_call_id,
        tool_name,
        tool_kind,
        input,
    } = invocation;
    if is_exec_tool_name(&tool_name) {
        return Err(FunctionCallError::RespondToModel(format!(
            "{PUBLIC_TOOL_NAME} cannot invoke itself"
        )));
    }

    let payload = match build_nested_tool_payload(tool_kind, &tool_name, input) {
        Ok(payload) => payload,
        Err(error) => return Err(FunctionCallError::RespondToModel(error)),
    };

    let call = ToolCall {
        tool_name: tool_name.with_default_namespace(),
        call_id: format!("{PUBLIC_TOOL_NAME}-{}", uuid::Uuid::new_v4()),
        payload,
        encrypted_function_args: None,
    };
    exec.session
        .services
        .analytics_events_client
        .track_code_mode_tool_call(codex_analytics::CodeModeToolCallFact::ChildStarted {
            thread_id: exec.session.thread_id.to_string(),
            turn_id: exec.turn.sub_id.clone(),
            call_id: call.call_id.clone(),
            cell_id: cell_id.to_string(),
        });
    let result = tool_runtime.handle_tool_call_with_source(
        call,
        ToolCallSource::CodeMode {
            cell_id: cell_id.to_string(),
            runtime_tool_call_id,
        },
        cancellation_token,
    );
    Ok(async move { Ok(result.await?.code_mode_result()) })
}

fn build_nested_tool_payload(
    tool_kind: CodeModeToolKind,
    tool_name: &ToolName,
    input: Option<JsonValue>,
) -> Result<ToolPayload, String> {
    match tool_kind {
        CodeModeToolKind::Function => build_function_tool_payload(tool_name, input),
        CodeModeToolKind::Freeform => build_freeform_tool_payload(tool_name, input),
    }
}

fn build_function_tool_payload(
    tool_name: &ToolName,
    input: Option<JsonValue>,
) -> Result<ToolPayload, String> {
    let arguments = serialize_function_tool_arguments(tool_name, input)?;
    Ok(ToolPayload::Function { arguments })
}

fn serialize_function_tool_arguments(
    tool_name: &ToolName,
    input: Option<JsonValue>,
) -> Result<String, String> {
    match input {
        None => Ok("{}".to_string()),
        Some(JsonValue::Object(map)) => serde_json::to_string(&JsonValue::Object(map))
            .map_err(|err| format!("failed to serialize tool `{tool_name}` arguments: {err}")),
        Some(_) => Err(format!(
            "tool `{tool_name}` expects a JSON object for arguments"
        )),
    }
}

fn build_freeform_tool_payload(
    tool_name: &ToolName,
    input: Option<JsonValue>,
) -> Result<ToolPayload, String> {
    match input {
        Some(JsonValue::String(input)) => Ok(ToolPayload::Custom { input }),
        _ => Err(format!("tool `{tool_name}` expects a string input")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use super::build_nested_tool_payload;
    use super::truncate_code_mode_result;
    use super::truncate_spilled_code_mode_result;
    use crate::session::step_context::StepContext;
    use crate::session::tests::make_session_and_context;
    use crate::tools::context::ToolPayload;
    use crate::tools::registry::ToolRegistry;
    use crate::tools::router::ToolRouter;
    use crate::turn_diff_tracker::TurnDiffTracker;
    use codex_code_mode::CodeModeToolKind;
    use codex_protocol::models::FunctionCallOutputContentItem;
    use codex_protocol::openai_models::ToolMode;
    use codex_tools::ToolName;
    use serde_json::json;

    #[tokio::test]
    async fn turn_worker_uses_step_router_mode_instead_of_admitted_turn() {
        let (session, turn) = make_session_and_context().await;
        assert_eq!(
            crate::tools::effective_tool_mode(&turn, turn.model_info()),
            ToolMode::Direct
        );
        let session = Arc::new(session);
        let step_context = StepContext::for_test(Arc::new(turn));
        let router = Arc::new(ToolRouter::from_parts(
            ToolRegistry::empty_for_test(),
            Vec::new(),
            ToolMode::CodeModeOnly,
            BTreeMap::new(),
            /*tool_namespaces_info*/ None,
            &[],
        ));
        let step_context = step_context.with_tool_router_for_test(router);
        let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));

        let worker =
            session
                .services
                .code_mode_service
                .start_turn_worker(&session, step_context, tracker);

        assert!(worker.is_some());
    }

    #[test]
    fn build_nested_tool_payload_uses_function_kind() {
        let payload = build_nested_tool_payload(
            CodeModeToolKind::Function,
            &ToolName::plain("example"),
            Some(json!({ "value": 1 })),
        )
        .expect("function payload should serialize");

        match payload {
            ToolPayload::Function { arguments } => {
                assert_eq!(arguments, r#"{"value":1}"#.to_string());
            }
            other => panic!("expected function payload, got {other:?}"),
        }
    }

    #[test]
    fn build_nested_tool_payload_uses_freeform_kind() {
        let payload = build_nested_tool_payload(
            CodeModeToolKind::Freeform,
            &ToolName::plain("example"),
            Some(json!("hello")),
        )
        .expect("freeform payload should preserve string input");

        match payload {
            ToolPayload::Custom { input } => {
                assert_eq!(input, "hello".to_string());
            }
            other => panic!("expected freeform payload, got {other:?}"),
        }
    }

    #[test]
    fn truncated_text_output_starts_with_warning() {
        let items = vec![FunctionCallOutputContentItem::InputText {
            text: "0123456789012345678901234567890123456789".to_string(),
        }];

        assert_eq!(
            truncate_code_mode_result(items, Some(5)),
            vec![FunctionCallOutputContentItem::InputText {
                text: concat!(
                    "Warning: truncated output (original token count: 10)\n",
                    "Total output lines: 1\n\n",
                    "0123456789…5 tokens truncated…0123456789"
                )
                .to_string(),
            }]
        );
    }

    #[test]
    fn over_budget_audio_output_is_omitted() {
        let items = vec![FunctionCallOutputContentItem::InputAudio {
            audio_url: format!("data:audio/wav;base64,{}", "A".repeat(100)),
        }];

        assert_eq!(
            truncate_code_mode_result(items, Some(5)),
            vec![FunctionCallOutputContentItem::InputText {
                text: "[omitted 1 audio items ...]".to_string(),
            }]
        );
    }

    #[test]
    fn spilled_output_preserves_metadata_and_omits_over_budget_audio() {
        let spill = FunctionCallOutputContentItem::InputText {
            text: "Lines: total=2 returned=1-1 remaining=1 complete=no".to_string(),
        };
        let audio = FunctionCallOutputContentItem::InputAudio {
            audio_url: format!("data:audio/wav;base64,{}", "A".repeat(100)),
        };

        assert_eq!(
            truncate_spilled_code_mode_result(
                vec![spill.clone(), audio],
                Some(5),
                /*excerpt_token_count*/ 5,
            ),
            vec![
                spill,
                FunctionCallOutputContentItem::InputText {
                    text: "[omitted 1 audio items ...]".to_string(),
                },
            ]
        );
    }
}
