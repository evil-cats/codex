#!/usr/bin/env bash
set -euo pipefail

# Применение локального patch к уже подготовленному удаленному зеркалу.
#
# Скрипт запускается на локальном хосте после `remote-prepare-host.sh`. Он
# предполагает, что `f-ms-dev:/home/slader/Projects/codex` уже находится на той
# же ветке и том же `HEAD`, что локальный checkout, и что удаленная рабочая
# копия чистая. Скрипт создает локальный patch из `git diff --binary HEAD`,
# копирует его во временный каталог на `f-ms-dev`, применяет через
# `git apply --index --binary` и сравнивает удаленный diff с локальным по
# контрольным суммам списка файлов и полного binary diff.
#
# Скрипт не выполняет `reset` или `clean` на удаленной стороне: если удаленная
# база не подготовлена, он останавливается до применения patch. Локальные
# untracked-файлы тоже блокируют запуск, потому что они не попадут в
# `git diff HEAD`; новые файлы нужно предварительно добавить в индекс через
# `git add` или отметить через `git add -N`. Полный вывод пишется в
# `target/fork-migration/remote-patch-logs/`; при ошибке на экран выводятся
# только последние строки, а причину нужно разбирать по полному логу.
#
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/fork-migration/common.sh
source "${script_dir}/common.sh"
fork_migration_setup_path
repo_root="$(cd "${script_dir}/../.." && pwd)"

# Печатает краткую справку по применению patch на remote mirror.
# Вход: нет.
# Выход: справка в stdout; локальные/удаленные файлы не меняет.
usage() {
  cat <<'EOF'
Использование:
  scripts/fork-migration/remote-apply-patch.sh <version>

Пример:
  scripts/fork-migration/remote-apply-patch.sh 0.141.0

Назначение:
  Применить локальный намеренный diff к уже подготовленному зеркалу
  `f-ms-dev:/home/slader/Projects/codex` и доказать, что удаленный diff
  совпадает с локальным diff.

Предусловие:
  Сначала запусти `scripts/fork-migration/remote-prepare-host.sh <version>`.
  Удаленный checkout должен совпадать с локальной веткой и локальным `HEAD` и
  быть чистым.

Что делает:
  Создает patch через `git diff --binary HEAD`, копирует его во временный
  каталог `mktemp` на `f-ms-dev`, применяет через `git apply --index --binary`
  и сверяет список измененных файлов и полный binary diff по `sha256sum`.

Границы:
  Скрипт не делает reset/clean удаленного checkout, не запускает форматирование,
  генераторы, тесты, сборку, fix, commit, push или stash.

Важно:
  Обычные untracked-файлы не входят в `git diff HEAD`. Для новых файлов сначала
  используй `git add` или `git add -N`, иначе скрипт остановится до remote
  действий.

Логи:
  Полный вывод пишется в target/fork-migration/remote-patch-logs/. При ошибке
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
log_dir="${repo_root}/target/fork-migration/remote-patch-logs"
patch_dir="${repo_root}/target/fork-migration/patches"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
patch_file="${patch_dir}/${version}-${timestamp}.patch"
local_files_file="${patch_dir}/${version}-${timestamp}.local-files"
log_file="${log_dir}/${version}-remote-patch-${timestamp}.log"
FORK_MIGRATION_MODE="remote-apply-patch"

# Выполняет локальную команду и сохраняет ее stdout в отдельный файл.
# Вход: label шага, output_file и argv команды.
# Выход: stdout команды записан в output_file, stderr уходит в log_file; при
# ошибке вызывает print_failure. Используется для patch и списка файлов, чтобы
# эти артефакты можно было сверить и при необходимости изучить отдельно.
capture_stdout() {
  local label="$1"
  local output_file="$2"
  shift 2

  echo
  echo "==> ${label}"
  log_command "${label}" "$@"

  if "$@" > "${output_file}" 2>> "${log_file}"; then
    echo "OK"
    return 0
  else
    local status=$?
  fi

  print_failure "${status}" "${label}" "$@"
}

# Проверяет, что локальный checkout находится в ожидаемой Hermione-ветке.
# Вход: имя текущей локальной ветки.
# Выход: при несовпадении завершает скрипт до создания patch и remote-действий.
check_local_state() {
  local local_branch="$1"

  if [[ "${local_branch}" != "${expected_branch}" ]]; then
    echo "expected local branch ${expected_branch}, got ${local_branch}" >&2
    exit 1
  fi
}

# Блокирует запуск при обычных untracked-файлах вне target/fork-migration.
# Вход: текущий checkout.
# Выход: OK, если таких файлов нет; иначе первые найденные пути пишутся в лог,
# а скрипт завершается. Это нужно потому, что `git diff HEAD` не включает
# untracked-файлы, и remote mirror получил бы неполный patch.
check_no_untracked_files() {
  local untracked_file="${patch_dir}/${version}-${timestamp}.untracked"

  echo
  echo "==> local untracked files"
  log_command "local untracked files" git ls-files --others --exclude-standard -- ':!target/fork-migration'

  git ls-files --others --exclude-standard -- ':!target/fork-migration' > "${untracked_file}"
  if [[ ! -s "${untracked_file}" ]]; then
    echo "OK"
    return 0
  fi

  {
    echo "local untracked files are not included in git diff HEAD:"
    sed -n '1,50p' "${untracked_file}"
  } >> "${log_file}"
  print_failure 1 "local untracked files" git ls-files --others --exclude-standard -- ':!target/fork-migration'
}

# Считает SHA256 файла и печатает только hex digest.
# Вход: путь к файлу.
# Выход: digest в stdout.
sha256_file() {
  local path="$1"
  sha256sum "${path}" | awk '{ print $1 }'
}

# Считает SHA256 потока stdin и печатает только hex digest.
# Вход: stdin.
# Выход: digest в stdout.
sha256_stdin() {
  sha256sum | awk '{ print $1 }'
}

# Печатает shell-программу precheck, выполняемую на f-ms-dev через `bash -s`.
# Вход remote-программы: remote_dir, expected_branch, expected_head,
# expected_host как позиционные аргументы.
# Выход remote-программы: только проверка; checkout не меняется. Она
# подтверждает host/origin/branch/HEAD и чистое состояние до применения patch.
remote_precheck_script() {
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

remote_origin="$(git remote get-url origin)"
case "${remote_origin}" in
  *evil-cats/codex*|*evilcats/codex*|*openai/codex*)
    ;;
  *)
    echo "unexpected remote origin: ${remote_origin}" >&2
    exit 1
    ;;
esac

remote_branch="$(git branch --show-current)"
remote_head="$(git rev-parse HEAD)"
remote_status="$(git status --short)"

echo "REMOTE_HOST=${remote_host_short}"
echo "REMOTE_DIR=${remote_dir}"
echo "REMOTE_BRANCH=${remote_branch}"
echo "REMOTE_HEAD=${remote_head}"
echo "REMOTE_STATUS:"
git status --short

if [[ "${remote_branch}" != "${expected_branch}" ]]; then
  echo "remote branch mismatch: ${remote_branch}" >&2
  exit 1
fi

if [[ "${remote_head}" != "${expected_head}" ]]; then
  echo "remote HEAD mismatch: ${remote_head}" >&2
  exit 1
fi

if [[ -n "${remote_status}" ]]; then
  echo "remote checkout must be clean before applying patch" >&2
  exit 1
fi
EOF
}

# Печатает shell-программу применения patch, выполняемую на f-ms-dev.
# Вход remote-программы: remote_dir, patch_path, expected_branch,
# expected_head, expected_host, expected_diff_sha, expected_files_sha,
# remote_tmp_dir.
# Выход remote-программы: patch применен в index/worktree remote checkout, а
# remote diff и список файлов совпадают с локальными SHA256. Временный каталог
# удаляется через trap.
remote_apply_script() {
  cat <<'EOF'
set -euo pipefail

remote_dir="$1"
patch_path="$2"
expected_branch="$3"
expected_head="$4"
expected_host="$5"
expected_diff_sha="$6"
expected_files_sha="$7"
remote_tmp_dir="$8"

# Временный каталог создан локальным wrapper-ом через `ssh mktemp -d`; удаляем
# его на remote side независимо от результата применения patch.
trap 'rm -rf "${remote_tmp_dir}"' EXIT

export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"

cd "${remote_dir}"

remote_host_short="$(hostname -s)"
if [[ "${remote_host_short}" != "${expected_host}" ]]; then
  echo "unexpected remote host: ${remote_host_short}" >&2
  exit 1
fi

remote_branch="$(git branch --show-current)"
remote_head="$(git rev-parse HEAD)"
remote_status="$(git status --short)"

if [[ "${remote_branch}" != "${expected_branch}" ]]; then
  echo "remote branch mismatch before apply: ${remote_branch}" >&2
  exit 1
fi

if [[ "${remote_head}" != "${expected_head}" ]]; then
  echo "remote HEAD mismatch before apply: ${remote_head}" >&2
  exit 1
fi

if [[ -n "${remote_status}" ]]; then
  echo "remote checkout must be clean before applying patch" >&2
  exit 1
fi

# Сначала валидируем patch вместе с index, чтобы не получить частично
# примененное состояние при несовместимом diff.
git apply --check --index --binary "${patch_path}"
git apply --index --binary "${patch_path}"

# Сверяем не только полный binary diff, но и список файлов. Отдельная сверка
# списка дает более понятную диагностику, если потерялся новый файл или путь.
remote_files_sha="$(git diff --name-only HEAD | sha256sum | awk '{ print $1 }')"
remote_diff_sha="$(git diff --binary HEAD | sha256sum | awk '{ print $1 }')"

echo "REMOTE_FILES_SHA256=${remote_files_sha}"
echo "REMOTE_DIFF_SHA256=${remote_diff_sha}"

if [[ "${remote_files_sha}" != "${expected_files_sha}" ]]; then
  echo "удаленный список измененных файлов не совпадает с локальным diff" >&2
  git diff --name-only HEAD >&2
  exit 1
fi

if [[ "${remote_diff_sha}" != "${expected_diff_sha}" ]]; then
  echo "удаленный diff не совпадает с локальным diff" >&2
  exit 1
fi
EOF
}

# Точка входа patch transfer.
# Вход: version и локальный checkout с намеренным diff.
# Выход: remote mirror получает тот же diff; локально создаются patch/log
# artifacts в target/fork-migration. При ошибке remote diff не считается
# синхронизированным.
main() {
  cd "${repo_root}"
  mkdir -p "${log_dir}" "${patch_dir}"

  require_command git
  require_command scp
  require_command sha256sum
  require_command ssh

  local local_branch
  local local_head
  local patch_sha
  local diff_sha
  local files_sha
  local remote_tmp_dir
  local remote_patch_path
  local_branch="$(git branch --show-current)"
  local_head="$(git rev-parse HEAD)"

  check_local_state "${local_branch}"

  echo "fork migration remote apply patch"
  echo "version: ${version}"
  echo "host: ${remote_host}"
  echo "remote_dir: ${remote_dir}"
  echo "branch: ${local_branch}"
  echo "head: ${local_head}"
  echo "log: ${log_file}"

  {
    echo "fork migration remote apply patch"
    echo "version: ${version}"
    echo "host: ${remote_host}"
    echo "remote_dir: ${remote_dir}"
    echo "branch: ${local_branch}"
    echo "head: ${local_head}"
  } >> "${log_file}"

  run_step "local branch check" test "${local_branch}" = "${expected_branch}"
  run_step "local HEAD exists" git cat-file -e "${local_head}^{commit}"
  check_no_untracked_files

  # Patch строится из `git diff HEAD`, поэтому перед этим check_no_untracked_files
  # гарантирует, что новые нужные файлы уже staged или хотя бы `git add -N`.
  capture_stdout "local patch create" "${patch_file}" git diff --binary HEAD
  capture_stdout "local changed files" "${local_files_file}" git diff --name-only HEAD --

  if [[ ! -s "${patch_file}" ]]; then
    echo "локальный diff пуст" >> "${log_file}"
    print_failure 1 "local patch create" git diff --binary HEAD
  fi

  patch_sha="$(sha256_file "${patch_file}")"
  # Эти контрольные суммы являются контрактом переноса: remote apply считается
  # успешным только если после применения patch они совпали на f-ms-dev.
  diff_sha="$(git diff --binary HEAD | sha256_stdin)"
  files_sha="$(git diff --name-only HEAD -- | sha256_stdin)"
  echo
  echo "==> remote temp dir"
  log_command "remote temp dir" ssh "${remote_host}" mktemp -d /tmp/fork-migration-patch.XXXXXX
  if remote_tmp_dir="$(ssh "${remote_host}" mktemp -d /tmp/fork-migration-patch.XXXXXX 2>> "${log_file}")"; then
    echo "OK"
  else
    print_failure "$?" "remote temp dir" ssh "${remote_host}" mktemp -d /tmp/fork-migration-patch.XXXXXX
  fi

  remote_patch_path="${remote_tmp_dir}/$(basename "${patch_file}")"

  {
    echo "PATCH_FILE=${patch_file}"
    echo "PATCH_SHA256=${patch_sha}"
    echo "LOCAL_DIFF_SHA256=${diff_sha}"
    echo "LOCAL_FILES_SHA256=${files_sha}"
    echo "REMOTE_PATCH_PATH=${remote_patch_path}"
  } >> "${log_file}"

  run_remote_step "remote precheck" bash -s -- "${remote_dir}" "${expected_branch}" "${local_head}" "${remote_host}" \
    <<<"$(remote_precheck_script)"
  run_step "copy patch" scp "${patch_file}" "${remote_host}:${remote_patch_path}"
  run_remote_step "remote apply patch" bash -s -- "${remote_dir}" "${remote_patch_path}" "${expected_branch}" \
    "${local_head}" "${remote_host}" "${diff_sha}" "${files_sha}" "${remote_tmp_dir}" <<<"$(remote_apply_script)"

  echo
  echo "RESULT: ok"
  echo "MODE: remote-apply-patch"
  echo "HOST: ${remote_host}"
  echo "REMOTE_DIR: ${remote_dir}"
  echo "BRANCH: ${local_branch}"
  echo "HEAD: ${local_head}"
  echo "PATCH_SHA256: ${patch_sha}"
  echo "DIFF_SHA256: ${diff_sha}"
  echo "LOG: ${log_file}"
}

main "$@"
