//! Компактный PID-1 reaper для Bubblewrap namespace.
//!
//! Модуль запускает пользовательскую команду дочерним процессом, заменяет
//! долгоживущий `argv` helper через self-`exec`, пересылает сигналы и собирает
//! завершившихся потомков без изменения status основной команды.

use super::FORWARDED_SIGNALS;
use super::ForwardedSignalHandlers;
use super::ForwardedSignalMask;
use super::exec_or_panic;
use super::exit_with_wait_status;
use super::install_bwrap_signal_forwarders;
use super::reset_forwarded_signal_handlers_to_default;
use crate::exec_util::argv_to_cstrings;
use codex_sandboxing::landlock::CODEX_LINUX_SANDBOX_ARG0;
use std::ffi::CString;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::path::PathBuf;

const INTERNAL_REAPER_ARG: &str = "--codex-internal-namespace-reaper";

/// Сохраняет путь executable до применения ограничений inner stage.
///
/// Ошибка подготовки не отменяет команду: `run_command` оставляет действующим
/// прежний in-process reaper, если self-`exec` недоступен.
pub(super) struct ReaperExecutable {
    path: Option<PathBuf>,
}

impl ReaperExecutable {
    pub(super) fn capture() -> Self {
        Self {
            path: std::env::current_exe().ok(),
        }
    }
}

/// Возобновляет компактный reaper до обычного разбора CLI helper.
///
/// Внутренняя форма принимается только с одним PID основной команды и только
/// процессом PID 1 внутри namespace. Обычный запуск возвращается вызывающему
/// коду для разбора `LandlockCommand`.
pub(super) fn resume_if_requested() {
    let mut args = std::env::args_os();
    let _argv0 = args.next();
    if args.next().as_deref() != Some(OsStr::new(INTERNAL_REAPER_ARG)) {
        return;
    }

    let command_pid = args
        .next()
        .and_then(|pid| pid.into_string().ok())
        .and_then(|pid| pid.parse::<libc::pid_t>().ok())
        .filter(|pid| *pid > 1)
        .unwrap_or_else(|| panic!("namespace reaper requires a valid command pid"));
    if args.next().is_some() {
        panic!("namespace reaper received unexpected trailing arguments");
    }
    if unsafe { libc::getpid() } != 1 {
        panic!("namespace reaper must run as pid 1");
    }

    let signal_forwarders = install_bwrap_signal_forwarders(command_pid);
    unblock_forwarded_signals();
    wait_for_descendants(command_pid, signal_forwarders);
}

/// Запускает основную команду и превращает родительский PID 1 в компактный
/// reaper.
///
/// Сигналы блокируются до `fork`, чтобы ни один из них не потерялся между
/// созданием команды и установкой обработчиков после self-`exec`. Если замена
/// процесса не удалась, функция продолжает прежний цикл ожидания в исходном
/// helper.
pub(super) fn run_command(command: Vec<String>, reaper_executable: ReaperExecutable) -> ! {
    let signal_mask = ForwardedSignalMask::block();
    // SAFETY: после `fork` обе ветки используют только локальное состояние до
    // немедленного `exec` либо установки обработчиков сигналов.
    let command_pid = unsafe { libc::fork() };
    if command_pid < 0 {
        let err = std::io::Error::last_os_error();
        panic!("failed to fork sandboxed command: {err}");
    }

    if command_pid == 0 {
        reset_forwarded_signal_handlers_to_default();
        signal_mask.restore();
        exec_or_panic(command);
    }

    if let Some(path) = reaper_executable.path {
        replace_with_compact_reaper(&path, command_pid);
    }

    let signal_forwarders = install_bwrap_signal_forwarders(command_pid);
    signal_mask.restore();
    wait_for_descendants(command_pid, signal_forwarders);
}

/// Заменяет текущий PID 1 тем же executable с минимальным внутренним `argv`.
///
/// Успешный `execv` не возвращается. Любая ошибка оставляет вызывающему коду
/// возможность продолжить in-process reaper без изменения команды.
fn replace_with_compact_reaper(executable: &Path, command_pid: libc::pid_t) {
    let Ok(program) = CString::new(executable.as_os_str().as_bytes()) else {
        return;
    };
    let argv = vec![
        CODEX_LINUX_SANDBOX_ARG0.to_string(),
        INTERNAL_REAPER_ARG.to_string(),
        command_pid.to_string(),
    ];
    let cstrings = argv_to_cstrings(&argv);
    let mut argv_ptrs = cstrings
        .iter()
        .map(CString::as_c_str)
        .map(std::ffi::CStr::as_ptr)
        .collect::<Vec<_>>();
    argv_ptrs.push(std::ptr::null());

    // SAFETY: `program` и все элементы `argv_ptrs` являются корректными
    // C-строками и остаются живы на время вызова.
    unsafe {
        libc::execv(program.as_ptr(), argv_ptrs.as_ptr());
    }
}

/// Разблокирует сигналы, унаследованные компактным reaper через `exec`.
///
/// Остальная маска процесса сохраняется; это соответствует прежнему
/// `ForwardedSignalMask::restore`, который также всегда разблокировал набор
/// пересылаемых сигналов.
fn unblock_forwarded_signals() {
    let mut forwarded: libc::sigset_t = unsafe { std::mem::zeroed() };
    // SAFETY: `forwarded` инициализирован, а каждый номер сигнала является
    // допустимой константой libc.
    unsafe {
        libc::sigemptyset(&mut forwarded);
        for signal in FORWARDED_SIGNALS {
            libc::sigaddset(&mut forwarded, *signal);
        }
        if libc::sigprocmask(libc::SIG_UNBLOCK, &forwarded, std::ptr::null_mut()) < 0 {
            let err = std::io::Error::last_os_error();
            panic!("failed to unblock namespace reaper signals: {err}");
        }
    }
}

/// Собирает всех завершившихся потомков и завершает PID 1 со status основной
/// команды.
fn wait_for_descendants(command_pid: libc::pid_t, signal_forwarders: ForwardedSignalHandlers) -> ! {
    loop {
        let mut status = 0;
        let reaped_pid = unsafe { libc::waitpid(-1, &mut status, 0) };
        if reaped_pid == command_pid {
            let exit_signal_mask = ForwardedSignalMask::block();
            signal_forwarders.restore();
            exit_signal_mask.restore();
            exit_with_wait_status(status);
        }
        if reaped_pid >= 0 {
            continue;
        }

        let err = std::io::Error::last_os_error();
        if err.raw_os_error() != Some(libc::EINTR) {
            panic!("failed to reap sandboxed child: {err}");
        }
    }
}

#[cfg(test)]
#[path = "namespace_reaper_tests.rs"]
mod tests;
