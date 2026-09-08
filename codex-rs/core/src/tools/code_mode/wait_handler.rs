//! Продолжает или завершает ячейку Code Mode и использует общий путь ответа модели.

use serde::Deserialize;
use std::time::Duration;

use crate::function_tool::FunctionCallError;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::context::WaitWakeReason;
use crate::tools::context::boxed_tool_output;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::PostToolUsePayload;
use crate::tools::registry::PreToolUsePayload;
use crate::tools::registry::ToolExecutor;
use codex_tools::ToolName;
use codex_tools::ToolSpec;

use super::ExecContext;
use super::WAIT_TOOL_NAME;
use super::handle_runtime_response;
use super::telemetry::CodeModeToolCallGuard;
use super::wait_spec::create_wait_tool;

pub struct CodeModeWaitHandler {
    default_wait_timeout_ms: u64,
}

impl Default for CodeModeWaitHandler {
    fn default() -> Self {
        Self::new(codex_code_mode::DEFAULT_WAIT_YIELD_TIME_MS)
    }
}

#[derive(Debug, Deserialize)]
struct ExecWaitArgs {
    cell_id: String,
    yield_time_ms: Option<u64>,
    #[serde(default)]
    max_tokens: Option<usize>,
    #[serde(default)]
    terminate: bool,
}

fn parse_arguments<T>(arguments: &str) -> Result<T, FunctionCallError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str(arguments).map_err(|err| {
        FunctionCallError::RespondToModel(format!("failed to parse function arguments: {err}"))
    })
}

impl ToolExecutor<ToolInvocation> for CodeModeWaitHandler {
    fn tool_name(&self) -> ToolName {
        ToolName::plain(WAIT_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        create_wait_tool(self.default_wait_timeout_ms)
    }

    fn handle<'a>(&'a self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'a>
    where
        ToolInvocation: 'a,
    {
        Box::pin(self.handle_call(invocation))
    }
}

impl CodeModeWaitHandler {
    pub(crate) fn new(default_wait_timeout_ms: u64) -> Self {
        Self {
            default_wait_timeout_ms,
        }
    }

    /// Ждёт очередной ответ ячейки и передаёт общему пути ответа текущий
    /// `call_id` вызова `wait` вместе с длительностью от хост-процесса или
    /// резервным локальным замером.
    async fn handle_call(
        &self,
        invocation: ToolInvocation,
    ) -> Result<Box<dyn crate::tools::context::ToolOutput>, FunctionCallError> {
        let ToolInvocation {
            session,
            turn,
            call_id,
            tool_name,
            payload,
            ..
        } = invocation;

        let mut telemetry = CodeModeToolCallGuard::new(
            session.services.analytics_events_client.clone(),
            session.thread_id.to_string(),
            turn.sub_id.clone(),
            turn.turn_metadata_state.clone(),
            call_id.clone(),
            WAIT_TOOL_NAME,
        );
        let result = match payload {
            ToolPayload::Function { arguments }
                if tool_name.is_default_namespace()
                    && tool_name.name.as_str() == WAIT_TOOL_NAME =>
            {
                let args: ExecWaitArgs = parse_arguments(&arguments).inspect_err(|_error| {
                    telemetry.finish(/*success*/ false);
                })?;
                let exec = ExecContext { session, turn };
                let started_at = std::time::Instant::now();
                telemetry.cell_id = Some(args.cell_id.clone());
                let cell_id = codex_code_mode::CellId::new(args.cell_id);
                let timeout_ms = args.yield_time_ms.unwrap_or(self.default_wait_timeout_ms);
                let (wait_response, wait_wake_reason) = if args.terminate {
                    let response = exec
                        .session
                        .services
                        .code_mode_service
                        .terminate(cell_id)
                        .await;
                    (response, WaitWakeReason::Completed)
                } else {
                    let turn_state = exec
                        .session
                        .input_queue
                        .turn_state_for_sub_id(&exec.session.active_turn, &exec.turn.sub_id)
                        .await;
                    let mut steer_subscription = exec
                        .session
                        .input_queue
                        .subscribe_steer(turn_state.as_deref())
                        .await;
                    let wait_started_at = std::time::Instant::now();
                    let wait = exec.session.services.code_mode_service.wait(
                        codex_code_mode::WaitRequest {
                            cell_id: cell_id.clone(),
                            yield_time_ms: timeout_ms,
                        },
                    );
                    tokio::pin!(wait);
                    tokio::select! {
                        biased;
                        response = &mut wait => {
                            let reason = match response.as_ref() {
                                Ok(codex_code_mode::WaitOutcome::LiveCell(
                                    codex_code_mode::RuntimeResponse::Yielded { .. },
                                )) => {
                                    let observed_wait = response
                                        .as_ref()
                                        .ok()
                                        .and_then(codex_code_mode::WaitOutcome::code_mode_host_duration)
                                        .unwrap_or_else(|| wait_started_at.elapsed());
                                    if observed_wait >= Duration::from_millis(timeout_ms) {
                                        WaitWakeReason::TimedOut
                                    } else {
                                        WaitWakeReason::Activity
                                    }
                                }
                                Ok(_) => WaitWakeReason::Completed,
                                Err(_) => WaitWakeReason::Completed,
                            };
                            (response, reason)
                        }
                        _ = steer_subscription.wait() => (
                            Ok(codex_code_mode::WaitOutcome::LiveCell(
                                codex_code_mode::RuntimeResponse::Yielded {
                                    cell_id,
                                    content_items: Vec::new(),
                                    code_mode_host_duration: None,
                                },
                            )),
                            WaitWakeReason::Steered,
                        ),
                    }
                };
                let wait_response = wait_response.map_err(|error| {
                    telemetry.finish(/*success*/ false);
                    FunctionCallError::RespondToModel(error)
                })?;
                if let codex_code_mode::WaitOutcome::LiveCell(response) = &wait_response {
                    let runtime_cell_id = match response {
                        codex_code_mode::RuntimeResponse::Yielded { cell_id, .. }
                        | codex_code_mode::RuntimeResponse::Terminated { cell_id, .. }
                        | codex_code_mode::RuntimeResponse::Result { cell_id, .. } => cell_id,
                    };
                    telemetry.cell_id = Some(runtime_cell_id.to_string());
                    if let Some(executed_tool_calls) =
                        exec.session.services.executed_tool_calls.as_ref()
                    {
                        executed_tool_calls.register_cell(runtime_cell_id, &call_id);
                    }
                    if !matches!(response, codex_code_mode::RuntimeResponse::Yielded { .. }) {
                        exec.session
                            .services
                            .rollout_thread_trace
                            .code_cell_trace_context(
                                exec.turn.sub_id.as_str(),
                                runtime_cell_id.as_str(),
                            )
                            .record_ended(response);
                        exec.session
                            .services
                            .code_mode_service
                            .finish_cell_dispatch(runtime_cell_id);
                        exec.session
                            .services
                            .analytics_events_client
                            .track_code_mode_tool_call(
                                codex_analytics::CodeModeToolCallFact::CellClosed {
                                    thread_id: exec.session.thread_id.to_string(),
                                    turn_id: exec.turn.sub_id.clone(),
                                    cell_id: runtime_cell_id.to_string(),
                                },
                            );
                    }
                }
                if let Some(code_mode_host_duration) = wait_response.code_mode_host_duration() {
                    telemetry.record_code_mode_host_duration(code_mode_host_duration);
                }
                exec.session.services.elicitations.wait_until_clear().await;
                let wall_time = wait_response
                    .code_mode_host_duration()
                    .unwrap_or_else(|| started_at.elapsed());
                let mut output = handle_runtime_response(
                    &exec,
                    &call_id,
                    wait_response.into(),
                    args.max_tokens,
                    wall_time,
                )
                .await
                .map_err(FunctionCallError::RespondToModel)?;
                let wait_reason = format!("Wait wake reason: {}\n", wait_wake_reason.as_str());
                if let Some(codex_protocol::models::FunctionCallOutputContentItem::InputText {
                    text,
                }) = output.body.first_mut()
                {
                    text.insert_str(0, &wait_reason);
                } else {
                    output.body.insert(
                        0,
                        codex_protocol::models::FunctionCallOutputContentItem::InputText {
                            text: wait_reason,
                        },
                    );
                }
                Ok(boxed_tool_output(output))
            }
            _ => Err(FunctionCallError::RespondToModel(format!(
                "{WAIT_TOOL_NAME} expects JSON arguments"
            ))),
        };
        telemetry.finish(
            result
                .as_ref()
                .is_ok_and(codex_tools::ToolOutput::success_for_logging),
        );
        result
    }
}

impl CoreToolRuntime for CodeModeWaitHandler {
    fn pre_tool_use_payload(&self, _invocation: &ToolInvocation) -> Option<PreToolUsePayload> {
        // Code-mode `wait` is runtime control for an existing code cell, not a
        // standalone user action. Tool calls made from code mode still flow
        // through normal dispatch, but hooks should not block or rewrite the
        // wait loop itself.
        None
    }

    fn post_tool_use_payload(
        &self,
        _invocation: &ToolInvocation,
        _result: &dyn ToolOutput,
    ) -> Option<PostToolUsePayload> {
        // The wait result feeds code-mode control flow, so do not let
        // PostToolUse replace it with model-facing hook feedback.
        None
    }
}
