//! Регрессионные tests контекстной дедупликации `read_file`.
//!
//! Проверки моделируют уже нормализованную model-visible history и доказывают,
//! что matcher использует ровно один typed content-bearing output.

use std::collections::HashSet;

use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseItem;
use codex_utils_path_uri::PathUri;
use pretty_assertions::assert_eq;
use serde_json::Map;
use serde_json::Value;
use serde_json::json;

use super::*;
use crate::tools::handlers::read_file::read_file_output;
use crate::tools::handlers::read_file::split_lines_preserving_endings;

const CURRENT_WINDOW_ID: &str = "thread:0";

fn args(path: &str, start_line: Option<usize>, end_line: Option<usize>) -> ReadFileArgs {
    ReadFileArgs {
        path: path.to_string(),
        start_line,
        end_line,
        line_numbers: true,
        environment_id: None,
    }
}

fn serialized_args(args: &ReadFileArgs) -> String {
    let mut value = Map::from_iter([
        ("path".to_string(), json!(args.path)),
        ("line_numbers".to_string(), json!(args.line_numbers)),
    ]);
    if let Some(start_line) = args.start_line {
        value.insert("start_line".to_string(), json!(start_line));
    }
    if let Some(end_line) = args.end_line {
        value.insert("end_line".to_string(), json!(end_line));
    }
    if let Some(environment_id) = &args.environment_id {
        value.insert("environment_id".to_string(), json!(environment_id));
    }
    Value::Object(value).to_string()
}

fn read_call(call_id: &str, args: &ReadFileArgs) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: READ_FILE_TOOL_NAME.to_string(),
        namespace: None,
        arguments: serialized_args(args),
        call_id: call_id.to_string(),
        encrypted_function_args: None,
        internal_chat_message_metadata_passthrough: None,
    }
}

fn read_output(call_id: &str, args: &ReadFileArgs, content: &str) -> ResponseItem {
    let output = read_file_output(args, content, 10_000).expect("read_file output");
    output_item(call_id, output, Some(true))
}

fn output_item(call_id: &str, output: String, success: Option<bool>) -> ResponseItem {
    ResponseItem::FunctionCallOutput {
        id: None,
        call_id: Some(call_id.to_string()),
        name: None,
        namespace: None,
        output: FunctionCallOutputPayload {
            body: FunctionCallOutputBody::Text(output),
            success,
        },
        internal_chat_message_metadata_passthrough: None,
    }
}

fn source(environment_id: &str, path: &str) -> ReadFileSource {
    ReadFileSource::new(
        environment_id.to_string(),
        PathUri::parse(&format!("file:///workspace/{path}")).expect("valid test PathUri"),
    )
}

fn previous_source(args: &ReadFileArgs) -> ReadFileSource {
    source(
        args.environment_id.as_deref().unwrap_or("primary"),
        &args.path,
    )
}

fn context_index(history: &[ResponseItem]) -> ReadFileContextIndex {
    let index = ReadFileContextIndex::default();
    for item in history {
        if let ResponseItem::FunctionCall {
            name,
            arguments,
            call_id,
            ..
        } = item
            && name == READ_FILE_TOOL_NAME
        {
            let args = serde_json::from_str::<ReadFileArgs>(arguments).expect("valid test args");
            index.record(CURRENT_WINDOW_ID, call_id.clone(), previous_source(&args));
        }
    }
    index
}

fn coverage(
    history: &[ResponseItem],
    current_args: &ReadFileArgs,
    current_content: &str,
    current_source: ReadFileSource,
) -> Option<ReadFileContextCoverage> {
    let index = context_index(history);
    index.synchronize(CURRENT_WINDOW_ID, history);
    coverage_with_index(
        history,
        current_args,
        current_content,
        current_source,
        &index,
    )
}

fn coverage_with_index(
    history: &[ResponseItem],
    current_args: &ReadFileArgs,
    current_content: &str,
    current_source: ReadFileSource,
    index: &ReadFileContextIndex,
) -> Option<ReadFileContextCoverage> {
    let lines = split_lines_preserving_endings(current_content);
    let requested_range = normalize_range(current_args, lines.len())
        .expect("valid current range")
        .expect("non-empty current range");
    find_context_coverage(
        history,
        &ReadFileContextRequest {
            args: current_args,
            source: current_source,
            lines: &lines,
            requested_range,
        },
        index,
    )
}

#[test]
fn read_file_context_full_output_covers_nested_range() {
    let content = "one\ntwo\nthree\nfour\n";
    let previous_args = args("example.txt", None, None);
    let history = vec![
        read_call("call-full", &previous_args),
        read_output("call-full", &previous_args, content),
    ];
    let current_args = args("example.txt", Some(2), Some(3));

    assert_eq!(
        coverage(
            &history,
            &current_args,
            content,
            source("primary", "example.txt"),
        ),
        Some(ReadFileContextCoverage {
            call_id: "call-full".to_string(),
            available_range: LineRange { start: 1, end: 4 },
        })
    );
}

#[test]
fn read_file_context_larger_range_covers_same_and_nested_ranges() {
    let content = "one\ntwo\nthree\nfour\n";
    let previous_args = args("example.txt", Some(2), Some(4));
    let history = vec![
        read_call("call-range", &previous_args),
        read_output("call-range", &previous_args, content),
    ];

    for current_args in [
        args("example.txt", Some(2), Some(4)),
        args("example.txt", Some(3), Some(3)),
    ] {
        assert_eq!(
            coverage(
                &history,
                &current_args,
                content,
                source("primary", "example.txt"),
            ),
            Some(ReadFileContextCoverage {
                call_id: "call-range".to_string(),
                available_range: LineRange { start: 2, end: 4 },
            })
        );
    }
}

#[test]
fn read_file_context_does_not_union_partial_outputs() {
    let content = "one\ntwo\nthree\nfour\n";
    let first_args = args("example.txt", Some(1), Some(2));
    let second_args = args("example.txt", Some(3), Some(4));
    let history = vec![
        read_call("call-first", &first_args),
        read_output("call-first", &first_args, content),
        read_call("call-second", &second_args),
        read_output("call-second", &second_args, content),
    ];
    let current_args = args("example.txt", Some(2), Some(3));

    assert_eq!(
        coverage(
            &history,
            &current_args,
            content,
            source("primary", "example.txt"),
        ),
        None
    );
}

#[test]
fn read_file_context_returns_none_for_larger_or_full_request() {
    let content = "one\ntwo\nthree\nfour\n";
    let previous_args = args("example.txt", Some(2), Some(3));
    let history = vec![
        read_call("call-range", &previous_args),
        read_output("call-range", &previous_args, content),
    ];

    for current_args in [
        args("example.txt", Some(1), Some(3)),
        args("example.txt", None, None),
    ] {
        assert_eq!(
            coverage(
                &history,
                &current_args,
                content,
                source("primary", "example.txt"),
            ),
            None
        );
    }
}

#[test]
fn read_file_context_requires_current_content_and_matching_rendering() {
    let previous_content = "one\ntwo\nthree\n";
    let previous_args = args("example.txt", Some(1), Some(2));
    let history = vec![
        read_call("call-old", &previous_args),
        read_output("call-old", &previous_args, previous_content),
    ];
    let current_args = args("example.txt", Some(1), Some(2));

    assert_eq!(
        coverage(
            &history,
            &current_args,
            "one\nchanged\nthree\n",
            source("primary", "example.txt"),
        ),
        None
    );

    let raw_args = ReadFileArgs {
        line_numbers: false,
        ..args("example.txt", Some(1), Some(2))
    };
    assert_eq!(
        coverage(
            &history,
            &raw_args,
            previous_content,
            source("primary", "example.txt"),
        ),
        None
    );
}

#[test]
fn read_file_context_ignores_changes_outside_nested_request() {
    let previous_content = "old-one\ntwo\nthree\nold-four\n";
    let current_content = "new-one\ntwo\nthree\nnew-four\n";
    let previous_args = args("example.txt", None, None);
    let history = vec![
        read_call("call-full", &previous_args),
        read_output("call-full", &previous_args, previous_content),
    ];
    let current_args = args("example.txt", Some(2), Some(3));

    assert_eq!(
        coverage(
            &history,
            &current_args,
            current_content,
            source("primary", "example.txt"),
        ),
        Some(ReadFileContextCoverage {
            call_id: "call-full".to_string(),
            available_range: LineRange { start: 1, end: 4 },
        })
    );
}

#[test]
fn read_file_context_compares_nested_raw_lines_without_number_prefixes() {
    let content = "one\ntwo\nthree\nfour\n";
    let previous_args = ReadFileArgs {
        line_numbers: false,
        ..args("example.txt", None, None)
    };
    let history = vec![
        read_call("call-raw", &previous_args),
        read_output("call-raw", &previous_args, content),
    ];
    let current_args = ReadFileArgs {
        line_numbers: false,
        ..args("example.txt", Some(2), Some(3))
    };

    assert_eq!(
        coverage(
            &history,
            &current_args,
            content,
            source("primary", "example.txt"),
        ),
        Some(ReadFileContextCoverage {
            call_id: "call-raw".to_string(),
            available_range: LineRange { start: 1, end: 4 },
        })
    );
}

#[test]
fn read_file_context_requires_matching_environment_and_path_uri() {
    let content = "one\ntwo\n";
    let previous_args = ReadFileArgs {
        environment_id: Some("env-a".to_string()),
        ..args("example.txt", None, None)
    };
    let history = vec![
        read_call("call-env", &previous_args),
        read_output("call-env", &previous_args, content),
    ];
    let current_args = previous_args;

    for current_source in [source("env-b", "example.txt"), source("env-a", "other.txt")] {
        assert_eq!(
            coverage(&history, &current_args, content, current_source),
            None
        );
    }
}

#[test]
fn read_file_context_uses_recorded_source_for_implicit_primary_environment() {
    let content = "one\ntwo\n";
    let previous_args = args("example.txt", None, None);
    let history = vec![
        read_call("call-primary", &previous_args),
        read_output("call-primary", &previous_args, content),
    ];
    let index = ReadFileContextIndex::default();
    index.record(
        CURRENT_WINDOW_ID,
        "call-primary".to_string(),
        source("previous-primary", "example.txt"),
    );
    index.synchronize(CURRENT_WINDOW_ID, &history);
    let lines = split_lines_preserving_endings(content);
    let requested_range = normalize_range(&previous_args, lines.len())
        .expect("valid range")
        .expect("non-empty range");

    assert_eq!(
        find_context_coverage(
            &history,
            &ReadFileContextRequest {
                args: &previous_args,
                source: source("current-primary", "example.txt"),
                lines: &lines,
                requested_range,
            },
            &index,
        ),
        None
    );
}

#[test]
fn read_file_context_rehydrates_coverage_after_cold_resume() {
    let content = "one\ntwo\nthree\n";
    let previous_args = args("example.txt", None, None);
    let persisted_output =
        read_file_output(&previous_args, content, 10_000).expect("read_file output");
    let history = vec![
        read_call("call-before-resume", &previous_args),
        output_item("call-before-resume", persisted_output, None),
    ];
    let current_args = args("example.txt", Some(2), Some(3));
    let index = ReadFileContextIndex::default();
    index.restore(CURRENT_WINDOW_ID.to_string(), &history, &HashSet::new());

    assert_eq!(
        coverage_with_index(
            &history,
            &current_args,
            content,
            source("primary-after-resume", "example.txt"),
            &index,
        ),
        Some(ReadFileContextCoverage {
            call_id: "call-before-resume".to_string(),
            available_range: LineRange { start: 1, end: 3 },
        })
    );
}

#[test]
fn read_file_context_resume_fallback_requires_matching_logical_source_arguments() {
    let content = "one\ntwo\n";
    let previous_args = ReadFileArgs {
        environment_id: Some("env-a".to_string()),
        ..args("example.txt", None, None)
    };
    let history = vec![
        read_call("call-before-resume", &previous_args),
        read_output("call-before-resume", &previous_args, content),
    ];
    let index = ReadFileContextIndex::default();
    index.restore(CURRENT_WINDOW_ID.to_string(), &history, &HashSet::new());

    for current_args in [
        ReadFileArgs {
            environment_id: Some("env-b".to_string()),
            ..args("example.txt", None, None)
        },
        ReadFileArgs {
            environment_id: Some("env-a".to_string()),
            ..args("other.txt", None, None)
        },
    ] {
        assert_eq!(
            coverage_with_index(
                &history,
                &current_args,
                content,
                source(
                    current_args.environment_id.as_deref().unwrap_or("primary"),
                    &current_args.path,
                ),
                &index,
            ),
            None
        );
    }
}

#[test]
fn read_file_context_rejects_reference_chaining_and_missing_content_output() {
    let content = "one\ntwo\n";
    let previous_args = args("example.txt", None, None);
    let reference = "ReadFile: example.txt\nStatus: already_in_context\nCoverage: requested=1-2 available=1-2 complete=yes\nLineNumbers: yes\nCoveredBy: call-original\n";
    let reference_history = vec![
        read_call("call-reference", &previous_args),
        output_item("call-reference", reference.to_string(), Some(true)),
    ];

    assert_eq!(
        coverage(
            &reference_history,
            &previous_args,
            content,
            source("primary", "example.txt"),
        ),
        None
    );
    assert_eq!(
        coverage(
            &[read_call("call-without-output", &previous_args)],
            &previous_args,
            content,
            source("primary", "example.txt"),
        ),
        None
    );
}

#[test]
fn read_file_context_index_drops_sources_absent_from_active_history() {
    let index = ReadFileContextIndex::default();
    index.record(
        CURRENT_WINDOW_ID,
        "call-removed".to_string(),
        source("primary", "removed.txt"),
    );

    index.synchronize(CURRENT_WINDOW_ID, &[]);

    assert_eq!(index.provenance("call-removed"), None);
}

#[test]
fn read_file_context_window_change_invalidates_retained_output() {
    let content = "one\ntwo\n";
    let previous_args = args("example.txt", None, None);
    let history = vec![
        read_call("call-before-compaction", &previous_args),
        read_output("call-before-compaction", &previous_args, content),
    ];
    let index = context_index(&history);

    assert!(
        coverage_with_index(
            &history,
            &previous_args,
            content,
            source("primary", "example.txt"),
            &index,
        )
        .is_some()
    );

    index.synchronize("thread:1", &history);

    assert_eq!(
        coverage_with_index(
            &history,
            &previous_args,
            content,
            source("primary", "example.txt"),
            &index,
        ),
        None
    );
}

#[test]
fn read_file_context_resume_excludes_compaction_replacement_calls() {
    let content = "one\ntwo\n";
    let previous_args = args("example.txt", None, None);
    let call_id = "call-copied-by-compaction";
    let history = vec![
        read_call(call_id, &previous_args),
        read_output(call_id, &previous_args, content),
    ];
    let index = ReadFileContextIndex::default();
    index.restore(
        "thread:1".to_string(),
        &history,
        &HashSet::from([call_id.to_string()]),
    );

    assert_eq!(
        coverage_with_index(
            &history,
            &previous_args,
            content,
            source("primary", "example.txt"),
            &index,
        ),
        None
    );
}

#[test]
fn read_file_context_ignores_unsuccessful_output() {
    let content = "one\ntwo\n";
    let previous_args = args("example.txt", None, None);
    let output = read_file_output(&previous_args, content, 10_000).expect("read_file output");
    let history = vec![
        read_call("call-failed", &previous_args),
        output_item("call-failed", output, Some(false)),
    ];

    assert_eq!(
        coverage(
            &history,
            &previous_args,
            content,
            source("primary", "example.txt"),
        ),
        None
    );
}

/// Проверяет, что `FunctionCallOutput` без `call_id` не создаёт контекстное покрытие.
#[test]
fn read_file_context_ignores_output_without_call_id() {
    let content = "one\ntwo\n";
    let previous_args = args("example.txt", None, None);
    let mut output = read_output("call-without-output-id", &previous_args, content);
    let ResponseItem::FunctionCallOutput { call_id, .. } = &mut output else {
        panic!("expected function call output");
    };
    *call_id = None;
    let history = vec![read_call("call-without-output-id", &previous_args), output];

    assert_eq!(
        coverage(
            &history,
            &previous_args,
            content,
            source("primary", "example.txt"),
        ),
        None
    );
}

#[test]
fn read_file_context_reference_output_has_stable_contract() {
    let current_args = args("docs/example.md", Some(40), Some(80));
    let output = render_already_in_context(
        &current_args,
        LineRange { start: 40, end: 80 },
        &ReadFileContextCoverage {
            call_id: "call_123".to_string(),
            available_range: LineRange { start: 1, end: 240 },
        },
    );

    assert_eq!(
        output,
        "ReadFile: docs/example.md\nStatus: already_in_context\nCoverage: requested=40-80 available=1-240 complete=yes\nLineNumbers: yes\nCoveredBy: call_123\n"
    );
}
