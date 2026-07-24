---
id: fork-codex-agent-env-var
status: active
created: 2026-06-17
updated: 2026-07-21
source_scope: working-tree
---

# Runtime-переменные окружения Codex

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая добавляет и обслуживает
runtime-переменные окружения для CLI-команд, запускаемых Codex:

- `CODEX_AGENT` - человекочитаемое имя текущего root-агента или subagent;
- `CODEX_CALL_ID` - логический id конкретного tool-вызова или пользовательской
  shell-команды;
- `CODEX_ROLLOUT` - best-effort путь к текущему rollout JSONL;
- `CODEX_THREAD_ID` - уже существующая связанная переменная текущего thread.

`CODEX_AGENT` заполняется тем же именем агента, которое `get_thread_info`
возвращает в поле `agent_name`. `CODEX_CALL_ID` совпадает с `call_id`, по
которому в rollout связаны модельный `FunctionCall`, `ItemStarted` и
`ItemCompleted` для `CommandExecutionItem`, производные legacy-события
`ExecCommandBegin` и `ExecCommandEnd`, а также `FunctionCallOutput`.
`CODEX_ROLLOUT` дает дочернему процессу путь к jsonl-файлу текущего thread, если
этот путь удалось получить без блокировки запуска команды.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Runtime-переменные | `CODEX_AGENT`, `CODEX_CALL_ID`, `CODEX_ROLLOUT` |
| Связанная существующая runtime-переменная | `CODEX_THREAD_ID` |
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
- если команда запускается из tool invocation, дочерний процесс получает
  `CODEX_CALL_ID=<call_id этого invocation>`;
- если пользовательская `/shell`-команда запускается вне модельного tool call,
  дочерний процесс получает UUID `CODEX_CALL_ID`, который затем используется
  как `CommandExecutionItem.id`; производные legacy-события
  `ExecCommandBegin` и `ExecCommandEnd` сохраняют тот же id;
- если текущий rollout удалось materialize/read, дочерний процесс получает
  `CODEX_ROLLOUT=<путь к jsonl>`;
- если rollout materialize/read не удался, команда все равно запускается, а
  `CODEX_ROLLOUT` просто не добавляется;
- имя вычисляется через общий helper, который также использует
  `get_thread_info`;
- runtime-переменные добавляются после `ShellEnvironmentPolicy.include_only`,
  `exclude` и `set`, как служебные переменные идентичности текущего запуска;
- runtime-переменные намеренно перезаписывают одноименные переменные из
  родительского окружения или пользовательской shell env policy, если значение
  текущего запуска известно;
- если имя агента честно определить нельзя, `CODEX_AGENT` не добавляется.

## Зачем это нужно

`CODEX_THREAD_ID` уже дает CLI-командам однозначный идентификатор текущего
thread. Этого достаточно, чтобы найти thread, но для практической диагностики
часто нужны еще три привязки:

- кто именно запустил команду: root-агент или конкретный subagent;
- какой конкретный tool-вызов или пользовательская shell-команда соответствует
  текущему процессу;
- в каком rollout JSONL искать записи этого запуска.

Hermione workflow использует несколько агентов и subagents. Для shell-скриптов,
диагностических команд, локальных helper-ов и будущих инструментов удобно иметь
простую runtime-подсказку:

- root-команда может видеть, что ее запустила `Hermione`;
- subagent-команда может видеть имя своей роли, например `Researcher`;
- дочерний скрипт может залогировать `CODEX_CALL_ID` и потом найти связанные
  строки через `rg '"call_id":"<id>"' "$CODEX_ROLLOUT"`;
- helper может сразу открыть текущий rollout-файл без отдельного вызова
  `get_thread_info`;
- внешние CLI helper-ы могут логировать агентную принадлежность без отдельного
  вызова `get_thread_info`;
- shell snapshots и unified exec не должны терять identity-переменные при
  восстановлении окружения.

Доработка не заменяет `get_thread_info`. Runtime-переменные дают короткие
подсказки дочернему процессу, а `get_thread_info` остается introspection tool
для `thread_id`, `session_id`, `rollout_path` и полного контракта `agent_name`.

## Согласованные решения

| Пункт | Итоговое решение | Причина |
| --- | --- | --- |
| Имя переменной | `CODEX_AGENT` | Короткое имя рядом с `CODEX_THREAD_ID`; значение описывает текущего агента, а не хранилище профиля |
| Id вызова | `CODEX_CALL_ID` | Пользователь выбрал короткое имя; значение является логическим ключом команды, а не номером строки rollout |
| Путь rollout | `CODEX_ROLLOUT` | Дочерний процесс может сразу искать текущий jsonl без отдельного tool call |
| Источник значения | Общий helper `current_agent_name(TurnContext)` | `CODEX_AGENT` и `get_thread_info.agent_name` не должны расходиться |
| Root-сессия | `name` из effective config, затем active profile name | Это тот же контракт, что у текущего root `get_thread_info` |
| Subagent | `agent_role`, затем leaf `agent_path`, затем `agent_nickname` | `name` из agent TOML сохраняется как role/name metadata |
| Неизвестное имя | Не добавлять `CODEX_AGENT` | Пустое или выдуманное имя хуже отсутствующей переменной |
| Ошибка получения rollout | Не добавлять `CODEX_ROLLOUT` и продолжать запуск | Переменная является удобной подсказкой, а не причиной блокировать CLI-команду |
| Shell policy | Добавлять после `include_only` и `set` | Runtime-идентичность должна переживать фильтры policy, как `CODEX_THREAD_ID` |
| Конфликт с родительским окружением | Перезаписывать runtime-значением | Дочерний процесс должен видеть текущего агента, а не устаревшее значение родительского процесса |
| Обертка shell snapshot | Восстанавливать весь набор runtime-переменных | Snapshot, подключенный через `source`, может перезаписать env; runtime-идентичность нужно вернуть после snapshot |
| Unified exec remote overlay | Считать runtime-переменные runtime-only изменением | Exec-server должен получить переменные как runtime-изменение, а не как базовое policy env |
| Remote build | На `f-ms-dev` только сборка и тесты, исходники правятся локально | `f-ms-dev` является сборочным зеркалом, а не местом разработки |

## Карта файлов

| Файл | Ответственность |
| --- | --- |
| `codex-rs/core/src/agent/agent_name.rs` | Общий helper для вычисления имени текущего агента из `TurnContext`, root config/profile и subagent `SessionSource`; helper для сохраненных `StoredThread`-полей |
| `codex-rs/core/src/agent/agent_name_tests.rs` | Unit tests для root fallback и subagent fallback, ранее находившиеся рядом с `thread_info` |
| `codex-rs/core/src/agent/mod.rs` | Подключает модуль `agent_name` |
| `codex-rs/core/src/tools/handlers/thread_info.rs` | Использует общий helper, чтобы поле `agent_name` осталось единым с `CODEX_AGENT` |
| `codex-rs/core/src/tools/handlers/thread_info_tests.rs` | Оставляет tests разбора `thread_id`; tests helper-а перенесены к owner-модулю |
| `codex-rs/protocol/src/shell_environment.rs` | Добавляет константы `CODEX_AGENT_ENV_VAR`, `CODEX_CALL_ID_ENV_VAR`, `CODEX_ROLLOUT_ENV_VAR`; вводит `RuntimeEnv`; вставляет служебные runtime-переменные после shell env policy |
| `codex-rs/core/src/exec_env.rs` | Экспортирует runtime env constants; вводит core-level `RuntimeEnv` с `ThreadId`; конвертирует значения в protocol `RuntimeEnv` |
| `codex-rs/core/src/exec_env_tests.rs` | Проверяет, что runtime-переменные вставляются после фильтров policy и перезаписывают родительское окружение |
| `codex-rs/core/src/tools/handlers/shell/shell_command.rs` | Передает имя агента, `call_id` и best-effort `rollout_path` в env для обычного `shell_command`, используя текущую `turn_context.config.permissions.shell_environment_policy` |
| `codex-rs/core/src/tools/handlers/shell_tests.rs` | Проверяет expected env через `create_env_with_runtime(...)` |
| `codex-rs/core/src/tasks/user_shell.rs` | Передает `CODEX_AGENT`, UUID `CODEX_CALL_ID` и best-effort `CODEX_ROLLOUT` для пользовательского `/shell` task; использует тот же UUID как `CommandExecutionItem.id` и сохраняет upstream-проверку `cwd` на совместимость с host Codex через `to_abs_path()` |
| `codex-rs/core/src/tools/runtimes/mod.rs` | Восстанавливает runtime-переменные после обертки shell snapshot |
| `codex-rs/core/src/tools/runtimes/mod_tests.rs` | Проверяет сохранение `CODEX_AGENT`, `CODEX_CALL_ID`, `CODEX_ROLLOUT` и `CODEX_THREAD_ID` после snapshot |
| `codex-rs/core/src/unified_exec/process_manager.rs` | Добавляет runtime-переменные в env unified exec sandbox session, но не в `local_policy_env` |
| `codex-rs/core/src/unified_exec/process_manager_tests.rs` | Проверяет, что exec-server overlay содержит runtime-переменные как runtime-изменение |
| `docs/fork/codex-agent-env-var.md` | Владеющий handoff-артефакт для runtime-переменных идентичности |
| `docs/fork/core-thread-info-tool.md` | Связанная карточка: фиксирует общий helper `agent_name` и прежний контракт tool |

Намеренно не менялись:

| Зона | Почему не меняется |
| --- | --- |
| Config schema | Runtime-переменные не добавляют config key; значения вычисляются из уже загруженной config/session metadata и текущего запуска |
| App-server protocol | Внешний app-server API не меняется |
| TUI | Переменная нужна дочерним CLI-командам, а не отдельной UI-поверхности |
| Rollout/thread data model | Используются существующие `TurnContext`, `SessionSource`, `ConfigLayerStack`, `StoredThread`-поля, `call_id` и live rollout path; отдельный номер строки rollout не вводится |
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

### Значение `CODEX_CALL_ID`

`CODEX_CALL_ID` содержит логический id текущего запуска команды:

- для обычного `shell_command` и unified `exec_command`: `ToolInvocation.call_id`;
- для пользовательской `/shell`-команды: UUID, который генерируется перед
  сборкой env и затем используется как `CommandExecutionItem.id` в
  `ItemStarted`/`ItemCompleted`; производные legacy-события
  `ExecCommandBegin`/`ExecCommandEnd` сохраняют тот же id;
- для shell snapshots: live значение восстанавливается после `source`, если
  snapshot-файл перезаписал переменную.

`CODEX_CALL_ID` не является номером строки rollout. Это более стабильный ключ:
по нему можно найти все связанные записи команды в jsonl, даже если файл
дописывается дальше.

### Значение `CODEX_ROLLOUT`

`CODEX_ROLLOUT` содержит путь к текущему rollout JSONL, если Codex смог получить
его без блокировки запуска команды.

Для live shell-команд путь берется через `Session::hook_transcript_path()`:

- метод best-effort materialize текущий rollout;
- если materialize или чтение пути не удалось, метод логирует предупреждение и
  возвращает `None`;
- при `None` переменная `CODEX_ROLLOUT` не добавляется;
- команда продолжает запускаться.

Это намеренно отличается от `get_thread_info`: introspection tool может
сообщать ошибку получения rollout path, а runtime env не должен ломать CLI
команду только из-за отсутствующей диагностической подсказки.

### Поведение shell env policy

`codex_protocol::shell_environment::populate_env_with_runtime(...)` строит
окружение так:

1. выбирает базовые переменные по `ShellEnvironmentPolicy.inherit`;
2. применяет default excludes, если они не отключены;
3. применяет custom `exclude`;
4. применяет `set`;
5. применяет `include_only`;
6. добавляет служебные runtime-переменные идентичности:
   - `CODEX_AGENT`, если имя агента передано;
   - `CODEX_CALL_ID`, если id вызова передан;
   - `CODEX_ROLLOUT`, если путь rollout передан;
   - `CODEX_THREAD_ID`, если thread id передан.

Следствие: runtime-переменные переживают `include_only` и перезаписывают
одноименные значения из inherited env или `set`.

### Runtime-точки вызова

`shell_command`:

- `ShellCommandHandler::to_exec_params(...)` получает `TurnContext`;
- вызывает `current_agent_name(turn_context)`;
- получает `call_id` из `ToolInvocation`;
- получает best-effort `rollout_path` через `Session::hook_transcript_path()`;
- передает `RuntimeEnv` в `create_env_with_runtime(...)`;
- дочерний процесс получает доступные runtime-переменные.

`/shell` user task:

- `execute_user_shell_command(...)` получает live `Session` и `TurnContext`;
- вызывает `current_agent_name(turn_context.as_ref())`;
- заранее генерирует UUID `call_id`;
- получает best-effort `rollout_path` через `Session::hook_transcript_path()`;
- передает значения в `create_env_with_runtime(...)` вместе с
  `Session.thread_id`;
- snapshot preparation и proxy stripping работают поверх уже собранного env.

Unified exec sandbox session:

- базовое `local_policy_env` создается без runtime-переменных;
- затем runtime env получает `CODEX_THREAD_ID` и `CODEX_CALL_ID`;
- затем, если `current_agent_name(...)` вернул имя, добавляется `CODEX_AGENT`;
- затем, если `Session::hook_transcript_path()` вернул путь, добавляется
  `CODEX_ROLLOUT`;
- `ExecServerEnvConfig.local_policy_env` остается без этих runtime-переменных
  идентичности, чтобы exec-server overlay видел их как runtime-изменение.

Обертка shell snapshot:

- `maybe_wrap_shell_lc_with_snapshot(...)` получает explicit overrides и полный
  live env отдельно;
- после source snapshot обертка восстанавливает `CODEX_AGENT`,
  `CODEX_CALL_ID`, `CODEX_ROLLOUT` и `CODEX_THREAD_ID` из live env;
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
CODEX_CALL_ID=call-1
CODEX_ROLLOUT=/tmp/rollout.jsonl
CODEX_THREAD_ID=thread-1
```

### Поиск текущей команды в rollout

Команда, запущенная из shell tool, может сохранить ключ:

```bash
printf '%s\n' "$CODEX_CALL_ID"
```

Если `CODEX_ROLLOUT` задан, связанные записи можно найти по `call_id`:

```bash
rg "\"call_id\":\"$CODEX_CALL_ID\"" "$CODEX_ROLLOUT"
```

Это находит логически связанные записи, а не конкретный номер строки. В одном
rollout обычно есть несколько записей с тем же `call_id`: модельный вызов tool,
`ItemStarted`/`ItemCompleted` с `CommandExecutionItem`, производные
`ExecCommandBegin`/`ExecCommandEnd` и output item.

### Отсутствующий rollout path не ломает команду

Если Codex не смог materialize/read текущий rollout path, дочерний процесс
получает остальные runtime-переменные, но не получает `CODEX_ROLLOUT`:

```text
CODEX_AGENT=Hermione
CODEX_CALL_ID=call-1
CODEX_THREAD_ID=thread-1
```

Это ожидаемое best-effort поведение, а не ошибка запуска команды.

## Архитектурное решение

Доработка разделяет ответственность по слоям:

- `codex-protocol` знает только имена runtime-переменных окружения и умеет
  вставить готовые строки после shell env policy;
- `codex-protocol::shell_environment::RuntimeEnv` группирует runtime-значения
  без раздувания сигнатур несколькими позиционными `Option`;
- `codex-core` знает live `TurnContext`, `Session`, `ToolInvocation.call_id` и
  вычисляет значения для runtime env;
- `get_thread_info` и shell env используют один helper, чтобы не расходиться;
- unified exec сохраняет различие между базой policy и runtime overlay.

Ключевое решение для `CODEX_AGENT` - не читать agent TOML заново при запуске
команды. На момент turn нужная идентичность уже находится в
`TurnContext.session_source` или effective config. Повторное чтение файлов было
бы менее надежным:

- agent config path может измениться после запуска subagent;
- persisted metadata уже содержит `agent_role`;
- root effective config уже загружен и отражает активный profile/config stack;
- shell command launch path не должен зависеть от дополнительного filesystem IO.

Ключевое решение для `CODEX_CALL_ID` - использовать уже существующий `call_id`,
а не вычислять номер строки rollout:

- `ResponseItem::FunctionCall` уже содержит `call_id`;
- `ToolInvocation` несет тот же `call_id` в tool handler;
- `CommandExecutionItem.id` содержит тот же `call_id`, а
  `EventMsg::as_legacy_events(...)` переносит его в `ExecCommandBegin` и
  `ExecCommandEnd`;
- output item тоже связан тем же `call_id`;
- строка jsonl является физической позицией append-only файла, а `call_id`
  является логическим ключом команды.

Ключевое решение для `CODEX_ROLLOUT` - использовать best-effort
`Session::hook_transcript_path()`, а не строгий путь ошибок из
`get_thread_info`.
Дочерняя CLI-команда не должна падать только потому, что диагностический путь к
rollout не удалось получить.

Отклоненные альтернативы:

| Альтернатива | Почему отклонена |
| --- | --- |
| Дублировать helper из `thread_info.rs` в env-коде | Контракты `get_thread_info.agent_name` и `CODEX_AGENT` могли бы разойтись |
| Всегда ставить `CODEX_AGENT=""` при неизвестном имени | Пустая переменная хуже отсутствующей: дочерний скрипт не может отличить неизвестность от намеренного пустого значения |
| Оставить `CODEX_AGENT` под контролем `ShellEnvironmentPolicy.include_only` | Runtime-идентичность должна быть доступна так же надежно, как `CODEX_THREAD_ID` |
| Считать `CODEX_AGENT` частью `local_policy_env` unified exec | Exec-server overlay потерял бы информацию, что переменная является runtime-изменением |
| Искать имя persisted root thread из текущего profile | Это неверно для старых root threads и не относится к live-окружению CLI-команды |
| Добавлять `CODEX_ROLLOUT_LINE` | Номер строки физически удобен человеку, но плохо подходит для live append-only файла и не является уже существующим runtime key |
| Называть ключ `CODEX_TOOL_CALL_ID` | Имя точнее для model tool calls, но пользователь выбрал более короткий `CODEX_CALL_ID`; `/shell` тоже получает id запуска, хотя это не model tool call |
| Делать отсутствие rollout path ошибкой запуска | Пользователь явно выбрал best-effort контракт: если path не получили, переменная отсутствует, а команда продолжает работать |
| Класть `CODEX_ROLLOUT` в `local_policy_env` unified exec | Exec-server потерял бы различие между policy env и runtime overlay текущего запуска |

## Порядок повторения при переносе

При переносе на новый upstream:

1. Использовать project skill `fork`, эту карточку и
   `docs/fork/core-thread-info-tool.md`.
2. Проверить, как upstream собирает shell env и где объявлен
   `CODEX_THREAD_ID`.
3. Добавить `CODEX_AGENT_ENV_VAR`, `CODEX_CALL_ID_ENV_VAR` и
   `CODEX_ROLLOUT_ENV_VAR` рядом с `CODEX_THREAD_ID_ENV_VAR` в protocol shell
   environment module.
4. Добавить или восстановить `RuntimeEnv` в protocol shell environment module.
5. Оставить совместимые `create_env`, `create_env_from_vars` и `populate_env`
   для старых точек вызова, но добавить варианты с `RuntimeEnv` для новых
   значений.
6. Убедиться, что служебные runtime-переменные идентичности добавляются после
   `include_only`.
7. Вынести или восстановить общий helper имени агента:
   - root: `name` effective config, затем active profile;
   - subagent: `agent_role`, затем leaf `agent_path`, затем `agent_nickname`;
   - persisted fields: `agent_role`, затем leaf `agent_path`, затем
     `agent_nickname`.
8. Переключить `get_thread_info` на общий helper.
9. Передать `current_agent_name(...)`, `call_id` и best-effort `rollout_path` в
   shell command env и `/shell` user task. На `rust-v0.142.5` не восстанавливать
   старый доступ через `turn_context.shell_environment_policy`; использовать
   `turn_context.config.permissions.shell_environment_policy`.
10. Для `/shell` генерировать UUID `call_id` до сборки env, а потом использовать
    тот же id как `CommandExecutionItem.id` в `ItemStarted`/`ItemCompleted` и
    производных legacy-событиях `ExecCommandBegin`/`ExecCommandEnd`, сохраняя
    upstream-проверку `turn_environment.cwd().to_abs_path()` перед подготовкой
    snapshot и env.
11. Для unified exec не класть runtime-переменные в `local_policy_env`;
    добавлять их только в runtime env.
12. Использовать `Session::hook_transcript_path()` для `CODEX_ROLLOUT`, чтобы
    отсутствие path не блокировало запуск команды.
13. Обновить обертку shell snapshot, чтобы она восстанавливала весь набор
    runtime-переменных.
14. Перенести tests для env policy, agent helper, shell snapshot и unified exec
    overlay.
15. Запустить форматирование и проверки на `f-ms-dev`, затем синхронизировать
    remote-generated изменения обратно в локальный checkout.

## Проверки

### Смысловое покрытие

Проверочное покрытие этой карточки должно подтверждать следующие контракты:

- `codex-protocol` объявляет `CODEX_AGENT_ENV_VAR`, `CODEX_CALL_ID_ENV_VAR`,
  `CODEX_ROLLOUT_ENV_VAR` рядом с `CODEX_THREAD_ID_ENV_VAR`.
- `RuntimeEnv` добавляет `CODEX_AGENT`, `CODEX_CALL_ID`, `CODEX_ROLLOUT` и
  `CODEX_THREAD_ID` после `ShellEnvironmentPolicy.include_only`, `exclude` и
  `set`.
- Runtime-переменные перезаписывают одноименные значения из inherited env и
  shell env policy, если значение текущего запуска известно.
- `CODEX_AGENT` использует тот же helper имени агента, что
  `get_thread_info.agent_name`.
- `shell_command` передает в env имя агента, `ToolInvocation.call_id`,
  `Session.thread_id` и best-effort `Session::hook_transcript_path()`.
- Пользовательская `/shell`-команда генерирует UUID `CODEX_CALL_ID` до сборки
  env и использует тот же id как `CommandExecutionItem.id` в
  `ItemStarted`/`ItemCompleted` и производных legacy-событиях
  `ExecCommandBegin`/`ExecCommandEnd`.
- Unified exec добавляет runtime-переменные поверх `local_policy_env`, чтобы
  exec-server overlay видел их как runtime-only изменение.
- Обертка shell snapshot восстанавливает `CODEX_AGENT`, `CODEX_CALL_ID`,
  `CODEX_ROLLOUT` и `CODEX_THREAD_ID` из live env после `source` snapshot.
- `CODEX_ROLLOUT` остается best-effort подсказкой: отсутствие пути не блокирует
  запуск CLI-команды.
- Config schema, app-server protocol, TUI и persisted data model не меняются
  этой доработкой.

### Владелец исполняемой карты

Проверки уровня карточки запускает skill-owned command `fork tests` для карточки
`fork-codex-agent-env-var`. Внутренние argv не являются runbook карточки: они
живут в блоке `fork-tests.v1` ниже как данные исполняемой карты, которые читает
`fork tests`.

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "exec environment",
      "argv": ["just", "test", "-p", "codex-core", "exec_env"]
    },
    {
      "purpose": "agent name",
      "argv": ["just", "test", "-p", "codex-core", "agent_name"]
    },
    {
      "purpose": "thread info",
      "argv": ["just", "test", "-p", "codex-core", "thread_info"]
    },
    {
      "purpose": "identity restore",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "maybe_wrap_shell_lc_with_snapshot_restores_codex_identity_from_env"
      ]
    },
    {
      "purpose": "exec env overlay",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "env_overlay_for_exec_server_keeps_runtime_changes_only"
      ]
    },
    {
      "purpose": "shell context",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "shell_command_handler_to_exec_params_uses_selected_environment"
      ]
    },
    {
      "purpose": "protocol shell env",
      "argv": ["just", "test", "-p", "codex-protocol", "shell_environment"]
    }
  ]
}
```

### Дополнительные gates

- `fork format` покрывает обязательное форматирование после кодовых изменений.
  Для текущей карточки этот gate нужен, когда перенос меняет Rust-код или
  сгенерированные артефакты, связанные с Rust.
- `fork build-fast` покрывает release-fast сборку, если общий проход должен
  подтвердить устанавливаемый fork-бинарник.
- `fork install` не является обязательным gate карточки; установка выполняется
  только после отдельного явного решения.
- `fork generators` для этой карточки `not-applicable`: доработка не вводит
  config key, app-server API, protocol schema fixture или другой сгенерированный
  артефакт.

### Исторические результаты

Исторические команды ниже сохранены как подтверждение старого удаленного workflow на
`f-ms-dev:/home/slader/Projects/codex`. Они не являются нормативным runbook
карточки; текущий запуск проверок должен идти через skill-owned commands.

Фактические результаты 2026-06-17:

| Команда | Результат |
| --- | --- |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just fmt` | прошла на `f-ms-dev`; rustfmt изменил `codex-rs/core/src/exec_env_tests.rs` и `codex-rs/protocol/src/shell_environment.rs`, изменения синхронизированы локально |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core exec_env` | прошла: 12 тестов запущены, 12 прошли |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core agent_name` | прошла: 6 тестов запущены, 6 прошли |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core thread_info` | прошла: 5 тестов запущены, 5 прошли |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core maybe_wrap_shell_lc_with_snapshot_restores_codex_identity_from_env` | прошла: 1 тест запущен, 1 прошел |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core env_overlay_for_exec_server_keeps_runtime_changes_only` | прошла: 1 тест запущен, 1 прошел |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-protocol shell_environment` | прошла: 2 теста запущены, 2 прошли |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core shell_command_handler_to_exec_params_uses_selected_environment` | прошла: 1 тест запущен, 1 прошел |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" cargo check -p codex-cli -p codex-app-server -p codex-rmcp-client -p codex-exec-server -p codex-linux-sandbox` | прошла за 2m00s; проверены crates с обновленными точками вызова `create_env(..., agent_name)` |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just fix -p codex-core` | прошла за 1m05s |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just fix -p codex-protocol` | прошла за 37.68s |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just build-fast-release` | прошла за 5m59s |

Фактические результаты 2026-06-18 для расширения `CODEX_CALL_ID` и
`CODEX_ROLLOUT`:

| Команда | Результат |
| --- | --- |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just fmt` | прошла на `f-ms-dev`; remote diff совпал с локальным diff |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core exec_env` | прошла: 12 тестов запущены, 12 прошли |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-protocol shell_environment` | прошла: 2 теста запущены, 2 прошли |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core shell_command_handler_to_exec_params_uses_selected_environment` | прошла: 1 тест запущен, 1 прошел |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core maybe_wrap_shell_lc_with_snapshot_restores_codex_identity_from_env` | прошла: 1 тест запущен, 1 прошел |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core env_overlay_for_exec_server_keeps_runtime_changes_only` | прошла: 1 тест запущен, 1 прошел |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core shell_command_handler_defaults_to_non_login_when_disallowed` | прошла: 1 тест запущен, 1 прошел |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-protocol` | прошла: 229 тестов запущены, 229 прошли |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core` | запуск выполнен; итог `2680 passed, 68 failed, 15 skipped`. Видимые причины падений относятся к окружению сборочной машины: `bwrap: loopback: Failed RTM_NEWADDR: Operation not permitted`, отсутствие `target/debug/test_stdio_server` для тестов stdio MCP и тайм-ауты в code-mode/network-denial сценариях |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just fix -p codex-core` | прошла за 1m59s |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just fix -p codex-protocol` | прошла за 14.75s |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just build-fast-release` | прошла за 5m53s |

Фактическая проверка 2026-06-19 после merge нового upstream-релиза:

| Область | Результат |
| --- | --- |
| `codex-rs/protocol/src/shell_environment.rs` | Константы `CODEX_AGENT_ENV_VAR`, `CODEX_CALL_ID_ENV_VAR`, `CODEX_ROLLOUT_ENV_VAR` и `CODEX_THREAD_ID_ENV_VAR` присутствуют; `RuntimeEnv` добавляет runtime-переменные после `include_only` |
| `codex-rs/core/src/exec_env.rs` | `RuntimeEnv` уровня core прокидывает `ThreadId`, `agent_name`, `call_id` и `rollout_path` в сборщик окружения protocol |
| `codex-rs/core/src/agent/agent_name.rs` и `codex-rs/core/src/tools/handlers/thread_info.rs` | `CODEX_AGENT` и `get_thread_info.agent_name` используют общий helper имени агента |
| `codex-rs/core/src/tools/handlers/shell/shell_command.rs` | `shell_command` передает `current_agent_name(...)`, `ToolInvocation.call_id`, `Session.thread_id` и best-effort `Session::hook_transcript_path()` в env |
| `codex-rs/core/src/tasks/user_shell.rs` | Пользовательская `/shell`-команда генерирует UUID `CODEX_CALL_ID` до сборки env и использует тот же id в `ExecCommandBegin`/`ExecCommandEnd` |
| `codex-rs/core/src/unified_exec/process_manager.rs` | Unified exec добавляет runtime-переменные поверх `local_policy_env`, не записывая их в базовый policy env |
| `codex-rs/core/src/tools/runtimes/mod.rs` | Обертка snapshot восстанавливает `CODEX_AGENT`, `CODEX_CALL_ID`, `CODEX_ROLLOUT` и `CODEX_THREAD_ID` из live-окружения после `source` snapshot |

Кодовых изменений по этой карточке после проверки 2026-06-19 не потребовалось.

Фактическая проверка 2026-07-05 после merge `rust-v0.142.5`:

| Область | Результат |
| --- | --- |
| `codex-rs/core/src/tools/handlers/shell/shell_command.rs` | Конфликт разрешен в пользу `create_env_with_runtime(...)` с `CODEX_AGENT`, `CODEX_CALL_ID`, `CODEX_ROLLOUT` и `CODEX_THREAD_ID`, но с актуальным upstream-источником policy: `turn_context.config.permissions.shell_environment_policy` |
| `codex-rs/core/src/tools/handlers/shell_tests.rs` | Ожидаемое окружение теста синхронизировано с `create_env_with_runtime(...)`, `ToolInvocation.call_id` и best-effort `rollout_path` |
| `codex-rs/core/src/tasks/user_shell.rs` | Конфликт разрешен с сохранением upstream-проверки `to_abs_path()` для `cwd`, совместимого с host Codex, и fork-контракта runtime env через UUID `CODEX_CALL_ID`, `current_agent_name(...)` и `Session::hook_transcript_path()` |

Проверочные команды, сборка, форматирование, генераторы и `fix` в one-card
запуске 2026-07-05 не выполнялись по skill-owned one-card правилу; общий
агент должен запустить нужные проверки отдельно.

Фактическая проверка 2026-07-08 после merge `rust-v0.143.0`:

| Область | Результат |
| --- | --- |
| `codex-rs/core/src/exec_env.rs` | Конфликт разрешен совмещением fork `RuntimeEnv` с upstream `CODEX_PERMISSION_PROFILE_ENV_VAR` и helper-ом `inject_permission_profile_env(...)` |
| `codex-rs/core/src/tools/handlers/shell/shell_command.rs` | `shell_command` сохраняет upstream `TurnEnvironment`/`cwd`, передает `CODEX_AGENT`, `CODEX_CALL_ID`, `CODEX_ROLLOUT` и `CODEX_THREAD_ID` через `create_env_with_runtime(...)`, затем добавляет runtime permission profile |
| `codex-rs/core/src/tools/handlers/shell_tests.rs` | Ожидаемое окружение синхронизировано с выбранным `TurnEnvironment`, runtime identity env и `CODEX_PERMISSION_PROFILE` |
| `codex-rs/core/src/tools/runtimes/mod.rs` | Snapshot wrapper восстанавливает `CODEX_AGENT`, `CODEX_CALL_ID`, `CODEX_ROLLOUT`, `CODEX_THREAD_ID` и upstream `CODEX_PERMISSION_PROFILE` из live env; отсутствующий permission profile остается unset |
| `codex-rs/core/src/unified_exec/process_manager.rs` | Unified exec добавляет runtime identity env и active permission profile поверх `local_policy_env`, не записывая runtime-only значения в базовый policy env |

Проверочные команды, сборка, форматирование, генераторы и `fix` в one-card
запуске 2026-07-08 не выполнялись по skill-owned one-card правилу; общий
агент должен запустить нужные проверки отдельно.

Фактическая проверка 2026-07-14 после merge `rust-v0.144.4`:

| Область | Результат |
| --- | --- |
| `codex-rs/protocol/src/shell_environment.rs` и `codex-rs/core/src/exec_env.rs` | `RuntimeEnv` по-прежнему добавляет `CODEX_AGENT`, `CODEX_CALL_ID`, `CODEX_ROLLOUT` и `CODEX_THREAD_ID` после shell env policy |
| `codex-rs/core/src/tasks/user_shell.rs` | Fork-контракт runtime env совмещен с новым upstream-представлением жизненного цикла через `CommandExecutionItem`: UUID создается до сборки env, используется как `CODEX_CALL_ID` и `CommandExecutionItem.id`, а производные legacy-события сохраняют тот же id |
| `codex-rs/core/src/tools/handlers/shell/shell_command.rs` | `shell_command` сохраняет `ToolInvocation.call_id`, общий helper имени агента, best-effort `rollout_path` и актуальный `TurnEnvironment` |
| `codex-rs/core/src/unified_exec/process_manager.rs` и `codex-rs/core/src/unified_exec/process_manager_tests.rs` | Runtime-переменные идентичности остаются overlay поверх `local_policy_env`; тесты проверяют `CODEX_AGENT`, `CODEX_CALL_ID`, `CODEX_ROLLOUT` и `CODEX_THREAD_ID` в runtime-окружении exec-server |
| `codex-rs/core/src/tools/runtimes/mod.rs` | Обертка shell snapshot восстанавливает полный набор runtime-переменных идентичности после `source` вместе с актуальным permission profile |

Кодовых изменений по этой карточке после проверки 2026-07-14 не потребовалось.
Карточка уточнена под новое upstream-представление жизненного цикла команд.

Проверочные команды, сборка, форматирование, генераторы и `fix` в one-card
запуске 2026-07-14 не выполнялись по skill-owned one-card правилу; общий
агент должен запустить нужные проверки отдельно.

Фактическая проверка 2026-07-18 после слияния `rust-v0.144.6`:

| Область | Результат |
| --- | --- |
| Изменения upstream `rust-v0.144.4..rust-v0.144.6` | Файлы из карты ответственности карточки не менялись; дополнительных конфликтов и адаптации к новому upstream не потребовалось |
| `codex-rs/protocol/src/shell_environment.rs` и `codex-rs/core/src/exec_env.rs` | `RuntimeEnv` сохраняет полный набор runtime-переменных идентичности и добавляет их после shell env policy |
| `codex-rs/core/src/agent/agent_name.rs` и `codex-rs/core/src/tools/handlers/thread_info.rs` | `CODEX_AGENT` и `get_thread_info.agent_name` по-прежнему используют общий helper имени агента |
| `codex-rs/core/src/tools/handlers/shell/shell_command.rs` и `codex-rs/core/src/tasks/user_shell.rs` | Инструмент `shell_command` и пользовательская `/shell`-команда сохраняют контракт `CODEX_AGENT`, `CODEX_CALL_ID`, `CODEX_ROLLOUT` и `CODEX_THREAD_ID`; UUID `/shell` остается идентификатором `CommandExecutionItem` |
| `codex-rs/core/src/unified_exec/process_manager.rs` и `codex-rs/core/src/tools/runtimes/mod.rs` | Unified exec сохраняет runtime-слой поверх `local_policy_env`, а обертка snapshot восстанавливает runtime-переменные после `source` |
| Тестовое покрытие карточки | Сохранились тесты shell env policy, helper имени агента, `shell_command`, восстановления snapshot и unified exec overlay; блок `fork-tests.v1` остается актуальным |

Кодовых изменений по этой карточке после проверки 2026-07-18 не потребовалось.

Проверочные команды, сборка, форматирование, генераторы и `fix` в one-card
запуске 2026-07-18 не выполнялись по skill-owned one-card правилу; общий
агент должен запустить нужные проверки отдельно.

Фактическая проверка 2026-07-21 после слияния `rust-v0.145.0`:

| Область | Результат |
| --- | --- |
| `codex-rs/protocol/src/shell_environment.rs` и `codex-rs/core/src/exec_env.rs` | Константы и `RuntimeEnv` сохраняют `CODEX_AGENT`, `CODEX_CALL_ID`, `CODEX_ROLLOUT` и `CODEX_THREAD_ID`; runtime-значения по-прежнему добавляются после shell env policy |
| `codex-rs/core/src/agent/agent_name.rs` и `codex-rs/core/src/tools/handlers/thread_info.rs` | Runtime env и `get_thread_info.agent_name` используют общий helper имени агента |
| `codex-rs/core/src/tools/handlers/shell/shell_command.rs` и `codex-rs/core/src/tasks/user_shell.rs` | Shell tool передает `ToolInvocation.call_id`, а `/shell` создает UUID до сборки env и использует его как `CommandExecutionItem.id`; обе точки сохраняют best-effort `rollout_path` |
| `codex-rs/core/src/unified_exec/process_manager.rs` и `codex-rs/core/src/tools/runtimes/mod.rs` | Unified exec сохраняет runtime overlay поверх `local_policy_env`, а snapshot wrapper восстанавливает полный набор runtime-переменных после `source` |
| Незавершенный конфликт в `codex-rs/core/src/unified_exec/process_manager.rs` | Конфликт относится к обработке `output_spill`, не затрагивает card-owned env overlay и намеренно оставлен для другой карточки |
| Тестовое покрытие карточки | Сохранились тесты protocol env policy, agent helper, shell tool, snapshot restore и unified exec overlay; исполняемая карта `fork-tests.v1` остается актуальной |

Кодовых изменений по этой карточке после проверки 2026-07-21 не потребовалось.

Проверочные команды, сборка, форматирование, генераторы и `fix` в one-card
запуске 2026-07-21 не выполнялись по skill-owned one-card правилу; общий
агент должен запустить нужные проверки отдельно.

### Известные падения и пропуски

- Полный запуск `codex-core` 2026-06-18 из исторических результатов завершился с
  итогом `2680 passed, 68 failed, 15 skipped`. Видимые причины падений
  относились к окружению сборочной машины:
  `bwrap: loopback: Failed RTM_NEWADDR: Operation not permitted`, отсутствие
  `target/debug/test_stdio_server` для тестов stdio MCP и тайм-ауты в
  code-mode/network-denial сценариях.
- Полный workspace-тестовый проход исторически не запускался: проектное правило
  требовало отдельного подтверждения перед полным набором. Для этой доработки
  были выполнены узкие тесты, полный `codex-protocol`, полный `codex-core` с
  окруженческими падениями, `fix` и release-fast сборка.
- Проверочные команды, сборка, форматирование, генераторы и `fix` в one-card
  запуске 2026-06-19 не выполнялись по skill-owned one-card правилу; основной
  агент должен был запустить нужные проверки отдельно.
- При текущем обновлении формата раздела `Проверки` проверки также не
  запускались по one-card правилу; общий проверочный проход должен подтвердить
  форму карточки и исполняемую карту через skill-owned workflow.

## Runtime, сборка и установка

Изменение затрагивает окружение дочерних CLI-процессов. Актуальный
release-fast binary собран на `f-ms-dev` 2026-06-18:

| Поле | Значение |
| --- | --- |
| Путь | `/home/slader/Projects/codex/codex-rs/target/release-fast/codex` |
| Размер | `303M` |
| Формат | `ELF 64-bit LSB pie executable, x86-64` |
| Strip-статус | `stripped` |
| BuildID | `43e82d6f27a6987bd6fa113242379ab2a3735076` |
| Версия | `codex-cli 0.140.0+hermione` |

Бинарник установлен локально 2026-06-18 после явного решения на установку:

| Поле | Значение |
| --- | --- |
| Локальный путь | `/home/slader/.local/bin/codex-hermione` |
| Источник установки | `/tmp/codex-hermione-release-fast`, скопированный с `f-ms-dev` |
| Размер | `303M` |
| Формат | `ELF 64-bit LSB pie executable, x86-64` |
| Strip-статус | `stripped` |
| BuildID | `43e82d6f27a6987bd6fa113242379ab2a3735076` |
| Версия | `codex-cli 0.140.0+hermione` |

Важно: `f-ms-dev` остается сборочным зеркалом. Разработка, ручные правки и
commit происходят в локальном checkout `/home/slader/Projects/evilcats/codex`.

## Риски и ограничения

- `CODEX_AGENT` не является стабильным машинным идентификатором; это
  человекочитаемое имя
  агента.
- Если два subagent имеют одинаковый `agent_role`, `CODEX_AGENT` не различает
  их; для уникального id нужно использовать `CODEX_THREAD_ID`.
- Если имя root или subagent недоступно, переменная не добавляется.
- `CODEX_CALL_ID` является id вызова команды, а не токеном безопасности; его
  нельзя использовать как границу доверия.
- Для `/shell` значение `CODEX_CALL_ID` не приходит от модели: это локальный
  UUID запуска пользовательской shell-команды.
- `CODEX_ROLLOUT` является best-effort переменной и может отсутствовать, если
  rollout не удалось materialize/read.
- `CODEX_ROLLOUT` указывает на live jsonl, который может дописываться после
  запуска команды; скрипты должны искать по `CODEX_CALL_ID`, а не полагаться на
  последний номер строки.
- Старые persisted root threads не получают задним числом agent name; это
  ограничение относится к `get_thread_info`, а `CODEX_AGENT` работает только
  для live-окружения CLI-команды.
- Скрипты не должны использовать runtime-переменные как границу безопасности:
  значения находятся в обычном окружении процесса.
- Будущие изменения именования агентов должны обновлять общий helper, эту
  карточку и `docs/fork/core-thread-info-tool.md` вместе.

## Проверка покрытия

| Пункт | Статус |
| --- | --- |
| Root CLI получает имя root-агента | перенесено в карточку и реализовано через `current_agent_name` |
| Subagent CLI получает имя subagent | перенесено в карточку и реализовано через `current_agent_name` |
| `CODEX_AGENT` использует тот же контракт, что `get_thread_info.agent_name` | перенесено в карточку и реализовано через общий helper |
| `CODEX_AGENT` добавляется после фильтров shell env policy | перенесено в карточку и покрыто unit tests |
| `CODEX_CALL_ID` добавляется для shell tool и `/shell` user task | перенесено в карточку и реализовано |
| `CODEX_CALL_ID` является ключом поиска в rollout, а не номером строки | перенесено в карточку |
| `CODEX_ROLLOUT` добавляется best-effort и не блокирует запуск команды | перенесено в карточку и реализовано через `hook_transcript_path()` |
| Snapshot wrapper сохраняет runtime-переменные | перенесено в карточку и покрыто unit tests |
| Unified exec overlay считает runtime-переменные runtime-only изменением | перенесено в карточку и покрыто unit tests |
| `f-ms-dev` только для сборки и тестов, без разработки на mirror | перенесено в карточку |
| Проверки `fmt`, `test`, `fix`, `build-fast-release` | для `CODEX_AGENT` выполнено на `f-ms-dev`; для `CODEX_CALL_ID` и `CODEX_ROLLOUT` выполнены targeted tests, полный `codex-protocol`, `fix` и release-fast build; полный `codex-core` запуск зафиксирован с окруженческими failures |
