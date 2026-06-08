use super::*;
use pretty_assertions::assert_eq;
use serde_json::json;

#[test]
fn system_time_tool_defaults_are_documented() {
    let ToolSpec::Function(tool) = create_get_system_time_tool() else {
        panic!("expected function tool");
    };

    let properties = tool
        .parameters
        .properties
        .expect("get_system_time parameters should include properties");
    let offset = properties.get("offset").expect("offset should be exposed");
    let offset_description = offset
        .description
        .as_deref()
        .expect("offset should have a description");
    let full = properties.get("full").expect("full should be exposed");
    let full_description = full
        .description
        .as_deref()
        .expect("full should have a description");

    assert_eq!(tool.name, GET_SYSTEM_TIME_TOOL_NAME);
    assert!(offset_description.contains(r#"Defaults to "local""#));
    assert!(offset_description.contains("host system local time"));
    assert!(offset_description.contains("IANA timezone names"));
    assert!(full_description.contains("Defaults to false"));
    assert!(full_description.contains("returns only `formatted`"));
}

#[test]
fn system_time_output_schema_has_short_default_and_full_shapes() {
    let ToolSpec::Function(tool) = create_get_system_time_tool() else {
        panic!("expected function tool");
    };
    let output_schema = tool.output_schema.expect("output schema");

    assert_eq!(output_schema["oneOf"][0]["required"], json!(["formatted"]));
    assert_eq!(
        output_schema["oneOf"][1]["required"],
        json!([
            "formatted",
            "format",
            "offset",
            "resolved_offset",
            "resolved_offset_seconds",
            "unix_seconds",
            "unix_millis",
            "rfc3339",
            "utc_rfc3339"
        ])
    );
}
