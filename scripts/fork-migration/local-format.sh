#!/usr/bin/env bash
set -euo pipefail

# Локальная обертка форматирования для цепочки миграции.
#
# Скрипт запускается на локальном хосте после предварительной проверки и до
# удаленных тестов/сборки. В режиме `check` он проверяет форматирование через
# `just fmt-check` без изменения файлов. В режиме `apply` он запускает
# `just fmt`, поэтому может изменить файлы в рабочем дереве. Используй `apply`
# только когда принято решение привести diff к форматированному виду, а затем
# повтори `check`, если нужна отдельная фиксация результата.
#
# Скрипт намеренно не запускает тесты, сборку, генераторы, `fix`, markdownlint
# или git-операции. Полный вывод команд всегда пишется в
# `target/fork-migration/format-logs/`; на экран при ошибке выводятся только
# последние строки лога. Если скрипт уперся в sandbox, сеть или cache, повторяй
# сам скрипт с нужными правами, а не запускай его внутренние команды вручную.
#
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/fork-migration/common.sh
source "${script_dir}/common.sh"
fork_migration_setup_path
repo_root="$(cd "${script_dir}/../.." && pwd)"

# Печатает краткую справку по режимам форматирования.
# Вход: нет.
# Выход: справка в stdout; рабочее дерево не меняет.
usage() {
  cat <<'EOF'
Использование:
  scripts/fork-migration/local-format.sh check
  scripts/fork-migration/local-format.sh apply

Режимы:
  check  Проверить форматирование через `just fmt-check` без изменения файлов.
  apply  Применить форматирование через `just fmt`; этот режим может изменить
         рабочее дерево.

Назначение:
  Единая локальная точка для шагов форматирования в цепочке миграции. Обертка
  нужна, чтобы не путать форматирование с тестами, генераторами и удаленной
  сборкой, а также чтобы весь подробный вывод всегда уходил в лог.

Границы:
  Скрипт не запускает тесты, сборку, генераторы, fix, markdownlint или
  git-операции.

Логи:
  Полный вывод пишется в target/fork-migration/format-logs/. При ошибке на
  экран выводятся только последние 10 строк; по ним нужно открыть полный лог,
  исправить причину и повторить этот скрипт.
EOF
}

mode="${1:-}"

if [[ "${mode}" == "-h" || "${mode}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -ne 1 ]]; then
  usage >&2
  exit 2
fi

log_dir="${repo_root}/target/fork-migration/format-logs"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
log_file="${log_dir}/${mode}-${timestamp}.log"

# Точка входа format wrapper.
# Вход: mode=`check` или `apply`.
# Выход: запускает ровно одну форматирующую команду; `apply` может изменить
# рабочее дерево, `check` только проверяет.
main() {
  cd "${repo_root}"
  mkdir -p "${log_dir}"
  require_command just

  case "${mode}" in
    check)
      run_step "format check" just fmt-check
      ;;
    apply)
      run_step "format apply" just fmt
      ;;
    *)
      echo "unknown mode: ${mode}" >&2
      usage >&2
      exit 2
      ;;
  esac

  echo
  echo "RESULT: ok"
  echo "MODE: ${mode}"
  echo "LOG: ${log_file}"
}

main "$@"
