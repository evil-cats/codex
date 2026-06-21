#!/usr/bin/env bash
set -euo pipefail

# Подготовка `f-ms-dev` как чистого удаленного зеркала для цепочки миграции.
#
# Скрипт запускается на локальном хосте перед применением patch, удаленными
# тестами и сборкой. Он приводит `f-ms-dev:/home/slader/Projects/codex` к той же
# ветке `hermione-<version>` и тому же `HEAD`, что локальный checkout, затем
# очищает удаленную рабочую копию. Это destructive операция для удаленного
# зеркала: внутри mirror checkout выполняются `git reset --hard` и `git clean
# -fd`. Такой reset допустим только потому, что `FORK.md` определяет этот
# каталог как одноразовое сборочное и тестовое зеркало, а не место разработки.
#
# Скрипт не создает, не копирует и не применяет patch. Он также не запускает
# форматирование, генераторы, тесты, сборку, `fix`, commit, push или stash.
# После успешного запуска remote checkout должен быть чистой базой; следующий
# шаг - `remote-apply-patch.sh`. Полный вывод пишется в
# `target/fork-migration/remote-prepare-logs/`; при ошибке на экран выводятся
# только последние строки, а причину нужно разбирать по полному логу.
#
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/fork-migration/common.sh
source "${script_dir}/common.sh"
fork_migration_setup_path
repo_root="$(cd "${script_dir}/../.." && pwd)"

# Печатает краткую справку по подготовке удаленного зеркала.
# Вход: нет.
# Выход: справка в stdout; удаленный host не трогает.
usage() {
  cat <<'EOF'
Использование:
  scripts/fork-migration/remote-prepare-host.sh <version>

Пример:
  scripts/fork-migration/remote-prepare-host.sh 0.141.0

Назначение:
  Подготовить `f-ms-dev:/home/slader/Projects/codex` как чистое удаленное
  зеркало локальной ветки `hermione-<version>` и локального `HEAD`.

Что делает:
  На удаленном зеркале выполняет fetch, switch, `git reset --hard <local-head>`
  и `git clean -fd`, затем проверяет ветку, HEAD и чистое состояние.

Границы:
  Скрипт не создает, не копирует и не применяет patch. Он не запускает
  форматирование, генераторы, тесты, сборку, fix, commit, push или stash.

Логи:
  Полный вывод пишется в target/fork-migration/remote-prepare-logs/. При ошибке
  на экран выводятся только последние 10 строк; по ним нужно открыть полный лог,
  исправить причину и повторить этот скрипт.
EOF
}

version="${1:-}"

if [[ "${version}" == "-h" || "${version}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -ne 1 ]]; then
  usage >&2
  exit 2
fi

expected_branch="hermione-${version}"
remote_host="f-ms-dev"
remote_dir="/home/slader/Projects/codex"
log_dir="${repo_root}/target/fork-migration/remote-prepare-logs"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
log_file="${log_dir}/${version}-remote-prepare-${timestamp}.log"
FORK_MIGRATION_MODE="remote-prepare-host"

# Проверяет, что локальный checkout находится в ожидаемой Hermione-ветке.
# Вход: имя текущей локальной ветки.
# Выход: при несовпадении пишет ошибку в stderr и завершает скрипт до любых
# remote/destructive действий.
check_local_state() {
  local local_branch="$1"

  if [[ "${local_branch}" != "${expected_branch}" ]]; then
    echo "expected local branch ${expected_branch}, got ${local_branch}" >&2
    exit 1
  fi
}

# Печатает shell-программу, которая выполняется на f-ms-dev через `bash -s`.
# Вход remote-программы: remote_dir, expected_branch, expected_head,
# expected_host как позиционные аргументы.
# Выход remote-программы: remote checkout приведен к expected_branch и
# expected_head, рабочее дерево очищено; stdout содержит before/after metadata.
#
# Важно: это destructive remote-блок. Он делает `git reset --hard` и
# `git clean -fd`, поэтому до запуска проверяет hostname и origin удаленного
# checkout. Логика оставлена здесь, потому что prepare выполняется до
# применения patch и не может полагаться на новые файлы из локального diff.
remote_prepare_script() {
  cat <<'EOF'
set -euo pipefail

remote_dir="$1"
expected_branch="$2"
expected_head="$3"
expected_host="$4"

export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"

cd "${remote_dir}"

remote_host_short="$(hostname -s)"

if [[ "${remote_host_short}" != "${expected_host}" ]]; then
  echo "unexpected remote host: ${remote_host_short}" >&2
  exit 1
fi

echo "REMOTE_HOST=${remote_host_short}"
echo "REMOTE_DIR=${remote_dir}"
echo "REMOTE_BRANCH_BEFORE=$(git branch --show-current)"
echo "REMOTE_HEAD_BEFORE=$(git rev-parse HEAD)"
echo "REMOTE_STATUS_BEFORE:"
git status --short
echo "REMOTE_REMOTES:"
git remote -v

remote_origin="$(git remote get-url origin)"
case "${remote_origin}" in
  *evil-cats/codex*|*evilcats/codex*|*openai/codex*)
    ;;
  *)
    echo "unexpected remote origin: ${remote_origin}" >&2
    exit 1
    ;;
esac

git fetch origin
if git show-ref --verify --quiet "refs/heads/${expected_branch}"; then
  git switch --force "${expected_branch}"
else
  git switch --force --create "${expected_branch}" "origin/${expected_branch}"
fi

if ! git cat-file -e "${expected_head}^{commit}"; then
  echo "expected local HEAD is not available on remote after fetch: ${expected_head}" >&2
  exit 1
fi

git reset --hard "${expected_head}"
git clean -fd

remote_branch_after="$(git branch --show-current)"
remote_head_after="$(git rev-parse HEAD)"
remote_status_after="$(git status --short)"

if [[ "${remote_branch_after}" != "${expected_branch}" ]]; then
  echo "remote branch mismatch after prepare: ${remote_branch_after}" >&2
  exit 1
fi

if [[ "${remote_head_after}" != "${expected_head}" ]]; then
  echo "remote HEAD mismatch after prepare: ${remote_head_after}" >&2
  exit 1
fi

if [[ -n "${remote_status_after}" ]]; then
  echo "remote checkout is dirty after prepare:" >&2
  printf '%s\n' "${remote_status_after}" >&2
  exit 1
fi

echo "REMOTE_BRANCH_AFTER=${remote_branch_after}"
echo "REMOTE_HEAD_AFTER=${remote_head_after}"
echo "REMOTE_STATUS_AFTER=clean"
EOF
}

# Точка входа remote prepare.
# Вход: version и локальный checkout.
# Выход: f-ms-dev mirror reset/clean к локальному HEAD; локальный checkout не
# меняется. При ошибке подробности остаются в remote-prepare log.
main() {
  cd "${repo_root}"
  mkdir -p "${log_dir}"

  require_command git
  require_command ssh

  local local_branch
  local local_head
  local_branch="$(git branch --show-current)"
  local_head="$(git rev-parse HEAD)"

  check_local_state "${local_branch}"

  echo "fork migration remote prepare"
  echo "version: ${version}"
  echo "host: ${remote_host}"
  echo "remote_dir: ${remote_dir}"
  echo "branch: ${local_branch}"
  echo "head: ${local_head}"
  echo "log: ${log_file}"

  {
    echo "fork migration remote prepare"
    echo "version: ${version}"
    echo "host: ${remote_host}"
    echo "remote_dir: ${remote_dir}"
    echo "branch: ${local_branch}"
    echo "head: ${local_head}"
  } >> "${log_file}"

  run_step "local branch check" test "${local_branch}" = "${expected_branch}"
  run_step "local HEAD exists" git cat-file -e "${local_head}^{commit}"
  run_remote_step "remote reset to local HEAD" bash -s -- "${remote_dir}" "${expected_branch}" "${local_head}" "${remote_host}" \
    <<<"$(remote_prepare_script)"

  echo
  echo "RESULT: ok"
  echo "MODE: remote-prepare-host"
  echo "HOST: ${remote_host}"
  echo "REMOTE_DIR: ${remote_dir}"
  echo "BRANCH: ${local_branch}"
  echo "HEAD: ${local_head}"
  echo "LOG: ${log_file}"
}

main "$@"
