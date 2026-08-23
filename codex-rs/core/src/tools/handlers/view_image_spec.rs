//! Определяет model-visible контракт `view_image` и схему его структурированного результата.

use codex_protocol::models::VIEW_IMAGE_TOOL_NAME;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolSpec;
use serde_json::Value;
use serde_json::json;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewImageToolOptions {
    pub can_request_original_image_detail: bool,
    pub unified_image_budget: bool,
    pub include_environment_id: bool,
}

pub fn create_view_image_tool(options: ViewImageToolOptions) -> ToolSpec {
    let mut properties = BTreeMap::from([(
        "path".to_string(),
        JsonSchema::string(Some("Local filesystem path to an image file.".to_string())),
    )]);
    if options.can_request_original_image_detail && !options.unified_image_budget {
        properties.insert(
            "detail".to_string(),
            JsonSchema::string_enum(
                vec![json!("high"), json!("original")],
                Some(
                    "Image detail level. Defaults to `high`; use `original` to preserve exact resolution.".to_string(),
                ),
            ),
        );
    }
    properties.insert(
        "preview_size".to_string(),
        JsonSchema::string_enum(
            vec![json!("small"), json!("normal"), json!("large")],
            Some(
                "Optional TUI history preview size hint. Supported values are `small`, `normal`, and `large`; omit this field for the default normal preview. This only controls how large the image preview appears in the console history and does not affect the image sent to the model."
                    .to_string(),
            ),
        ),
    );
    if options.include_environment_id {
        properties.insert(
            "environment_id".to_string(),
            JsonSchema::string(Some(
                "Environment id from <environment_context>. Omit to use the primary environment."
                    .to_string(),
            )),
        );
    }

    ToolSpec::Function(ResponsesApiTool {
        name: VIEW_IMAGE_TOOL_NAME.to_string(),
        description: "View a local image file from the filesystem when visual inspection is needed. Use this for images already available on disk."
            .to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(properties, Some(vec!["path".to_string()]), Some(false.into())),
        output_schema: Some(view_image_output_schema(options)),
    })
}

fn view_image_output_schema(options: ViewImageToolOptions) -> Value {
    let mut schema = json!({
        "type": "object",
        "properties": {
            "image_url": {
                "type": "string",
                "description": "Data URL for the loaded image."
            }
        },
        "required": ["image_url"],
        "additionalProperties": false
    });
    if !options.unified_image_budget {
        schema["properties"]["detail"] = json!({
            "type": "string",
            "enum": ["high", "original"],
            "description": "Image detail hint returned by view_image. Returns `high` for default resized behavior or `original` when original resolution is preserved."
        });
        schema["required"] = json!(["image_url", "detail"]);
    }
    schema
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Проверяет ограниченный enum размера preview и отсутствие числового `preview_rows` в API.
    #[test]
    fn view_image_schema_exposes_preview_size_but_not_preview_rows() {
        let ToolSpec::Function(tool) = create_view_image_tool(ViewImageToolOptions {
            can_request_original_image_detail: false,
            unified_image_budget: false,
            include_environment_id: false,
        }) else {
            panic!("expected function tool");
        };

        let properties = tool
            .parameters
            .properties
            .expect("view_image parameters should include properties");
        let preview_size = properties
            .get("preview_size")
            .expect("preview_size should be exposed");

        assert_eq!(
            preview_size.enum_values.as_deref(),
            Some(&[json!("small"), json!("normal"), json!("large")][..])
        );
        assert!(
            !properties.contains_key("preview_rows"),
            "numeric rows must stay out of the model-visible tool API"
        );
    }
}
