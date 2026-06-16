use crate::config::Config;
use crate::function_tool::FunctionCallError;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::context::boxed_tool_output;
use crate::tools::handlers::parse_arguments;
use crate::tools::handlers::thread_info_spec::GET_THREAD_INFO_TOOL_NAME;
use crate::tools::handlers::thread_info_spec::create_get_thread_info_tool;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::ToolExecutor;
use codex_config::ConfigLayerSource;
use codex_protocol::AgentPath;
use codex_protocol::SessionId;
use codex_protocol::ThreadId;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_thread_store::ReadThreadParams;
use codex_thread_store::StoredThread;
use codex_thread_store::ThreadStore;
use codex_thread_store::ThreadStoreError;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use toml::Value as TomlValue;

const MAX_PARENT_CHAIN_DEPTH: usize = 64;

pub struct ThreadInfoHandler;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThreadInfoArgs {
    #[serde(default)]
    thread_id: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ThreadInfoResponse {
    thread_id: String,
    session_id: Option<String>,
    rollout_path: Option<PathBuf>,
    agent_name: Option<String>,
}

impl ToolExecutor<ToolInvocation> for ThreadInfoHandler {
    fn tool_name(&self) -> ToolName {
        ToolName::plain(GET_THREAD_INFO_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        create_get_thread_info_tool()
    }

    fn handle(&self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'_> {
        Box::pin(async move {
            let ToolInvocation {
                session,
                turn,
                payload,
                ..
            } = invocation;

            let arguments = match payload {
                ToolPayload::Function { arguments } => arguments,
                _ => {
                    return Err(FunctionCallError::RespondToModel(format!(
                        "{GET_THREAD_INFO_TOOL_NAME} handler received unsupported payload"
                    )));
                }
            };

            let args: ThreadInfoArgs = parse_arguments(&arguments)?;
            let response = thread_info_response(&session, &turn, args).await?;
            let content = serde_json::to_string(&response).map_err(|err| {
                FunctionCallError::Fatal(format!(
                    "failed to serialize {GET_THREAD_INFO_TOOL_NAME} response: {err}"
                ))
            })?;

            Ok(boxed_tool_output(FunctionToolOutput::from_text(
                content,
                Some(true),
            )))
        })
    }
}

impl CoreToolRuntime for ThreadInfoHandler {}

async fn thread_info_response(
    session: &Session,
    turn: &TurnContext,
    args: ThreadInfoArgs,
) -> Result<ThreadInfoResponse, FunctionCallError> {
    let current_thread_id = session.thread_id();
    let thread_id = parse_requested_thread_id(args.thread_id, current_thread_id)?;

    if thread_id == current_thread_id {
        return current_thread_info(session, turn).await;
    }

    stored_thread_info(session, thread_id).await
}

async fn current_thread_info(
    session: &Session,
    turn: &TurnContext,
) -> Result<ThreadInfoResponse, FunctionCallError> {
    let rollout_path = current_rollout_path(session).await?;
    Ok(ThreadInfoResponse {
        thread_id: session.thread_id().to_string(),
        session_id: Some(session.session_id().to_string()),
        rollout_path,
        agent_name: current_agent_name(turn),
    })
}

async fn stored_thread_info(
    session: &Session,
    thread_id: ThreadId,
) -> Result<ThreadInfoResponse, FunctionCallError> {
    let thread_store = Arc::clone(&session.services.thread_store);
    let stored_thread = thread_store
        .read_thread(read_thread_params(thread_id))
        .await
        .map_err(|err| read_thread_error(thread_id, err))?;
    let session_id = resolve_stored_session_id(&thread_store, &stored_thread).await;
    let agent_name = stored_agent_name(&stored_thread);
    Ok(ThreadInfoResponse {
        thread_id: stored_thread.thread_id.to_string(),
        session_id,
        rollout_path: stored_thread.rollout_path,
        agent_name,
    })
}

async fn current_rollout_path(session: &Session) -> Result<Option<PathBuf>, FunctionCallError> {
    session
        .try_ensure_rollout_materialized()
        .await
        .map_err(rollout_materialize_error)?;
    session
        .current_rollout_path()
        .await
        .map_err(rollout_path_error)
}

async fn resolve_stored_session_id(
    thread_store: &Arc<dyn ThreadStore>,
    stored_thread: &StoredThread,
) -> Option<String> {
    let mut thread_id = stored_thread.thread_id;
    let mut parent_thread_id = stored_thread.parent_thread_id;
    let mut seen_thread_ids = vec![thread_id];

    for _ in 0..MAX_PARENT_CHAIN_DEPTH {
        let Some(parent_id) = parent_thread_id else {
            return Some(SessionId::from(thread_id).to_string());
        };

        if seen_thread_ids.contains(&parent_id) {
            return None;
        }

        let parent_thread = thread_store
            .read_thread(read_thread_params(parent_id))
            .await
            .ok()?;
        thread_id = parent_thread.thread_id;
        parent_thread_id = parent_thread.parent_thread_id;
        seen_thread_ids.push(thread_id);
    }

    None
}

fn read_thread_params(thread_id: ThreadId) -> ReadThreadParams {
    ReadThreadParams {
        thread_id,
        include_archived: true,
        include_history: false,
    }
}

fn parse_requested_thread_id(
    requested_thread_id: Option<String>,
    current_thread_id: ThreadId,
) -> Result<ThreadId, FunctionCallError> {
    let Some(requested_thread_id) = requested_thread_id else {
        return Ok(current_thread_id);
    };
    let requested_thread_id = requested_thread_id.trim();
    if requested_thread_id.is_empty() {
        return Err(FunctionCallError::RespondToModel(
            "thread_id must be a non-empty UUID string when provided".to_string(),
        ));
    }

    ThreadId::from_string(requested_thread_id).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "invalid thread_id `{requested_thread_id}`: {err}"
        ))
    })
}

fn current_agent_name(turn: &TurnContext) -> Option<String> {
    if let Some(agent_name) = agent_name_from_session_source(&turn.session_source) {
        return Some(agent_name);
    }

    if turn.session_source.is_non_root_agent() {
        return None;
    }

    root_agent_name_from_config(&turn.config)
}

fn stored_agent_name(stored_thread: &StoredThread) -> Option<String> {
    agent_name_from_stored_fields(
        stored_thread.agent_role.as_deref(),
        stored_thread.agent_path.as_deref(),
        stored_thread.agent_nickname.as_deref(),
    )
}

fn agent_name_from_stored_fields(
    agent_role: Option<&str>,
    agent_path: Option<&str>,
    agent_nickname: Option<&str>,
) -> Option<String> {
    non_empty_trimmed_opt(agent_role)
        .or_else(|| agent_path.and_then(agent_path_name_from_str))
        .or_else(|| non_empty_trimmed_opt(agent_nickname))
}

fn agent_name_from_session_source(session_source: &SessionSource) -> Option<String> {
    let SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        agent_path,
        agent_nickname,
        agent_role,
        ..
    }) = session_source
    else {
        return None;
    };

    non_empty_trimmed_opt(agent_role.as_deref())
        .or_else(|| agent_path.as_ref().and_then(agent_path_name))
        .or_else(|| non_empty_trimmed_opt(agent_nickname.as_deref()))
}

fn agent_path_name(agent_path: &AgentPath) -> Option<String> {
    non_empty_trimmed(agent_path.name())
}

fn agent_path_name_from_str(agent_path: &str) -> Option<String> {
    agent_path.rsplit('/').find_map(non_empty_trimmed)
}

fn root_agent_name_from_config(config: &Config) -> Option<String> {
    let effective_config = config.config_layer_stack.effective_config();
    let active_profile = active_user_profile(config);
    root_agent_name_from_values(&effective_config, active_profile.as_deref())
}

fn root_agent_name_from_values(
    effective_config: &TomlValue,
    active_profile: Option<&str>,
) -> Option<String> {
    effective_config
        .get("name")
        .and_then(TomlValue::as_str)
        .and_then(non_empty_trimmed)
        .or_else(|| active_profile.and_then(non_empty_trimmed))
}

fn active_user_profile(config: &Config) -> Option<String> {
    let active_user_layer = config.config_layer_stack.get_active_user_layer()?;
    let ConfigLayerSource::User { profile, .. } = &active_user_layer.name else {
        return None;
    };
    profile.clone()
}

fn non_empty_trimmed_opt(value: Option<&str>) -> Option<String> {
    value.and_then(non_empty_trimmed)
}

fn non_empty_trimmed(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn read_thread_error(thread_id: ThreadId, err: ThreadStoreError) -> FunctionCallError {
    match err {
        ThreadStoreError::ThreadNotFound { .. } => {
            FunctionCallError::RespondToModel(format!("thread `{thread_id}` was not found"))
        }
        ThreadStoreError::InvalidRequest { message } => FunctionCallError::RespondToModel(message),
        err => FunctionCallError::Fatal(format!("failed to read thread `{thread_id}`: {err}")),
    }
}

fn rollout_materialize_error(err: std::io::Error) -> FunctionCallError {
    FunctionCallError::Fatal(format!("failed to materialize current rollout: {err}"))
}

fn rollout_path_error(err: anyhow::Error) -> FunctionCallError {
    FunctionCallError::Fatal(format!("failed to read current rollout path: {err}"))
}

#[cfg(test)]
#[path = "thread_info_tests.rs"]
mod tests;
