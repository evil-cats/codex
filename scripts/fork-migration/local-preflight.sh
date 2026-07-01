#!/usr/bin/env bash
set -euo pipefail

# Локальная предварительная проверка миграции Hermione fork.
#
# Скрипт запускается в текущем checkout перед форматированием, генераторами,
# тестами и сборкой. Его задача - быстро подтвердить, что рабочая копия
# находится в ожидаемой ветке миграции, migration-карта существует и не содержит
# незавершенных строк, а в текущем diff нет очевидных механических проблем:
# whitespace-ошибок, конфликтных маркеров и markdownlint-падений в fork/migration
# документации.
#
# Скрипт не изменяет рабочее дерево и не запускает форматирование, генераторы,
# тесты, сборку, `fix` или git-операции. Полный вывод команд всегда пишется в
# `target/fork-migration/preflight-logs/`; на экран при ошибке выводятся только
# последние строки лога. Короткий хвост нужен для диагностики, но не заменяет
# разбор причины: если скрипт печатает `RESULT: failed`, открой указанный лог,
# исправь ошибку и повтори этот скрипт.
#
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/fork-migration/common.sh
source "${script_dir}/common.sh"
fork_migration_setup_path
repo_root="$(cd "${script_dir}/../.." && pwd)"

# Печатает краткую справку по preflight.
# Вход: нет.
# Выход: справка в stdout; рабочее дерево не меняет.
usage() {
  cat <<'EOF'
Использование:
  scripts/fork-migration/local-preflight.sh <version> [--skip-branch-check]

Пример:
  scripts/fork-migration/local-preflight.sh 0.141.0

Назначение:
  Локальная предварительная проверка миграции перед остальными шагами pipeline.
  Скрипт проверяет ожидаемую Hermione-ветку, наличие migration-карты,
  отсутствие незавершенных строк карточек, сохранение reverted-строки, whitespace
  в diff, конфликтные маркеры и markdownlint для fork/migration документов.

Границы:
  Скрипт ничего не меняет в рабочем дереве и не запускает форматирование,
  генераторы, тесты, сборку, fix или git-операции.

Логи:
  Полный вывод пишется в target/fork-migration/preflight-logs/. При ошибке на
  экран выводятся только последние 10 строк; по ним нужно открыть полный лог,
  исправить причину и повторить этот скрипт.
EOF
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
markdownlint_config="${repo_root}/.markdownlint-cli2.yaml"
log_dir="${repo_root}/target/fork-migration/preflight-logs"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
log_file="${log_dir}/${version}-preflight-${timestamp}.log"
FORK_MIGRATION_MODE="preflight"

# Проверяет специальный invariant migration table для reverted-карточки.
# Вход: глобальная migration_card.
# Выход: OK, если строка отсутствует или имеет статус reverted; иначе ошибка.
# Это отдельная проверка, потому что карточка была сознательно исключена из
# переноса и не должна случайно вернуться в активный gate.
check_reverted_card_row() {
  local row

  echo
  echo "==> reverted card row"
  log_command \
    "reverted card row" \
    rg -n '^\| [^|]*multi-agent-v2-task-depth\.md[^|]* \|' "${migration_card}"

  row="$(rg -n '^\| [^|]*multi-agent-v2-task-depth\.md[^|]* \|' "${migration_card}" || true)"
  printf '%s\n' "${row}" >> "${log_file}"

  if [[ -z "${row}" || "${row}" == *reverted* ]]; then
    echo "OK"
    return 0
  fi

  print_failure 1 "reverted card row"
}

# Проверяет наличие markdownlint config перед запуском markdownlint-cli2.
# Вход: глобальный markdownlint_config.
# Выход: при отсутствии config завершает preflight до запуска markdownlint.
check_markdownlint_config() {
  if [[ ! -f "${markdownlint_config}" ]]; then
    echo "markdownlint config not found: ${markdownlint_config}" >> "${log_file}"
    print_failure 1 "markdownlint config exists"
  fi
}

# Точка входа preflight.
# Вход: version и опциональный --skip-branch-check.
# Выход: выполняет быстрые локальные проверки без изменения рабочего дерева.
main() {
  cd "${repo_root}"
  mkdir -p "${log_dir}"

  require_command git
  require_command rg
  require_command markdownlint-cli2

  echo "fork migration local preflight"
  echo "version: ${version}"
  echo "log: ${log_file}"

  echo
  echo "==> branch check"
  log_command "branch check"
  check_expected_branch "${repo_root}" "${expected_branch}" "${skip_branch_check}"
  echo "OK"

  echo
  echo "==> migration card exists"
  log_command "migration card exists"
  check_migration_card_exists "${migration_card}"
  echo "OK"

  check_no_unfinished_migration_rows "${migration_card}"

  check_reverted_card_row

  run_step "git diff whitespace check" git diff --check

  run_rg_no_match_step \
    "conflict marker scan" \
    '^(<<<<<<<|>>>>>>>|=======$)' \
    --glob '!target/**' \
    --glob '!codex-rs/target/**' \
    --glob '!.git/**' \
    --glob '!node_modules/**' \
    "${repo_root}"

  check_markdownlint_config
  # nullglob нужен, чтобы пустой `docs/fork/*.md` не превратился в буквальный
  # аргумент markdownlint-cli2. В нормальной миграции каталог не пустой, но
  # wrapper должен вести себя предсказуемо и в минимальном checkout.
  shopt -s nullglob
  local markdown_docs=(
    "FORK.md"
    "docs/migration-one-card-for-agent.md"
    "docs/fork/"*.md
  )
  shopt -u nullglob

  run_step \
    "markdownlint fork docs" \
    markdownlint-cli2 --config "${markdownlint_config}" "${markdown_docs[@]}"

  echo
  echo "RESULT: ok"
  echo "MODE: preflight"
  echo "LOG: ${log_file}"
}

main "$@"
