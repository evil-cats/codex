---
id: fork-migration-0.142.5
status: complete
created: 2026-07-05
updated: 2026-07-05
source_scope: rust-v0.142.5..hermione-0.142.5
---

# Migration check: `0.142.5`

## Обзор

Эта карточка фиксирует перенос Hermione fork с ветки `hermione-0.141.0` на
upstream tag `rust-v0.142.5`.

Цель миграции: не восстановить весь старый diff механически, а проверить и при
необходимости доработать только fork-доработки, у которых есть владеющая
карточка в `docs/fork/`.

| Поле | Значение |
| --- | --- |
| Статус | `complete` |
| Новая ветка | `hermione-0.142.5` |
| Upstream tag | `rust-v0.142.5` |
| Upstream commit | `1b30ea33f13533474db7c3ad6313ef280769e432` |
| Предыдущая fork-ветка | `hermione-0.141.0` |
| Локальный checkout | `/data/Projects/codex` |
| Workflow-команды | `.codex/skills/fork/scripts/fork` |
| Текущий проверочный статус | merge `rust-v0.142.5` разрешен по owner-карточкам; card gates, generators и `build-fast` прошли |

## Правило переноса

Старая Hermione-ветка содержит одновременно fork-доработки и код предыдущей
версии upstream. Поэтому признак "есть в `hermione-0.141.0`, но нет в
`rust-v0.142.5`" сам по себе не доказывает, что код нужно восстанавливать.

Для восстановления нужен один из признаков:

- существующая owner-карточка в `docs/fork/`;
- явное решение пользователя для текущей миграции;
- связь с уже описанным fork-контрактом, подтвержденная поиском по owner-файлам.

## Состояние слияния

- Upstream refs и tags обновлены перед созданием ветки.
- `rust-v0.142.5` найден как `1b30ea33f13533474db7c3ad6313ef280769e432`.
- Ветка `hermione-0.142.5` создана от `hermione-0.141.0`.
- Ветка `hermione-0.142.5` опубликована на `origin` и отслеживает
  `origin/hermione-0.142.5`.
- Merge `rust-v0.142.5` в `hermione-0.142.5` подготовлен командой
  `git merge --no-commit rust-v0.142.5`.
- Общий merge commit создается этим изменением после финального staging.
- Неразрешенных конфликтов не осталось.
- Текущий список конфликтующих файлов проверяется командой:

  ```bash
  git diff --name-only --diff-filter=U
  ```

### Текущие конфликтующие файлы

```text
нет
```

## Карта покрытия

| Карточка | Статус переноса на `0.142.5` | Живые признаки |
| --- | --- | --- |
| `codex-agent-env-var.md` | `перенесено` | Подагент разрешил конфликтные участки `shell_command`, `/shell` и теста; fork runtime env сохранен поверх актуального upstream `shell_environment_policy` и `to_abs_path()` |
| `core-read-file-tool.md` | `перенесено` | Подагент разрешил конфликт в config-области `read_file`; сохранены fork resolver `resolve_read_file_content_max_tokens` и upstream `resolve_orchestrator_feature_enabled` |
| `core-system-time-tool.md` | `перенесено` | Подагент подтвердил `SystemTimeHandler`, `get_system_time`, prompt tool cache entry и зависимость `chrono`; изменений не потребовалось |
| `core-thread-info-tool.md` | `перенесено` | Подагент подтвердил `get_thread_info` runtime/spec/registration и добавил tests для невалидного UUID и неизвестных полей аргументов |
| `developer-instructions-files.md` | `перенесено` | Подагент подтвердил `developer_instructions_files` loader/runtime/schema и усилил tests для warning/error с путями файлов |
| `environment-context-project-name.md` | `перенесено` | Подагент перенес `project_name` в новую upstream-модель `world_state::environment`, сохранил workspace roots/fallback и обновил tests |
| `exec-command-output-spill-files.md` | `перенесено` | Подагент перенес spill-контракт на `turn.model_info.truncation_policy.into()`, добавил runtime test и сохранил `SandboxDenied` без spill |
| `hermione-version-metadata.md` | `перенесено` | Подагент обновил workspace base version до `0.142.5`, CLI/TUI explicit versions до `0.142.5+hermione` и синхронизировал `Cargo.lock` |
| `internal-fork-docs-workflow.md` | `перенесено` | Documentation/workflow-contract карточка сверена: fork-specific internal docs в `docs/` разрешены, legacy docs-каталоги не восстанавливаются механически |
| `memory-read-template-path.md` | `перенесено` | Подагент подтвердил `[memories].read_template_path`, config/schema, extension и prompt builder; изменений не потребовалось |
| `release-fast-build-profile.md` | `перенесено` | Подагент подтвердил `[profile.release-fast]`, `build-fast-release` и обновил комментарий профиля на `release-fast build`; artifact подтвержден `fork build-fast` |
| `terminal-title-session-label.md` | `перенесено` | Подагент подтвердил `session-label` terminal title и добавил config-layer regression test для `[tui].terminal_title_label` |
| `tui-core-tool-activity.md` | `перенесено` | Подагент подтвердил `CoreToolActivity`; generated app-server schema conflicts разрешены вручную с сохранением upstream image/context definitions, подтверждено `fork generators` |
| `tui-history-image-previews.md` | `перенесено` | Подагент разрешил `view_image` конфликт: сохранены `ImagePreviewSize`/`preview_size`, upstream `PathUri`/`data_url_from_bytes`/`detail=original`, stale `Feature::ResizeAllImages` не восстановлен |
| `multi-agent-v2-task-depth.md` | `reverted` | card status is `reverted`; old fork patch should not be restored |

## Результаты по карточкам

### `codex-agent-env-var.md`

- Статус: `перенесено`.
- Что изменилось: конфликтные участки в
  `codex-rs/core/src/tools/handlers/shell/shell_command.rs`,
  `codex-rs/core/src/tools/handlers/shell_tests.rs` и
  `codex-rs/core/src/tasks/user_shell.rs` разрешены в области этой карточки.
  Сохранены fork runtime env через `create_env_with_runtime(...)`,
  `CODEX_AGENT`, `CODEX_CALL_ID`, `CODEX_ROLLOUT`, `CODEX_THREAD_ID`,
  `current_agent_name(...)` и `Session::hook_transcript_path()`. Одновременно
  сохранены upstream-изменения `turn_context.config.permissions.shell_environment_policy`
  и `to_abs_path()` для `/shell`.
- Какие проверки нужны: в общем проходе запустить card-level проверки через
  `fork tests --mode cards --card docs/fork/codex-agent-env-var.md --version 0.142.5`;
  перед этим проверить исполняемую карту через
  `fork tests --mode list --card docs/fork/codex-agent-env-var.md`.
- Блокирующие условия: в области карточки не осталось; общие сборка,
  форматирование, tests/gates и соседние конфликты еще не выполнялись.

### `core-read-file-tool.md`

- Статус: `перенесено`.
- Что изменилось: конфликт в `codex-rs/core/src/config/mod.rs` разрешен с
  сохранением `resolve_read_file_content_max_tokens(...)` и upstream helper
  `resolve_orchestrator_feature_enabled(...)`; import conflict в
  `codex-rs/core/src/config/config_tests.rs` разрешен с сохранением
  `ReadFileToolToml`.
- Какие проверки нужны: в общем проходе запустить card-level проверки через
  `fork tests --mode cards --card docs/fork/core-read-file-tool.md --version 0.142.5`;
  перед этим проверить исполняемую карту через
  `fork tests --mode list --card docs/fork/core-read-file-tool.md`. Так как
  карточка владеет config/schema для `[tools.read_file].content_max_tokens`,
  общий проход должен включать `fork generators`.
- Блокирующие условия: в области карточки не осталось; общие форматирование,
  generators/tests/build gates и соседние конфликты еще не выполнялись.

### `core-system-time-tool.md`

- Статус: `перенесено`.
- Что изменилось: кодовых и документационных изменений по карточке не
  потребовалось; текущая реализация уже содержит `SystemTimeHandler`, tool spec,
  тесты handler/spec, регистрацию в `spec_plan`, запись `"get_system_time"` в
  prompt-caching тесте и зависимость `chrono` в `codex-core`.
- Какие проверки нужны: в общем проходе запустить card-level проверки через
  `fork tests --mode cards --card docs/fork/core-system-time-tool.md --version 0.142.5`;
  перед этим проверить исполняемую карту через
  `fork tests --mode list --card docs/fork/core-system-time-tool.md`.
- Блокирующие условия: в области карточки не осталось; общие tests/gates и
  соседние конфликты еще не выполнялись.

### `core-thread-info-tool.md`

- Статус: `перенесено`.
- Что изменилось: runtime-код `get_thread_info` уже соответствовал контракту
  карточки. Добавлены unit tests в
  `codex-rs/core/src/tools/handlers/thread_info_tests.rs`, которые проверяют
  model-facing ошибку для невалидного UUID и отклонение неизвестных полей
  аргументов через `deny_unknown_fields`. Owner-карточка
  `docs/fork/core-thread-info-tool.md` обновлена результатом миграционной
  сверки.
- Какие проверки нужны: в общем проходе запустить card-level проверки через
  `fork tests --mode cards --card docs/fork/core-thread-info-tool.md --version 0.142.5`;
  перед этим проверить исполняемую карту через
  `fork tests --mode list --card docs/fork/core-thread-info-tool.md`.
- Блокирующие условия: в области карточки не осталось; общие форматирование,
  tests/gates и соседние конфликты еще не выполнялись.

### `developer-instructions-files.md`

- Статус: `перенесено`.
- Что изменилось: контракт `developer_instructions_files` подтвержден в
  `ConfigToml`, loader/runtime и schema. В
  `codex-rs/core/src/config/config_tests.rs` усилены проверки
  `developer_instructions_files_skip_empty_files_with_warning` и
  `developer_instructions_files_reject_missing_file`: предупреждение теперь
  сверяется с полным текстом и путем пустого файла, а ошибка отсутствующего
  файла - с префиксом сообщения, включающим путь. Owner-карточка
  `docs/fork/developer-instructions-files.md` обновлена результатом
  миграционной сверки.
- Какие проверки нужны: в общем проходе запустить card-level проверки через
  `fork tests --mode cards --card docs/fork/developer-instructions-files.md --version 0.142.5`;
  перед этим проверить исполняемую карту через
  `fork tests --mode list --card docs/fork/developer-instructions-files.md`.
- Блокирующие условия: в области карточки не осталось; общие форматирование,
  tests/gates и соседние конфликты еще не выполнялись.

### `environment-context-project-name.md`

- Статус: `перенесено`.
- Что изменилось: контракт `project_name` адаптирован к upstream-реорганизации
  `rust-v0.142.5`, где рендеринг `<environment_context>` живет в
  `codex-rs/core/src/context/world_state/environment.rs`. `project_name`
  перенесен в `EnvironmentsState`/`RenderedEnvironments`, вычисляется из того же
  снимка `workspace_roots`, что и `FileSystemContext`, участвует в
  `WorldStateSection::render_diff(...)`, а старый fallback через `cwd` для
  `TurnContextItem` сохранен. Тесты карточки перенесены в
  `codex-rs/core/src/context/world_state/environment_render_tests.rs`;
  owner-карточка `docs/fork/environment-context-project-name.md` обновлена под
  новую модель.
- Какие проверки нужны: в общем проходе запустить card-level проверки через
  `fork tests --mode cards --card docs/fork/environment-context-project-name.md --version 0.142.5`;
  перед этим проверить исполняемую карту через
  `fork tests --mode list --card docs/fork/environment-context-project-name.md`.
- Блокирующие условия: в области карточки не осталось; общие форматирование,
  tests/gates и соседние конфликты еще не выполнялись.

### `exec-command-output-spill-files.md`

- Статус: `перенесено`.
- Что изменилось: конфликт в
  `codex-rs/core/src/tools/handlers/unified_exec/exec_command.rs` разрешен с
  сохранением `output_spill: None` для `UnifiedExecError::SandboxDenied` и
  переходом с `turn.truncation_policy` на
  `turn.model_info.truncation_policy.into()`. В
  `codex-rs/core/src/unified_exec/process_manager.rs` сохранено вычисление
  effective inline limit по config, request limit и model truncation policy.
  Родительская проверка подтвердила, что large-output spill покрывается
  integration/card gate, а `unified_exec_enforces_glob_deny_read_policy` остается
  в `codex-rs/core/tests/suite/unified_exec.rs` и покрывает `SandboxDenied` без
  spill-файла. Owner-карточка
  `docs/fork/exec-command-output-spill-files.md` обновлена и исправлена с учетом
  найденного integration coverage.
- Какие проверки нужны: в общем проходе запустить card-level проверки через
  `fork tests --mode cards --card docs/fork/exec-command-output-spill-files.md --version 0.142.5`;
  перед этим проверить исполняемую карту через
  `fork tests --mode list --card docs/fork/exec-command-output-spill-files.md`.
  Так как карточка владеет config/schema для `[tools.exec].inline_output_max_tokens`,
  общий проход должен включать `fork generators`.
- Блокирующие условия: в области карточки не осталось; общие форматирование,
  generators/tests/build gates и соседние конфликты еще не выполнялись.

### `hermione-version-metadata.md`

- Статус: `перенесено`.
- Что изменилось: `codex-rs/Cargo.toml` разрешен с
  `workspace.package.version = "0.142.5"` без `+hermione`. В
  `codex-rs/cli/Cargo.toml` и `codex-rs/tui/Cargo.toml` стоят explicit versions
  `0.142.5+hermione`; `codex-rs/Cargo.lock` синхронизирован так, что
  `codex-cli` и `codex-tui` имеют `0.142.5+hermione`, а workspace packages
  имеют plain `0.142.5`. Owner-карточка
  `docs/fork/hermione-version-metadata.md` обновлена под перенос на
  `rust-v0.142.5`.
- Какие проверки нужны: в общем проходе запустить card-level проверки через
  `fork tests --mode cards --card docs/fork/hermione-version-metadata.md --version 0.142.5`;
  перед этим проверить исполняемую карту через
  `fork tests --mode list --card docs/fork/hermione-version-metadata.md`. Финальный
  `fork build-fast --version 0.142.5` должен подтвердить версию binary; install
  evidence нужен только если общий workflow решит устанавливать бинарник.
- Блокирующие условия: в области карточки не осталось; общие форматирование,
  tests/build gates и соседние конфликты еще не выполнялись.

### `internal-fork-docs-workflow.md`

- Статус: `перенесено`.
- Что изменилось: карточка классифицирована как documentation/workflow-contract,
  а не runtime-фича. Подтверждено текущее правило `AGENTS.md`: broad upstream
  product/user-facing docs в `docs/` не добавляются, но fork-specific internal
  docs для решений, проверок, maintenance notes, work tracking и handoff могут
  жить в `docs/`. В `docs/fork/internal-fork-docs-workflow.md` устаревшие
  упоминания текущей миграции `rust-v0.141.0` заменены на `rust-v0.142.5`, а
  ссылка на migration table сделана версионно-нейтральной через
  `docs/fork/migration-X.Y.Z.md`.
- Какие проверки нужны: `fork cards validate`; markdownlint для измененной
  документации через локальную `docs/.markdownlint-cli2.yaml` или текущий
  skill-owned preflight; финальный whitespace/diff check.
- Блокирующие условия: в области карточки не осталось; общие документационные
  checks/gates и соседние конфликты еще не выполнялись.

### `memory-read-template-path.md`

- Статус: `перенесено`.
- Что изменилось: кодовых и документационных изменений по карточке не
  потребовалось; текущая реализация уже содержит `read_template_path` в
  `MemoriesToml` и `MemoriesConfig`, default `None`, перенос из TOML в runtime
  config, schema entry, передачу пути из memories extension в prompt builder и
  fallback на embedded `memories/read_path.md`.
- Какие проверки нужны: в общем проходе запустить card-level проверки через
  `fork tests --mode cards --card docs/fork/memory-read-template-path.md --version 0.142.5`;
  перед этим проверить исполняемую карту через
  `fork tests --mode list --card docs/fork/memory-read-template-path.md`. Так как
  карточка владеет config/schema surface, общий проход должен включать
  `fork generators`.
- Блокирующие условия: в области карточки не осталось; общие generators/tests
  gates и соседние конфликты еще не выполнялись.

### `release-fast-build-profile.md`

- Статус: `перенесено`.
- Что изменилось: подтверждены anchors `[profile.release-fast]`,
  `codegen-units = 32`, `debug = "none"`, `strip = "symbols"` и target
  `build-fast-release` в `justfile`. Комментарий в `codex-rs/Cargo.toml`
  больше не говорит про `remote build`; новая формулировка указывает на
  `release-fast build`. Owner-карточка
  `docs/fork/release-fast-build-profile.md` обновлена результатом миграционной
  сверки `0.142.5`.
- Какие проверки нужны: финальный `fork build-fast --version 0.142.5`
  подтвердил release-fast artifact, binary metadata и binary version.
- Блокирующие условия: нет.

### `terminal-title-session-label.md`

- Статус: `перенесено`.
- Что изменилось: runtime/preview/snapshot anchors для `session-label` уже
  соответствовали контракту карточки. Добавлен test
  `load_config_resolves_tui_terminal_title_label` в
  `codex-rs/core/src/config/config_tests.rs`, чтобы покрыть effective
  `Config.tui_terminal_title_label` при загрузке `[tui].terminal_title_label`.
  Owner-карточка `docs/fork/terminal-title-session-label.md` обновила
  смысловое покрытие и `fork-tests.v1`.
- Какие проверки нужны: в общем проходе запустить card-level проверки через
  `fork tests --mode cards --card docs/fork/terminal-title-session-label.md --version 0.142.5`;
  перед этим проверить исполняемую карту через
  `fork tests --mode list --card docs/fork/terminal-title-session-label.md`.
- Блокирующие условия: в области карточки не осталось; общие форматирование,
  tests/snapshots/gates и соседние конфликты еще не выполнялись.

### `tui-core-tool-activity.md`

- Статус: `перенесено`.
- Что изменилось: подагент подтвердил сохранение `CoreToolActivity` в protocol,
  app-server v2 schema surface и TUI/history contract. Generated schema
  conflicts в app-server v2 артефактах разрешены вручную с сохранением обеих
  сторон: fork `CoreToolActivity` и upstream `ImagePreviewSize`,
  `LegacyAppPathString`, `McpToolCallAppContext`.
- Какие проверки нужны: финальный generator gate подтвердил ручное разрешение
  schema-артефактов:
  `fork generators --version 0.142.5`.
- Блокирующие условия: нет.

### `tui-history-image-previews.md`

- Статус: `перенесено`.
- Что изменилось: подагент разрешил конфликт в
  `codex-rs/core/src/tools/handlers/view_image.rs`, сохранив fork-контракт
  `ImagePreviewSize`/`view_image.preview_size` и актуальные upstream изменения
  `PathUri`, `data_url_from_bytes` и `detail = original`. Устаревшие
  `codex_features::Feature`, `Feature::ResizeAllImages`,
  `PromptImageMode` и `load_for_prompt_bytes` не восстановлены.
- Какие проверки нужны: перед общими gates проверить executable map:
  `fork tests --mode list --card docs/fork/tui-history-image-previews.md`,
  затем выполнить card tests:
  `fork tests --mode cards --card docs/fork/tui-history-image-previews.md --version 0.142.5`.
  Финальный `fork generators --version 0.142.5` подтвердил связанные
  config/app-server schema artifacts.
- Блокирующие условия: нет.

### `multi-agent-v2-task-depth.md`

- Статус: `reverted`.
- Что изменилось: не восстанавливается.
- Какие проверки нужны: не применимо для текущей миграции.
- Блокирующие условия: нет.

## Общие проверки

- `fork cards validate`: `OK`.
- `fork preflight --version 0.142.5`: `OK`.
- `fork format --fix`: `OK`.
- `fork format --check`: `OK`.
- `fork generators --version 0.142.5`: `OK`.
- `fork tests --mode list --version 0.142.5`: `OK`.
- `fork tests --mode cards --version 0.142.5`: `OK`.
- `fork build-fast --version 0.142.5`: `OK`.
- `fork tests --mode full`: не запускался; card gates покрывают owner-карточки
  текущей миграции, а полный workspace suite оставлен вне этого прохода.
