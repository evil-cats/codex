---
id: fork-codex-agent-env-var
status: active
created: 2026-06-17
updated: 2026-06-17
source_scope: working-tree
---

# Runtime-переменная окружения `CODEX_AGENT`

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая добавляет
runtime-переменную окружения `CODEX_AGENT` для CLI-команд, запускаемых Codex.
Переменная заполняется тем же именем агента, которое `get_thread_info`
возвращает в поле `agent_name`.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Runtime-переменная | `CODEX_AGENT` |
| Связанная runtime-переменная | `CODEX_THREAD_ID` |
| Crates | `codex-core`, `codex-protocol` |
| Общий helper имени агента | `codex-rs/core/src/agent/agent_name.rs` |
| Сборка shell env | `codex-rs/protocol/src/shell_environment.rs`, `codex-rs/core/src/exec_env.rs` |
| Основные точки вызова | `shell_command`, `/shell` user task, unified exec sandbox session |
| Связанная карточка | `docs/fork/core-thread-info-tool.md` |
| Удаленный host сборки | `f-ms-dev:/home/slader/Projects/codex` |

Главное runtime-поведение:

- если root-сессия запускает CLI-команду, дочерний процесс получает
  `CODEX_AGENT=<имя root-агента>`;
- если subagent запускает CLI-команду, дочерний процесс получает
  `CODEX_AGENT=<имя subagent>`;
- имя вычисляется через общий helper, который также использует
  `get_thread_info`;
- `CODEX_AGENT` добавляется после `ShellEnvironmentPolicy.include_only`,
  `exclude` и `set`, как служебная runtime-переменная идентичности;
- `CODEX_AGENT` намеренно перезаписывает одноименную переменную из
  родительского окружения или пользовательской shell env policy, если имя
  текущего агента известно;
- если имя агента честно определить нельзя, `CODEX_AGENT` не добавляется.

## Зачем это нужно

`CODEX_THREAD_ID` уже дает CLI-командам однозначный идентификатор текущего
thread. Этого достаточно, чтобы найти rollout JSONL, но недостаточно, чтобы
быстро понять, кто именно запустил команду: root-агент или конкретный subagent.

Hermione workflow использует несколько агентов и subagents. Для shell-скриптов,
диагностических команд, локальных helper-ов и будущих инструментов удобно иметь
простую runtime-подсказку:

- root-команда может видеть, что ее запустила `Hermione`;
- subagent-команда может видеть имя своей роли, например `Researcher`;
- внешние CLI helper-ы могут логировать агентную принадлежность без отдельного
  вызова `get_thread_info`;
- shell snapshots и unified exec не должны терять identity-переменные при
  восстановлении окружения.

Доработка не заменяет `get_thread_info`. `CODEX_AGENT` дает короткую runtime
метку для процесса, а `get_thread_info` остается introspection tool для
`thread_id`, `session_id`, `rollout_path` и полного контракта `agent_name`.

## Согласованные решения

| Пункт | Итоговое решение | Причина |
| --- | --- | --- |
| Имя переменной | `CODEX_AGENT` | Короткое имя рядом с `CODEX_THREAD_ID`; значение описывает текущего агента, а не хранилище профиля |
| Источник значения | Общий helper `current_agent_name(TurnContext)` | `CODEX_AGENT` и `get_thread_info.agent_name` не должны расходиться |
| Root-сессия | `name` из effective config, затем active profile name | Это тот же контракт, что у текущего root `get_thread_info` |
| Subagent | `agent_role`, затем leaf `agent_path`, затем `agent_nickname` | `name` из agent TOML сохраняется как role/name metadata |
| Неизвестное имя | Не добавлять `CODEX_AGENT` | Пустое или выдуманное имя хуже отсутствующей переменной |
| Shell policy | Добавлять после `include_only` и `set` | Runtime-идентичность должна переживать фильтры policy, как `CODEX_THREAD_ID` |
| Конфликт с родительским окружением | Перезаписывать runtime-значением | Дочерний процесс должен видеть текущего агента, а не устаревшее значение родительского процесса |
| Обертка shell snapshot | Восстанавливать `CODEX_AGENT` вместе с `CODEX_THREAD_ID` | Snapshot, подключенный через `source`, может перезаписать env; runtime-идентичность нужно вернуть после snapshot |
| Unified exec remote overlay | Считать `CODEX_AGENT` runtime-only изменением | Exec-server должен получить переменную как runtime-изменение, а не как базовое policy env |
| Remote build | На `f-ms-dev` только сборка и тесты, исходники правятся локально | `f-ms-dev` является сборочным зеркалом, а не местом разработки |

## Карта файлов

| Файл | Ответственность |
| --- | --- |
| `codex-rs/core/src/agent/agent_name.rs` | Общий helper для вычисления имени текущего агента из `TurnContext`, root config/profile и subagent `SessionSource`; helper для сохраненных `StoredThread`-полей |
| `codex-rs/core/src/agent/agent_name_tests.rs` | Unit tests для root fallback и subagent fallback, ранее находившиеся рядом с `thread_info` |
| `codex-rs/core/src/agent/mod.rs` | Подключает модуль `agent_name` |
| `codex-rs/core/src/tools/handlers/thread_info.rs` | Использует общий helper, чтобы поле `agent_name` осталось единым с `CODEX_AGENT` |
| `codex-rs/core/src/tools/handlers/thread_info_tests.rs` | Оставляет tests разбора `thread_id`; tests helper-а перенесены к owner-модулю |
| `codex-rs/protocol/src/shell_environment.rs` | Добавляет константу `CODEX_AGENT_ENV_VAR` и вставляет служебные runtime-переменные идентичности после shell env policy |
| `codex-rs/core/src/exec_env.rs` | Экспортирует `CODEX_AGENT_ENV_VAR` и принимает `agent_name` рядом с `thread_id` при сборке env |
| `codex-rs/core/src/exec_env_tests.rs` | Проверяет, что `CODEX_AGENT` вставляется после фильтров policy и перезаписывает родительское окружение |
| `codex-rs/core/src/tools/handlers/shell/shell_command.rs` | Передает имя текущего агента в env для обычного `shell_command` |
| `codex-rs/core/src/tools/handlers/shell_tests.rs` | Считает expected env через тот же `current_agent_name` |
| `codex-rs/core/src/tasks/user_shell.rs` | Передает `CODEX_AGENT` для пользовательского `/shell` task |
| `codex-rs/core/src/tools/runtimes/mod.rs` | Восстанавливает `CODEX_AGENT` после обертки shell snapshot |
| `codex-rs/core/src/tools/runtimes/mod_tests.rs` | Проверяет сохранение `CODEX_AGENT` и `CODEX_THREAD_ID` после snapshot |
| `codex-rs/core/src/unified_exec/process_manager.rs` | Добавляет `CODEX_AGENT` в runtime env unified exec sandbox session |
| `codex-rs/core/src/unified_exec/process_manager_tests.rs` | Проверяет, что exec-server overlay содержит `CODEX_AGENT` как runtime-изменение |
| `docs/fork/codex-agent-env-var.md` | Владеющий handoff-артефакт для переменной `CODEX_AGENT` |
| `docs/fork/core-thread-info-tool.md` | Связанная карточка: фиксирует общий helper `agent_name` и прежний контракт tool |

Намеренно не менялись:

| Зона | Почему не меняется |
| --- | --- |
| Config schema | `CODEX_AGENT` не добавляет config key; значение вычисляется из уже загруженной config/session metadata |
| App-server protocol | Внешний app-server API не меняется |
| TUI | Переменная нужна дочерним CLI-командам, а не отдельной UI-поверхности |
| Rollout/thread data model | Используются существующие `TurnContext`, `SessionSource`, `ConfigLayerStack` и `StoredThread`-поля |
| `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` и `CODEX_SANDBOX_ENV_VAR` | Эти зоны запрещены локальным `AGENTS.md` и не относятся к агентной identity |
| Persisted root thread lookup | `CODEX_AGENT` относится к текущему live turn; для старых persisted root threads `get_thread_info` по-прежнему не выдумывает имя из текущего profile |

## Итоговый контракт

### Значение `CODEX_AGENT`

`CODEX_AGENT` содержит человекочитаемое имя текущего агента:

- для root-сессии: top-level `name` из effective config;
- если top-level `name` отсутствует или пустой: active user profile name;
- для subagent: `SessionSource::SubAgent(ThreadSpawn.agent_role)`;
- если `agent_role` отсутствует или пустой: leaf `agent_path`;
- если leaf `agent_path` недоступен: `agent_nickname`;
- если ни один источник не дает непустого имени: переменная не добавляется.

Эта логика является общей с `get_thread_info.agent_name`. При будущих переносах
и изменениях нельзя менять один контракт отдельно от другого.

### Поведение shell env policy

`codex_protocol::shell_environment::populate_env(...)` строит окружение так:

1. выбирает базовые переменные по `ShellEnvironmentPolicy.inherit`;
2. применяет default excludes, если они не отключены;
3. применяет custom `exclude`;
4. применяет `set`;
5. применяет `include_only`;
6. добавляет служебные runtime-переменные идентичности:
   - `CODEX_AGENT`, если имя агента передано;
   - `CODEX_THREAD_ID`, если thread id передан.

Следствие: `CODEX_AGENT` и `CODEX_THREAD_ID` переживают `include_only` и
перезаписывают одноименные значения из inherited env или `set`.

### Runtime-точки вызова

`shell_command`:

- `ShellCommandHandler::to_exec_params(...)` получает `TurnContext`;
- вызывает `current_agent_name(turn_context)`;
- передает `agent_name.as_deref()` в `create_env(...)`;
- дочерний процесс получает `CODEX_AGENT`, если имя известно.

`/shell` user task:

- `execute_user_shell_command(...)` получает live `Session` и `TurnContext`;
- вызывает `current_agent_name(turn_context.as_ref())`;
- передает имя в `create_env(...)` вместе с `Session.thread_id`;
- snapshot preparation и proxy stripping работают поверх уже собранного env.

Unified exec sandbox session:

- базовое `local_policy_env` создается без `thread_id` и без `agent_name`;
- затем runtime env получает `CODEX_THREAD_ID`;
- затем, если `current_agent_name(...)` вернул имя, добавляется `CODEX_AGENT`;
- `ExecServerEnvConfig.local_policy_env` остается без этих runtime-переменных
  идентичности, чтобы exec-server overlay видел их как runtime-изменение.

Обертка shell snapshot:

- `maybe_wrap_shell_lc_with_snapshot(...)` получает explicit overrides и полный
  live env отдельно;
- после source snapshot обертка восстанавливает `CODEX_AGENT` и
  `CODEX_THREAD_ID` из live env;
- эти переменные не считаются явными shell policy overrides.

## Примеры поведения

Примеры иллюстрируют runtime-контракт. Конкретные значения зависят от текущего
profile, agent config и thread.

### Root запускает CLI

Root profile config содержит:

```toml
name = "Hermione"
```

Команда:

```bash
printf '%s\n' "$CODEX_AGENT"
```

Ожидаемый stdout:

```text
Hermione
```

### Subagent запускает CLI

Subagent был создан из agent config, где `name` сохранился в metadata как
`agent_role = "Researcher"`.

Команда:

```bash
printf '%s\n' "$CODEX_AGENT"
```

Ожидаемый stdout:

```text
Researcher
```

### `include_only` не удаляет runtime-идентичность

Даже если shell env policy оставляет только `PATH`, runtime-идентичность
добавляется после фильтра:

```text
PATH=/usr/bin
CODEX_AGENT=Hermione
CODEX_THREAD_ID=thread-1
```

## Архитектурное решение

Доработка разделяет ответственность по слоям:

- `codex-protocol` знает только имена runtime-переменных окружения и умеет
  вставить готовые строки после shell env policy;
- `codex-core` знает live `TurnContext` и вычисляет имя текущего агента;
- `get_thread_info` и shell env используют один helper, чтобы не расходиться;
- unified exec сохраняет различие между базой policy и runtime overlay.

Ключевое решение - не читать agent TOML заново при запуске команды. На момент
turn нужная идентичность уже находится в `TurnContext.session_source` или effective
config. Повторное чтение файлов было бы менее надежным:

- agent config path может измениться после запуска subagent;
- persisted metadata уже содержит `agent_role`;
- root effective config уже загружен и отражает активный profile/config stack;
- shell command launch path не должен зависеть от дополнительного filesystem IO.

Отклоненные альтернативы:

| Альтернатива | Почему отклонена |
| --- | --- |
| Дублировать helper из `thread_info.rs` в env-коде | Контракты `get_thread_info.agent_name` и `CODEX_AGENT` могли бы разойтись |
| Всегда ставить `CODEX_AGENT=""` при неизвестном имени | Пустая переменная хуже отсутствующей: дочерний script не может отличить неизвестность от намеренного пустого значения |
| Оставить `CODEX_AGENT` под контролем `ShellEnvironmentPolicy.include_only` | Runtime-идентичность должна быть доступна так же надежно, как `CODEX_THREAD_ID` |
| Считать `CODEX_AGENT` частью `local_policy_env` unified exec | Exec-server overlay потерял бы информацию, что переменная является runtime-изменением |
| Искать имя persisted root thread из текущего profile | Это неверно для старых root threads и не относится к live-окружению CLI-команды |

## Порядок повторения при переносе

При переносе на новый upstream:

1. Прочитать `FORK.md`, эту карточку и `docs/fork/core-thread-info-tool.md`.
2. Проверить, как upstream собирает shell env и где объявлен
   `CODEX_THREAD_ID`.
3. Добавить `CODEX_AGENT_ENV_VAR` рядом с `CODEX_THREAD_ID_ENV_VAR` в protocol
   shell environment module.
4. Расширить `create_env`, `create_env_from_vars` и `populate_env`
   необязательным `agent_name`.
5. Убедиться, что служебные runtime-переменные идентичности добавляются после
   `include_only`.
6. Вынести или восстановить общий helper имени агента:
   - root: `name` effective config, затем active profile;
   - subagent: `agent_role`, затем leaf `agent_path`, затем `agent_nickname`;
   - persisted fields: `agent_role`, затем leaf `agent_path`, затем
     `agent_nickname`.
7. Переключить `get_thread_info` на общий helper.
8. Передать `current_agent_name(...)` в shell command env, `/shell` user task и
   unified exec sandbox session.
9. Для unified exec не класть `CODEX_AGENT` в `local_policy_env`; добавлять его
   только в runtime env.
10. Обновить обертку shell snapshot, чтобы она восстанавливала `CODEX_AGENT`
    вместе с `CODEX_THREAD_ID`.
11. Перенести tests для env policy, agent helper, shell snapshot и unified exec
    overlay.
12. Запустить форматирование и проверки на `f-ms-dev`, затем синхронизировать
    remote-generated изменения обратно в локальный checkout.

## Проверки

Проверки должны запускаться на `f-ms-dev:/home/slader/Projects/codex`, потому
что локально в этом workflow исходники правятся, а Rust/Cargo/`just` сборка и
тесты выполняются на удаленной сборочной машине.

Запланированные проверки для этой доработки:

| Команда | Где запускать | Ожидаемый результат |
| --- | --- | --- |
| `just fmt` | `f-ms-dev`, `codex-rs/` | Форматирование применено; remote diff синхронизирован локально |
| `just test -p codex-core exec_env` | `f-ms-dev`, `codex-rs/` | Проходят tests сборки shell env и вставки `CODEX_AGENT` |
| `just test -p codex-core agent_name` | `f-ms-dev`, `codex-rs/` | Проходят tests общего helper-а имени агента |
| `just test -p codex-core thread_info` | `f-ms-dev`, `codex-rs/` | `get_thread_info` сохраняет прежний контракт `agent_name` через общий helper |
| `just test -p codex-core maybe_wrap_shell_lc_with_snapshot_restores_codex_identity_from_env` | `f-ms-dev`, `codex-rs/` | Snapshot wrapper сохраняет `CODEX_AGENT` и `CODEX_THREAD_ID` |
| `just test -p codex-core env_overlay_for_exec_server_keeps_runtime_changes_only` | `f-ms-dev`, `codex-rs/` | Exec-server overlay включает `CODEX_AGENT` как runtime-изменение |
| `just fix -p codex-core` | `f-ms-dev`, `codex-rs/` | Clippy/fix pass не оставляет обязательных исправлений |
| `just build-fast-release` | `f-ms-dev`, `codex-rs/` | Release-fast binary собирается для установки |

Фактические результаты 2026-06-17:

| Команда | Результат |
| --- | --- |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just fmt` | прошла на `f-ms-dev`; rustfmt изменил `codex-rs/core/src/exec_env_tests.rs` и `codex-rs/protocol/src/shell_environment.rs`, изменения синхронизированы локально |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core exec_env` | прошла: 12 tests run, 12 passed |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core agent_name` | прошла: 6 tests run, 6 passed |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core thread_info` | прошла: 5 tests run, 5 passed |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core maybe_wrap_shell_lc_with_snapshot_restores_codex_identity_from_env` | прошла: 1 test run, 1 passed |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core env_overlay_for_exec_server_keeps_runtime_changes_only` | прошла: 1 test run, 1 passed |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-protocol shell_environment` | прошла: 2 tests run, 2 passed |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core shell_command_handler_to_exec_params_uses_session_shell_and_turn_context` | прошла: 1 test run, 1 passed |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" cargo check -p codex-cli -p codex-app-server -p codex-rmcp-client -p codex-exec-server -p codex-linux-sandbox` | прошла за 2m00s; проверены crates с обновленными точками вызова `create_env(..., agent_name)` |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just fix -p codex-core` | прошла за 1m05s |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just fix -p codex-protocol` | прошла за 37.68s |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just build-fast-release` | прошла за 5m59s |

Полный workspace `just test` не запускался: проектное правило требует
отдельного подтверждения перед полным suite. Для текущей доработки выполнены
узкие tests, protocol tests, compile-check затронутых crates, `fix` и
release-fast сборка.

## Runtime, сборка и установка

Изменение затрагивает окружение дочерних CLI-процессов. Release-fast binary
собран на `f-ms-dev` 2026-06-17:

| Поле | Значение |
| --- | --- |
| Путь | `/home/slader/Projects/codex/codex-rs/target/release-fast/codex` |
| Размер | `303M` |
| Формат | `ELF 64-bit LSB pie executable, x86-64` |
| Strip-статус | `stripped` |
| BuildID | `702c38d8661e35aeecbab04a1e1a11eb10de7f2d` |
| Версия | `codex-cli 0.140.0+hermione` |

Бинарник не устанавливался локально в этом шаге. Если нужно проверить
установленный `codex-hermione`, используй обычный fork workflow: скопировать
remote artifact с `f-ms-dev` и установить в локальный путь только после явного
решения на установку.

Важно: `f-ms-dev` остается сборочным зеркалом. Разработка, ручные правки и
commit происходят в локальном checkout `/home/slader/Projects/evilcats/codex`.

## Риски и ограничения

- `CODEX_AGENT` не является стабильным машинным идентификатором; это
  человекочитаемое имя
  агента.
- Если два subagent имеют одинаковый `agent_role`, `CODEX_AGENT` не различает
  их; для уникального id нужно использовать `CODEX_THREAD_ID`.
- Если имя root или subagent недоступно, переменная не добавляется.
- Старые persisted root threads не получают задним числом agent name; это
  ограничение относится к `get_thread_info`, а `CODEX_AGENT` работает только
  для live-окружения CLI-команды.
- Скрипты не должны использовать `CODEX_AGENT` как границу безопасности: значение
  находится в обычном окружении процесса.
- Будущие изменения именования агентов должны обновлять общий helper, эту карточку и
  `docs/fork/core-thread-info-tool.md` вместе.

## Проверка покрытия

| Пункт | Статус |
| --- | --- |
| Root CLI получает имя root-агента | перенесено в карточку и реализовано через `current_agent_name` |
| Subagent CLI получает имя subagent | перенесено в карточку и реализовано через `current_agent_name` |
| `CODEX_AGENT` использует тот же контракт, что `get_thread_info.agent_name` | перенесено в карточку и реализовано через общий helper |
| `CODEX_AGENT` добавляется после фильтров shell env policy | перенесено в карточку и покрыто unit tests |
| Snapshot wrapper сохраняет `CODEX_AGENT` | перенесено в карточку и покрыто unit tests |
| Unified exec overlay считает `CODEX_AGENT` runtime-only изменением | перенесено в карточку и покрыто unit tests |
| `f-ms-dev` только для сборки и тестов, без разработки на mirror | перенесено в карточку |
| Проверки `fmt`, `test`, `fix`, `build-fast-release` | выполнено на `f-ms-dev`, результаты зафиксированы выше |
