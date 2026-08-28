//! Регрессии подтверждаемой записи начального stdin внутри elevated runner.
//! Они проверяют сохранение фактической ошибки `WriteFile` в ответном IPC-кадре.

use super::*;
use pretty_assertions::assert_eq;
use std::io::Seek;
use std::io::SeekFrom;
use windows_sys::Win32::Foundation::ERROR_INVALID_HANDLE;

/// Проверяет, что ошибка прямого `WriteFile` не заменяется синтетическим успехом
/// или общей ошибкой при формировании `InitialStdinResult` для родительского процесса.
#[test]
fn initial_stdin_result_preserves_runner_write_file_error() {
    let error = write_all_stdin_handle(/*handle*/ INVALID_HANDLE_VALUE, b"payload")
        .expect_err("invalid stdin handle must fail WriteFile");
    assert_eq!(
        error.raw_os_error(),
        Some(ERROR_INVALID_HANDLE as i32),
        "expected the raw WriteFile error, got {error:?}"
    );
    let expected_error = error.to_string();
    let result: std::io::Result<()> = Err(error);

    let pipe_write = Arc::new(StdMutex::new(
        tempfile::tempfile().expect("create runner result file"),
    ));
    send_initial_stdin_result(&pipe_write, &result, /*log_dir*/ None);

    let mut guard = pipe_write.lock().expect("lock runner result file");
    guard
        .seek(SeekFrom::Start(0))
        .expect("seek runner result file");
    let frame = read_frame(&mut *guard)
        .expect("read initial stdin result")
        .expect("initial stdin result frame");
    let Message::InitialStdinResult { payload } = frame.message else {
        panic!("expected initial-stdin-result frame");
    };
    assert_eq!(payload.error.as_deref(), Some(expected_error.as_str()));
}
