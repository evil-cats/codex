//! Регрессии начального stdin в общем транспорте elevated runner.
//! Они проверяют IPC-варианты, объединение записей, EOF и возврат ошибки runner.

use super::finish_driver_spawn;
use super::multiplex_driver_stdin;
use super::start_runner_pipe_writer;
use super::start_runner_stdin_writer;
use super::start_runner_stdout_reader;
use crate::ipc_framed::ExitPayload;
use crate::ipc_framed::FramedMessage;
use crate::ipc_framed::IPC_PROTOCOL_VERSION;
use crate::ipc_framed::InitialStdinResultPayload;
use crate::ipc_framed::Message;
use crate::ipc_framed::decode_bytes;
use crate::ipc_framed::read_frame;
use crate::ipc_framed::write_frame;
use codex_utils_pty::ProcessDriver;
use pretty_assertions::assert_eq;
use std::fs::OpenOptions;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::Path;
use std::time::Duration;
use std::time::Instant;
use tempfile::TempDir;
use tokio::runtime::Builder;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

/// Создаёт однопоточный runtime для детерминированного управления обоими stdin-каналами.
fn current_thread_runtime() -> tokio::runtime::Runtime {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime")
}

/// Читает IPC-файл до ожидаемого числа полных кадров; незавершённый последний кадр
/// перечитывается до короткого предельного срока, пока фоновый обработчик завершает запись.
fn wait_for_frame_count(frames_path: &Path, expected_frames: usize) -> Vec<Message> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let mut reader = OpenOptions::new()
            .read(true)
            .open(frames_path)
            .expect("open frame file for read");
        reader
            .seek(SeekFrom::Start(0))
            .expect("seek to start of frame file");

        let mut frames = Vec::new();
        loop {
            match read_frame(&mut reader) {
                Ok(Some(frame)) => frames.push(frame.message),
                Ok(None) => break,
                Err(_) => break,
            }
        }

        if frames.len() >= expected_frames {
            return frames;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {expected_frames} frames, saw {}",
            frames.len()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Собирает сессию с внешним драйвером, файловым IPC-выходом и переданным каналом
/// результата начальной записи; сам тест управляет подтверждением и закрытием stdin.
fn spawn_runner_test_driver(
    frames_path: &Path,
    initial_result_rx: std::sync::mpsc::Receiver<std::io::Result<()>>,
) -> codex_utils_pty::SpawnedProcess {
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(frames_path)
        .expect("create runner frame file");
    let outbound_tx = start_runner_pipe_writer(file);
    let (writer_tx, writer_rx) = mpsc::channel::<Vec<u8>>(/*buffer*/ 1);
    let (initial_stdin_tx, initial_stdin_rx) = mpsc::channel(/*buffer*/ 1);
    let writer_rx = multiplex_driver_stdin(writer_rx, initial_stdin_rx);
    let writer_handle = start_runner_stdin_writer(
        writer_rx,
        outbound_tx,
        initial_result_rx,
        /*normalize_newlines*/ false,
        /*stdin_open*/ true,
    );
    let (_stdout_tx, stdout_rx) = broadcast::channel::<Vec<u8>>(/*capacity*/ 1);
    let (exit_tx, exit_rx) = oneshot::channel::<i32>();
    drop(exit_tx);

    finish_driver_spawn(
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
    )
}

/// Проверяет, что подтверждаемый начальный ввод и последующая потоковая запись
/// проходят через один канал runner в исходном порядке, а закрытие обоих каналов
/// отправки завершается IPC-кадром EOF.
#[test]
fn initial_stdin_runner_merges_streaming_input_and_closes_on_eof() {
    let runtime = current_thread_runtime();
    runtime.block_on(async move {
        let tempdir = TempDir::new().expect("create tempdir");
        let frames_path = tempdir.path().join("runner-initial-stdin-frames.bin");
        let (initial_result_tx, initial_result_rx) = std::sync::mpsc::channel();
        initial_result_tx
            .send(Ok(()))
            .expect("queue successful initial stdin result");
        let spawned = spawn_runner_test_driver(&frames_path, initial_result_rx);

        spawned
            .session
            .write_initial_stdin(b"initial".to_vec())
            .await
            .expect("write acknowledged initial stdin");
        spawned
            .session
            .writer_sender()
            .send(b"stream".to_vec())
            .await
            .expect("write streaming stdin");
        spawned.session.close_stdin();

        let frames_path_for_wait = frames_path.clone();
        let frames = tokio::task::spawn_blocking(move || {
            wait_for_frame_count(&frames_path_for_wait, /*expected_frames*/ 3)
        })
        .await
        .expect("join runner frame wait");
        match &frames[0] {
            Message::InitialStdin { payload } => {
                let bytes = decode_bytes(&payload.data_b64).expect("decode initial stdin");
                assert_eq!(bytes, b"initial".to_vec());
            }
            other => panic!("expected initial-stdin frame, got {other:?}"),
        }
        match &frames[1] {
            Message::Stdin { payload } => {
                let bytes = decode_bytes(&payload.data_b64).expect("decode streaming stdin");
                assert_eq!(bytes, b"stream".to_vec());
            }
            other => panic!("expected stdin frame, got {other:?}"),
        }
        match &frames[2] {
            Message::CloseStdin { .. } => {}
            other => panic!("expected close-stdin frame, got {other:?}"),
        }
    });
}

/// Проверяет полный обратный путь `InitialStdinResult`: сообщение `runner` с текстом
/// ошибки должно завершить исходный `write_initial_stdin` той же ошибкой, а не успехом.
#[test]
fn initial_stdin_runner_returns_reported_error() {
    let runtime = current_thread_runtime();
    runtime.block_on(async move {
        let tempdir = TempDir::new().expect("create tempdir");
        let result_path = tempdir.path().join("runner-initial-stdin-result.bin");
        let mut result_file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .open(&result_path)
            .expect("create runner result file");
        let expected_error = "runner WriteFile failed with os error 109";
        write_frame(
            &mut result_file,
            &FramedMessage {
                version: IPC_PROTOCOL_VERSION,
                message: Message::InitialStdinResult {
                    payload: InitialStdinResultPayload {
                        error: Some(expected_error.to_string()),
                    },
                },
            },
        )
        .expect("write initial stdin result frame");
        write_frame(
            &mut result_file,
            &FramedMessage {
                version: IPC_PROTOCOL_VERSION,
                message: Message::Exit {
                    payload: ExitPayload {
                        exit_code: 0,
                        timed_out: false,
                    },
                },
            },
        )
        .expect("write runner exit frame");
        result_file
            .seek(SeekFrom::Start(0))
            .expect("seek runner result file");

        let (stdout_tx, _stdout_rx) = broadcast::channel::<Vec<u8>>(/*capacity*/ 1);
        let (initial_result_tx, initial_result_rx) = std::sync::mpsc::channel();
        let (exit_tx, exit_rx) = oneshot::channel();
        start_runner_stdout_reader(
            result_file,
            stdout_tx,
            /*stderr_tx*/ None,
            initial_result_tx,
            exit_tx,
        );
        let frames_path = tempdir.path().join("runner-initial-stdin-error-frames.bin");
        let spawned = spawn_runner_test_driver(&frames_path, initial_result_rx);

        let error = spawned
            .session
            .write_initial_stdin(b"payload".to_vec())
            .await
            .expect_err("runner error must reach initial stdin caller");
        assert_eq!(error.kind(), std::io::ErrorKind::Other);
        assert_eq!(error.to_string(), expected_error);
        assert_eq!(exit_rx.await.expect("runner exit result"), 0);

        spawned.session.close_stdin();
        let frames_path_for_wait = frames_path.clone();
        let frames = tokio::task::spawn_blocking(move || {
            wait_for_frame_count(&frames_path_for_wait, /*expected_frames*/ 2)
        })
        .await
        .expect("join runner frame wait");
        match &frames[0] {
            Message::InitialStdin { payload } => {
                let bytes = decode_bytes(&payload.data_b64).expect("decode initial stdin");
                assert_eq!(bytes, b"payload".to_vec());
            }
            other => panic!("expected initial-stdin frame, got {other:?}"),
        }
        match &frames[1] {
            Message::CloseStdin { .. } => {}
            other => panic!("expected close-stdin frame, got {other:?}"),
        }
    });
}
