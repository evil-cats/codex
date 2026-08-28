//! Регрессии подтверждаемой начальной записи stdin в прямом backend legacy.
//! Они проверяют ошибку настоящего Windows `WriteFile`, а не тестовой подмены.

use super::super::windows_common::finish_driver_spawn;
use super::super::windows_common::multiplex_driver_stdin;
use super::spawn_input_writer;
use codex_utils_pty::ProcessDriver;
use pretty_assertions::assert_eq;
use std::ptr;
use tokio::runtime::Builder;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::Foundation::ERROR_BROKEN_PIPE;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::System::Pipes::CreatePipe;

/// Проверяет, что прямой backend legacy возвращает вызывающему коду исходную ошибку
/// Windows `WriteFile`, если у канала stdin уже не осталось читающей стороны.
#[test]
fn initial_stdin_legacy_write_file_error_reaches_caller() {
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    runtime.block_on(async move {
        let mut read_handle: HANDLE = 0;
        let mut write_handle: HANDLE = 0;
        let created = unsafe {
            CreatePipe(
                &mut read_handle,
                &mut write_handle,
                ptr::null_mut(),
                /*nsize*/ 0,
            )
        };
        assert_ne!(created, 0, "create stdin pipe");
        unsafe {
            CloseHandle(read_handle);
        }

        let (writer_tx, writer_rx) = mpsc::channel::<Vec<u8>>(/*buffer*/ 1);
        let (initial_stdin_tx, initial_stdin_rx) = mpsc::channel(/*buffer*/ 1);
        let writer_rx = multiplex_driver_stdin(writer_rx, initial_stdin_rx);
        let writer_handle = spawn_input_writer(
            /*input_write*/ Some(write_handle),
            writer_rx,
            /*normalize_newlines*/ false,
        );
        let (_stdout_tx, stdout_rx) = broadcast::channel::<Vec<u8>>(/*capacity*/ 1);
        let (exit_tx, exit_rx) = oneshot::channel::<i32>();
        drop(exit_tx);
        let spawned = finish_driver_spawn(
            ProcessDriver {
                writer_tx,
                initial_stdin_tx: Some(initial_stdin_tx),
                stdout_rx,
                stderr_rx: None,
                exit_rx,
                terminator: None,
                writer_handle: Some(writer_handle),
                resizer: None,
                tty: false,
            },
            /*stdin_open*/ true,
        );

        let error = spawned
            .session
            .write_initial_stdin(b"payload".to_vec())
            .await
            .expect_err("closed pipe reader must reject initial stdin");
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
        assert_eq!(
            error.raw_os_error(),
            Some(ERROR_BROKEN_PIPE as i32),
            "expected the raw WriteFile error, got {error:?}"
        );
    });
}
