---
id: fork-migration-0.143.0
status: completed
created: 2026-07-08
updated: 2026-07-08
source_scope: rust-v0.143.0..hermione-0.143.0
---

# Migration check: `0.143.0`

## Обзор

Эта карточка фиксирует перенос Hermione fork с ветки `hermione-0.142.5` на
upstream tag `rust-v0.143.0`.

Цель миграции: не восстановить весь старый diff механически, а проверить и при
необходимости доработать только fork-доработки, у которых есть владеющая
карточка в `docs/fork/`.

| Поле | Значение |
| --- | --- |
| Статус | `completed` |
| Новая ветка | `hermione-0.143.0` |
| Upstream tag | `rust-v0.143.0` |
| Upstream commit | `b213653e584580ccbb6dbd17ca1a6561e4bf065a` |
| Предыдущая fork-ветка | `hermione-0.142.5` |
| Предыдущий fork commit | `288a9a5d28d437fad79b642360cc88d240ab2fb7` |
| Локальный checkout | `/data/Projects/codex` |
| Workflow-команды | `.codex/skills/fork/scripts/fork` |
| Codebase-memory index | обновлен после merge и после обработанных карточек для project `data-Projects-codex` |
| Codebase-memory ADR | не используется дальше; после карточек обновляется только index |
| Текущий проверочный статус | merge `rust-v0.143.0` разрешен по owner-карточкам; card gates, generators, `build-fast` и install прошли |

## Правило переноса

Старая Hermione-ветка содержит одновременно fork-доработки и код предыдущей
версии upstream. Поэтому признак "есть в `hermione-0.142.5`, но нет в
`rust-v0.143.0`" сам по себе не доказывает, что код нужно восстанавливать.

Для восстановления нужен один из признаков:

- существующая owner-карточка в `docs/fork/`;
- явное решение пользователя для текущей миграции;
- связь с уже описанным fork-контрактом, подтвержденная поиском по owner-файлам.

## Состояние слияния

- Upstream refs и tags обновлены перед созданием ветки.
- `rust-v0.143.0` найден как `b213653e584580ccbb6dbd17ca1a6561e4bf065a`.
- Ветка `hermione-0.143.0` создана от `hermione-0.142.5`.
- Ветка `hermione-0.143.0` опубликована на `origin` и отслеживает
  `origin/hermione-0.143.0`.
- Merge `rust-v0.143.0` в `hermione-0.143.0` подготовлен командой
  `git merge --no-commit rust-v0.143.0`.
- Общий merge commit еще не создан.
- Во время обработки карточек неразрешенные конфликты были допустимы; после
  parent conflict-resolution прохода текущий список конфликтующих файлов пуст.
- Родительский агент не строит заранее карту соответствия конфликтов карточкам.
- Текущий список конфликтующих файлов проверяется командой:

  ```bash
  git diff --name-only --diff-filter=U
  ```

### Текущие конфликтующие файлы

```text
<none>
```

## Карта покрытия

| Карточка | Статус переноса на `0.143.0` | Живые признаки |
| --- | --- | --- |
| `codex-agent-env-var.md` | `перенесено` | `RuntimeEnv` совмещен с upstream `CODEX_PERMISSION_PROFILE`; shell, snapshot wrapper и unified exec сохраняют fork identity env; MCP index обновлен после карточки |
| `core-read-file-tool.md` | `доработано` | Подагент подтвердил `read_file` handler/spec/config/schema/tests и разрешил visibility conflict в `spec_plan_tests.rs`; MCP доступен и использован |
| `core-system-time-tool.md` | `перенесено` | Подагент подтвердил handler/spec/tests, `get_system_time` в prompt cache и зависимость `chrono`; MCP доступен и использован; правки не требовались |
| `core-thread-info-tool.md` | `перенесено` | Подагент подтвердил `ThreadInfoHandler`, spec, prompt cache, rollout/session fields и agent-name fallback; MCP доступен и использован; правки не требовались |
| `developer-instructions-files.md` | `перенесено` | Подагент подтвердил config/loader/runtime/schema контракт; обновлена карточка с базой `rust-v0.143.0`; MCP доступен и использован |
| `environment-context-project-name.md` | `перенесено` | Подагент перенес `project_name` на upstream `WorldStateSection::snapshot`/`EnvironmentsSnapshot`; owner-файлы staged как resolved; MCP доступен и использован |
| `exec-command-output-spill-files.md` | `перенесено` | Подагент подтвердил spill contract для `exec_command`; правки не требовались; MCP доступен и использован |
| `hermione-version-metadata.md` | `доработано` | `codex-cli`/`codex-tui` bumped to `0.143.0+hermione`; `workspace.package.version` resolved to `0.143.0`; `Cargo.lock` owner entries and remaining common conflict resolved; lock consistency gate still pending |
| `internal-fork-docs-workflow.md` | `перенесено` | Документальный контракт fork-specific internal docs сохранен; карточка синхронизирована с `rust-v0.143.0`; MCP доступен и использован |
| `memory-read-template-path.md` | `перенесено` | Подагент подтвердил embedded memory read template, отсутствие active `read_template_path` и сохранение memory update policy; правки не требовались |
| `multi-agent-v1-spawn-agent-guidance.md` | `частично перенесено` | V1 `spawn_agent` guidance восстановлен и staged как resolved; outside-repo `session-policy` не содержит согласованный delegation block и требует отдельного решения |
| `multi-agent-v2-task-depth.md` | `reverted` | card status is `reverted`; old fork patch should not be restored |
| `release-fast-build-profile.md` | `перенесено` | Подагент подтвердил `[profile.release-fast]`, `build-fast-release` и related migration-repair anchors; карточка обновлена для `rust-v0.143.0`; build gate не запускался |
| `terminal-title-session-label.md` | `перенесено` | Подагент подтвердил config/effective Config/schema/selector/preview/runtime/tests/snapshots для `terminal_title_label` и `session-label`; правки не требовались |
| `tui-core-tool-activity.md` | `перенесено` | `CoreToolActivity` перенесен через protocol/app-server/TUI lifecycle; shared files окончательно staged после view-image карточки |
| `tui-history-image-previews.md` | `доработано` | `ImageView`/`view_image` перенесены на upstream `PathUri`/`LegacyAppPathString` с сохранением fork `preview_size`/`previewSize`; in-scope files staged как resolved |

## Результаты по карточкам

### `codex-agent-env-var.md`

- Статус: `перенесено`.
- Что изменилось: подагент разрешил конфликты в
  `codex-rs/core/src/exec_env.rs`,
  `codex-rs/core/src/tools/handlers/shell/shell_command.rs`,
  `codex-rs/core/src/tools/handlers/shell_tests.rs`,
  `codex-rs/core/src/tools/runtimes/mod.rs` и
  `codex-rs/core/src/unified_exec/process_manager.rs`. Fork `RuntimeEnv`
  сохранен вместе с upstream `CODEX_PERMISSION_PROFILE_ENV_VAR` и
  `inject_permission_profile_env(...)`; `shell_command` сохраняет upstream
  `TurnEnvironment`/`cwd` и добавляет `CODEX_AGENT`, `CODEX_CALL_ID`,
  `CODEX_ROLLOUT`, `CODEX_THREAD_ID`; snapshot wrapper и unified exec
  сохраняют runtime identity вместе с active permission profile.
- Какие проверки нужны: в общем проходе проверить исполняемую карту через
  `fork tests --mode list --card docs/fork/codex-agent-env-var.md`, затем
  запустить card-level проверки через
  `fork tests --mode cards --card docs/fork/codex-agent-env-var.md --version 0.143.0`;
  `fork generators` для этой карточки не требуется.
- Блокирующие условия: в области карточки не осталось; `rg` по owner-файлам не
  нашел conflict markers, а scoped `git add` отметил пять code-файлов как
  resolved. Общие сборка, форматирование, tests/gates и соседние конфликты еще
  не выполнялись.
- Codebase-memory index: обновлен после обработки карточки для project
  `data-Projects-codex`.

### `core-read-file-tool.md`

- Статус: `доработано`.
- Что изменилось: подагент разрешил конфликт в
  `codex-rs/core/src/tools/spec_plan_tests.rs`: ожидания видимости при
  нескольких окружениях сохраняют `read_file`, `view_image` и upstream
  `request_permissions`. Контракт `read_file` подтвержден в handler/spec/config,
  schema, integration и visibility tests.
- MCP: доступен подагенту; использованы `list_projects`, `search_graph` и
  `get_code_snippet` для `ReadFileHandler`, `create_read_file_tool` и
  `resolve_read_file_content_max_tokens`.
- Какие проверки нужны: в общем проходе проверить исполняемую карту
  `fork-core-read-file-tool` через `fork tests`; card validation уже повторяется
  родителем, schema/generator gate нужен только если общий merge изменит
  соответствующие generated artifacts.
- Блокирующие условия: в области карточки не осталось; `rg` по owner-файлам не
  нашел conflict markers, а scoped `git add` отметил
  `codex-rs/core/src/tools/spec_plan_tests.rs` как resolved. Общие сборка,
  форматирование, tests/gates и соседние конфликты еще не выполнялись.
- Codebase-memory index: обновлен после обработки карточки через MCP-only
  подагента `Socrates`. Последующие `adr_present=false` hints не разворачиваются
  в работу: для этой миграции дальше нужен только index update.

### `core-system-time-tool.md`

- Статус: `перенесено`.
- Что изменилось: кодовых и документационных изменений по карточке не
  потребовалось. Подагент подтвердил наличие `SystemTimeHandler`, tool spec,
  handler/spec tests, регистрацию `get_system_time` после `PlanHandler`, запись
  `"get_system_time"` в prompt cache list и зависимость `chrono`.
- MCP: доступен подагенту; использованы `list_projects`, `search_graph`,
  `search_code` и `get_code_snippet` для project `data-Projects-codex`.
- Какие проверки нужны: в общем проходе запустить card-owned `fork tests` для
  system time и prompt tool cache, затем общие fork gates по migration workflow.
- Блокирующие условия: в области карточки не осталось; owner-файлы без conflict
  markers. Соседние Git-level conflicts закрыты parent conflict-resolution
  проходом.
- Codebase-memory index: обновлен после обработки карточки через MCP-only
  подагента `Averroes` (`index_repository`, `mode=fast`, `persistence=true`).

### `core-thread-info-tool.md`

- Статус: `перенесено`.
- Что изменилось: кодовых и документационных изменений по карточке не
  потребовалось. Подагент подтвердил `ThreadInfoHandler`, spec с
  `thread_id`/`session_id`/`rollout_path`/`agent_name`, регистрацию рядом с
  `SystemTimeHandler`, запись `get_thread_info` в prompt cache list, runtime
  `include_archived: true`, `include_history: false`, `MAX_PARENT_CHAIN_DEPTH =
  64`, rollout materialization и fallback для `agent_name`.
- MCP: доступен подагенту; использованы `index_status`, `search_graph` и
  `get_code_snippet` для project `data-Projects-codex`.
- Какие проверки нужны: после общего разрешения конфликтов запустить
  card-level `fork tests --mode cards --card docs/fork/core-thread-info-tool.md`
  и обычные parent gates.
- Блокирующие условия: в области карточки не осталось; `git diff --cached -G
  'ThreadInfo|thread_info|get_thread_info'` по уже staged изменениям
  `handlers/mod.rs` и `spec_plan.rs` не показал изменений. Соседние Git-level
  conflicts закрыты parent conflict-resolution проходом.
- Codebase-memory index: обновлен после обработки карточки через MCP-only
  подагента `Bernoulli` (`index_repository`, `mode=fast`, `persistence=true`).

### `developer-instructions-files.md`

- Статус: `перенесено`.
- Что изменилось: кодовых изменений не потребовалось. Подагент подтвердил
  `ConfigToml.developer_instructions_files`, loader-нормализацию,
  runtime-сборку effective developer instructions, тестовое покрытие и schema
  artifact. Карточка `docs/fork/developer-instructions-files.md` обновлена:
  `updated` -> `2026-07-08`, текущая база проверки -> `rust-v0.143.0` /
  `hermione-0.143.0`, добавлен исторический результат миграционного прохода, а
  старое "текущий проход" уточнено как первоначальный проход 2026-06-08.
- MCP: доступен подагенту; использованы `list_projects`, `search_code` и
  `search_graph` для project `data-Projects-codex`.
- Какие проверки нужны: `fork cards validate`,
  `fork tests --mode cards --card docs/fork/developer-instructions-files.md --version 0.143.0`,
  `fork generators` для schema freshness и общий `fork build-fast`.
- Блокирующие условия: в области карточки не осталось; owner-файлы без conflict
  markers и без unmerged-состояния.
- Codebase-memory index: обновлен после обработки карточки через MCP-only
  подагента `Jason` (`index_repository`, `mode=fast`, `persistence=true`).

### `environment-context-project-name.md`

- Статус: `перенесено`.
- Что изменилось: подагент разрешил конфликты в
  `codex-rs/core/src/context/world_state/environment.rs` и
  `codex-rs/core/src/context/world_state/environment_render_tests.rs`, сохранив
  upstream `WorldStateSection::snapshot`/`PreviousSectionState` API и добавив
  `project_name` в snapshot-diff. Карточка обновлена под модель
  `rust-v0.143.0` с `EnvironmentsSnapshot`, новыми шагами воспроизведения,
  смысловым покрытием diff-теста и coverage table.
- MCP: доступен подагенту; использованы `list_projects`, `index_status`,
  `search_graph` и `search_code` для project `data-Projects-codex`.
- Какие проверки нужны: parent-side skill-owned проверки для карточки из
  `fork-tests.v1`, затем общий parent проход форматирования, тестов, сборки,
  генераторов и markdownlint.
- Блокирующие условия: в области карточки не осталось; `rg` по owner-файлам не
  нашел conflict markers, а scoped `git add` отметил два code-файла как
  resolved. Общие gates и соседние конфликты еще не выполнялись.
- Codebase-memory index: обновлен после обработки карточки через MCP-only
  подагента `Bacon` (`index_repository`, `mode=fast`, `persistence=true`);
  `adr_present=false` оставлен без дальнейших действий, потому что ADR для этой
  миграции не используется.

### `exec-command-output-spill-files.md`

- Статус: `перенесено`.
- Что изменилось: кодовых и документационных изменений по карточке не
  потребовалось. Подагент подтвердил `[tools.exec].inline_output_max_tokens`,
  default `1000`, schema entry, effective limit clamp по config/request/model
  policy, spill path under `exec_outputs`, exact raw bytes write,
  `response_text()`, model-visible `Output exceeded...` / `Output saved to...`
  / `Output excerpt:`, save-failure formatting, no spill для `SandboxDenied` и
  no spill для `write_stdin`/polling.
- MCP: доступен подагенту; использованы `list_projects`, `search_graph` и
  `get_code_snippet` для project `data-Projects-codex`.
- Какие проверки нужны: parent-owned `fork tests` по `fork-tests.v1` этой
  карточки, `fork cards validate`, schema/generator gate для
  `codex-rs/core/config.schema.json`.
- Блокирующие условия: в области карточки не осталось; `rg` по owner-файлам не
  нашел conflict markers, а `git diff --cached -G` по spill-якорям в owner-файлах
  не показал staged изменений. Соседние Git-level conflicts закрыты parent
  conflict-resolution проходом.
- Codebase-memory index: обновлен после обработки карточки через MCP-only
  подагента `Sartre` (`index_repository`, `mode=fast`, `persistence=true`);
  ADR hints игнорируются.

### `hermione-version-metadata.md`

- Статус: `доработано`.
- Что изменилось: `codex-rs/cli/Cargo.toml` и `codex-rs/tui/Cargo.toml`
  обновлены до `0.143.0+hermione`; `codex-rs/Cargo.toml` разрешен и
  зафиксирован в index с `workspace.package.version = "0.143.0"`;
  `codex-rs/Cargo.lock` содержит owner entries `codex-cli` и `codex-tui` с
  `0.143.0+hermione`, а оставшийся common conflict у `codex-code-mode-host`
  разрешен в parent conflict-resolution проходе.
- MCP: доступен подагенту; использованы `index_status`, `search_graph` и
  `search_code` для project `data-Projects-codex`; ADR-инструменты не
  запускались.
- Какие проверки нужны: parent-owned `fork tests` по `fork-tests.v1` этой
  карточки, итоговая lockfile consistency, общий fork validation/fmt/test gate.
- Блокирующие условия: в scope version metadata conflict markers не осталось;
  `codex-rs/Cargo.lock` staged как resolved после parent conflict-resolution
  прохода.
- Codebase-memory index: обновлен после обработки карточки через MCP-only
  подагента `Singer` (`index_repository`, project `data-Projects-codex`,
  status `indexed`); ADR hint проигнорирован по инструкции пользователя.

### `internal-fork-docs-workflow.md`

- Статус: `перенесено`.
- Что изменилось: кодовых изменений не потребовалось. Карточка обновлена:
  `updated: 2026-07-08`, три stale references на `rust-v0.142.5` заменены на
  `rust-v0.143.0`; контракт `AGENTS.md` про fork-specific internal docs,
  `docs/fork/`, markdownlint config и visual fixture сохранен.
- MCP: доступен подагенту; `index_status` вернул `ready`, `search_code`
  использован точечно для owner-строк; reindex и ADR-инструменты не
  запускались.
- Какие проверки нужны: общий parent-owned `fork cards validate`,
  `fork tests --mode list`, markdownlint/whitespace и финальная проверка
  конфликтов.
- Блокирующие условия: в scope карточки нет; legacy-каталоги
  `docs/architecture`, `docs/plans`, `docs/follow-ups` и `docs/backlog` не
  восстановлены и не требуются.
- Codebase-memory index: обновлен после обработки карточки через MCP-only
  подагента `Hubble` (`index_repository`, project `data-Projects-codex`,
  status `indexed`); ADR не используется.

### `memory-read-template-path.md`

- Статус: `перенесено`.
- Что изменилось: правки не потребовались. Parent-сверка подтвердила, что
  active code/config/schema/README не содержит `read_template_path`, embedded
  `read_path.md` содержит `Updating memories`, safe ad-hoc update path и
  `extensions/ad_hoc/INDEX.md`, а tests проверяют отсутствие старого запрета
  `only when explicitly asked by the user`.
- MCP: доступен подагенту, но `index_status`/`search_graph` в его thread вернули
  `project not found or not indexed` для `data-Projects-codex`, хотя
  `list_projects` показывал project; подагент продолжил scoped `rg`/`read_file`
  без reindex и без ADR-инструментов.
- Какие проверки нужны: parent-owned `fork tests` по `fork-tests.v1` этой
  карточки, schema/generator gate, format gate и общий `fork cards validate`.
- Блокирующие условия: в scope карточки нет; профильный
  `~/.codex/hermione.config.toml` не читался и не менялся.
- Codebase-memory index: обновлен после обработки карточки через MCP-only
  подагента `Archimedes` (`index_repository`, project `data-Projects-codex`,
  status `indexed`); ADR hint в выводе не было.

### `multi-agent-v1-spawn-agent-guidance.md`

- Статус: `частично перенесено`.
- Что изменилось: `codex-rs/core/src/tools/handlers/multi_agents_spec.rs`
  разрешен в scope V1 `spawn_agent` и staged как resolved. Дефолтный V1 prompt
  снова содержит согласованный короткий tool-specific guidance для already
  selected concrete bounded subtask; старый explicit-request guard,
  `Requests for depth...`, `{agent_role_usage_hint}` как authorization guard и
  V2 sidecar-work wording не возвращены в V1. V2 description не менялся.
- MCP: доступен подагенту; `index_status` вернул `ready`, использованы
  `search_graph`, `search_code` и `get_code_snippet`; reindex и ADR-инструменты
  не запускались.
- Outside-repo policy: read-only проверка
  `${HOME}/.codex/policies/session-policy.md` не нашла согласованный раздел
  `## Делегирование задач подагентам`. В файле есть только общие строки
  контекстной гигиены про работу с подагентами, поэтому root-visible часть
  контракта требует отдельного preview/согласования перед правкой sensitive
  profile policy.
- Какие проверки нужны: parent-owned `fork tests` по `fork-tests.v1` этой
  карточки, Rust format gate, profile check
  `${HOME}/.codex/scripts/check-hermione-profile.py` после отдельного
  согласованного policy update, общий `fork cards validate`.
- Блокирующие условия: repo-side V1 prompt conflict markers не осталось;
  карточка не полностью закрыта из-за отсутствующего outside-repo
  `session-policy` delegation block.
- Codebase-memory index: обновлен после обработки карточки через MCP-only
  подагента `Copernicus` (`index_repository`, project `data-Projects-codex`,
  status `indexed`); ADR-подсказок в output не было.

### `multi-agent-v2-task-depth.md`

- Статус: `reverted`.
- Что изменилось: карточка уже имеет статус `reverted`; old fork patch не
  восстанавливается в текущей миграции.
- Какие проверки нужны: не применимо, если статус карточки не меняется.
- Блокирующие условия: нет для текущего переноса.

### `release-fast-build-profile.md`

- Статус: `перенесено`.
- Что изменилось: кодовых правок не потребовалось. Карточка обновлена:
  `updated: 2026-07-08`, добавлен `Migration check: 0.143.0`, строка в
  historical results и строка coverage для текущего переноса.
- Что подтверждено: `[profile.release-fast]` сохраняет `lto = "thin"`,
  `codegen-units = 32`, `debug = "none"`, `strip = "symbols"`;
  `justfile` сохраняет owned target `build-fast-release`; related anchors
  `Config.tui_terminal_title_label`, `HistoryCellDisplayItem`/`HyperlinkLine`
  conversions и `line_to_static` перед `plain_hyperlink_lines(...)`
  присутствуют.
- MCP: доступен подагенту; `index_status` вернул `ready`, использованы
  `search_graph` и `get_code_snippet`; reindex и ADR-инструменты не
  запускались.
- Какие проверки нужны: parent-owned `fork build-fast` для stripped artifact и
  version evidence, общий `fork cards validate`, markdownlint/whitespace gates
  по финальному плану.
- Блокирующие условия: в scope карточки нет; сборка намеренно не запускалась в
  карточном подагенте.
- Codebase-memory index: обновлен после обработки карточки через MCP-only
  подагента `Gibbs` (`index_repository`, project `data-Projects-codex`,
  status `indexed`, mode `fast`, persistence enabled); ADR hint проигнорирован
  по инструкции пользователя.

### `terminal-title-session-label.md`

- Статус: `перенесено`.
- Что изменилось: правки не потребовались. Parent-сверка подтвердила anchors
  `terminal_title_label`, `session-label` и `SessionLabel` в config type,
  effective `Config`, generated schema, selector, preview, runtime terminal
  title, тестах и snapshots; default terminal title не включает `session-label`.
- MCP: доступен подагенту; `index_status` вернул `ready`, использованы
  `search_code` и `search_graph`; reindex и ADR-инструменты не запускались.
- Какие проверки нужны: parent-owned `fork tests` по `fork-tests.v1` этой
  карточки, schema/generator gate, snapshot review/accept gate при UI diff,
  общий `fork cards validate`.
- Блокирующие условия: в scope карточки нет; Git-level conflicts resolved.
- Codebase-memory index: обновлен после обработки карточки через MCP-only
  подагента `Aquinas` (`index_repository`, project `data-Projects-codex`,
  status `indexed`, nodes `85212`, edges `445691`); ADR hint проигнорирован по
  инструкции пользователя.

### `tui-core-tool-activity.md`

- Статус: `перенесено`.
- Что изменилось: `CoreToolActivity` перенесен для `rust-v0.143.0`.
  Подагент сохранил upstream `handle_materialized_item_lifecycle` в
  `thread_history` и добавил `CoreToolActivity` в materialized/upsert path;
  `legacy_events.rs` явно игнорирует `CoreToolActivity`, чтобы новая
  UI-поверхность не меняла legacy/model-visible поток. Карточка обновлена:
  `updated: 2026-07-08`, добавлен `codex-rs/protocol/src/legacy_events.rs` в
  карту файлов и исторические строки для текущего переноса.
- Что staged: `codex-rs/protocol/src/legacy_events.rs` и
  `docs/fork/tui-core-tool-activity.md`.
- Что оставалось вне scope: shared `ImageView`/view-image файлы закрыты после
  обработки `tui-history-image-previews.md` и staged как resolved в parent
  проходе.
- MCP: доступен подагенту; `index_status` вернул `ready`, использованы
  `search_graph` и `search_code`; reindex и ADR-инструменты не запускались.
- Какие проверки нужны: parent-owned `fork tests` по `fork-tests.v1` этой
  карточки, app-server schema generators при финальном API shape, pending
  snapshots, format gate, общий `fork cards validate` и build/test проход после
  разрешения оставшихся конфликтов.
- Блокирующие условия: для `CoreToolActivity` scope не осталось; Git-level
  conflicts resolved.
- Codebase-memory index: обновлен после обработки карточки через MCP-only
  подагента `Plato` (`index_repository`, project `data-Projects-codex`,
  mode `fast`, persistence enabled); ADR не использовалась.

### `tui-history-image-previews.md`

- Статус: `доработано`.
- Что изменилось: in-scope `ImageView`/`view_image` конфликты разрешены.
  Сохранены upstream `PathUri`/`LegacyAppPathString` surfaces и fork
  `preview_size`/`previewSize`. `ImageViewItem.path` остается `PathUri`,
  app-server v2 и generated TS используют `LegacyAppPathString`, а
  `preview_size` проходит через core item, legacy event, app-server item,
  thread history, TUI replay/tool lifecycle и focused tests.
- Что staged: `codex-rs/protocol/src/items.rs`,
  `codex-rs/protocol/src/protocol.rs`, `codex-rs/protocol/src/legacy_events.rs`,
  `codex-rs/app-server-protocol/src/protocol/v2/item.rs`,
  `codex-rs/app-server-protocol/src/protocol/thread_history.rs`,
  `codex-rs/app-server-protocol/src/protocol/v2/tests.rs`,
  `codex-rs/app-server-protocol/schema/typescript/v2/ThreadItem.ts`,
  `codex-rs/core/src/tools/handlers/view_image.rs`,
  `codex-rs/core/tests/suite/view_image.rs`,
  `codex-rs/tui/src/chatwidget/tool_lifecycle.rs`,
  `codex-rs/tui/src/chatwidget/tests/helpers.rs` и
  `docs/fork/tui-history-image-previews.md`.
- MCP: доступен подагенту; `index_status` вернул `ready`, использованы
  `search_graph` и `search_code`; reindex и ADR-инструменты не запускались.
- Какие проверки нужны: parent-owned `fork generators` для app-server/config
  schema surfaces, `fork tests` по `fork-tests.v1` этой карточки, pending
  snapshots, format gate, общий `fork cards validate`, build/markdownlint и
  финальный diff review после разрешения оставшихся конфликтов.
- Блокирующие условия: в scope карточки не осталось; оставшиеся
  multi-agent/client/app-server/model popup/patch history/Cargo conflicts
  закрыты parent conflict-resolution проходом.
- Codebase-memory index: обновлен после обработки карточки через MCP-only
  подагента `Locke` (`index_repository`, project `data-Projects-codex`,
  status `indexed`, mode `fast`, persistence artifact created); ADR hint
  проигнорирован по инструкции пользователя.

## Общие проверки

- `fork cards validate`: `OK`.
- `fork preflight --version 0.143.0`: `OK`.
- `fork format --fix`: `OK`.
- `fork format --check`: `OK`.
- `fork generators`: `OK`.
- `fork tests --mode list --version 0.143.0`: `OK`.
- `fork tests --mode cards --version 0.143.0`: `OK`.
- `fork build-fast --version 0.143.0`: `OK`; binary version verified.
- `fork install`: `OK`; installed binary target
  `/home/slader/.local/bin/codex-hermione`, version verified.
- `fork tests --mode full`: не запускался; card gates покрывают owner-карточки
  текущей миграции, а полный workspace suite оставлен вне этого прохода.

Фиксируй только command/gate, результат и существенное подтверждение вроде
версии, binary/install target или причины падения. Не записывай локальные
пути, служебные breadcrumbs wrapper'а или сырой stdout/stderr как смысловое
подтверждение.
