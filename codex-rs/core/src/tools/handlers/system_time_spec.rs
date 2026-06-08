//! Responses API tool definition for reading the host system time.

use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolSpec;
use serde_json::json;
use std::collections::BTreeMap;

pub const GET_SYSTEM_TIME_TOOL_NAME: &str = "get_system_time";
pub const DEFAULT_SYSTEM_TIME_FORMAT: &str = "%H:%M";
pub const DEFAULT_SYSTEM_TIME_OFFSET: &str = "local";

pub fn create_get_system_time_tool() -> ToolSpec {
    let properties = BTreeMap::from([
        (
            "format".to_string(),
            JsonSchema::string(Some(
                "Optional chrono strftime format string. Defaults to `%H:%M`.".to_string(),
            )),
        ),
        (
            "offset".to_string(),
            JsonSchema::string(Some(
                r#"Optional offset to use when formatting the current host system time. Defaults to "local". Use "local" for the host system local time, "utc" for UTC, or "+HH:MM"/"-HH:MM" for a fixed offset. IANA timezone names such as "Europe/Moscow" are not supported."#
                    .to_string(),
            )),
        ),
        (
            "full".to_string(),
            JsonSchema::boolean(Some(
                "Optional. Defaults to false. When false, returns only `formatted`; when true, returns formatted time plus offset and timestamp metadata.".to_string(),
            )),
        ),
    ]);

    ToolSpec::Function(ResponsesApiTool {
        name: GET_SYSTEM_TIME_TOOL_NAME.to_string(),
        description: "Get the current host system time without running a shell command."
            .to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(properties, Some(Vec::new()), Some(false.into())),
        output_schema: Some(get_system_time_output_schema()),
    })
}

fn get_system_time_output_schema() -> serde_json::Value {
    json!({
        "oneOf": [
            {
                "type": "object",
                "properties": {
                    "formatted": {
                        "type": "string",
                        "description": "The current time formatted with the requested chrono strftime format."
                    }
                },
                "required": ["formatted"],
                "additionalProperties": false
            },
            {
                "type": "object",
                "properties": {
                    "formatted": {
                        "type": "string",
                        "description": "The current time formatted with the requested chrono strftime format."
                    },
                    "format": {
                        "type": "string",
                        "description": "The chrono strftime format used for `formatted`."
                    },
                    "offset": {
                        "type": "string",
                        "description": "The requested offset mode: `local`, `utc`, or a fixed offset."
                    },
                    "resolved_offset": {
                        "type": "string",
                        "description": "The concrete offset used for formatting, in `+HH:MM`/`-HH:MM` form."
                    },
                    "resolved_offset_seconds": {
                        "type": "integer",
                        "description": "The concrete offset used for formatting, in seconds east of UTC."
                    },
                    "unix_seconds": {
                        "type": "integer",
                        "description": "Unix timestamp for the sampled instant, in seconds."
                    },
                    "unix_millis": {
                        "type": "integer",
                        "description": "Unix timestamp for the sampled instant, in milliseconds."
                    },
                    "rfc3339": {
                        "type": "string",
                        "description": "RFC3339 timestamp for the sampled instant in the selected offset."
                    },
                    "utc_rfc3339": {
                        "type": "string",
                        "description": "RFC3339 UTC timestamp for the sampled instant."
                    }
                },
                "required": [
                    "formatted",
                    "format",
                    "offset",
                    "resolved_offset",
                    "resolved_offset_seconds",
                    "unix_seconds",
                    "unix_millis",
                    "rfc3339",
                    "utc_rfc3339"
                ],
                "additionalProperties": false
            }
        ]
    })
}

#[cfg(test)]
#[path = "system_time_spec_tests.rs"]
mod tests;
