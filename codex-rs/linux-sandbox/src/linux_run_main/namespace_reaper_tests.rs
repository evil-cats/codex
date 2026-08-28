//! Регрессионные проверки резервного пути компактного namespace reaper.
//!
//! Тест запускает отдельную копию тестового бинарника, потому что проверяемый
//! жизненный цикл завершает процесс со статусом основной команды и не может
//! выполняться внутри процесса родительской тестовой обвязки.

use std::path::PathBuf;
use std::process::Command;

use pretty_assertions::assert_eq;

use super::*;

const CHILD_MODE_ENV_VAR: &str = "CODEX_NAMESPACE_REAPER_FALLBACK_TEST_CHILD";
const COMMAND_MARKER_ENV_VAR: &str = "CODEX_NAMESPACE_REAPER_FALLBACK_TEST_MARKER";
const REAPER_EXECUTABLE_ENV_VAR: &str = "CODEX_NAMESPACE_REAPER_FALLBACK_TEST_EXECUTABLE";
const COMMAND_EXIT_CODE: i32 = 37;
const TEST_NAME: &str = "linux_run_main::namespace_reaper::tests::compact_self_exec_failure_falls_back_without_cancelling_command";

/// Принудительно ломает self-`exec` после `fork` основной команды и проверяет,
/// что in-process reaper дожидается её завершения и возвращает исходный код.
#[test]
fn compact_self_exec_failure_falls_back_without_cancelling_command() {
    if std::env::var_os(CHILD_MODE_ENV_VAR).is_none() {
        let temp_dir = tempfile::tempdir()
            .expect("не удалось создать временный каталог проверки namespace reaper");
        let command_marker = temp_dir.path().join("command-completed");
        let missing_reaper_executable = temp_dir.path().join("missing-reaper-executable");
        let output = Command::new(
            std::env::current_exe().expect("не удалось найти текущий тестовый бинарник"),
        )
        .args([TEST_NAME, "--exact", "--nocapture"])
        .env(CHILD_MODE_ENV_VAR, "1")
        .env(COMMAND_MARKER_ENV_VAR, &command_marker)
        .env(REAPER_EXECUTABLE_ENV_VAR, &missing_reaper_executable)
        .output()
        .expect("не удалось запустить дочернюю проверку namespace reaper");

        let observed = (
            output.status.code(),
            std::fs::read_to_string(command_marker).ok(),
        );
        assert_eq!(
            observed,
            (Some(COMMAND_EXIT_CODE), Some("completed".to_string())),
            "резервный путь namespace reaper вернул неожиданный результат\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        return;
    }

    let reaper_executable = std::env::var_os(REAPER_EXECUTABLE_ENV_VAR)
        .map(PathBuf::from)
        .expect("дочернему процессу не передан путь исполняемого файла reaper");
    let command = vec![
        "/bin/sh".to_string(),
        "-c".to_string(),
        format!("printf completed > \"${COMMAND_MARKER_ENV_VAR}\"; exit {COMMAND_EXIT_CODE}"),
    ];

    run_command(
        command,
        ReaperExecutable {
            path: Some(reaper_executable),
        },
    );
}
