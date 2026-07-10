---
id: fork-migration-0.144.1
status: draft
created: 2026-07-10
updated: 2026-07-10
source_scope: rust-v0.144.1..hermione-0.144.1
---

# Migration check: `0.144.1`

## Обзор

Эта карточка фиксирует перенос Hermione fork с ветки `hermione-0.143.0` на
upstream tag `rust-v0.144.1`.

Цель миграции: не восстановить весь старый diff механически, а проверить и при
необходимости доработать только fork-доработки, у которых есть владеющая
карточка в `docs/fork/`.

| Поле | Значение |
| --- | --- |
| Статус | `draft` |
| Новая ветка | `hermione-0.144.1` |
| Upstream tag | `rust-v0.144.1` |
| Upstream commit | `44918ea10c0f99151c6710411b4322c2f5c96bea` |
| Предыдущая fork-ветка | `hermione-0.143.0` |
| Предыдущий fork commit | `433519605296d1342cbf3f93e1c1749006bb7474` |
| Локальный checkout | `/data/Projects/codex` |
| Workflow-команды | `.codex/skills/fork/scripts/fork` |
| Codebase-memory index | обновлять перед передачей каждой карточки свежему подагенту |
| Codebase-memory ADR | не используется дальше; для карточек обновляется только index |
| Текущий проверочный статус | owner-карточки обработаны; merge-конфликты разрешены; общие gates еще не завершены |

## Правило переноса

Старая Hermione-ветка содержит одновременно fork-доработки и код предыдущей
версии upstream. Поэтому признак "есть в `hermione-0.143.0`, но нет в
`rust-v0.144.1`" сам по себе не доказывает, что код нужно восстанавливать.

Для восстановления нужен один из признаков:

- существующая owner-карточка в `docs/fork/`;
- явное решение пользователя для текущей миграции;
- связь с уже описанным fork-контрактом, подтвержденная поиском по owner-файлам.

## Состояние слияния

- Upstream refs и tags обновлены перед созданием ветки.
- `rust-v0.144.1` найден как `44918ea10c0f99151c6710411b4322c2f5c96bea`.
- Ветка `hermione-0.144.1` создана от `hermione-0.143.0`.
- Ветка `hermione-0.144.1` еще не опубликована на `origin`.
- Merge `rust-v0.144.1` в `hermione-0.144.1` подготовлен командой
  `git merge --no-commit rust-v0.144.1`.
- Общий merge commit еще не создан.
- Во время обработки карточек неразрешенные конфликты допустимы.
- Родительский агент не строит заранее карту соответствия конфликтов карточкам.
- Текущий список конфликтующих файлов проверяется командой:

  ```bash
  git diff --name-only --diff-filter=U
  ```

### Текущие конфликтующие файлы

```text
none
```

## Карта покрытия

| Карточка | Статус переноса на `0.144.1` | Живые признаки |
| --- | --- | --- |
| `codex-agent-env-var.md` | `перенесено` | Подагент подтвердил `CODEX_AGENT`, `CODEX_CALL_ID`, `CODEX_ROLLOUT`, `CODEX_THREAD_ID` и `RuntimeEnv`; parent-side проверка не нашла conflict markers в owner-файлах |
| `core-read-file-tool.md` | `перенесено` | Подагент подтвердил handler/spec/config/schema/tests; parent-side проверка не нашла conflict markers в owner-файлах |
| `core-system-time-tool.md` | `перенесено` | Подагент подтвердил handler/spec/tests/registration; generic TUI activity оставлен для `tui-core-tool-activity.md` |
| `core-thread-info-tool.md` | `перенесено` | Карточка обновлена для `rust-v0.144.1`: core tool activity и protocol kind добавлены в карту; `fork tests --mode list --card` показал три проверки |
| `developer-instructions-files.md` | `перенесено` | Карточка обновлена под `rust-v0.144.1`; `fork tests --mode list --card` показал проверку `developer instructions` |
| `environment-context-project-name.md` | `перенесено` | Подагент подтвердил `project_name` в environment context; `fork tests --mode list --card` показал `environment context` |
| `exec-command-output-spill-files.md` | `перенесено` | Подагент подтвердил inline limit/output spill contract; `fork tests --mode list --card` показал шесть проверок |
| `hermione-version-metadata.md` | `перенесено` | `Cargo.toml` staged как resolved; CLI/TUI manifests и lockfile обновлены на `0.144.1+hermione`; lockfile normalization остается общим gate |
| `internal-fork-docs-workflow.md` | `перенесено` | Карточка синхронизирована с `rust-v0.144.1`; `fork tests --mode list --card` ожидаемо не имеет argv, exception `manual-required` присутствует |
| `mcp-rollout-diagnostics.md` | `перенесено` | Подагент подтвердил persisted `McpDiagnostic*` и recovery paths; generated/schema состояние вынесено в общий gate |
| `mcp-stderr-thread-logs.md` | `перенесено` | Подагент подтвердил thread-attributed MCP stderr fields; `fork tests --mode list --card` показал две проверки |
| `memory-read-template-path.md` | `перенесено` | Доработан embedded memory template guard: unknown placeholders теперь panic при lazy parse; `fork tests --mode list --card` показал две проверки |
| `multi-agent-v1-spawn-agent-guidance.md` | `перенесено` | Legacy policy-split удален; V1 `spawn_agent` prompt и тест синхронизированы с tool-owned guidance; policy не правился |
| `release-fast-build-profile.md` | `перенесено` | `[profile.release-fast]` сохранен; `messages.rs` staged как resolved с `sanitize_user_text` и `LocalImage`; build-fast остается общим gate |
| `terminal-title-session-label.md` | `перенесено` | Подагент подтвердил `terminal_title_label`/`session-label`; `fork tests --mode list --card` показал две проверки |
| `tui-core-tool-activity.md` | `перенесено` | `CoreToolActivity` сохранен в live/replay/history lifecycle; конфликты в `protocol.rs`, `replay.rs` и `thread_history.rs` staged как resolved; `fork tests --mode list --card` показал пять проверок |
| `tui-history-image-previews.md` | `перенесено` | `ThreadItem.ts` staged как resolved: upstream `WebSearchItem`/`ImageGenerationItem` сохранены, fork `previewSize` для `imageView` сохранен; отсутствующий `docs/architecture/...` оставлен как отдельный вопрос |
| `tui-thread-runtime-unload.md` | `перенесено` | Подагент и parent-side anchor check подтвердили `thread/unload`, app-server teardown и TUI unload paths; правки не потребовались |

`multi-agent-v2-task-depth.md` не включена в карту покрытия, потому что текущий
статус карточки `reverted`; старый fork patch по ней не должен
восстанавливаться автоматически.

## Результаты по карточкам

### `codex-agent-env-var.md`

- Статус: `перенесено`.
- Что изменилось: кодовые и документационные правки не потребовались; upstream
  merge уже изменил `codex-rs/core/src/tasks/user_shell.rs`,
  `codex-rs/core/src/unified_exec/process_manager.rs` и
  `codex-rs/core/src/unified_exec/process_manager_tests.rs`, но контракт
  runtime env сохранен.
- Какие проверки нужны: в общем проходе запустить skill-owned card-level
  покрытие для `fork-codex-agent-env-var`.
- Блокирующие условия: нет; parent-side `rg` не нашел conflict markers в
  owner-файлах карточки.
- Codebase-memory index: full reindex выполнен перед передачей карточки
  подагенту; `skipped_count=0`, `parse_partial_count=9` из-за текущих
  конфликтов и одного shell-файла.

### `core-read-file-tool.md`

- Статус: `перенесено`.
- Что изменилось: кодовые и документационные правки не потребовались; upstream
  merge уже изменил config/schema/spec-plan owner-файлы, но контракт
  `read_file` сохранен.
- Какие проверки нужны: в общем проходе запустить skill-owned card-level
  покрытие для `fork-core-read-file-tool`.
- Блокирующие условия: нет; parent-side `rg` не нашел conflict markers в
  owner-файлах карточки.
- Codebase-memory index: full reindex выполнен перед передачей карточки
  подагенту; `skipped_count=0`, `parse_partial_count=0`.

### `core-system-time-tool.md`

- Статус: `перенесено`.
- Что изменилось: кодовые и документационные правки не потребовались; текущий
  код сохраняет `SystemTimeHandler`, tool spec `get_system_time`, defaults
  `%H:%M`/`local`, проверку offset и регистрацию в `add_core_utility_tools(...)`.
- Какие проверки нужны: в общем проходе запустить skill-owned card-level
  покрытие для `fork-core-system-time-tool`, включая prompt tool cache.
- Блокирующие условия: нет; parent-side `rg` не нашел conflict markers в
  owner-файлах карточки. Generic `CoreToolActivityKind::SystemTime`/TUI display
  будет проверяться вместе с owner-карточкой `tui-core-tool-activity.md`.
- Codebase-memory index: full reindex выполнен перед передачей карточки
  подагенту; `skipped_count=0`, `parse_partial_count=0`.

### `core-thread-info-tool.md`

- Статус: `перенесено`.
- Что изменилось: Rust-код дорабатывать не потребовалось; обновлена owner-карточка
  `docs/fork/core-thread-info-tool.md` с текущими owner-зонами
  `codex-rs/core/src/tools/core_tool_activity.rs` и
  `codex-rs/protocol/src/items.rs`, контрактом `CoreToolActivityKind::ThreadInfo`
  и миграционной сверкой `rust-v0.144.1`.
- Какие проверки нужны: в общем проходе запустить skill-owned card-level
  покрытие для `fork-core-thread-info-tool`; `fork tests --mode list --card
  docs/fork/core-thread-info-tool.md` показал проверки `agent name`,
  `thread info` и `prompt tool cache`.
- Блокирующие условия: нет; parent-side diff review карточки выполнен,
  conflict markers в owner-файлах не найдены, редакторская вычитка измененного
  русского текста выполнена.
- Codebase-memory index: full reindex выполнен перед передачей карточки
  подагенту; `skipped_count=0`, `parse_partial_count=0`.

### `developer-instructions-files.md`

- Статус: `перенесено`.
- Что изменилось: кодовые правки не потребовались; owner-карточка обновлена для
  текущей базы проверки `rust-v0.144.1` / `hermione-0.144.1` и получила новую
  историческую запись о миграционной сверке 2026-07-10.
- Какие проверки нужны: в общем проходе запустить skill-owned card-level
  покрытие для `fork-developer-instructions-files`; `fork tests --mode list
  --card docs/fork/developer-instructions-files.md` показал проверку
  `developer instructions`. Также нужен общий `fork generators`, потому что
  карточка владеет config/schema surface.
- Блокирующие условия: нет; parent-side diff review карточки выполнен,
  conflict markers в owner-файлах не найдены, редакторская вычитка измененного
  русского текста выполнена.
- Codebase-memory index: full reindex выполнен перед передачей карточки
  подагенту; `skipped_count=0`, `parse_partial_count=0`.

### `environment-context-project-name.md`

- Статус: `перенесено`.
- Что изменилось: кодовые и документационные правки не потребовались; текущий
  код сохраняет `project_name` в `EnvironmentsState`, `RenderedEnvironments` и
  `EnvironmentsSnapshot`, а render/diff покрывают XML escaping и смену имени
  проекта.
- Какие проверки нужны: в общем проходе запустить skill-owned card-level
  покрытие для `fork-environment-context-project-name`; `fork tests --mode list
  --card docs/fork/environment-context-project-name.md` показал проверку
  `environment context`.
- Блокирующие условия: нет; parent-side `rg` не нашел conflict markers в
  owner-файлах карточки.
- Codebase-memory index: full reindex выполнен перед передачей карточки
  подагенту; `skipped_count=0`, `parse_partial_count=0`.

### `exec-command-output-spill-files.md`

- Статус: `перенесено`.
- Что изменилось: кодовые и документационные правки не потребовались; текущий
  код сохраняет `[tools.exec].inline_output_max_tokens`, default `1000`,
  `exec_outputs` spill path, save-failure metadata и форматирование ответов
  `exec_command`.
- Какие проверки нужны: в общем проходе запустить skill-owned card-level
  покрытие для `fork-exec-command-output-spill-files`; `fork tests --mode list
  --card docs/fork/exec-command-output-spill-files.md` показал проверки
  `inline token limit`, `spill output`, `spill formatting`,
  `spill save failure formatting`, `large output spill` и `timeout poll`.
- Блокирующие условия: нет; parent-side `rg` не нашел conflict markers в
  owner-файлах карточки.
- Codebase-memory index: full reindex выполнен перед передачей карточки
  подагенту; `skipped_count=0`, `parse_partial_count=0`.

### `hermione-version-metadata.md`

- Статус: `перенесено`.
- Что изменилось: разрешен card-owned конфликт в `codex-rs/Cargo.toml`; parent
  подтвердил итоговый `workspace.package.version = "0.144.1"` и staged
  `codex-rs/Cargo.toml` как resolved. `codex-rs/cli/Cargo.toml`,
  `codex-rs/tui/Cargo.toml` и соответствующие entries `codex-rs/Cargo.lock`
  обновлены на `0.144.1+hermione`; owner-карточка переведена на
  `rust-v0.144.1` / `hermione-0.144.1`.
- Какие проверки нужны: в общем проходе запустить skill-owned card-level
  покрытие для `fork-hermione-version-metadata`; `fork tests --mode list --card
  docs/fork/hermione-version-metadata.md` показал проверки `cli metadata` и
  `tui metadata`. Также нужен общий build/version evidence и lockfile
  normalization после обработки карточек.
- Блокирующие условия: нет в области карточки; parent-side `rg` подтвердил
  отсутствие conflict markers и `0.143.0+hermione` в owner-файлах версии.
  `codex-rs/Cargo.lock` сейчас имеет staged merge + unstaged version edits и
  будет проверяться в общем финальном проходе.
- Codebase-memory index: full reindex выполнен перед передачей карточки
  подагенту; `skipped_count=0`, `parse_partial_count=0`.

### `internal-fork-docs-workflow.md`

- Статус: `перенесено`.
- Что изменилось: owner-карточка обновлена: `updated=2026-07-10`, три
  упоминания текущей миграции заменены с `rust-v0.143.0` на `rust-v0.144.1`.
  `source_scope` не менялся, потому что описывает исторический исходный диапазон.
- Какие проверки нужны: общий `fork cards validate`, markdownlint по локальной
  `docs/.markdownlint-cli2.yaml` и общий whitespace/diff check. `fork tests
  --mode list --card docs/fork/internal-fork-docs-workflow.md` ожидаемо вернул
  отсутствие `fork-tests.v1`; карточка содержит machine-checkable exception
  `manual-required`.
- Блокирующие условия: нет; parent-side diff review выполнен, conflict markers в
  связанных файлах не найдены, редакторская вычитка измененного русского текста
  выполнена.
- Codebase-memory index: full reindex выполнен перед передачей карточки
  подагенту; `skipped_count=0`, `parse_partial_count=0`.

### `mcp-rollout-diagnostics.md`

- Статус: `перенесено`.
- Что изменилось: кодовые и документационные правки не потребовались; текущий
  код сохраняет `RolloutItem::McpDiagnostic`, `McpDiagnosticItem`,
  `McpDiagnosticEvent`, persistence policy, reconstruction-ignore path,
  recovery/retry для `TransportClosed`/`BrokenPipe` и bounded stderr tail.
- Какие проверки нужны: в общем проходе запустить skill-owned card-level
  покрытие для `fork-mcp-rollout-diagnostics`; `fork tests --mode list --card
  docs/fork/mcp-rollout-diagnostics.md` показал четыре проверки. Дополнительно
  общий `fork generators` должен подтвердить schema/generated состояние.
- Блокирующие условия: нет в области карточки; parent-side `rg` не нашел
  conflict markers в owner-файлах. Отсутствие generated-представлений
  `RolloutItem` не классифицировано как дефект до общего schema/generator gate.
- Codebase-memory index: full reindex выполнен перед передачей карточки
  подагенту; `skipped_count=0`, `parse_partial_count=0`.

### `mcp-stderr-thread-logs.md`

- Статус: `перенесено`.
- Что изменилось: кодовые и документационные правки не потребовались; текущий
  код сохраняет `MCP server stderr` events с `server_name`, `program` и
  `stderr_line`, передачу `server_name` через stdio call chain и thread
  attribution через `session_init.mcp_manager_init`/`LogDbLayer`.
- Какие проверки нужны: в общем проходе запустить skill-owned card-level
  покрытие для `fork-mcp-stderr-thread-logs`; `fork tests --mode list --card
  docs/fork/mcp-stderr-thread-logs.md` показал проверки `mcp stderr log
  attribution` и `rmcp stdio call chain`.
- Блокирующие условия: нет; parent-side `rg` не нашел conflict markers в
  owner-файлах карточки.
- Codebase-memory index: full reindex выполнен перед передачей карточки
  подагенту; `skipped_count=0`, `parse_partial_count=0`.

### `memory-read-template-path.md`

- Статус: `перенесено`.
- Что изменилось: подагент доработал `codex-rs/ext/memories/src/prompts.rs`:
  `parse_embedded_template` теперь валидирует allowed placeholders
  `base_path` и `memory_summary` при lazy initialization embedded-шаблона и
  падает на unsupported placeholder. Добавлен тест
  `parse_embedded_template_rejects_unknown_placeholder`; owner-карточка
  обновлена в разделе регрессионного покрытия.
- Какие проверки нужны: в общем проходе запустить skill-owned card-level
  покрытие для `fork-memory-read-template-path`; `fork tests --mode list --card
  docs/fork/memory-read-template-path.md` показал проверки `core config` и
  `memories extension`. Нужен format gate из-за Rust-правки; schema/generator
  gate для этой карточки не требуется, потому что config/schema shape не менялся.
- Блокирующие условия: нет; parent-side diff review выполнен, точный conflict
  marker regex в owner-файлах пуст, редакторская вычитка измененного русского
  текста выполнена.
- Codebase-memory index: full reindex выполнен перед передачей карточки
  подагенту; `skipped_count=0`, `parse_partial_count=0`.

### `multi-agent-v1-spawn-agent-guidance.md`

- Статус: `перенесено`.
- Что изменилось: по решению пользователя legacy policy-split удален из
  owner-карточки; `${HOME}/.codex/policies/session-policy.md` не правился. V1
  `spawn_agent` description теперь сам содержит tool-owned критерии useful
  delegation, запрещает trivial/vague/tightly coupled delegation и parallel
  activity ради самой параллельности, а также требует wait/close/integrate без
  локального дублирования delegated work. Тест переименован в
  `spawn_agent_tool_v1_uses_tool_owned_delegation_guidance` и закрепляет
  отсутствие legacy `session delegation policy` ссылки.
- Какие проверки нужны: `fork tests --mode list --card
  docs/fork/multi-agent-v1-spawn-agent-guidance.md` показал проверку
  `multi agent v1 spawn agent guidance`; в общем проходе нужен card-level test.
- Блокирующие условия: нет; sensitive policy edit исключен из scope текущим
  решением пользователя.
- Codebase-memory index: full reindex выполнен перед передачей карточки
  подагенту; `skipped_count=0`, `parse_partial_count=0`.

### `release-fast-build-profile.md`

- Статус: `перенесено`.
- Что изменилось: `[profile.release-fast]` в `codex-rs/Cargo.toml` сохраняет
  `inherits = "release"`, `lto = "thin"`, `codegen-units = 32`,
  `debug = "none"` и `strip = "symbols"`; `justfile` сохраняет target
  `build-fast-release`. Подагент разрешил конфликт в
  `codex-rs/tui/src/history_cell/messages.rs`: `raw_lines` теперь вызывает
  `sanitize_user_text(...)`, а rich-режим продолжает возвращать
  `HistoryCellDisplayItem::LocalImage`. Parent-side staged этот файл как
  resolved; пересечение будет отдельно проверено карточкой
  `tui-history-image-previews.md`.
- Какие проверки нужны: `fork build-fast` в общем проходе должен подтвердить
  release-fast build, stripped binary и version evidence. `fork tests --mode
  list --card docs/fork/release-fast-build-profile.md` ожидаемо вернул
  отсутствие `fork-tests.v1`; карточка содержит `not-applicable`, потому что
  owner проверки - build gate, а не card-level argv.
- Блокирующие условия: нет в области карточки; parent-side diff review выполнен,
  conflict markers отсутствуют, редакторская вычитка измененного русского текста
  выполнена. Оставшиеся unmerged-файлы вне области карточки будут закрываться
  другими owner-карточками или общим cleanup.
- Codebase-memory index: full reindex выполнен перед передачей карточки
  подагенту; `skipped_count=0`, `parse_partial_count=0`.

### `terminal-title-session-label.md`

- Статус: `перенесено`.
- Что изменилось: кодовые и документационные правки не потребовались; текущий
  код сохраняет `[tui].terminal_title_label`, effective
  `tui_terminal_title_label`, schema entry, item `session-label`, truncation `24`
  и default terminal title `["activity", "project-name"]`.
- Какие проверки нужны: в общем проходе запустить skill-owned card-level
  покрытие для `fork-terminal-title-session-label`; `fork tests --mode list
  --card docs/fork/terminal-title-session-label.md` показал проверки
  `config terminal title label` и `terminal title`.
- Блокирующие условия: нет; parent-side `rg` не нашел conflict markers в
  owner-файлах карточки.
- Codebase-memory index: full reindex выполнен перед передачей карточки
  подагенту; `skipped_count=0`, `parse_partial_count=0`.

### `tui-core-tool-activity.md`

- Статус: `перенесено`.
- Что изменилось: подагент разрешил конфликты в
  `codex-rs/tui/src/chatwidget/protocol.rs`,
  `codex-rs/tui/src/chatwidget/replay.rs` и
  `codex-rs/app-server-protocol/src/protocol/thread_history.rs`. Parent-side
  проверка подтвердила, что live path сохраняет `CoreToolActivity` и upstream
  `WebSearch`/`ImageGeneration`, replay path сохраняет completed-only
  `CoreToolActivity`, `WebSearch`, `ImageGeneration` и review lifecycle, а
  app-server thread history upsert-ит `CoreToolActivity`, `Extension`,
  `EnteredReviewMode` и `ExitedReviewMode` из materialized item lifecycle.
- Какие проверки нужны: в общем проходе запустить skill-owned card-level
  покрытие для `fork-tui-core-tool-activity`; `fork tests --mode list --card
  docs/fork/tui-core-tool-activity.md` показал проверки `tui snapshots and
  lifecycle`, `thread history replay`, `protocol item model`, `analytics
  reducer` и `pending snapshots`. Нужны общие format, generator/schema,
  snapshot и build gates из-за touched TUI/app-server protocol файлов.
- Блокирующие условия: нет в области карточки; parent-side `rg` не нашел
  conflict markers в трех бывших конфликтных файлах, diff review выполнен, файлы
  staged как resolved.
- Codebase-memory index: full reindex выполнен перед передачей карточки
  подагенту; `skipped_count=0`, `parse_partial_count=0`.

### `tui-history-image-previews.md`

- Статус: `перенесено`.
- Что изменилось: подагент разрешил конфликт в generated TypeScript
  `codex-rs/app-server-protocol/schema/typescript/v2/ThreadItem.ts`: сохранены
  upstream tagged items `{ "type": "webSearch" } & WebSearchItem` и
  `{ "type": "imageGeneration" } & ImageGenerationItem`, а fork поле
  `previewSize: ImagePreviewSize` оставлено в `imageView`. Устаревший inline
  `AbsolutePathBuf` import не возвращался, потому что `savedPath` теперь живет в
  `ImageGenerationItem`. Owner-карточка обновлена заметкой для переноса на
  `rust-v0.144.1`.
- Какие проверки нужны: в общем проходе запустить `fork generators`, чтобы
  подтвердить generated-surface, и skill-owned card-level покрытие для
  `fork-tui-history-image-previews`; `fork tests --mode list --card
  docs/fork/tui-history-image-previews.md` показал проверки `core view image`,
  `tui render`, `app server protocol`, `protocol` и `pending snapshots`. Нужны
  общие snapshot/format/build gates.
- Блокирующие условия: нет для переноса карточки; parent-side diff review,
  conflict marker scan и `git diff --check` выполнены, `ThreadItem.ts` staged
  как resolved. Отдельный документационный вопрос: карточка ссылается на
  `docs/architecture/features/tui-history-image-previews.md`, но
  `docs/architecture` отсутствует в текущем checkout.
- Codebase-memory index: full reindex выполнен перед передачей карточки
  подагенту; `skipped_count=0`, `parse_partial_count=0`.

### `tui-thread-runtime-unload.md`

- Статус: `перенесено`.
- Что изменилось: кодовые и документационные правки не потребовались; текущий
  код сохраняет app-server v2 метод `thread/unload` с
  `ThreadUnloadParams`/`ThreadUnloadResponse`/`ThreadUnloadStatus`, обработчик
  unload через `wait_for_thread_shutdown` без archive/delete, cleanup
  state/watch/pending requests, а также TUI unload paths для `/resume`,
  `/clear`, `NewSession`, `/fork`, shutdown-first exit и side discard.
- Какие проверки нужны: в общем проходе запустить skill-owned card-level
  покрытие для `fork-tui-thread-runtime-unload`; `fork tests --mode list --card
  docs/fork/tui-thread-runtime-unload.md` показал проверки app-server runtime
  unload, TUI resume, TUI clear, TUI fork и TUI side close. Полезный, но не
  блокирующий ручной риск из карточки остается прежним: runtime smoke с реальным
  stdio MCP fixture.
- Блокирующие условия: нет; parent-side `rg` не нашел conflict markers в
  owner-файлах карточки.
- Codebase-memory index: full reindex выполнен перед передачей карточки
  подагенту; `skipped_count=0`, `parse_partial_count=0`.

## Общие проверки

- `fork preflight`: `ok`; branch, migration card, unfinished rows, reverted
  card row, whitespace, conflict marker scan, markdownlint и fork CLI unit tests
  прошли.
- `fork tests --mode list`: `ok`; исполняемая карта печатает все active
  card-level проверки, включая tool-owned
  `fork-multi-agent-v1-spawn-agent-guidance`.
- `fork cards validate`: `ok`; проверено 24 карточки, ошибок формы нет.
- `fork format --check`: `ok`; финальная проверка форматирования прошла без
  изменений.
- `fork generators`: `ok`; config schema, experimental app-server schema и
  app-server schema подтвердились без дополнительного unstaged drift.
- `fork tests --mode cards`: `ok`; общий card-level проход для `0.144.1` прошел
  все активные карточки после двух compile-fixes текущего merge:
  `McpConnectionManager::new` test call синхронизирован с новым
  `diagnostic_context`, `TurnContextItem` fixtures получили
  `approvals_reviewer`, а `thread_unload` переведен на текущий
  `TestAppServer::builder()`.
- `fork build-fast`: `ok`; release-fast build, binary metadata и binary version
  подтверждены для `0.144.1`.
- `fork install`: `ok`; installed binary metadata и binary version подтверждены
  для `${HOME}/.local/bin/codex-hermione`.
- `fork tests --mode full`: не запускался; полный регрессионный gate не был
  запрошен и пока не требовался после успешных card-level checks и build-fast.

Фиксируй только команду/gate, результат и существенное подтверждение вроде
версии, binary/install target или причины падения. Не записывай локальные пути
и служебные breadcrumb-строки wrapper-а.
