use super::*;
use pretty_assertions::assert_eq;

fn args(path: &str) -> ReadFileArgs {
    ReadFileArgs {
        path: path.to_string(),
        start_line: None,
        end_line: None,
        line_numbers: true,
        environment_id: None,
    }
}

fn assert_model_error(err: FunctionCallError, expected: &str) {
    let FunctionCallError::RespondToModel(message) = err else {
        panic!("expected model-facing error");
    };
    assert!(
        message.contains(expected),
        "expected error to contain `{expected}`, got `{message}`"
    );
}

#[test]
fn reads_whole_file_with_line_numbers_by_default() {
    let output = read_file_output(&args("docs/example.md"), "alpha\nbeta\n", 10_000)
        .expect("read file output");

    assert_eq!(
        output,
        "ReadFile: docs/example.md\nLines: total=2 requested=1-2 returned=1-2 complete=yes\nLineNumbers: yes\n\n1 | alpha\n2 | beta\n"
    );
}

#[test]
fn reads_inclusive_line_range() {
    let output = read_file_output(
        &ReadFileArgs {
            start_line: Some(2),
            end_line: Some(3),
            ..args("src/lib.rs")
        },
        "one\ntwo\nthree\nfour\n",
        10_000,
    )
    .expect("read file output");

    assert_eq!(
        output,
        "ReadFile: src/lib.rs\nLines: total=4 requested=2-3 returned=2-3 complete=yes\nLineNumbers: yes\n\n2 | two\n3 | three\n"
    );
}

#[test]
fn line_numbers_false_returns_raw_content() {
    let output = read_file_output(
        &ReadFileArgs {
            start_line: Some(2),
            end_line: Some(3),
            line_numbers: false,
            ..args("plain.txt")
        },
        "one\ntwo\nthree\n",
        10_000,
    )
    .expect("read file output");

    assert_eq!(
        output,
        "ReadFile: plain.txt\nLines: total=3 requested=2-3 returned=2-3 complete=yes\nLineNumbers: no\n\ntwo\nthree\n"
    );
}

#[test]
fn empty_file_returns_empty_ranges() {
    let output = read_file_output(&args("empty.txt"), "", 10_000).expect("read file output");

    assert_eq!(
        output,
        "ReadFile: empty.txt\nLines: total=0 requested=empty returned=empty complete=yes\nLineNumbers: yes\n\n"
    );
}

#[test]
fn trims_only_whole_trailing_lines_to_fit_limit() {
    let output =
        read_file_output(&args("long.txt"), "aaaa\nbbbb\ncccc\n", 3).expect("read file output");

    assert_eq!(
        output,
        "ReadFile: long.txt\nLines: total=3 requested=1-3 returned=1-2 complete=no\nLineNumbers: yes\n\n1 | aaaa\n2 | bbbb\n"
    );
}

#[test]
fn header_and_line_numbers_do_not_count_against_content_limit() {
    let output = read_file_output(
        &ReadFileArgs {
            path: "numbered.txt".to_string(),
            start_line: Some(1000),
            end_line: Some(1000),
            line_numbers: true,
            environment_id: None,
        },
        &format!("{}\n", "\n".repeat(999)),
        1,
    )
    .expect("read file output");

    assert!(
        output.contains("Lines: total=1000 requested=1000-1000 returned=1000-1000 complete=yes")
    );
    assert!(output.ends_with("1000 | \n"));
}

#[test]
fn reports_first_line_that_exceeds_limit_with_header() {
    let output =
        read_file_output(&args("too-long.txt"), "aaaa\nbbbb\n", 1).expect("read file output");

    assert_eq!(
        output,
        "ReadFile: too-long.txt\nLines: total=2 requested=1-2 returned=empty complete=no\nError: line 1 exceeds ReadFile content token limit\n"
    );
}

#[test]
fn clamps_end_line_to_total_lines() {
    let output = read_file_output(
        &ReadFileArgs {
            start_line: Some(2),
            end_line: Some(99),
            ..args("clamped.txt")
        },
        "one\ntwo\nthree\n",
        10_000,
    )
    .expect("read file output");

    assert!(output.contains("Lines: total=3 requested=2-3 returned=2-3 complete=yes"));
}

#[test]
fn rejects_partial_or_invalid_ranges() {
    let err = read_file_output(
        &ReadFileArgs {
            start_line: Some(1),
            end_line: None,
            ..args("partial.txt")
        },
        "one\n",
        10_000,
    )
    .expect_err("partial range should fail");
    assert_model_error(err, "start_line and end_line must be provided together");

    let err = read_file_output(
        &ReadFileArgs {
            start_line: Some(0),
            end_line: Some(1),
            ..args("zero.txt")
        },
        "one\n",
        10_000,
    )
    .expect_err("zero start line should fail");
    assert_model_error(err, "start_line must be greater than or equal to 1");

    let err = read_file_output(
        &ReadFileArgs {
            start_line: Some(2),
            end_line: Some(1),
            ..args("reversed.txt")
        },
        "one\n",
        10_000,
    )
    .expect_err("reversed range should fail");
    assert_model_error(
        err,
        "end_line (1) must be greater than or equal to start_line (2)",
    );

    let err = read_file_output(
        &ReadFileArgs {
            start_line: Some(2),
            end_line: Some(2),
            ..args("eof.txt")
        },
        "one\n",
        10_000,
    )
    .expect_err("start past eof should fail");
    assert_model_error(err, "start_line (2) exceeds total lines (1)");
}
