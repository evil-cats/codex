#!/usr/bin/env bash
set -euo pipefail

# Локальная обертка тестов для цепочки миграции.
#
# Скрипт запускается в текущем checkout. Для Hermione/Codex migration workflow
# canonical checkout находится на `f-ms-dev` в `/home/slader/Projects/codex`.
# Скрипт не синхронизирует рабочее дерево и не применяет patch; его задача -
# выполнить тестовый gate в уже подготовленном checkout.
#
# Режим `cards` запускает набор тестов, связанный с активными fork-карточками
# миграции. Этот список нужно обновлять, когда карточки добавляют, удаляют или
# меняют обязательные проверки. Режим `full` запускает полный тестовый проход и
# проверку pending snapshots, а режим `list` печатает план без запуска тестов.
#
# Скрипт не принимает snapshots и не делает commit, push, stash или reset.
# Полный вывод пишется в `target/fork-migration/test-logs/`; при ошибке на
# экран выводятся только последние строки лога.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/fork-migration/common.sh
source "${script_dir}/common.sh"
fork_migration_setup_path
repo_root="$(cd "${script_dir}/../.." && pwd)"

# Печатает краткую справку по режимам test gate.
# Вход: нет.
# Выход: справка в stdout; тесты не запускает.
usage() {
  printf '%s\n' \
    "Использование:" \
    "  scripts/fork-migration/local-tests.sh <version> cards [--skip-branch-check]" \
    "  scripts/fork-migration/local-tests.sh <version> full [--skip-branch-check]" \
    "  scripts/fork-migration/local-tests.sh <version> list" \
    "" \
    "Примеры:" \
    "  scripts/fork-migration/local-tests.sh 0.141.0 cards" \
    "  scripts/fork-migration/local-tests.sh 0.141.0 full" \
    "  scripts/fork-migration/local-tests.sh 0.141.0 list" \
    "" \
    "Режимы:" \
    "  cards  Запустить тесты, требуемые активными fork-карточками миграции." \
    "  full   Запустить полный тестовый проход через just test." \
    "  list   Показать план card-тестов без запуска." \
    "" \
    "Границы:" \
    "  Скрипт не синхронизирует checkout, не применяет patch, не принимает" \
    "  snapshots и не делает commit, push, stash или reset."
}

version="${1:-}"
mode="${2:-}"
skip_branch_check=false

if [[ "${version}" == "-h" || "${version}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -lt 2 ]]; then
  usage >&2
  exit 2
fi

shift 2
while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-branch-check)
      skip_branch_check=true
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

expected_branch="hermione-${version}"
migration_card="${repo_root}/docs/fork/migration-${version}.md"
log_dir="${repo_root}/target/fork-migration/test-logs"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
log_file="${log_dir}/${version}-${mode}-${timestamp}.log"

# Проверяет, что текущий checkout готов к запуску test gate.
# Вход: глобальные version/mode/expected_branch/migration_card.
# Выход: при успехе ничего не меняет; при ошибке завершает скрипт. Проверка
# незавершенной migration table остается здесь намеренно, чтобы test gate не
# подтверждал неполный перенос карточек.
check_preconditions() {
  require_command git
  require_command rg
  require_command just
  require_command cargo

  check_migration_card_exists "${migration_card}"
  check_expected_branch "${repo_root}" "${expected_branch}" "${skip_branch_check}"
  check_no_unfinished_migration_rows "${migration_card}"
}

# Регистрирует или запускает одну card-проверку.
# Вход: label fork-карточки и argv команды проверки.
# Выход: в режиме list печатает план без запуска; в режиме cards выполняет
# команду через run_step и пишет полный вывод в test log.
card_test() {
  local label="$1"
  shift

  if [[ "${mode}" == "list" ]]; then
    printf '%-34s' "${label}"
    printf ' %q' "$@"
    echo
  else
    run_step "${label}" "$@"
  fi
}

# Исполняемая карта проверок активных fork-карточек.
# Вход: текущий mode через card_test.
# Выход: в режиме list печатает команды; в режиме cards запускает их по
# порядку. Новые/измененные карточки должны обновлять именно этот список.
run_card_tests() {
  # Держи этот список явным. Это исполняемая карта покрытия для активных
  # fork-карточек; ее нужно обновлять, когда карточки добавляют обязательные
  # тесты или меняют контракт проверки.
  card_test "codex-agent-env-var" just test -p codex-core exec_env
  card_test "codex-agent-env-var" just test -p codex-core agent_name
  card_test "codex-agent-env-var" just test -p codex-core thread_info
  card_test "codex-agent-env-var" just test -p codex-core maybe_wrap_shell_lc_with_snapshot_restores_codex_identity_from_env
  card_test "codex-agent-env-var" just test -p codex-core env_overlay_for_exec_server_keeps_runtime_changes_only
  card_test "codex-agent-env-var" just test -p codex-core shell_command_handler_to_exec_params_uses_session_shell_and_turn_context
  card_test "codex-agent-env-var" just test -p codex-protocol shell_environment
  card_test "core-system-time-tool" just test -p codex-core system_time
  card_test "core-system-time-tool" just test -p codex-core prompt_tools_are_consistent_across_requests
  card_test "core-thread-info-tool" just test -p codex-core agent_name
  card_test "core-thread-info-tool" just test -p codex-core thread_info
  card_test "core-thread-info-tool" just test -p codex-core prompt_tools_are_consistent_across_requests
  card_test "developer-instructions-files" just test -p codex-core developer_instructions
  card_test "environment-context-project-name" just test -p codex-core environment_context
  card_test "exec-command-output-spill-files" just test -p codex-core inline_output_max_tokens
  card_test "exec-command-output-spill-files" just test -p codex-core output_spill
  card_test "exec-command-output-spill-files" just test -p codex-core exec_command_tool_output_formats_spill
  card_test "exec-command-output-spill-files" just test -p codex-core exec_command_spills_large_completed_output_to_file
  card_test "exec-command-output-spill-files" just test -p codex-core unified_exec_enforces_glob_deny_read_policy
  card_test "exec-command-output-spill-files" just test -p codex-core unified_exec_timeout_and_followup_poll
  card_test "hermione-version-metadata" just test -p codex-cli
  # Этот IDE IPC test проверяет отдельный security contract и на `f-ms-dev`
  # зависит от permissions временной директории. Он не относится к контракту
  # `hermione-version-metadata`, поэтому не должен блокировать card gate.
  card_test "hermione-version-metadata" just test -p codex-tui -- --skip ide_context::ipc::tests::fetch_ide_context_uses_unregistered_request_route
  # Часть filtered `codex-core config` тестов запускает MCP stdio fixture из
  # `codex-rmcp-client`. После очистки target этот helper binary нужно собрать
  # явно, иначе `cargo_bin("test_stdio_server")` не находит файл.
  card_test "memory-read-template-path" cargo build --manifest-path codex-rs/Cargo.toml -p codex-rmcp-client --bin test_stdio_server
  card_test "memory-read-template-path" just test -p codex-core config
  card_test "memory-read-template-path" just test -p codex-memories-extension
  card_test "terminal-title-session-label" just test -p codex-tui terminal_title
  card_test "tui-history-image-previews" just test -p codex-core view_image
  card_test "tui-history-image-previews" just test -p codex-tui -- --skip ide_context::ipc::tests::fetch_ide_context_uses_unregistered_request_route
  card_test "tui-history-image-previews" just test -p codex-app-server-protocol
  card_test "tui-history-image-previews" just test -p codex-protocol
  card_test "tui-snapshots" cargo insta pending-snapshots --manifest-path codex-rs/tui/Cargo.toml
}

# Точка входа test wrapper.
# Вход: version, mode и опциональный --skip-branch-check.
# Выход: list печатает план; cards/full выполняют проверки и печатают RESULT.
main() {
  cd "${repo_root}"
  mkdir -p "${log_dir}"

  case "${mode}" in
    list)
      run_card_tests
      ;;
    cards)
      check_preconditions
      run_card_tests
      echo
      echo "RESULT: ok"
      echo "MODE: ${mode}"
      echo "LOG: ${log_file}"
      ;;
    full)
      check_preconditions
      run_step "full test suite" just test
      run_step "tui pending snapshots" cargo insta pending-snapshots -p codex-tui
      echo
      echo "RESULT: ok"
      echo "MODE: ${mode}"
      echo "LOG: ${log_file}"
      ;;
    -h|--help)
      usage
      ;;
    *)
      echo "unknown mode: ${mode}" >&2
      usage >&2
      exit 2
      ;;
  esac
}

main "$@"
