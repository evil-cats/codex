//! Responses API tool definition for reading Codex thread metadata.

use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolSpec;
use serde_json::Value as JsonValue;
use serde_json::json;
use std::collections::BTreeMap;

pub const GET_THREAD_INFO_TOOL_NAME: &str = "get_thread_info";

const GET_THREAD_INFO_TOOL_DESCRIPTION: &str = "\
Get metadata for a concrete Codex thread. Returns thread_id, session_id, \
rollout_path, and agent_name. The thread_id identifies the persisted \
thread/rollout and is the key to a specific JSONL log. The session_id \
identifies the shared root-agent session tree; it equals thread_id for a root \
session and may differ for subagent threads. Use thread_id to inspect a \
specific rollout, and session_id to group related root/subagent threads.";

pub fn create_get_thread_info_tool() -> ToolSpec {
    let properties = BTreeMap::from([(
        "thread_id".to_string(),
        JsonSchema::string(Some(
            "Optional concrete Codex thread id. Defaults to the current thread when omitted. Pass a thread_id when you need metadata for a specific JSONL rollout.".to_string(),
        )),
    )]);

    ToolSpec::Function(ResponsesApiTool {
        name: GET_THREAD_INFO_TOOL_NAME.to_string(),
        description: GET_THREAD_INFO_TOOL_DESCRIPTION.to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(properties, Some(Vec::new()), Some(false.into())),
        output_schema: Some(get_thread_info_output_schema()),
    })
}

fn nullable_string(description: &str) -> JsonValue {
    json!({
        "type": ["string", "null"],
        "description": description,
    })
}

fn get_thread_info_output_schema() -> JsonValue {
    json!({
        "type": "object",
        "properties": {
            "thread_id": {
                "type": "string",
                "description": "Concrete persisted thread id that identifies the returned rollout log."
            },
            "session_id": nullable_string(
                "Shared root-agent session tree id. Equals thread_id for a root session and may differ for subagent threads. Null when it cannot be resolved."
            ),
            "rollout_path": nullable_string(
                "Local path to the JSONL rollout log for this thread, or null when unavailable."
            ),
            "agent_name": nullable_string(
                "Configured agent role name for subagents, or profile/config name for the root session. Null when unavailable."
            )
        },
        "required": ["thread_id", "session_id", "rollout_path", "agent_name"],
        "additionalProperties": false
    })
}

#[cfg(test)]
#[path = "thread_info_spec_tests.rs"]
mod tests;
