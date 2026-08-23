---
id: fork-codex-agent-env-var
status: active
created: 2026-06-17
updated: 2026-08-22
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

## Карта файлов

| Файл | Ответственность |
| --- | --- |
| `codex-rs/core/src/agent/agent_name.rs` | Общий helper для вычисления имени текущего агента из `TurnContext`, root config/profile и subagent `SessionSource`; helper для сохраненных `StoredThread`-полей |
| `codex-rs/core/src/agent/agent_name_tests.rs` | Unit tests для root fallback и subagent fallback, ранее находившиеся рядом с `thread_info` |
| `codex-rs/core/src/agent/mod.rs` | Подключает модуль `agent_name` |
| `codex-rs/core/src/tools/handlers/thread_info.rs` | Использует общий helper, чтобы поле `agent_name` осталось единым с `CODEX_AGENT` |
| `codex-rs/core/src/tools/handlers/thread_info_tests.rs` | Оставляет tests разбора `thread_id`; tests helper-а перенесены к owner-модулю |
| `codex-rs/protocol/src/shell_environment.rs` | Добавляет константы `CODEX_AGENT_ENV_VAR`, `CODEX_CALL_ID_ENV_VAR`, `CODEX_ROLLOUT_ENV_VAR`; вводит `RuntimeEnv`; вставляет служебные runtime-переменные после shell env policy; сохраняет актуальные upstream-сигнатуры вспомогательных функций |
| `codex-rs/protocol/src/shell_environment_tests.rs` | Проверяет добавление runtime-переменных после `include_only` вместе с upstream-проверками очистки non-inheritable env |
| `codex-rs/core/src/exec_env.rs` | Экспортирует fork-константы runtime environment вместе с upstream `CODEX_SESSION_ID_ENV_VAR`; вводит core-level `RuntimeEnv` с `ThreadId`; конвертирует значения в protocol `RuntimeEnv` |
| `codex-rs/core/src/exec_env_tests.rs` | Проверяет, что runtime-переменные вставляются после фильтров policy и перезаписывают родительское окружение |
| `codex-rs/exec-server/src/local_process.rs` | Использует актуальную upstream-сигнатуру `shell_environment::create_env(...)` при построении policy env для локального процесса |
| `codex-rs/rmcp-client/src/stdio_server_launcher.rs` | Использует актуальную upstream-сигнатуру `shell_environment::create_env_from_vars(...)` в проверке remote exec policy |
| `codex-rs/core/src/tools/handlers/shell/shell_command.rs` | Передает имя агента, `call_id` и best-effort `rollout_path` в env для обычного `shell_command`, используя `ShellEnvironmentPolicy` выбранного `TurnEnvironment`; сохраняет upstream `CODEX_SESSION_ID` и apply-patch env |
| `codex-rs/core/src/tools/handlers/shell_tests.rs` | Проверяет expected env через `create_env_with_runtime(...)` |
| `codex-rs/core/src/tasks/user_shell.rs` | Передает `CODEX_AGENT`, UUID `CODEX_CALL_ID` и best-effort `CODEX_ROLLOUT` для пользовательского `/shell` task; использует `ShellEnvironmentPolicy` выбранного `TurnEnvironment`, тот же UUID как `CommandExecutionItem.id` и сохраняет upstream-проверку `cwd` на совместимость с host Codex через `to_abs_path()` |
| `codex-rs/core/src/tools/runtimes/mod.rs` | Восстанавливает fork-переменные и связанные upstream runtime-переменные после обертки shell snapshot |
| `codex-rs/core/src/tools/runtimes/mod_tests.rs` | Проверяет сохранение `CODEX_AGENT`, `CODEX_CALL_ID`, `CODEX_ROLLOUT` и `CODEX_THREAD_ID` после snapshot |
| `codex-rs/core/src/unified_exec/process_manager.rs` | Добавляет runtime-переменные в env unified exec sandbox session из `ShellEnvironmentPolicy` выбранного `TurnEnvironment`, но не в `local_policy_env`; сохраняет upstream session/apply-patch env |
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
- использует `ShellEnvironmentPolicy` выбранного `TurnEnvironment`;
- передает `RuntimeEnv` в `create_env_with_runtime(...)`;
- после этого добавляет upstream `CODEX_SESSION_ID` и apply-patch env;
- дочерний процесс получает доступные runtime-переменные.

`/shell` user task:

- `execute_user_shell_command(...)` получает live `Session` и `TurnContext`;
- вызывает `current_agent_name(turn_context.as_ref())`;
- заранее генерирует UUID `call_id`;
- получает best-effort `rollout_path` через `Session::hook_transcript_path()`;
- использует `ShellEnvironmentPolicy` выбранного `TurnEnvironment`;
- передает значения в `create_env_with_runtime(...)` вместе с
  `Session.thread_id`;
- после этого добавляет upstream `CODEX_SESSION_ID` и apply-patch env;
- snapshot preparation и proxy stripping работают поверх уже собранного env.

Unified exec sandbox session:

- базовое `local_policy_env` создается из policy выбранного `TurnEnvironment` без
  runtime-переменных;
- затем runtime env получает `CODEX_THREAD_ID` и `CODEX_CALL_ID`;
- затем, если `current_agent_name(...)` вернул имя, добавляется `CODEX_AGENT`;
- затем, если `Session::hook_transcript_path()` вернул путь, добавляется
  `CODEX_ROLLOUT`;
- затем добавляются upstream `CODEX_SESSION_ID` и apply-patch env;
- `ExecServerEnvConfig.local_policy_env` остается без этих runtime-переменных
  идентичности, чтобы exec-server overlay видел их как runtime-изменение.

Обертка shell snapshot:

- `maybe_wrap_shell_lc_with_snapshot(...)` получает explicit overrides и полный
  live env отдельно;
- после source snapshot обертка восстанавливает `CODEX_AGENT`,
  `CODEX_CALL_ID`, `CODEX_ROLLOUT`, `CODEX_THREAD_ID` и связанные upstream
  runtime-переменные из live env;
- эти переменные не считаются явными shell policy overrides.

### Примеры поведения

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

## Порядок повторения при переносе

При переносе на новый upstream:

1. Проверить, как upstream собирает shell env и где объявлен
   `CODEX_THREAD_ID`.
2. Добавить `CODEX_AGENT_ENV_VAR`, `CODEX_CALL_ID_ENV_VAR` и
   `CODEX_ROLLOUT_ENV_VAR` рядом с `CODEX_THREAD_ID_ENV_VAR` в protocol shell
   environment module.
3. Добавить или восстановить `RuntimeEnv` в protocol shell environment module.
4. Оставить `create_env`, `create_env_from_vars` и `populate_env` совместимыми с
   актуальными сигнатурами upstream `policy + thread_id`, а fork-значения
   передавать через варианты с `RuntimeEnv`.
5. Убедиться, что служебные runtime-переменные идентичности добавляются после
   `include_only`.
6. Вынести или восстановить общий helper имени агента:
   - root: `name` effective config, затем active profile;
   - subagent: `agent_role`, затем leaf `agent_path`, затем `agent_nickname`;
   - persisted fields: `agent_role`, затем leaf `agent_path`, затем
     `agent_nickname`.
7. Переключить `get_thread_info` на общий helper.
8. Передать `current_agent_name(...)`, `call_id` и best-effort `rollout_path` в
   shell command env и `/shell` user task. Для shell policy использовать
   выбранный `TurnEnvironment`, не общий `TurnContext.config`.
9. Для `/shell` генерировать UUID `call_id` до сборки env, а потом использовать
    тот же id как `CommandExecutionItem.id` в `ItemStarted`/`ItemCompleted` и
    производных legacy-событиях `ExecCommandBegin`/`ExecCommandEnd`, сохраняя
    upstream-проверку `turn_environment.cwd().to_abs_path()` перед подготовкой
    snapshot и env.
10. Для unified exec строить `local_policy_env` из выбранного
    `TurnEnvironment`, не класть туда runtime-переменные и добавлять их только в
    runtime env вместе с upstream session/apply-patch env.
11. Использовать `Session::hook_transcript_path()` для `CODEX_ROLLOUT`, чтобы
    отсутствие path не блокировало запуск команды.
12. Обновить обертку shell snapshot, чтобы она восстанавливала fork-набор и
    связанные upstream runtime-переменные, а отсутствующие live-значения не
    воскресали из snapshot.
13. Перенести tests для env policy, agent helper, shell snapshot и unified exec
    overlay.

## Проверки

Исполняемая карта card-level regression tests:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "runtime identity variables в окружении shell tool",
      "argv": ["just", "test", "-p", "codex-core", "exec_env"]
    },
    {
      "purpose": "единое разрешение имени root и subagent для runtime surfaces",
      "argv": ["just", "test", "-p", "codex-core", "agent_name"]
    },
    {
      "purpose": "metadata текущего и persisted thread, session tree и rollout path",
      "argv": ["just", "test", "-p", "codex-core", "thread_info"]
    },
    {
      "purpose": "shell snapshot восстанавливает runtime identity variables",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "maybe_wrap_shell_lc_with_snapshot_restores_codex_identity_from_env"
      ]
    },
    {
      "purpose": "unified exec хранит identity variables в runtime overlay",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "env_overlay_for_exec_server_keeps_runtime_changes_only"
      ]
    },
    {
      "purpose": "shell tool использует выбранное environment и передаёт стабильную runtime identity",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "shell_command_handler_to_exec_params_uses_selected_environment"
      ]
    },
    {
      "purpose": "shell environment policy сохраняет runtime identity variables",
      "argv": ["just", "test", "-p", "codex-protocol", "shell_environment"]
    },
    {
      "purpose": "exec-server проверяет актуальную сигнатуру shell environment policy",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-exec-server",
        "child_env_applies_policy_then_overlay"
      ]
    },
    {
      "purpose": "remote env policy в rmcp-client использует актуальную сигнатуру helper",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-rmcp-client",
        "remote_env_policy_effectively_filters_unrequested_vars"
      ]
    }
  ]
}
```

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
