//! Проверяет выбор spill, точность файла и последовательный построчный фрагмент.

use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_output_truncation::TruncationPolicy;
use pretty_assertions::assert_eq;

use super::ExecCommandOutputSpill;
use super::ExecCommandOutputSpillResult;
use super::RenderedOutputSpill;
use super::effective_inline_output_max_tokens;
use super::maybe_spill_exec_command_output;
use super::render_output_spill;

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

/// Spill возвращает максимальный префикс из целых строк без суффикса или маркера.
#[test]
fn output_spill_excerpt_returns_complete_prefix_lines() {
    let path = AbsolutePathBuf::from_absolute_path(std::env::temp_dir().join("output.log"))
        .expect("temp path should be absolute");
    let spill = ExecCommandOutputSpill {
        inline_limit_tokens: 3,
        result: ExecCommandOutputSpillResult::Saved { path: path.clone() },
    };

    assert_eq!(
        render_output_spill(b"alpha\nbeta\ngamma\nomega", &spill),
        RenderedOutputSpill {
            body: format!(
                "Lines: total=4 returned=1-2 remaining=2 complete=no\nFull output: {}\nOutput excerpt:\n\nalpha\nbeta\n",
                path.display()
            ),
            excerpt_token_count: 3,
        }
    );
}

/// Если первая строка не помещается, тело фрагмента отсутствует, а метаданные остаются полными.
#[test]
fn output_spill_excerpt_reports_oversized_first_line() {
    let path = AbsolutePathBuf::from_absolute_path(std::env::temp_dir().join("output.log"))
        .expect("temp path should be absolute");
    let spill = ExecCommandOutputSpill {
        inline_limit_tokens: 2,
        result: ExecCommandOutputSpillResult::Saved { path: path.clone() },
    };

    assert_eq!(
        render_output_spill(b"abcdefghij\nsecond\n", &spill),
        RenderedOutputSpill {
            body: format!(
                "Lines: total=2 returned=none remaining=2 complete=no\nFull output: {}\nError: first line exceeds excerpt token limit",
                path.display()
            ),
            excerpt_token_count: 0,
        }
    );
}

/// Ошибка записи внешнего файла Code Mode сохраняет тот же префикс и видимую причину.
#[test]
fn code_mode_outer_spill_save_failure_returns_line_prefix() {
    let spill = ExecCommandOutputSpill {
        inline_limit_tokens: 3,
        result: ExecCommandOutputSpillResult::SaveFailed {
            error: "permission denied".to_string(),
        },
    };

    assert_eq!(
        render_output_spill(b"alpha\nbeta\ngamma\n", &spill),
        RenderedOutputSpill {
            body: "Lines: total=3 returned=1-2 remaining=1 complete=no\nFailed to save output: permission denied\nOutput excerpt:\n\nalpha\nbeta\n".to_string(),
            excerpt_token_count: 3,
        }
    );
}
