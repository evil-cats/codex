#!/usr/bin/env bash
set -euo pipefail

# Локальная быстрая release-сборка для цепочки миграции.
#
# Скрипт запускается в текущем checkout. Для Hermione/Codex migration workflow
# canonical checkout находится на `f-ms-dev` в `/home/slader/Projects/codex`.
# Скрипт не синхронизирует рабочее дерево и не применяет patch; его задача -
# проверить предусловия текущего checkout, выполнить `just build-fast-release`,
# проверить бинарник `codex-rs/target/release-fast/codex`, вывести metadata файла
# и версию собранного бинарника.
# Полный вывод команд пишется в `target/fork-migration/build-logs/`; при
# ошибке на экран выводятся только последние строки лога.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/fork-migration/common.sh
source "${script_dir}/common.sh"
fork_migration_setup_path
repo_root="$(cd "${script_dir}/../.." && pwd)"

# Печатает краткую справку по CLI.
# Вход: нет.
# Выход: справка в stdout; состояние checkout не меняет.
usage() {
  printf '%s\n' \
    "Использование:" \
    "  scripts/fork-migration/local-build-fast.sh <version> [--skip-branch-check]" \
    "" \
    "Пример:" \
    "  scripts/fork-migration/local-build-fast.sh 0.141.0" \
    "" \
    "Назначение:" \
    "  Запустить быструю release-сборку в текущем checkout." \
    "" \
    "Границы:" \
    "  Скрипт не синхронизирует checkout, не применяет patch, не запускает" \
    "  тесты, генераторы, форматирование, fix, commit, push, stash или reset." \
    "" \
    "Логи:" \
    "  Полный вывод пишется в target/fork-migration/build-logs/."
}

version="${1:-}"
skip_branch_check=false

if [[ "${version}" == "-h" || "${version}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -lt 1 ]]; then
  usage >&2
  exit 2
fi

shift
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
binary_path="${repo_root}/codex-rs/target/release-fast/codex"
log_dir="${repo_root}/target/fork-migration/build-logs"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
log_file="${log_dir}/${version}-build-fast-${timestamp}.log"
FORK_MIGRATION_MODE="build-fast"

# Проверяет, что текущий checkout готов к fast build gate.
# Вход: глобальные version/expected_branch/migration_card/skip_branch_check.
# Выход: при успехе ничего не меняет; при ошибке завершает скрипт через
# common.sh helper. Проверка незавершенных строк остается здесь намеренно:
# сборочный gate не должен проходить при незавершенной migration table.
check_preconditions() {
  require_command git
  require_command rg
  require_command just
  require_command cargo
  require_command file

  check_migration_card_exists "${migration_card}"
  check_expected_branch "${repo_root}" "${expected_branch}" "${skip_branch_check}"
  check_no_unfinished_migration_rows "${migration_card}"
}

# Точка входа build gate.
# Вход: распарсенные CLI-параметры и текущий checkout.
# Выход: выполняет `just build-fast-release`, проверяет бинарник и печатает
# RESULT. При ошибке подробности остаются в build log.
main() {
  cd "${repo_root}"
  mkdir -p "${log_dir}"

  check_preconditions

  run_step "release-fast build" just build-fast-release

  # `just build-fast-release` может завершиться успешно только формально, если
  # ожидаемый artifact переехал или не был создан. Отдельно проверяем именно
  # исполняемый файл, который потом устанавливается как fork binary.
  if [[ ! -x "${binary_path}" ]]; then
    echo
    echo "RESULT: failed"
    echo "MODE: build-fast"
    echo "FAILED_STEP: binary existence"
    echo "LOG: ${log_file}"
    echo "expected executable not found: ${binary_path}" >&2
    exit 1
  fi

  run_step "binary file metadata" file "${binary_path}"
  run_step "binary version" "${binary_path}" --version

  echo
  echo "RESULT: ok"
  echo "MODE: build-fast"
  echo "BINARY: ${binary_path}"
  echo "LOG: ${log_file}"
}

main "$@"
