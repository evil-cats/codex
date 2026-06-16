//! Responses API tool definition for reading Codex thread metadata.

use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolSpec;
use serde_json::Value as JsonValue;
use serde_json::json;
use std::collections::BTreeMap;

pub const GET_THREAD_INFO_TOOL_NAME: &str = "get_thread_info";

pub fn create_get_thread_info_tool() -> ToolSpec {
    let properties = BTreeMap::from([(
        "thread_id".to_string(),
        JsonSchema::string(Some(
            "Optional concrete Codex thread id. Defaults to the current thread when omitted."
                .to_string(),
        )),
    )]);

    ToolSpec::Function(ResponsesApiTool {
        name: GET_THREAD_INFO_TOOL_NAME.to_string(),
        description: "Get metadata for a concrete Codex thread, including its rollout JSONL path, shared session id, and configured agent or profile name. The thread id identifies the persisted thread/rollout; the session id identifies the shared root agent session tree.".to_string(),
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
                "description": "Concrete Codex thread id for the returned thread."
            },
            "session_id": nullable_string(
                "Shared root-agent session tree id, when it can be resolved."
            ),
            "rollout_path": nullable_string(
                "Local path to the rollout JSONL for this thread, when available."
            ),
            "agent_name": nullable_string(
                "Configured agent role name for a subagent, or configured profile name for the current root thread, when available."
            )
        },
        "required": ["thread_id", "session_id", "rollout_path", "agent_name"],
        "additionalProperties": false
    })
}

#[cfg(test)]
#[path = "thread_info_spec_tests.rs"]
mod tests;
