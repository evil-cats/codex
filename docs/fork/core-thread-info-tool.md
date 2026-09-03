---
id: fork-core-thread-info-tool
status: active
created: 2026-06-16
updated: 2026-09-02
---

# Утилитарный core tool `get_thread_info`

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая добавляет
`get_thread_info`: встроенный core tool для чтения metadata текущего или
указанного Codex thread.

## Зачем это нужно

В fork Hermione есть устойчивый workflow, где агенту нужно быстро понять,
какой именно JSONL пишет текущий разговор или subagent. Раньше это приходилось
делать через косвенные признаки: rollout filename, `SessionConfiguredEvent`,
ручной поиск в `~/.codex/sessions/` или внешнюю память о текущей сессии.

Это было неудобно и легко смешивало два разных понятия: `thread_id` как
конкретный persisted thread, по которому однозначно ищется rollout JSONL, и
`session_id` как общий идентификатор root-agent session tree для root и
созданных внутри него subagents.

Для однозначной идентификации лога нужен именно `thread_id`. Новый tool делает
это явным model-facing контрактом и одновременно показывает `session_id`, чтобы
не терять связь subagent thread с общим деревом.

## Карта файлов

| Файл | Ответственность |
| --- | --- |
| `codex-rs/core/src/agent/agent_name.rs` | Общий helper для вычисления `agent_name`: текущий root через config/profile, текущий subagent через `SessionSource`, persisted thread через сохраненные поля |
| `codex-rs/core/src/agent/agent_name_tests.rs` | Unit tests для fallback-контракта `agent_name`: root config/profile, metadata текущего subagent и поля persisted thread |
| `codex-rs/core/src/agent/mod.rs` | Подключает общий модуль `agent_name` |
| `codex-rs/config/src/config_toml.rs` | Задаёт секцию `[tools.get_thread_info].enabled`, включённую по умолчанию |
| `codex-rs/core/src/config/mod.rs` | Разрешает итоговое значение настройки в `Config::get_thread_info_enabled` |
| `codex-rs/core/src/config/config_tests.rs` | Проверяет значение `true` по умолчанию и явное отключение `get_thread_info` через конфигурацию |
| `codex-rs/core/config.schema.json` | Содержит сгенерированную схему конфигурации для секции `[tools.get_thread_info]` |
| `codex-rs/core/src/tools/handlers/thread_info.rs` | Runtime-обработчик: разбор `thread_id`, чтение текущей или persisted thread metadata, materialize текущего rollout, использование общего helper-а `agent_name`, model-facing ошибки |
| `codex-rs/core/src/tools/handlers/thread_info_spec.rs` | Описание Responses API tool: имя, описание, input schema, output schema |
| `codex-rs/core/src/tools/handlers/thread_info_tests.rs` | Unit tests для parsing `thread_id` |
| `codex-rs/core/src/tools/handlers/thread_info_spec_tests.rs` | Unit tests для spec-контракта: optional `thread_id` и nullable output fields |
| `codex-rs/core/tests/suite/thread_info.rs` | Интеграционные тесты настоящего runtime-обработчика: текущий root, реально созданный subagent, архивный сохранённый thread, сериализация metadata и ограничение обхода parent chain при цикле, превышении глубины и нечитаемом родителе |
| `codex-rs/core/src/tools/handlers/mod.rs` | Подключает `thread_info` и `thread_info_spec`, экспортирует `ThreadInfoHandler` |
| `codex-rs/core/src/tools/spec_plan.rs` | Условно добавляет `ThreadInfoHandler` в `add_core_utility_tools(...)` по итоговому значению настройки |
| `codex-rs/core/src/tools/spec_plan_tests.rs` | Проверяет регистрацию по умолчанию и полное удаление `get_thread_info` из registry и model-visible spec при отключённой настройке |
| `codex-rs/core/src/tools/core_tool_activity.rs` | Отображает вызов `get_thread_info` в core tool activity как `kind = ThreadInfo` с кратким полем `detail` по текущему или указанному `thread_id` |
| `codex-rs/core/src/tools/core_tool_activity_tests.rs` | Проверяет, что `get_thread_info` остаётся видимым core tool activity с `kind = ThreadInfo` |
| `codex-rs/protocol/src/items.rs` | Содержит `CoreToolActivityKind::ThreadInfo` для `CoreToolActivityItem` |
| `codex-rs/app-server-protocol/src/protocol/v2/item.rs` | Отображает `CoreToolActivityKind::ThreadInfo` в v2 `ThreadItem::CoreToolActivity` без добавления отдельного RPC-метода |
| `codex-rs/app-server-protocol/schema/typescript/v2/CoreToolActivityKind.ts` | Содержит сгенерированное значение протокола `threadInfo`; JSON schema ответов и уведомлений включает тот же вариант через общий enum |
| `codex-rs/core/tests/suite/prompt_caching.rs` | Обновляет ожидаемый список prompt tools, чтобы cache-sensitive тест видел новый tool |
| `codex-rs/core/tests/suite/mod.rs` | Подключает integration suite `thread_info` |
| `codex-rs/tui/src/temporary_structured_request.rs` | Явно отключает `get_thread_info` во временном structured thread, которому нужен пустой набор tools |
| `codex-rs/thread-manager-sample/src/main.rs` | Сохраняет включённое по умолчанию значение в полном литерале `Config` |
| `docs/fork/core-thread-info-tool.md` | Владеющий handoff-артефакт: контракт, перенос, проверки и ограничения fork-доработки |
| `docs/fork/codex-agent-env-var.md` | Связанная fork-карточка runtime env: `CODEX_AGENT` использует тот же helper и тот же контракт `agent_name`; `CODEX_ROLLOUT` использует тот же live rollout path как best-effort env-подсказку |

Намеренно не менялись:

| Зона | Почему не меняется |
| --- | --- |
| `Cargo.toml` и `Cargo.lock` | Новые зависимости не нужны |
| Набор методов app-server RPC | Новые методы и поля payload не добавляются; существующий v2 `CoreToolActivityKind` только отражает новый вариант `threadInfo` |
| TUI | Отдельная UI-поверхность не нужна: tool доступен через core tool planning |
| Rollout filename format | Уже использует `ThreadId`; tool только раскрывает путь и id |
| Thread/session data model | Tool использует существующие `Session`, `ThreadStore`, `StoredThread` и `SessionSource` |
| `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` и `CODEX_SANDBOX_ENV_VAR` | Эти зоны запрещены локальным `AGENTS.md` и не относятся к metadata thread |

## Итоговый контракт

### Настройка регистрации

`get_thread_info` управляется штатной секцией:

```toml
[tools.get_thread_info]
enabled = false
```

Если секция или поле `enabled` отсутствуют, итоговое значение равно `true`:
обычные Responses requests и чувствительный к кэшированию набор tools сохраняют
`get_thread_info`. При `enabled = false` handler не регистрируется и tool
отсутствует как в registry, так и в model-visible spec.

Временный structured thread с закрытым набором возможностей явно передаёт
`tools.get_thread_info.enabled = false` вместе с остальными настройками
статических tools. Поэтому служебный structured request получает пустой список
`tools`, не меняя значение по умолчанию для обычных threads.

### Tool spec

`get_thread_info` регистрируется как `ToolSpec::Function`. `strict` остается
`false`, как у соседних core utility tools, но runtime-args разбираются через
`serde` с `deny_unknown_fields`.

Input schema содержит один необязательный параметр:

| Параметр | Тип | Встроенное значение | Контракт |
| --- | --- | --- | --- |
| `thread_id` | string | текущий `Session::thread_id()` | UUID конкретного thread; пустая строка является model-facing ошибкой. Описание параметра явно говорит, что `thread_id` нужно передавать для метаданных конкретного JSONL rollout |

Output schema содержит объект с required ключами:

| Поле | Тип | Контракт |
| --- | --- | --- |
| `thread_id` | string | Идентификатор сохраненного thread, который указывает на возвращенный rollout JSONL |
| `session_id` | string или null | Общий идентификатор root-agent session tree; равен `thread_id` для root session и может отличаться для subagent threads; `null`, если его нельзя определить |
| `rollout_path` | string или null | Локальный путь к JSONL rollout для этого thread, или `null`, если путь недоступен |
| `agent_name` | string или null | Имя роли агента для subagents или имя profile/config для root session; `null`, если имя недоступно |

Описание tool, видимое модели, должно явно объяснять агенту:

- tool возвращает `thread_id`, `session_id`, `rollout_path` и `agent_name`;
- `thread_id` идентифицирует сохраненный thread/rollout и является ключом к
  конкретному JSONL-логу;
- `session_id` идентифицирует общее root-agent session tree; для root session
  он равен `thread_id`, а для subagent threads может отличаться;
- `thread_id` используется, когда нужно посмотреть конкретный rollout;
- `session_id` используется, когда нужно сгруппировать связанные root и
  subagent threads.

### Runtime-разбор аргументов

`ThreadInfoArgs` имеет `#[serde(deny_unknown_fields)]`. Неизвестные ключи не
должны молча игнорироваться, потому что tool является introspection API и
ошибка в названии аргумента должна быть видна модели.

`thread_id`:

- если отсутствует, используется `Session::thread_id()` текущего invocation;
- перед parse обрезается через `trim()`;
- пустая строка после `trim()` возвращает `FunctionCallError::RespondToModel`;
- невалидный UUID возвращает `FunctionCallError::RespondToModel`;
- UUID текущего thread обслуживается через live `Session`;
- UUID другого thread читается через `SessionServices.thread_store.read_thread(...)`.

### Текущий thread

Для текущего thread handler сначала вызывает
`Session::try_ensure_rollout_materialized(PersistContext::Standard)`, затем
`Session::current_rollout_path()`. Это важно: вызов tool должен вернуть path к
уже материализованному rollout, если текущая session поддерживает локальное
persisted хранилище.

Ответ для текущего thread:

- `thread_id` = `Session::thread_id()`;
- `session_id` = `Session::session_id()`;
- `rollout_path` = результат `current_rollout_path()`;
- `agent_name` вычисляется через `codex-rs/core/src/agent/agent_name.rs`:
  - для subagent: из `SessionSource::SubAgent(ThreadSpawn.agent_role)`,
    затем leaf `agent_path`, затем `agent_nickname`;
  - для root: из top-level `name` merged config, затем active profile name.

### Другой persisted thread

Для явного `thread_id`, отличного от текущего, handler читает
`StoredThread` из `ThreadStore` с:

```rust
ReadThreadParams {
    thread_id,
    include_archived: true,
    include_history: false,
}
```

`include_archived: true` нужен, потому что tool является introspection helper,
а не пользовательским list/search API. Если UUID известен, archived thread тоже
должен быть читаем.

`include_history: false` важно для bounded context: tool не должен подтягивать
полную историю JSONL, если нужны только metadata.

Ответ для persisted thread:

- `thread_id` = `StoredThread.thread_id`;
- `rollout_path` = `StoredThread.rollout_path`;
- `agent_name` = `StoredThread.agent_role`, затем leaf `StoredThread.agent_path`,
  затем `StoredThread.agent_nickname`;
- `session_id` определяется обходом `parent_thread_id` к root thread и
  преобразованием root `ThreadId` в `SessionId`.

Обход parent chain ограничен `MAX_PARENT_CHAIN_DEPTH = 64` и возвращает `null`
при цикле, слишком глубокой цепочке или ошибке чтения родителя. Это защищает
tool от неограниченного чтения и поврежденной metadata.

### Примеры поведения

Значения UUID и paths ниже иллюстративные. Формы JSON и наборы полей являются
частью контракта.

### Текущий root thread

Вход:

```json
{}
```

Ответ:

```json
{
  "thread_id": "019b2345-1111-7222-9333-abcdefabcdef",
  "session_id": "019b2345-1111-7222-9333-abcdefabcdef",
  "rollout_path": "/home/slader/.codex/sessions/2026/06/16/rollout-2026-06-16T12-30-00-019b2345-1111-7222-9333-abcdefabcdef.jsonl",
  "agent_name": "Hermione"
}
```

### Текущий subagent thread

Вход:

```json
{}
```

Ответ:

```json
{
  "thread_id": "019b2345-aaaa-7222-9333-abcdefabcdef",
  "session_id": "019b2345-1111-7222-9333-abcdefabcdef",
  "rollout_path": "/home/slader/.codex/sessions/2026/06/16/rollout-2026-06-16T12-31-00-019b2345-aaaa-7222-9333-abcdefabcdef.jsonl",
  "agent_name": "Researcher"
}
```

### Пример другого persisted thread

Вход:

```json
{"thread_id":"019b2345-aaaa-7222-9333-abcdefabcdef"}
```

Ответ:

```json
{
  "thread_id": "019b2345-aaaa-7222-9333-abcdefabcdef",
  "session_id": "019b2345-1111-7222-9333-abcdefabcdef",
  "rollout_path": "/home/slader/.codex/sessions/2026/06/16/rollout-2026-06-16T12-31-00-019b2345-aaaa-7222-9333-abcdefabcdef.jsonl",
  "agent_name": "Researcher"
}
```

## Архитектурное решение

Tool размещен в `codex-core` рядом с `get_system_time`, потому что ему нужен
доступ к live `ToolInvocation.session`, `ToolInvocation.turn`,
`SessionServices.thread_store` и текущей config layer stack. Это introspection
tool текущего runtime, а не app-server API и не extension tool.

Ключевые решения:

- не читать rollout JSONL history для обычного ответа;
- не искать root profile `name` для произвольного старого root thread, потому
  что `StoredThread` не хранит effective config `name`;
- возвращать `null`, если metadata нельзя определить честно;
- для текущего root thread использовать live config, потому что именно она
  достоверно содержит active profile `name`;
- для subagents использовать `agent_role`, потому что parser agent TOML берет
  `name` и сохраняет его как role/name metadata;
- держать вычисление имени агента в общем helper-е
  `codex-rs/core/src/agent/agent_name.rs`, потому что `get_thread_info` и
  runtime-переменная `CODEX_AGENT` должны отвечать одинаково для текущего turn;
- управлять регистрацией через отдельную настройку, включённую по умолчанию,
  чтобы обычный набор tools сохранял introspection tool, а ограниченные
  временные threads могли явно исключить его без специальной ветки в registry;
- оставить строгий контракт ошибок rollout path в `get_thread_info`, но
  разрешить связанной runtime env переменной `CODEX_ROLLOUT` быть best-effort,
  чтобы отсутствие диагностического path не блокировало запуск CLI-команды.

## Порядок повторения при переносе

При переносе на новый upstream:

1. Проверить, как в новом upstream устроены `ToolExecutor`, `ToolInvocation`,
   `Session`, `TurnContext`, `ThreadStore`, `StoredThread`, `SessionSource`, и
   привести `ThreadInfoHandler::handle` к точной сигнатуре текущего
   `ToolExecutor`, явно повторив lifetime-параметр и ограничение
   `ToolInvocation: 'a`.
2. Перенести `thread_info.rs` и `thread_info_spec.rs` в owner-зону core tools.
3. Перенести общий helper `codex-rs/core/src/agent/agent_name.rs`, если он уже
   используется связанной runtime env доработкой.
4. Подключить modules/exports в `codex-rs/core/src/tools/handlers/mod.rs`.
5. Перенести `[tools.get_thread_info].enabled` со значением `true` по умолчанию,
   итоговое поле `Config::get_thread_info_enabled` и сгенерированную схему
   конфигурации.
6. Условно зарегистрировать `ThreadInfoHandler` по итоговому значению настройки в
   `add_core_utility_tools(...)` рядом с `SystemTimeHandler` или ближайшим
   актуальным core utility block.
7. Явно отключить настройку во временном structured thread с закрытым набором
   возможностей; не менять значение по умолчанию для обычных threads.
8. Перенести unit tests схемы и разбора аргументов, разрешения настройки и
   условной регистрации в registry, а также интеграционные тесты
   текущих root и subagent sessions, архивного сохранённого thread и повреждённых
   цепочек родителей.
9. Если upstream поменял model-visible prompt tool list tests, обновить
   соответствующие ожидаемые списки.
10. Сверить итоговый diff с контрактом: все описанные поля, ошибки, fallbacks и
   bounded parent-chain logic должны остаться на месте.

## Проверки

Исполняемая карта card-level regression tests:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "единое разрешение имени root и subagent для runtime surfaces",
      "argv": ["just", "test", "-p", "codex-core", "agent_name"]
    },
    {
      "purpose": "включённая по умолчанию настройка, условная регистрация, runtime-обработчик threads, строгие аргументы, схемы и ограниченный обход parent chain",
      "argv": ["just", "test", "-p", "codex-core", "thread_info"]
    },
    {
      "purpose": "get_thread_info отображается как core tool activity вида ThreadInfo",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "normalized_default_namespace_remains_visible"
      ]
    },
    {
      "purpose": "direct core tool остаётся доступным в cached tool set",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "prompt_tools_are_consistent_across_requests"
      ]
    },
    {
      "purpose": "временный recap request с закрытым набором возможностей не получает статические tools",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-tui",
        "recap_generation_uses_bounded_structured_request_and_inserts_result"
      ]
    }
  ]
}
```

## Риски и ограничения

- `rollout_path` может быть `null`, если thread store не является локальным
  filesystem-backed store или путь недоступен.
- `session_id` для другого persisted thread может быть `null`, если parent chain
  поврежден, слишком глубокий, циклический или родительский thread не читается.
- `agent_name` для произвольного старого root thread обычно `null`, потому что
  `StoredThread` не хранит top-level profile `name`.
- Для текущего root thread `agent_name` зависит от effective config на момент
  вызова tool.
- Явное `[tools.get_thread_info].enabled = false` полностью убирает tool из
  model-visible и runtime registry; это намеренно используется только там, где
  вызывающий workflow требует закрытого набора tools.
- Tool не является API для чтения истории, transcript или contents rollout.
- Tool не должен расширяться до unbounded scan/search без отдельного design
  review.
