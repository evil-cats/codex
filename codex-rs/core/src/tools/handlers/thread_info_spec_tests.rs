use super::*;
use pretty_assertions::assert_eq;
use serde_json::json;

#[test]
fn thread_info_tool_declares_optional_thread_id() {
    let ToolSpec::Function(tool) = create_get_thread_info_tool() else {
        panic!("expected function tool");
    };

    let parameters = &tool.parameters;
    let properties = tool
        .parameters
        .properties
        .as_ref()
        .expect("get_thread_info parameters should include properties");
    let thread_id = properties
        .get("thread_id")
        .expect("thread_id should be exposed");
    let description = thread_id
        .description
        .as_deref()
        .expect("thread_id should have a description");

    assert_eq!(tool.name, GET_THREAD_INFO_TOOL_NAME);
    assert_eq!(parameters.required, Some(Vec::new()));
    assert_eq!(parameters.additional_properties, Some(false.into()));
    assert!(description.contains("Defaults to the current thread"));
}

#[test]
fn thread_info_output_schema_has_expected_fields() {
    let ToolSpec::Function(tool) = create_get_thread_info_tool() else {
        panic!("expected function tool");
    };
    let output_schema = tool.output_schema.expect("output schema");

    assert_eq!(
        output_schema["required"],
        json!(["thread_id", "session_id", "rollout_path", "agent_name"])
    );
    assert_eq!(
        output_schema["properties"]["session_id"]["type"],
        json!(["string", "null"])
    );
    assert_eq!(
        output_schema["properties"]["rollout_path"]["type"],
        json!(["string", "null"])
    );
    assert_eq!(
        output_schema["properties"]["agent_name"]["type"],
        json!(["string", "null"])
    );
}
