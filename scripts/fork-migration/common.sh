#!/usr/bin/env bash

# Общие shell-функции для скриптов миграции fork.
#
# Этот файл не запускается напрямую. Его подключают скрипты из
# `scripts/fork-migration/`, чтобы одинаково настраивать PATH, писать команды в
# лог, показывать только последние строки при ошибке и выполнять общие короткие
# проверки checkout.

# Добавляет стандартные пользовательские каталоги инструментов в начало PATH.
# Вход: использует только переменную окружения HOME, если она задана.
# Выход: обновляет PATH текущего shell-процесса; файловую систему не меняет.
fork_migration_setup_path() {
  if [[ -n "${HOME:-}" ]]; then
    export PATH="${HOME}/.cargo/bin:${HOME}/.local/bin:${PATH}"
  fi
}

# Проверяет, что команда доступна в текущем PATH.
# Вход: имя команды без аргументов.
# Выход: при успехе ничего не печатает; при ошибке пишет сообщение в stderr и
# завершает скрипт с кодом 1.
require_command() {
  local command_name="$1"
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "required command not found: ${command_name}" >&2
    exit 1
  fi
}

# Записывает заголовок шага и точный argv вызова в общий лог скрипта.
# Вход: label шага и опциональная команда с аргументами.
# Выход: дописывает в файл из глобальной переменной log_file.
log_command() {
  local label="$1"
  shift

  {
    echo
    echo "==> ${label}"
    if [[ $# -gt 0 ]]; then
      printf '+'
      printf ' %q' "$@"
      echo
    fi
  } >> "${log_file:?log_file must be set}"
}

# Печатает унифицированный короткий отчет об ошибке и завершает скрипт.
# Вход: код возврата, label шага и опциональная команда с аргументами.
# Выход: stdout содержит RESULT/FAILED_STEP/LOG и последние 10 строк лога;
# процесс завершается с переданным кодом.
print_failure() {
  local status="$1"
  local label="$2"
  local mode_name="${FORK_MIGRATION_MODE:-${mode:-unknown}}"
  shift 2

  echo "FAILED"
  echo
  echo "RESULT: failed"
  echo "MODE: ${mode_name}"
  echo "FAILED_STEP: ${label}"
  if [[ $# -gt 0 ]]; then
    printf 'FAILED_COMMAND:'
    printf ' %q' "$@"
    echo
  fi
  echo "LOG: ${log_file:?log_file must be set}"
  echo "LAST_LOG_LINES:"
  tail -n 10 "${log_file}" || true
  exit "${status}"
}

# Выполняет локальную команду как атомарный шаг с логированием.
# Вход: label шага и argv команды.
# Выход: stdout получает короткий OK/FAILED; полный stdout/stderr команды
# уходит в log_file. При ошибке вызывает print_failure.
run_step() {
  local label="$1"
  shift

  echo
  echo "==> ${label}"
  log_command "${label}" "$@"

  if "$@" >> "${log_file:?log_file must be set}" 2>&1; then
    echo "OK"
    return 0
  else
    local status=$?
  fi

  print_failure "${status}" "${label}" "$@"
}

# Проверяет, что rg не нашел совпадений по заданному pattern.
# Вход: label, регулярное выражение rg и список путей/опций rg.
# Выход: OK, если rg вернул 1 (совпадений нет); ошибка, если совпадения есть
# или rg завершился внутренней ошибкой.
run_rg_no_match_step() {
  local label="$1"
  local pattern="$2"
  shift 2
  local paths=("$@")

  echo
  echo "==> ${label}"
  log_command "${label}" rg -n "${pattern}" "${paths[@]}"

  set +e
  rg -n "${pattern}" "${paths[@]}" >> "${log_file:?log_file must be set}" 2>&1
  local status=$?
  set -e

  case "${status}" in
    0)
      print_failure 1 "${label}" rg -n "${pattern}" "${paths[@]}"
      ;;
    1)
      echo "OK"
      ;;
    *)
      print_failure "${status}" "${label}" rg -n "${pattern}" "${paths[@]}"
      ;;
  esac
}

# Проверяет текущую git-ветку checkout, если skip_branch_check=false.
# Вход: repo_root, ожидаемая ветка, флаг пропуска проверки.
# Выход: пишет expected/actual в log_file; при несовпадении завершает скрипт.
check_expected_branch() {
  local repo_root="$1"
  local expected_branch="$2"
  local skip_branch_check="$3"

  if [[ "${skip_branch_check}" == true ]]; then
    return 0
  fi

  local current_branch
  current_branch="$(git -C "${repo_root}" branch --show-current)"
  {
    echo "expected: ${expected_branch}"
    echo "actual: ${current_branch}"
  } >> "${log_file:?log_file must be set}"

  if [[ "${current_branch}" != "${expected_branch}" ]]; then
    print_failure 1 "branch check"
  fi
}

# Проверяет наличие migration-карты.
# Вход: путь к docs/fork/migration-X.Y.Z.md.
# Выход: при отсутствии пишет путь в log_file и завершает скрипт.
check_migration_card_exists() {
  local migration_card="$1"

  if [[ ! -f "${migration_card}" ]]; then
    echo "migration card not found: ${migration_card}" >> "${log_file:?log_file must be set}"
    print_failure 1 "migration card exists"
  fi
}

# Проверяет, что в migration table не осталось строк карточек с финально
# блокирующими статусами.
# Вход: путь к migration-карте.
# Выход: OK, если нет pending/open question/требует исправления; иначе ошибка
# с найденными строками в log_file.
check_no_unfinished_migration_rows() {
  local migration_card="$1"

  run_rg_no_match_step \
    "unfinished migration rows" \
    '\| [^|]*\.md[^|]* \| [^|]*(pending|open question|требует исправления)[^|]* \|' \
    "${migration_card}"
}
