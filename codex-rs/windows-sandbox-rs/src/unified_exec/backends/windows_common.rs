use crate::ipc_framed::EmptyPayload;
use crate::ipc_framed::FramedMessage;
use crate::ipc_framed::IPC_PROTOCOL_VERSION;
use crate::ipc_framed::InitialStdinResultPayload;
use crate::ipc_framed::Message;
use crate::ipc_framed::OutputStream;
use crate::ipc_framed::ResizePayload;
use crate::ipc_framed::StdinPayload;
use crate::ipc_framed::decode_bytes;
use crate::ipc_framed::encode_bytes;
use anyhow::Result;
use codex_utils_pty::InitialStdinWrite;
use codex_utils_pty::ProcessDriver;
use codex_utils_pty::SpawnedProcess;
use codex_utils_pty::TerminalSize;
use codex_utils_pty::WindowsTtyInputNormalizer;
use codex_utils_pty::spawn_from_driver;
use std::fs::File;
use std::io;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

pub(crate) fn finish_driver_spawn(driver: ProcessDriver, stdin_open: bool) -> SpawnedProcess {
    let spawned = spawn_from_driver(driver);
    if !stdin_open {
        spawned.session.close_stdin();
    }
    spawned
}

/// Запись stdin, которую драйвер Windows должен либо передать потоком, либо подтвердить.
pub(crate) enum DriverStdinWrite {
    Stream(Vec<u8>),
    Initial(InitialStdinWrite),
}

/// Объединяет обычный поток stdin и подтверждаемый начальный stdin в очередь драйвера.
pub(crate) fn multiplex_driver_stdin(
    mut writer_rx: mpsc::Receiver<Vec<u8>>,
    mut initial_stdin_rx: mpsc::Receiver<InitialStdinWrite>,
) -> mpsc::Receiver<DriverStdinWrite> {
    let (driver_tx, driver_rx) = mpsc::channel(128);
    tokio::spawn(async move {
        loop {
            let write = tokio::select! {
                Some(initial) = initial_stdin_rx.recv() => DriverStdinWrite::Initial(initial),
                Some(bytes) = writer_rx.recv() => DriverStdinWrite::Stream(bytes),
                else => break,
            };
            if driver_tx.send(write).await.is_err() {
                break;
            }
        }
    });
    driver_rx
}

struct RunnerOutboundRequest {
    message: FramedMessage,
    result_tx: Option<std::sync::mpsc::Sender<io::Result<()>>>,
}

#[derive(Clone)]
/// Канал сообщений для `runner` с подтверждением записи IPC-кадра.
pub(crate) struct RunnerPipeWriter {
    tx: std::sync::mpsc::Sender<RunnerOutboundRequest>,
}

impl RunnerPipeWriter {
    /// Ставит обычное сообщение для `runner` в очередь записи без ожидания результата.
    pub(crate) fn send(&self, message: FramedMessage) -> Result<()> {
        self.tx
            .send(RunnerOutboundRequest {
                message,
                result_tx: None,
            })
            .map_err(|_| anyhow::anyhow!("runner pipe writer closed"))
    }

    /// Ждёт фактической записи IPC-кадра в канал `runner`.
    fn send_acknowledged(&self, message: FramedMessage) -> io::Result<()> {
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        self.tx
            .send(RunnerOutboundRequest {
                message,
                result_tx: Some(result_tx),
            })
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "runner pipe writer closed"))?;
        result_rx.recv().map_err(|_| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "runner pipe writer stopped before acknowledging the frame",
            )
        })?
    }
}

/// Запускает блокирующий обработчик исходящего IPC-канала `runner`.
pub(crate) fn start_runner_pipe_writer(mut pipe_write: File) -> RunnerPipeWriter {
    let (outbound_tx, outbound_rx) = std::sync::mpsc::channel::<RunnerOutboundRequest>();
    tokio::task::spawn_blocking(move || {
        while let Ok(request) = outbound_rx.recv() {
            let result = crate::ipc_framed::write_frame(&mut pipe_write, &request.message)
                .map_err(|err| io::Error::other(err.to_string()));
            let failed = result.is_err();
            if let Some(result_tx) = request.result_tx {
                let _ = result_tx.send(result);
            }
            if failed {
                break;
            }
        }
    });
    RunnerPipeWriter { tx: outbound_tx }
}

/// Пересылает stdin в `runner` и для начального stdin ждёт результат записи в дочерний процесс.
pub(crate) fn start_runner_stdin_writer(
    mut writer_rx: mpsc::Receiver<DriverStdinWrite>,
    outbound_tx: RunnerPipeWriter,
    initial_result_rx: std::sync::mpsc::Receiver<io::Result<()>>,
    normalize_newlines: bool,
    stdin_open: bool,
) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        let mut windows_input = WindowsTtyInputNormalizer::default();
        while let Some(write) = writer_rx.blocking_recv() {
            let (bytes, result_tx) = match write {
                DriverStdinWrite::Stream(bytes) => (bytes, None),
                DriverStdinWrite::Initial(write) => {
                    let (bytes, result_tx) = write.into_parts();
                    (bytes, Some(result_tx))
                }
            };
            let bytes = if normalize_newlines {
                windows_input.normalize(&bytes)
            } else {
                bytes
            };
            let msg = FramedMessage {
                version: IPC_PROTOCOL_VERSION,
                message: if result_tx.is_some() {
                    Message::InitialStdin {
                        payload: StdinPayload {
                            data_b64: encode_bytes(&bytes),
                        },
                    }
                } else {
                    Message::Stdin {
                        payload: StdinPayload {
                            data_b64: encode_bytes(&bytes),
                        },
                    }
                },
            };
            let result = if result_tx.is_some() {
                outbound_tx.send_acknowledged(msg).and_then(|()| {
                    initial_result_rx.recv().map_err(|_| {
                        io::Error::new(
                            io::ErrorKind::BrokenPipe,
                            "runner stopped before reporting the initial stdin result",
                        )
                    })?
                })
            } else {
                outbound_tx
                    .send(msg)
                    .map_err(|err| io::Error::other(err.to_string()))
            };
            let failed = result.is_err();
            if let Some(result_tx) = result_tx {
                let _ = result_tx.send(result);
            }
            if failed {
                break;
            }
        }
        if stdin_open {
            let _ = outbound_tx.send(FramedMessage {
                version: IPC_PROTOCOL_VERSION,
                message: Message::CloseStdin {
                    payload: EmptyPayload::default(),
                },
            });
        }
    })
}

/// Пересылает вывод и завершение `runner`, а результат начального stdin направляет
/// ожидающему обработчику.
pub(crate) fn start_runner_stdout_reader(
    mut pipe_read: File,
    stdout_tx: broadcast::Sender<Vec<u8>>,
    stderr_tx: Option<broadcast::Sender<Vec<u8>>>,
    initial_result_tx: std::sync::mpsc::Sender<io::Result<()>>,
    exit_tx: oneshot::Sender<i32>,
) {
    std::thread::spawn(move || {
        loop {
            let msg = match crate::ipc_framed::read_frame(&mut pipe_read) {
                Ok(Some(v)) => v,
                Ok(None) => {
                    send_runner_error(
                        "runner pipe closed before exit",
                        &stdout_tx,
                        stderr_tx.as_ref(),
                    );
                    let _ = exit_tx.send(-1);
                    break;
                }
                Err(err) => {
                    send_runner_error(
                        &format!("runner read failed: {err}"),
                        &stdout_tx,
                        stderr_tx.as_ref(),
                    );
                    let _ = exit_tx.send(-1);
                    break;
                }
            };

            match msg.message {
                Message::Output { payload } => {
                    if let Ok(data) = decode_bytes(&payload.data_b64) {
                        match payload.stream {
                            OutputStream::Stdout => {
                                let _ = stdout_tx.send(data);
                            }
                            OutputStream::Stderr => {
                                if let Some(stderr_tx) = stderr_tx.as_ref() {
                                    let _ = stderr_tx.send(data);
                                } else {
                                    let _ = stdout_tx.send(data);
                                }
                            }
                        }
                    }
                }
                Message::Exit { payload } => {
                    let _ = exit_tx.send(payload.exit_code);
                    break;
                }
                Message::Error { payload } => {
                    send_runner_error(&payload.message, &stdout_tx, stderr_tx.as_ref());
                    let _ = exit_tx.send(-1);
                    break;
                }
                Message::InitialStdinResult {
                    payload: InitialStdinResultPayload { error },
                } => {
                    let result = error.map_or(Ok(()), |message| Err(io::Error::other(message)));
                    let _ = initial_result_tx.send(result);
                }
                Message::SpawnReady { .. }
                | Message::Stdin { .. }
                | Message::InitialStdin { .. }
                | Message::CloseStdin { .. }
                | Message::Resize { .. }
                | Message::SpawnRequest { .. }
                | Message::Terminate { .. } => {}
            }
        }
    });
}

pub(crate) fn make_runner_resizer(
    outbound_tx: RunnerPipeWriter,
) -> Box<dyn FnMut(TerminalSize) -> Result<()> + Send> {
    Box::new(move |size: TerminalSize| {
        outbound_tx.send(FramedMessage {
            version: IPC_PROTOCOL_VERSION,
            message: Message::Resize {
                payload: ResizePayload {
                    rows: size.rows,
                    cols: size.cols,
                },
            },
        })
    })
}

fn send_runner_error(
    message: &str,
    stdout_tx: &broadcast::Sender<Vec<u8>>,
    stderr_tx: Option<&broadcast::Sender<Vec<u8>>>,
) {
    let formatted = format!("runner error: {message}\n").into_bytes();
    if let Some(stderr_tx) = stderr_tx {
        let _ = stderr_tx.send(formatted);
    } else {
        let _ = stdout_tx.send(formatted);
    }
}
