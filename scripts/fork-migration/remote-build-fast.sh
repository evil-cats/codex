#!/usr/bin/env bash
set -euo pipefail

# Удаленный запуск быстрой release-сборки для цепочки миграции.
#
# Скрипт запускается с управляющего локального checkout и не выполняет сборку
# сам. Он подключается к `f-ms-dev`, переходит в
# `/home/slader/Projects/codex` и запускает там
# `scripts/fork-migration/local-build-fast.sh` с теми же аргументами. Поэтому
# вся реальная логика сборки живет в одном локальном скрипте, который можно
# запустить на любом подготовленном сервере.
#
# Перед этим remote checkout должен быть подготовлен через
# `remote-prepare-host.sh` и синхронизирован с локальным diff через
# `remote-apply-patch.sh`. Этот wrapper не создает patch, не применяет patch,
# не запускает тесты, генераторы, форматирование, `fix`, commit, push, stash
# или reset. Подробный вывод сборки пишет удаленный `local-build-fast.sh` в
# свой `target/fork-migration/build-logs/`; этот wrapper сохраняет только
# короткий SSH-транскрипт в `target/fork-migration/remote-build-logs/`.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/fork-migration/common.sh
source "${script_dir}/common.sh"
fork_migration_setup_path
repo_root="$(cd "${script_dir}/../.." && pwd)"

# Печатает краткую справку по remote build wrapper.
# Вход: нет.
# Выход: справка в stdout; SSH-соединение не открывает.
usage() {
  printf '%s\n' \
    "Использование:" \
    "  scripts/fork-migration/remote-build-fast.sh <version> [--skip-branch-check]" \
    "" \
    "Пример:" \
    "  scripts/fork-migration/remote-build-fast.sh 0.141.0" \
    "" \
    "Назначение:" \
    "  Подключиться к f-ms-dev и запустить там local-build-fast.sh." \
    "" \
    "Предусловие:" \
    "  Сначала выполни remote-prepare-host.sh и remote-apply-patch.sh." \
    "" \
    "Границы:" \
    "  Скрипт не синхронизирует checkout, не применяет patch, не запускает" \
    "  тесты, генераторы, форматирование, fix, commit, push, stash или reset."
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

remote_host="f-ms-dev"
remote_dir="/home/slader/Projects/codex"
log_dir="${repo_root}/target/fork-migration/remote-build-logs"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
log_file="${log_dir}/${version}-remote-build-fast-${timestamp}.log"
FORK_MIGRATION_MODE="remote-build-fast"

# Запускает уже собранную shell-команду на f-ms-dev через ssh.
# Вход: одна строка remote_command, уже прошедшая shell_quote/shell_join.
# Выход: remote stdout/stderr одновременно виден локально и пишется в локальный
# SSH-лог; при ошибке показывает только последние 10 строк. Код возврата
# берется от ssh, а не от tee.
run_remote() {
  local remote_command="$1"
  shift

  echo
  echo "==> remote build-fast"
  {
    echo
    echo "==> remote build-fast"
    printf '+ ssh %q %q\n' "${remote_host}" "${remote_command}"
  } >> "${log_file}"

  set +e
  # shellcheck disable=SC2029
  ssh "${remote_host}" "${remote_command}" 2>&1 | tee -a "${log_file}"
  local status=${PIPESTATUS[0]}
  set -e

  if [[ "${status}" -eq 0 ]]; then
    return 0
  fi

  echo "FAILED"
  echo
  echo "RESULT: failed"
  echo "MODE: remote-build-fast"
  echo "FAILED_STEP: remote build-fast"
  echo "HOST: ${remote_host}"
  echo "LOCAL_LOG: ${log_file}"
  echo "LAST_LOG_LINES:"
  tail -n 10 "${log_file}" || true
  exit "${status}"
}

# Точка входа remote build wrapper.
# Вход: version и опциональный --skip-branch-check.
# Выход: запускает remote checkout command, который делегирует всю build-логику
# удаленному `local-build-fast.sh`.
main() {
  cd "${repo_root}"
  mkdir -p "${log_dir}"
  require_command ssh
  check_not_remote_host \
    "${remote_host}" \
    "remote-build-fast.sh" \
    "scripts/fork-migration/local-build-fast.sh"

  local remote_argv
  remote_argv=(scripts/fork-migration/local-build-fast.sh "${version}")
  if [[ "${skip_branch_check}" == true ]]; then
    remote_argv+=(--skip-branch-check)
  fi

  local remote_command
  # SSH получает одну строку shell-команды, поэтому remote_dir и argv команды
  # обязательно проходят через printf %q helpers. PATH задается внутри remote
  # shell, потому что неинтерактивный ssh не обязан читать login shell profile.
  remote_command="cd $(shell_quote "${remote_dir}") && PATH=\"\$HOME/.cargo/bin:\$HOME/.local/bin:\$PATH\" $(shell_join "${remote_argv[@]}")"

  run_remote "${remote_command}"

  echo
  echo "RESULT: ok"
  echo "MODE: remote-build-fast"
  echo "HOST: ${remote_host}"
  echo "REMOTE_DIR: ${remote_dir}"
  echo "LOCAL_LOG: ${log_file}"
}

main "$@"
