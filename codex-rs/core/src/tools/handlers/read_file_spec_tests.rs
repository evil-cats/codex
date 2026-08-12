use super::*;
use pretty_assertions::assert_eq;

#[test]
fn read_file_tool_declares_core_arguments_without_token_limit() {
    let ToolSpec::Function(tool) = create_read_file_tool(/*include_environment_id*/ false) else {
        panic!("expected function tool");
    };

    let properties = tool
        .parameters
        .properties
        .expect("read_file parameters should include properties");

    assert_eq!(tool.name, READ_FILE_TOOL_NAME);
    assert_eq!(tool.parameters.required, Some(vec!["path".to_string()]));
    assert_eq!(tool.parameters.additional_properties, Some(false.into()));
    assert!(tool.output_schema.is_none());
    assert!(properties.contains_key("path"));
    assert!(properties.contains_key("start_line"));
    assert!(properties.contains_key("end_line"));
    assert!(properties.contains_key("line_numbers"));
    assert!(!properties.contains_key("environment_id"));
    assert!(!properties.contains_key("token_limit"));
    assert!(!properties.contains_key("max_tokens"));
    assert!(tool.description.contains("without running a shell command"));
    assert!(tool.description.contains("instead of shell readers"));
    assert!(tool.description.contains("Continue using rg/rg --files"));
    assert!(tool.description.contains("complete=yes/no"));
    assert!(tool.description.contains("Status: already_in_context"));
    assert!(
        tool.description
            .contains("one earlier content-bearing output")
    );
    assert!(
        tool.description
            .contains("do not assemble it from partial overlaps")
    );
}

#[test]
fn read_file_tool_can_include_environment_id_for_multiple_environments() {
    let ToolSpec::Function(tool) = create_read_file_tool(/*include_environment_id*/ true) else {
        panic!("expected function tool");
    };

    let properties = tool
        .parameters
        .properties
        .expect("read_file parameters should include properties");

    assert!(properties.contains_key("environment_id"));
}
