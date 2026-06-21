#!/usr/bin/env bash
set -euo pipefail

# Локальная обертка генераторов schema-файлов для цепочки миграции.
#
# Скрипт запускается на локальном хосте после базовых локальных проверок, когда
# нужно привести сгенерированные schema-файлы к состоянию текущего кода. Он
# последовательно выполняет `just write-config-schema`, experimental smoke
# генерации app-server schema и финальную stable генерацию
# `just write-app-server-schema`. Команда `just write-app-server-schema
# --experimental` пишет в тот же checked-in каталог fixtures, поэтому stable
# генерация обязательно выполняется последней: именно stable fixture set
# проверяют тесты `codex-app-server-protocol`.
#
# Скрипт не запускает форматирование, тесты, сборку, `fix`, markdownlint или
# git-операции. Если генератор создал новые файлы, их нужно явно добавить в
# индекс или хотя бы отметить через `git add -N`, иначе последующий patch-based
# перенос на `f-ms-dev` не увидит эти файлы. Полный вывод пишется в
# `target/fork-migration/generator-logs/`; при ошибке на экран выводятся только
# последние строки, а причину нужно разбирать по полному логу.
#
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/fork-migration/common.sh
source "${script_dir}/common.sh"
fork_migration_setup_path
repo_root="$(cd "${script_dir}/../.." && pwd)"

# Печатает краткую справку по генераторам.
# Вход: нет.
# Выход: справка в stdout; состояние checkout не меняет.
usage() {
  cat <<'EOF'
Использование:
  scripts/fork-migration/local-generators.sh

Запускает:
  - `just write-config-schema`
  - `just write-app-server-schema --experimental`
  - `just write-app-server-schema`

Назначение:
  Единая локальная точка для обновления сгенерированных schema-файлов после
  миграции fork-доработок. Experimental app-server schema прогоняется как smoke,
  но stable app-server schema пишется последней, чтобы checked-in fixtures
  совпадали с тестовым контрактом.

Границы:
  Скрипт не запускает форматирование, тесты, сборку, fix, markdownlint или
  git-операции.

Важно:
  Новые сгенерированные файлы не попадут в patch сами по себе. Добавь их в
  индекс через `git add` или `git add -N`, иначе remote-apply-patch остановится на
  проверке untracked-файлов.

Логи:
  Полный вывод пишется в target/fork-migration/generator-logs/. При ошибке на
  экран выводятся только последние 10 строк; по ним нужно открыть полный лог,
  исправить причину и повторить этот скрипт.
EOF
}

arg="${1:-}"

if [[ "${arg}" == "-h" || "${arg}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -ne 0 ]]; then
  usage >&2
  exit 2
fi

log_dir="${repo_root}/target/fork-migration/generator-logs"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
log_file="${log_dir}/generators-${timestamp}.log"
FORK_MIGRATION_MODE="generators"

# Обновляет checked-in config schema из текущих Rust config типов.
# Вход: текущий checkout.
# Выход: может изменить `codex-rs/core/config.schema.json`.
run_config_schema() {
  run_step "config schema" just write-config-schema
}

# Обновляет stable app-server schema fixtures.
# Вход: текущий checkout.
# Выход: может изменить файлы в `codex-rs/app-server-protocol/schema/`.
run_app_server_schema() {
  run_step "app-server schema" just write-app-server-schema
}

# Прогоняет experimental app-server schema generation как smoke.
# Вход: текущий checkout.
# Выход: пишет в тот же fixture-каталог, поэтому main запускает stable
# generation после этого шага, чтобы итоговый checked-in набор был stable.
run_app_server_experimental_schema() {
  run_step "app-server experimental schema" just write-app-server-schema --experimental
}

# Точка входа generator wrapper.
# Вход: без CLI-аргументов.
# Выход: последовательно запускает schema generators и печатает ожидаемые пути
# generated artifacts для последующего diff review.
main() {
  cd "${repo_root}"
  mkdir -p "${log_dir}"
  require_command just
  require_command cargo

  run_config_schema
  run_app_server_experimental_schema
  run_app_server_schema

  echo
  echo "RESULT: ok"
  echo "MODE: generators"
  echo "LOG: ${log_file}"
  echo "EXPECTED_GENERATED_PATHS:"
  echo "codex-rs/core/config.schema.json"
  echo "codex-rs/app-server-protocol/schema/"
}

main "$@"
