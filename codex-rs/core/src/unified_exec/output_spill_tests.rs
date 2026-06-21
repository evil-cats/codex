use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_output_truncation::TruncationPolicy;
use pretty_assertions::assert_eq;

use super::effective_inline_output_max_tokens;
use super::maybe_spill_exec_command_output;
use crate::tools::context::ExecCommandOutputSpillResult;

#[test]
fn effective_limit_uses_config_request_and_turn_policy() {
    assert_eq!(
        effective_inline_output_max_tokens(1000, Some(250), TruncationPolicy::Tokens(500)),
        250
    );
    assert_eq!(
        effective_inline_output_max_tokens(1000, None, TruncationPolicy::Tokens(500)),
        500
    );
    assert_eq!(
        effective_inline_output_max_tokens(1000, Some(2000), TruncationPolicy::Tokens(5000)),
        1000
    );
}

#[tokio::test]
async fn below_limit_does_not_create_spill_file() {
    let codex_home = tempfile::tempdir().expect("create temp dir");
    let codex_home = AbsolutePathBuf::from_absolute_path(codex_home.path())
        .expect("temp dir should be absolute");

    let spill = maybe_spill_exec_command_output(
        &codex_home,
        "thread-1",
        "call-1",
        "chunk-1",
        b"short output",
        /*original_token_count*/ 2,
        /*inline_limit_tokens*/ 2,
    )
    .await;

    assert_eq!(spill, None);
    assert!(!codex_home.join("exec_outputs").as_path().exists());
}

#[tokio::test]
async fn writes_exact_bytes_to_sanitized_path() {
    let codex_home = tempfile::tempdir().expect("create temp dir");
    let codex_home = AbsolutePathBuf::from_absolute_path(codex_home.path())
        .expect("temp dir should be absolute");
    let raw_output = b"\xffbinary\ntext\n";

    let spill = maybe_spill_exec_command_output(
        &codex_home,
        "thread/one",
        "call:two",
        "chunk three",
        raw_output,
        /*original_token_count*/ 20,
        /*inline_limit_tokens*/ 5,
    )
    .await
    .expect("output should spill");

    assert_eq!(spill.inline_limit_tokens, 5);
    let ExecCommandOutputSpillResult::Saved { path } = spill.result else {
        panic!("expected saved spill result");
    };
    assert_eq!(
        path,
        codex_home
            .join("exec_outputs")
            .join("thread_one")
            .join("call_two-chunk_three.log")
    );
    assert_eq!(
        std::fs::read(path.as_path()).expect("read spill file"),
        raw_output
    );
}

#[tokio::test]
async fn save_failure_returns_bounded_error_metadata() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let codex_home_file = temp.path().join("codex-home-file");
    std::fs::write(&codex_home_file, b"not a directory").expect("write codex home file");
    let codex_home = AbsolutePathBuf::from_absolute_path(&codex_home_file)
        .expect("file path should be absolute");

    let spill = maybe_spill_exec_command_output(
        &codex_home,
        "thread-1",
        "call-1",
        "chunk-1",
        b"long output",
        /*original_token_count*/ 20,
        /*inline_limit_tokens*/ 5,
    )
    .await
    .expect("output should still return spill metadata");

    let ExecCommandOutputSpillResult::SaveFailed { error } = spill.result else {
        panic!("expected save failure result");
    };
    assert!(!error.is_empty());
    assert!(
        error.chars().count() <= 243,
        "error should be bounded: {error}"
    );
}
