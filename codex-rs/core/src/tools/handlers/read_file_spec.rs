//! Responses API tool definition for reading UTF-8 text files.

use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolSpec;
use std::collections::BTreeMap;

pub const READ_FILE_TOOL_NAME: &str = "read_file";

const READ_FILE_TOOL_DESCRIPTION: &str = "\
Read a known UTF-8 text file from the workspace, optionally by inclusive \
1-based line range, without running a shell command. Use this tool to read a \
selected text file or line range instead of shell readers such as cat, sed -n, \
nl, head, or tail. Continue using rg/rg --files for search and discovery. The \
result includes total/requested/returned line metadata and complete=yes/no; \
content may be shortened only by dropping whole trailing lines to fit the \
configured content token limit. If complete=no, continue with another range \
before treating the requested content as fully read.";

pub fn create_read_file_tool(include_environment_id: bool) -> ToolSpec {
    let mut properties = BTreeMap::from([
        (
            "path".to_string(),
            JsonSchema::string(Some(
                "Path to a regular UTF-8 text file within the readable workspace/sandbox scope."
                    .to_string(),
            )),
        ),
        (
            "start_line".to_string(),
            JsonSchema::integer(Some(
                "Optional 1-based inclusive start line. Must be provided together with end_line. Omit both start_line and end_line to request the whole file.".to_string(),
            )),
        ),
        (
            "end_line".to_string(),
            JsonSchema::integer(Some(
                "Optional 1-based inclusive end line. Must be provided together with start_line. Omit both start_line and end_line to request the whole file.".to_string(),
            )),
        ),
        (
            "line_numbers".to_string(),
            JsonSchema::boolean(Some(
                "Optional. Defaults to true. When true, prefixes each returned line with its source line number. Set false only when raw file content is needed for exact copying, formatting, or comparison.".to_string(),
            )),
        ),
    ]);
    if include_environment_id {
        properties.insert(
            "environment_id".to_string(),
            JsonSchema::string(Some(
                "Environment id from <environment_context>. Omit to use the primary environment."
                    .to_string(),
            )),
        );
    }

    ToolSpec::Function(ResponsesApiTool {
        name: READ_FILE_TOOL_NAME.to_string(),
        description: READ_FILE_TOOL_DESCRIPTION.to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            properties,
            Some(vec!["path".to_string()]),
            Some(false.into()),
        ),
        output_schema: None,
    })
}

#[cfg(test)]
#[path = "read_file_spec_tests.rs"]
mod tests;
