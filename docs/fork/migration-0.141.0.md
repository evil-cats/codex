---
id: fork-migration-0.141.0
status: active
created: 2026-06-19
updated: 2026-06-21
source_scope: rust-v0.141.0..working-tree
---

# Migration check: `0.141.0`

## Обзор

Эта карточка фиксирует перенос Hermione fork с ветки `hermione-0.140.0` на
upstream tag `rust-v0.141.0`.

Цель миграции: не восстановить весь старый diff механически, а проверить и при
необходимости доработать только fork-доработки, у которых есть владеющая
карточка в `docs/fork/`.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Новая ветка | `hermione-0.141.0` |
| Upstream tag | `rust-v0.141.0` |
| Предыдущая fork-ветка | `hermione-0.140.0` |
| Код правится | локально, `/mnt/ml/Projects/evilcats/codex` |
| Сборка и Rust/`just` проверки | после прохода карточек, через родительского агента |
| Текущий проверочный статус | upstream merge выполнен, механические conflict markers убраны, активные fork-карточки из таблицы покрытия проверены или добавлены текущей реализацией; проверка карточек и release-fast build на `f-ms-dev` прошли |

## Правило переноса

Старая Hermione-ветка содержит одновременно fork-доработки и код предыдущей
версии upstream. Поэтому признак "есть в `hermione-0.140.0`, но нет в
`rust-v0.141.0`" сам по себе не доказывает, что код нужно восстанавливать.

Для восстановления нужен один из признаков:

- существующая owner-карточка в `docs/fork/`;
- явное решение пользователя для текущей миграции;
- связь с уже описанным fork-контрактом, подтвержденная поиском по owner-файлам.

## Карта покрытия

| Карточка | Статус переноса на `0.141.0` | Живые признаки |
| --- | --- | --- |
| `codex-agent-env-var.md` | `перенесено` | Подагент подтвердил наличие `CODEX_AGENT`, `CODEX_CALL_ID`, `CODEX_ROLLOUT`, `CODEX_THREAD_ID` в текущем коде; кодовых изменений не потребовалось; проверки должен запустить родительский агент |
| `core-system-time-tool.md` | `перенесено` | Подагент подтвердил `SystemTimeHandler`, `get_system_time`, tool spec, регистрацию и prompt tool list; изменений не потребовалось; проверки должен запустить родительский агент |
| `core-thread-info-tool.md` | `перенесено` | Подагент подтвердил `get_thread_info`, persisted thread metadata, thread/session ids; добавлены unit tests для fallback-контракта `agent_name`; проверки должен запустить родительский агент |
| `developer-instructions-files.md` | `перенесено` | Подагент подтвердил `developer_instructions_files`, config loader и schema entry; добавлен тест `developer_instructions_override_skips_files`; проверки должен запустить родительский агент |
| `environment-context-project-name.md` | `перенесено` | Подагент подтвердил `project_name`, `effective_workspace_roots`, environment context rendering и diff path; изменений не потребовалось; проверки должен запустить родительский агент |
| `exec-command-output-spill-files.md` | `перенесено` | Текущая реализация добавила spill-файлы для immediate-finished `exec_command` output выше inline-лимита, config key `[tools.exec].inline_output_max_tokens`, schema, формат `Output excerpt:`, точечные тесты и строки в исполняемую карту; запуск проверок отложен до общего gate |
| `hermione-version-metadata.md` | `перенесено` | Подагент обновил `codex-cli` и `codex-tui` до `0.141.0+hermione`, синхронизировал `Cargo.lock`, owner-карточку обновил |
| `internal-fork-docs-workflow.md` | `перенесено` | Подагент выявил устаревшую привязку к старым docs-каталогам; `AGENTS.md` и owner-карточка обновлены под текущий `docs/`/`docs/fork/` workflow |
| `memory-read-template-path.md` | `перенесено` | Подагент подтвердил `[memories].read_template_path`, schema/config/tests и prompt builder; синхронизированы README и owner-карточка с фактическим runtime-шаблоном |
| `release-fast-build-profile.md` | `перенесено` | Подагент подтвердил `[profile.release-fast]` с `codegen-units = 32`, `strip = "symbols"` и target `build-fast-release`; изменений не потребовалось |
| `terminal-title-session-label.md` | `перенесено` | Подагент подтвердил `[tui].terminal_title_label`, `session-label`, runtime terminal title rendering и snapshot coverage; изменений не потребовалось |
| `tui-history-image-previews.md` | `перенесено` | Подагент подтвердил typed `LocalImage`, `InsertLocalImage`, `ImagePreviewSize`, `preview_size`, app-server `previewSize` и Kitty placeholder path; изменений не потребовалось |
| `multi-agent-v2-task-depth.md` | `reverted` | card status is `reverted`; old fork patch should not be restored |

## Выполненная подготовка

- Upstream refs и tags обновлены.
- `rust-v0.141.0` найден как `3508d8e226f4fa070a8b38298555ff1ab7cda33c`.
- Merge `rust-v0.141.0` в `hermione-0.141.0` выполнен без commit.
- Механические conflict markers убраны в:
  - `codex-rs/Cargo.toml`;
  - `codex-rs/app-server-protocol/schema/typescript/v2/ThreadItem.ts`;
  - `codex-rs/core/src/tasks/user_shell.rs`;
  - `codex-rs/core/src/tools/handlers/mod.rs`;
  - `codex-rs/core/src/tools/spec_plan.rs`.
- Fork-карточка `codex-agent-env-var.md` проверена подагентом без
  `fork_context`; кодовых изменений не потребовалось, owner-карточка обновлена
  фактической read-only сверкой.
- Fork-карточка `core-system-time-tool.md` проверена подагентом без
  `fork_context`; кодовых изменений и обновления owner-карточки не
  потребовалось.
- Fork-карточка `core-thread-info-tool.md` проверена подагентом без
  `fork_context`; добавлены unit tests для fallback-контракта `agent_name`,
  owner-карточка обновлена миграционной сверкой.
- Fork-карточка `developer-instructions-files.md` проверена подагентом без
  `fork_context`; добавлен регрессионный тест runtime override, owner-карточка
  обновлена миграционной сверкой.
- Fork-карточка `environment-context-project-name.md` проверена подагентом без
  `fork_context`; кодовых изменений и обновления owner-карточки не
  потребовалось.
- Fork-карточка `hermione-version-metadata.md` проверена подагентом без
  `fork_context`; explicit versions `codex-cli` и `codex-tui` обновлены до
  `0.141.0+hermione`, `Cargo.lock` синхронизирован, owner-карточка обновлена.
- Fork-карточка `internal-fork-docs-workflow.md` проверена подагентом без
  `fork_context`; после подтверждения preview убрана обязательность старых
  docs-каталогов, `AGENTS.md` и owner-карточка синхронизированы с текущим
  `docs/`/`docs/fork/` workflow.
- Fork-карточка `memory-read-template-path.md` проверена подагентом без
  `fork_context`; подтверждены config key, schema/config/tests и prompt builder,
  `codex-rs/memories/README.md` и owner-карточка синхронизированы с фактическим
  runtime-шаблоном `codex-rs/ext/memories/templates/memories/read_path.md`.
- Fork-карточка `release-fast-build-profile.md` проверена подагентом без
  `fork_context`; подтверждены профиль `[profile.release-fast]`, target
  `build-fast-release` и deferred remote build проверки; изменений не
  потребовалось.
- Fork-карточка `terminal-title-session-label.md` проверена подагентом без
  `fork_context`; подтверждены config key, runtime terminal title rendering,
  truncation behavior и snapshot coverage; изменений не потребовалось.
- Fork-карточка `tui-history-image-previews.md` проверена подагентом без
  `fork_context`; подтверждены typed local-image history items, preview sizing,
  app-server schema surface и Kitty placeholder path; изменений не потребовалось.

## Выполненные проверки общего прохода

Проверки выполнялись после добавления `exec-command-output-spill-files.md` в
таблицу покрытия и переноса текущего локального diff на `f-ms-dev`.

| Проверка | Результат | Лог |
| --- | --- | --- |
| `scripts/fork-migration/local-preflight.sh 0.141.0` | `RESULT: ok` | `target/fork-migration/preflight-logs/0.141.0-preflight-20260621T181627Z.log` |
| `scripts/fork-migration/local-format.sh check` | `RESULT: ok` | `target/fork-migration/format-logs/check-20260621T181632Z.log` |
| `scripts/fork-migration/local-generators.sh` | `RESULT: ok` | `target/fork-migration/generator-logs/generators-20260621T181642Z.log` |
| `scripts/fork-migration/remote-prepare-host.sh 0.141.0` | `RESULT: ok` | `target/fork-migration/remote-prepare-logs/0.141.0-remote-prepare-20260621T181709Z.log` |
| `scripts/fork-migration/remote-apply-patch.sh 0.141.0` | `RESULT: ok`; удаленный diff совпал с локальным diff, `DIFF_SHA256=a76fb3733702eb359397da93730f36755ae4092d60d190c250de0f0d90d88dc9` | `target/fork-migration/remote-patch-logs/0.141.0-remote-patch-20260621T181718Z.log` |
| `scripts/fork-migration/remote-tests.sh 0.141.0 cards` | `RESULT: ok`; удаленный `local-tests.sh 0.141.0 cards` тоже завершился с `RESULT: ok` | локальный лог: `target/fork-migration/remote-test-logs/0.141.0-cards-remote-tests-20260621T183240Z.log`; удаленный лог: `/home/slader/Projects/codex/target/fork-migration/test-logs/0.141.0-cards-20260621T183241Z.log` |
| `scripts/fork-migration/remote-prepare-host.sh 0.141.0` | `RESULT: ok`; remote checkout повторно подготовлен перед fast build после обновления migration-карты | `target/fork-migration/remote-prepare-logs/0.141.0-remote-prepare-20260621T190723Z.log` |
| `scripts/fork-migration/remote-apply-patch.sh 0.141.0` | `RESULT: ok`; удаленный diff совпал с локальным diff, `DIFF_SHA256=191fce319705094fc4a6e9cb30f2d3e4c9a1e1b88f039393052dd65ab8486dbb` | `target/fork-migration/remote-patch-logs/0.141.0-remote-patch-20260621T190740Z.log` |
| `scripts/fork-migration/remote-build-fast.sh 0.141.0` | `RESULT: ok`; release-fast build, binary file metadata и binary version прошли; binary: `/home/slader/Projects/codex/codex-rs/target/release-fast/codex` | локальный лог: `target/fork-migration/remote-build-logs/0.141.0-remote-build-fast-20260621T190934Z.log`; удаленный лог: `/home/slader/Projects/codex/target/fork-migration/build-logs/0.141.0-build-fast-20260621T190935Z.log` |

## Проверенные карточки

### `codex-agent-env-var.md`

Статус: `перенесено`.

Подагент подтвердил, что runtime-переменные `CODEX_AGENT`, `CODEX_CALL_ID`,
`CODEX_ROLLOUT` и `CODEX_THREAD_ID` присутствуют в текущем коде после merge
`rust-v0.141.0`. Кодовых изменений по этой карточке не потребовалось.

Обновлена owner-карточка `docs/fork/codex-agent-env-var.md`: добавлена
фактическая проверка от 2026-06-19 и отмечено, что команды проверки в запуске
подагента не выполнялись.

Проверки, которые должен запустить родительский агент после прохода карточек:

- `just test -p codex-core exec_env`;
- `just test -p codex-core agent_name`;
- `just test -p codex-core thread_info`;
- `just test -p codex-core maybe_wrap_shell_lc_with_snapshot_restores_codex_identity_from_env`;
- `just test -p codex-core env_overlay_for_exec_server_keeps_runtime_changes_only`;
- `just test -p codex-core shell_command_handler_to_exec_params_uses_session_shell_and_turn_context`;
- `just test -p codex-protocol shell_environment`.

### `core-system-time-tool.md`

Статус: `перенесено`.

Подагент подтвердил, что `get_system_time` присутствует в текущем коде после
merge `rust-v0.141.0`: runtime handler, tool spec, регистрация,
prompt-caching entry и тестовые owner-файлы соответствуют контракту карточки.
Кодовых и документационных изменений по этой карточке не потребовалось.

Проверки, которые должен запустить родительский агент после прохода карточек:

- `just test -p codex-core system_time`;
- `just test -p codex-core prompt_tools_are_consistent_across_requests`;
- release-fast/build/live-tool проверка, если общий релизный прогон требует
  заново подтвердить установочный артефакт.

### `core-thread-info-tool.md`

Статус: `перенесено`.

Подагент подтвердил, что `get_thread_info` присутствует в текущем коде после
merge `rust-v0.141.0`: handler, spec, регистрация, prompt tool list,
materialize/read path текущего rollout и чтение persisted thread metadata
соответствуют контракту карточки.

Кодовая доработка: добавлены unit tests в
`codex-rs/core/src/agent/agent_name_tests.rs` для fallback на
`agent_nickname` у текущего subagent, приоритета `agent_role` у persisted
thread и fallback на `agent_nickname` у persisted thread.

Обновлена owner-карточка `docs/fork/core-thread-info-tool.md`: добавлена
ручная миграционная сверка от 2026-06-19 и уточнена строка
`agent_name_tests.rs` в карте файлов.

Проверки, которые должен запустить родительский агент после прохода карточек:

- `just fmt` в `codex-rs/`;
- `just test -p codex-core agent_name`;
- `just test -p codex-core thread_info`;
- `just test -p codex-core prompt_tools_are_consistent_across_requests`.

### `developer-instructions-files.md`

Статус: `перенесено`.

Подагент подтвердил, что `developer_instructions_files` присутствует в текущем
коде после merge `rust-v0.141.0`: поле есть в `ConfigToml`, relative path
normalization покрывает массив путей, runtime-сборка читает файлы в порядке
списка, missing/unreadable file даёт ошибку, empty file даёт warning, schema
содержит ключ.

Кодовая доработка: добавлен тест
`developer_instructions_override_skips_files`, который закрепляет, что runtime
override `developer_instructions` не читает файлы из config.

Обновлена owner-карточка `docs/fork/developer-instructions-files.md`:
регрессионное покрытие, проверки и сводка покрытия теперь включают новый тест.

Проверки, которые должен запустить родительский агент после прохода карточек:

- targeted `just test -p codex-core developer_instructions`;
- `just write-config-schema`, если общий schema diff после merge потребует
  подтверждения;
- `just fmt` в `codex-rs/`;
- `just fix -p codex-core`.

### `environment-context-project-name.md`

Статус: `перенесено`.

Подагент подтвердил, что model-visible `project_name` присутствует в текущем
коде после merge `rust-v0.141.0`: `EnvironmentContext` содержит
`project_name: Option<String>`, rendering добавляет `<project_name>`,
`equals_except_shell(...)` и `diff_from_turn_context_item(...)` учитывают поле,
а реконструкция из `TurnContextItem.workspace_roots` сохраняет общий источник
для `project_name` и `filesystem`.

Кодовых и документационных изменений по этой карточке не потребовалось.

Проверки, которые должен запустить родительский агент после прохода карточек:

- `just fmt` в `codex-rs/`;
- `just test -p codex-core environment_context`.

### `hermione-version-metadata.md`

Статус: `перенесено`.

Подагент подтвердил, что workspace version после merge `rust-v0.141.0`
составляет `0.141.0`, а fork-метка нужна только для shipped crates
`codex-cli` и `codex-tui`. Логика update-check уже игнорирует Cargo build
metadata после `+` в CLI и TUI paths.

Кодовая доработка: explicit versions в `codex-rs/cli/Cargo.toml` и
`codex-rs/tui/Cargo.toml` обновлены с `0.140.0+hermione` до
`0.141.0+hermione`. `codex-rs/Cargo.lock` синхронизирован для пакетов
`codex-cli` и `codex-tui`.

Обновлена owner-карточка `docs/fork/hermione-version-metadata.md`:
`source_scope`, текущая версия, карта файлов, итоговый контракт, пошаговое
воспроизведение, проверочные команды и сводка покрытия теперь описывают
`0.141.0+hermione`.

Проверки, которые должен запустить родительский агент после прохода карточек:

- `git diff --check`;
- `rg -n "0\\.141\\.0\\+hermione|split_once\\('\\+'\\)|CARGO_PKG_VERSION" codex-rs`;
- `just fmt` в `codex-rs/`;
- `just test -p codex-cli`;
- `just test -p codex-tui`;
- release-fast build и проверка `codex --version`, если общий migration gate
  требует подтвердить установленный binary.

### `internal-fork-docs-workflow.md`

Статус: `перенесено`.

Подагент подтвердил, что старая карточка все еще описывала обязательные каталоги
`docs/architecture`, `docs/plans`, `docs/follow-ups` и `docs/backlog`, хотя в
текущем checkout живыми внутренними fork-документами являются `docs/fork/`,
`docs/migration-one-card-for-agent.md`, `docs/.markdownlint-cli2.yaml` и
`docs/table-rendering-long-links-test.md`.

Документационная доработка: в `AGENTS.md` правило о `docs/` теперь разрешает
fork-specific internal development documentation в `docs/` без привязки к
конкретным старым подкаталогам. В owner-карточке
`docs/fork/internal-fork-docs-workflow.md` старые каталоги оставлены как
исторический контекст, а текущий контракт переписан вокруг `docs/`,
`docs/fork/`, migration-таблицы и инструкции для подагента одной карточки.

Проверки, которые должен запустить родительский агент после прохода карточек:

- `git diff --check`;
- прямой markdownlint-вызов для затронутых Markdown-файлов, если общий
  документационный gate требует lint-проверку;
- редакторская вычитка затронутого русского технического текста.

### `memory-read-template-path.md`

Статус: `перенесено`.

Подагент подтвердил, что после merge `rust-v0.141.0` контракт
`[memories].read_template_path` присутствует в config types, effective config,
schema, config tests, extension config, prompt builder и tests. Runtime-владелец
read-path developer-instruction injection находится в
`codex-rs/ext/memories/src/prompts.rs`, а canonical read-path template живет в
`codex-rs/ext/memories/templates/memories/read_path.md`.

Документационная доработка: `codex-rs/memories/README.md` больше не указывает
исторический путь `codex-rs/memories/read/templates/...` для runtime-шаблона.
Owner-карточка `docs/fork/memory-read-template-path.md` обновлена под базу
`rust-v0.141.0..HEAD` / `hermione-0.141.0` и фиксирует требование
синхронизировать README с фактическим владельцем runtime-шаблона.

Проверки, которые должен запустить родительский агент после прохода карточек:

- `git diff --check`;
- markdownlint или общий docs-check для затронутых Markdown-файлов;
- `just write-config-schema`, если общий migration gate требует подтвердить
  отсутствие schema drift;
- целевые проверки config / `codex-ext-memories`, если они входят в общий
  прогон после прохода карточек.

### `release-fast-build-profile.md`

Статус: `перенесено`.

Подагент подтвердил, что после merge `rust-v0.141.0` профиль
`[profile.release-fast]` присутствует в `codex-rs/Cargo.toml`, наследует
`release` и задает параметры install artifact build: `lto = "thin"`,
`codegen-units = 32`, `debug = "none"` и `strip = "symbols"`.

Target `build-fast-release` присутствует в root `justfile` и собирает
`codex-cli` через `cargo build -p codex-cli --profile release-fast`. Кодовых и
документационных изменений по этой карточке не потребовалось.

Проверки, которые должен запустить родительский агент после прохода карточек:

- `just build-fast-release` на `f-ms-dev:/home/slader/Projects/codex`;
- `file codex-rs/target/release-fast/codex`;
- проверка version metadata установленного артефакта, если общий migration gate
  требует подтвердить release-fast binary.

### `terminal-title-session-label.md`

Статус: `перенесено`.

Подагент подтвердил, что после merge `rust-v0.141.0` контракт terminal title
session label присутствует в текущей кодовой базе: config key
`[tui].terminal_title_label`, effective `Config.tui_terminal_title_label`,
mapping item `session-label`, preview value, runtime terminal title rendering,
24-character truncation и optional omission через `None`.

Кодовых и документационных изменений по этой карточке не потребовалось.

Проверки, которые должен запустить родительский агент после прохода карточек:

- targeted проверки `codex-tui` для terminal-title и snapshots;
- schema verification/generation, если общий migration gate требует подтвердить
  отсутствие schema drift.

### `tui-history-image-previews.md`

Статус: `перенесено`.

Подагент подтвердил, что после merge `rust-v0.141.0` контракт TUI image history
previews присутствует в текущей кодовой базе: typed `LocalImage` path,
`AppEvent::InsertLocalImage`, `ImagePreviewSize`, `preview_size`, app-server
`previewSize`, `ImageGeneration.saved_path`, replay/reflow path и Kitty
placeholder path.

Кодовых и документационных изменений по этой карточке не потребовалось.

Проверки, которые должен запустить родительский агент после прохода карточек:

- targeted проверки `codex-tui` для image/history paths и snapshots;
- targeted проверки `codex-core` для config и `view_image`;
- targeted проверки `codex-app-server-protocol`;
- schema generators, `fmt`, `fix` и markdownlint в общем migration gate, если
  они требуются перед commit.

## Правило работы с подагентами

Для проверки одной строки таблицы основной агент запускает свежего подагента
без `fork_context` и передает ему:

- `docs/migration-one-card-for-agent.md`;
- одну выбранную карточку `docs/fork/<card>.md`.

Подагент не читает `FORK.md`, не читает другие `docs/fork/*.md` целиком, не
запускает сборку, тесты, генераторы, форматирование или `fix`, не делает
`git commit` и `git push`. Родительский агент проверяет результат и обновляет
эту карту покрытия перед переходом к следующей карточке.
