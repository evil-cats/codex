---
id: fork-migration-0.140.0
status: completed
created: 2026-06-16
updated: 2026-06-16
source_scope: rust-v0.140.0..hermione-0.140.0
---

# Migration check: `0.140.0`

## Обзор

Эта карточка фиксирует перенос Hermione fork с ветки `hermione-0.137.0` на
upstream tag `rust-v0.140.0`.

Цель миграции: не восстановить все отличия старой ветки механически, а
перенести только fork-доработки, у которых есть владеющая карточка в
`docs/fork/`, и явно отличить их от upstream-кода, который мог измениться между
`0.137.0` и `0.140.0`.

| Поле | Значение |
| --- | --- |
| Статус | `completed` |
| Новая ветка | `hermione-0.140.0` |
| Upstream tag | `rust-v0.140.0` |
| Предыдущая fork-ветка | `hermione-0.137.0` |
| Код правится | локально, `/home/slader/Projects/evilcats/codex` |
| Сборка и Rust/`just` проверки | только `f-ms-dev:/home/slader/Projects/codex` |
| Текущий проверочный статус | локальный merge, `rg`-аудит, remote generators, targeted tests, scoped `just fix` и `build-fast-release` выполнены |

## Важное правило переноса

Старая Hermione-ветка содержит одновременно fork-доработки и обычный upstream
код версии `0.137.0`. Поэтому признак "есть в `hermione-0.137.0`, но нет в
`rust-v0.140.0`" сам по себе не доказывает, что код нужно восстанавливать.

Для восстановления нужен один из признаков:

- существующая owning-карточка в `docs/fork/`;
- явное решение пользователя для текущей миграции;
- связь с уже описанным fork-контрактом, подтвержденная поиском по owner-файлам.

Отдельно проверено: `goal` tools не имеют карточки в `docs/fork/` и не являются
известной fork-доработкой. Поэтому `get_goal`, `create_goal`, `update_goal`,
`codex-rs/core/src/tools/handlers/goal.rs`,
`codex-rs/core/src/tools/handlers/goal_spec.rs` и каталог
`codex-rs/core/src/tools/handlers/goal/` не восстанавливаются при переносе на
`0.140.0`.

## Карта покрытия

| Карточка | Статус переноса на `0.140.0` | Живые признаки |
| --- | --- | --- |
| `core-system-time-tool.md` | перенесено и проверено на `f-ms-dev` | `SystemTimeHandler`, `get_system_time`, `system_time.rs`, `system_time_spec.rs`, prompt-caching entry |
| `developer-instructions-files.md` | перенесено и проверено на `f-ms-dev` | `developer_instructions_files`, config loader path normalization, config tests, schema entry |
| `environment-context-project-name.md` | перенесено и проверено на `f-ms-dev` | `project_name`, `effective_workspace_roots`, render/diff tests |
| `hermione-version-metadata.md` | перенесено и проверено на `f-ms-dev` | `codex-cli` and `codex-tui` use `0.140.0+hermione`; `Cargo.lock` reflects the same |
| `memory-read-template-path.md` | перенесено и проверено на `f-ms-dev` | `[memories].read_template_path`, `build_memory_tool_developer_instructions`, configured template tests |
| `release-fast-build-profile.md` | перенесено и собрано на `f-ms-dev` | `[profile.release-fast]`, `codegen-units = 32`, `just build-fast-release` |
| `terminal-title-session-label.md` | перенесено и проверено на `f-ms-dev` | `terminal_title_label`, `tui_terminal_title_label`, `session-label`, `SessionLabel` snapshots |
| `tui-history-image-previews.md` | перенесено и проверено на `f-ms-dev` | `LocalImage`, `InsertLocalImage`, `ImagePreviewSize`, `preview_size`, Kitty placeholder path |
| `internal-fork-docs-workflow.md` | перенесено и проверено review-сверкой | `AGENTS.md` keeps fork-internal docs exception and upstream `0.140.0` test guidance |
| `multi-agent-v2-task-depth.md` | исторически не применяется | card status is `reverted`; old fork patch is not restored |

## Merge decisions

### `AGENTS.md`

Conflict resolution keeps both sides:

- upstream `0.140.0` rules:
  - do not add tests for statically defined values;
  - do not add negative tests for removed logic;
- Hermione fork docs boundary:
  - do not add broad upstream product/user docs under `docs/`;
  - allow fork-internal engineering docs under controlled roots.

### `release-fast`

Upstream `0.140.0` changed `[profile.release]` from the older fat-LTO,
single-codegen-unit shape to `lto = "thin"` and `codegen-units = 4`. Hermione
keeps `[profile.release-fast]` as a named local/remote compile-check path with
`codegen-units = 32`.

### `view_image` and TUI local image previews

Conflict resolution keeps upstream `ResizeAllImages` and `PathUri` handling, and
also keeps Hermione `preview_size` propagation into `ImageViewItem`.

In TUI, local image insertion now uses upstream `App::insert_history_cell(...)`
instead of the older fork-local helper. This preserves upstream token-activity
history behavior while keeping `AppEvent::InsertLocalImage`.

### `goal` tools

`get_goal`, `create_goal`, and `update_goal` are not part of the recorded
Hermione fork delta. They are not present in `rust-v0.140.0` and have no
`docs/fork/` owner card. The migration intentionally does not restore them.

## Локальная проверка без сборки

Выполнены локальные проверки поиска и diff:

- `rg -n "SystemTimeHandler|get_system_time|system_time" ...`;
- `rg -n "developer_instructions_files|developer instructions file" ...`;
- `rg -n "project_name|effective_workspace_roots|diff_from_turn_context_item" ...`;
- `rg -n "0\.140\.0\+hermione|split_once\('\+'\)|CARGO_PKG_VERSION" ...`;
- `rg -n "read_template_path|build_memory_tool_developer_instructions" ...`;
- `rg -n "release-fast|build-fast-release|codegen-units = 32" ...`;
- `rg -n "terminal_title_label|session-label|SessionLabel" ...`;
- `rg -n "LocalImage|InsertLocalImage|ImagePreviewSize|preview_size|history_image_preview" ...`;
- `rg -n "goal|get_goal|create_goal|update_goal|ThreadGoal" docs/fork` returned no fork-card owner for goal tools.

Эти проверки подтверждают наличие code anchors, но не заменяют сборку,
форматирование, schema generation или targeted tests.

## Проверки на `f-ms-dev`

Remote checkout синхронизирован по `FORK.md`: disposable mirror
`f-ms-dev:/home/slader/Projects/codex` приведен к чистой базе
`rust-v0.140.0`, применен локальный patch, а итоговый remote staged patch
сравнен с локальным staged patch по checksum.

Выполнены:

- `just fmt`;
- `just write-config-schema`;
- `just write-app-server-schema`;
- `just test -p codex-core system_time`;
- `just test -p codex-core prompt_tools_are_consistent_across_requests`;
- `just test -p codex-core developer_instructions_files`;
- `just test -p codex-core test_toml_parsing`;
- `just test -p codex-memories-extension build_memory_tool_developer_instructions_uses_configured_template`;
- `just test -p codex-core project_name`;
- `just test -p codex-core history_image_preview`;
- `just test -p codex-core view_image`;
- `just test -p codex-tui view_image_tool_call_preserves_preview_size_hint`;
- `just test -p codex-tui terminal_title`;
- `just test -p codex-app-server-protocol`;
- `just build-fast-release`;
- scoped `just fix -p ...` for changed Rust packages.

До запуска использовался такой порядок:

1. Проверить локальный intended diff.
2. Проверить branch, `HEAD`, remotes и `git status --short` на
   `f-ms-dev:/home/slader/Projects/codex`.
3. Привести disposable remote checkout к чистой базе `rust-v0.140.0` или
   согласованной ветке миграции.
4. Перенести локальный patch, включая новые файлы.
5. Сравнить список измененных файлов.
6. Запустить проверки только на `f-ms-dev`.

## Открытые вопросы

- No open migration blocker is currently recorded.
- `git diff --cached --check rust-v0.140.0` still reports trailing whitespace
  in the TUI snapshot
  `codex_tui__bottom_pane__title_setup__tests__terminal_title_setup_basic.snap`;
  this is snapshot padding and was not stripped mechanically.

## Проверка покрытия

| Пункт | Статус | Где отражено |
| --- | --- | --- |
| Перенести все active `docs/fork/` fork-доработки на `0.140.0` | перенесено и проверено на `f-ms-dev` | "Карта покрытия", "Проверки на `f-ms-dev`" |
| Не восстанавливать неописанные old-upstream tools как fork code | перенесено | "Важное правило переноса", "`goal` tools" |
| Зафиксировать remote-only правило сборки | перенесено | "Проверки на `f-ms-dev`" |
| Обновить `+hermione` version metadata до `0.140.0+hermione` | перенесено и проверено на `f-ms-dev` | `hermione-version-metadata.md`, "Карта покрытия" |
| Подтвердить сборкой и тестами | подтверждено | "Проверки на `f-ms-dev`" |
