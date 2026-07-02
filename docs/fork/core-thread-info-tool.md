---
id: fork-core-thread-info-tool
status: active
created: 2026-06-16
updated: 2026-06-19
source_scope: working-tree
---

# Утилитарный core tool `get_thread_info`

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая добавляет
`get_thread_info`: встроенный core tool для чтения metadata текущего или
указанного Codex thread.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Имя tool | `get_thread_info` |
| Crate | `codex-core` |
| Основной обработчик | `codex-rs/core/src/tools/handlers/thread_info.rs` |
| Описание tool | `codex-rs/core/src/tools/handlers/thread_info_spec.rs` |
| Регистрация | `codex-rs/core/src/tools/spec_plan.rs` |
| Аргумент | `thread_id?: string` |
| Default thread | текущий concrete thread текущей сессии |
| Основной идентификатор лога | `thread_id` |
| Путь к JSONL | поле ответа `rollout_path` |
| Имя агента | поле ответа `agent_name` |
| Общий helper имени агента | `codex-rs/core/src/agent/agent_name.rs` |
| Удаленный host сборки | `f-ms-dev:/home/slader/Projects/codex` |

Главное runtime-поведение:

- вызов `{}` возвращает metadata текущего concrete thread;
- `thread_id` в аргументе выбирает другой persisted thread из `ThreadStore`;
- ответ всегда содержит ключи `thread_id`, `session_id`, `rollout_path`,
  `agent_name`;
- `thread_id` обозначает конкретный thread и соответствует UUID в имени
  rollout-файла;
- `session_id` обозначает общий root-agent session tree;
- для текущего root thread `agent_name` берется из `name` effective config, а
  если его нет - из имени активного профиля;
- для subagent `agent_name` берется из role/name его agent config, сохраненного
  как `agent_role`, с fallback на leaf `agent_path` и затем `agent_nickname`.

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

## Согласованные решения

| Пункт | Итоговое решение | Причина |
| --- | --- | --- |
| Имя tool | `get_thread_info` | Tool возвращает metadata concrete thread, а не абстрактной session |
| Аргумент | `thread_id?: string` | Оmitted argument означает текущий concrete thread; явный UUID читает другой persisted thread |
| Основной id | `thread_id` | Rollout filename и `SessionMeta.id` используют `ThreadId`; это однозначный ключ для JSONL |
| `session_id` в ответе | Возвращать отдельным nullable полем | Он полезен для связи root и subagents, но не заменяет `thread_id` |
| Путь к JSONL | `rollout_path` | Это имя уже используется в protocol/thread-store и честно описывает local rollout |
| Root `agent_name` | Читать top-level `name` из effective config, затем active profile name | У профиля Hermione имя агента живет в config `name`; profile name является fallback |
| Subagent `agent_name` | Читать `agent_role`, затем leaf `agent_path`, затем `agent_nickname` | `name` из agent TOML превращается в role name и хранится в session/thread metadata |
| Не использовать `session_id` как filename key | Явно разделить `thread_id` и `session_id` | У subagents свои `thread_id` в общем `session_id`; filename должен находиться по `thread_id` |
| Remote build | На `f-ms-dev` только сборка и тесты, исходники правятся локально | `f-ms-dev` является сборочным зеркалом, а не местом разработки |

## Карта файлов

| Файл | Ответственность |
| --- | --- |
| `codex-rs/core/src/agent/agent_name.rs` | Общий helper для вычисления `agent_name`: текущий root через config/profile, текущий subagent через `SessionSource`, persisted thread через сохраненные поля |
| `codex-rs/core/src/agent/agent_name_tests.rs` | Unit tests для fallback-контракта `agent_name`: root config/profile, metadata текущего subagent и поля persisted thread |
| `codex-rs/core/src/tools/handlers/thread_info.rs` | Runtime-обработчик: разбор `thread_id`, чтение текущей или persisted thread metadata, materialize текущего rollout, использование общего helper-а `agent_name`, model-facing ошибки |
| `codex-rs/core/src/tools/handlers/thread_info_spec.rs` | Описание Responses API tool: имя, описание, input schema, output schema |
| `codex-rs/core/src/tools/handlers/thread_info_tests.rs` | Unit tests для parsing `thread_id` |
| `codex-rs/core/src/tools/handlers/thread_info_spec_tests.rs` | Unit tests для spec-контракта: optional `thread_id` и nullable output fields |
| `codex-rs/core/src/tools/handlers/mod.rs` | Подключает `thread_info` и `thread_info_spec`, экспортирует `ThreadInfoHandler` |
| `codex-rs/core/src/tools/spec_plan.rs` | Добавляет `ThreadInfoHandler` в `add_core_utility_tools(...)` рядом с `get_system_time` |
| `codex-rs/core/tests/suite/prompt_caching.rs` | Обновляет ожидаемый список prompt tools, чтобы cache-sensitive тест видел новый tool |
| `docs/fork/core-thread-info-tool.md` | Владеющий handoff-артефакт: контракт, перенос, проверки и ограничения fork-доработки |
| `docs/fork/codex-agent-env-var.md` | Связанная fork-карточка runtime env: `CODEX_AGENT` использует тот же helper и тот же контракт `agent_name`; `CODEX_ROLLOUT` использует тот же live rollout path как best-effort env-подсказку |

Намеренно не менялись:

| Зона | Почему не меняется |
| --- | --- |
| `Cargo.toml` и `Cargo.lock` | Новые зависимости не нужны |
| Config schema | Tool не добавляет config key и не требует пользовательской настройки |
| App-server protocol | Внешний app-server API не меняется |
| TUI | Отдельная UI-поверхность не нужна: tool доступен через core tool planning |
| Rollout filename format | Уже использует `ThreadId`; tool только раскрывает путь и id |
| Thread/session data model | Tool использует существующие `Session`, `ThreadStore`, `StoredThread` и `SessionSource` |
| `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` и `CODEX_SANDBOX_ENV_VAR` | Эти зоны запрещены локальным `AGENTS.md` и не относятся к metadata thread |

## Итоговый контракт

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
`Session::try_ensure_rollout_materialized()`, затем
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

## Примеры поведения

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
- оставить строгий контракт ошибок rollout path в `get_thread_info`, но
  разрешить связанной runtime env переменной `CODEX_ROLLOUT` быть best-effort,
  чтобы отсутствие диагностического path не блокировало запуск CLI-команды.

Отклоненные альтернативы:

| Альтернатива | Почему отклонена |
| --- | --- |
| Назвать tool `get_session_info` | Смешивает общий `session_id` дерева и конкретный `thread_id` JSONL |
| Принимать `session_id` как аргумент | Не позволяет однозначно выбрать subagent rollout внутри session tree |
| Возвращать только `thread_id` и path | Теряется связь с root-agent session tree |
| Возвращать только `session_id` | Нельзя однозначно найти rollout JSONL subagent |
| Читать полный JSONL history ради metadata | Лишняя цена и риск unbounded context; `StoredThread` уже содержит нужные поля |
| Выдумывать `agent_name` для старого root thread из текущего profile | Это может быть неверно, если thread был создан другим profile/config |

## Порядок повторения при переносе

При переносе на новый upstream:

1. Использовать project skill `fork` и эту карточку.
2. Проверить, как в новом upstream устроены `ToolExecutor`, `ToolInvocation`,
   `Session`, `TurnContext`, `ThreadStore`, `StoredThread`, `SessionSource`.
3. Перенести `thread_info.rs` и `thread_info_spec.rs` в owner-зону core tools.
4. Перенести общий helper `codex-rs/core/src/agent/agent_name.rs`, если он уже
   используется связанной runtime env доработкой.
5. Подключить modules/exports в `codex-rs/core/src/tools/handlers/mod.rs`.
6. Зарегистрировать `ThreadInfoHandler` в `add_core_utility_tools(...)` рядом с
   `SystemTimeHandler` или ближайшим актуальным core utility block.
7. Перенести tests для spec и runtime-контрактов helper-а.
8. Если upstream поменял model-visible prompt tool list tests, обновить
   соответствующие ожидаемые списки.
9. Запустить форматирование и проверки через skill-owned gates в текущем
   checkout.
10. Сверить diff с этой карточкой: все описанные поля, ошибки, fallbacks и
   bounded parent-chain logic должны остаться на месте.

## Проверки

Проверки должны запускаться на `f-ms-dev:/home/slader/Projects/codex`, потому
что локально в этом workflow исходники правятся, а Rust/Cargo/`just` сборка и
тесты выполняются на удаленной сборочной машине.

Запланированные проверки для этой доработки:

| Команда | Где запускать | Ожидаемый результат |
| --- | --- | --- |
| `just fmt` | `f-ms-dev`, `codex-rs/` | Форматирование применено; remote diff синхронизирован локально |
| `just test -p codex-core thread_info` | `f-ms-dev`, `codex-rs/` | Проходят unit tests `thread_info*` |
| `just test -p codex-core agent_name` | `f-ms-dev`, `codex-rs/` | Проходят unit tests общего helper-а `agent_name` |
| `just test -p codex-core prompt_tools_are_consistent_across_requests` | `f-ms-dev`, `codex-rs/` | Cache-sensitive prompt tool list остается согласованным |
| `just test -p codex-core` | `f-ms-dev`, `codex-rs/` | Проходит crate-level regression suite для `codex-core` |
| `just fix -p codex-core` | `f-ms-dev`, `codex-rs/` | Clippy/fix pass не оставляет обязательных исправлений |

Фактические результаты 2026-06-16:

| Команда | Результат |
| --- | --- |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just fmt` | прошла на `f-ms-dev`; remote-generated изменения синхронизированы локально |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core thread_info` | прошла: 10 tests run, 10 passed |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core prompt_tools_are_consistent_across_requests` | прошла: 1 test run, 1 passed |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core` | запускалась; итог: 2747 tests run, 2679 passed, 68 failed, 15 skipped |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just fix -p codex-core` | прошла; remote-generated изменения синхронизированы локально |

Первый запуск `just test -p codex-core thread_info` поймал compile error
`borrow of partially moved value: stored_thread`; ошибка исправлена до успешных
проверок.

Падения полного `just test -p codex-core` не связаны с `get_thread_info`:
ключевой общий симптом - remote sandbox failure
`bwrap: loopback: Failed RTM_NEWADDR: Operation not permitted`; также были
падения code-mode/MCP tests из-за missing `test_stdio_server` и timeouts.
Узкие проверки нового tool и cache-sensitive prompt tool list прошли.

Дополнительные фактические результаты 2026-06-17 после выделения общего helper-а
`codex-rs/core/src/agent/agent_name.rs`:

| Команда | Результат |
| --- | --- |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core agent_name` | прошла: 6 tests run, 6 passed |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core thread_info` | прошла: 5 tests run, 5 passed |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just test -p codex-core shell_command_handler_to_exec_params_uses_session_shell_and_turn_context` | прошла: 1 test run, 1 passed |
| `PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" just fix -p codex-core` | прошла за 1m05s |

Эти проверки подтверждают, что `get_thread_info.agent_name` продолжает
использовать прежний контракт через общий helper. Связанная доработка
runtime env и ее дополнительные проверки зафиксированы в
`docs/fork/codex-agent-env-var.md`.

Миграционная сверка 2026-06-19 после обновления upstream:

- вручную сверены файлы-владельцы выбранной карточки:
  `codex-rs/core/src/tools/handlers/thread_info.rs`,
  `codex-rs/core/src/tools/handlers/thread_info_spec.rs`,
  `codex-rs/core/src/agent/agent_name.rs`,
  `codex-rs/core/src/tools/handlers/mod.rs`,
  `codex-rs/core/src/tools/spec_plan.rs`,
  `codex-rs/core/tests/suite/prompt_caching.rs`;
- вручную сверены прямо используемые API текущей кодовой базы:
  `ReadThreadParams`, `StoredThread`, `ThreadStore`,
  `ToolInvocation`, `Session::thread_id()`, `Session::session_id()`,
  `Session::try_ensure_rollout_materialized()`,
  `Session::current_rollout_path()`, `SessionSource`,
  `SubAgentSource` и `AgentPath::name()`;
- подтверждено, что runtime-контракт `get_thread_info` остался на месте:
  текущий thread обслуживается через live `Session`, другой persisted thread
  читается через `ThreadStore`, `include_archived: true` и
  `include_history: false` сохранены, обход parent chain ограничен
  `MAX_PARENT_CHAIN_DEPTH = 64`;
- доработано тестовое покрытие `agent_name`: добавлены проверки fallback на
  `agent_nickname` для текущего subagent, приоритета `agent_role` для
  persisted thread и fallback на `agent_nickname` для persisted thread;
- команды форматирования, сборки, тестов, генераторов и `fix` в рамках этой
  миграционной сверки одной карточки не запускались по ограничению запуска.

## Runtime, сборка и установка

Эта доработка не требует отдельного install step. Если нужно проверить installed
binary, используй обычный fork workflow для remote release-fast build и
локальной установки, описанный в других fork-карточках. Для этой карточки
достаточно проверки `codex-core`, если не менялись сборочные профили,
установка или TUI.

## Риски и ограничения

- `rollout_path` может быть `null`, если thread store не является локальным
  filesystem-backed store или путь недоступен.
- `session_id` для другого persisted thread может быть `null`, если parent chain
  поврежден, слишком глубокий, циклический или родительский thread не читается.
- `agent_name` для произвольного старого root thread обычно `null`, потому что
  `StoredThread` не хранит top-level profile `name`.
- Для текущего root thread `agent_name` зависит от effective config на момент
  вызова tool.
- Tool не является API для чтения истории, transcript или contents rollout.
- Tool не должен расширяться до unbounded scan/search без отдельного design
  review.

## Проверка покрытия

| Пункт | Статус |
| --- | --- |
| Пользовательская цель: получить путь к JSONL текущей/указанной thread | перенесено в карточку |
| Переименование `session` в `thread` после уточнения модели | перенесено в карточку |
| Различие `thread_id` и `session_id` | перенесено в карточку |
| `thread_id` как однозначный ключ rollout filename | перенесено в карточку |
| Видимое модели описание различает `thread_id` для JSONL rollout и `session_id` для группировки дерева | перенесено в карточку и закреплено spec-тестом |
| `agent_name` из subagent config `name` через `agent_role` | перенесено в карточку |
| `agent_name` текущего root из profile/config `name` | перенесено в карточку |
| Общий helper `agent_name` для `get_thread_info` и `CODEX_AGENT` | перенесено в карточку и связанную карточку `codex-agent-env-var.md` |
| Связь `rollout_path` с best-effort `CODEX_ROLLOUT` | перенесено в связанную карточку `codex-agent-env-var.md` |
| Remote build только на `f-ms-dev`, без разработки на mirror | перенесено в карточку |
| Не читать полный JSONL history ради metadata | перенесено в карточку |
| Проверки `fmt`, `test`, `fix` | выполнены; полный `codex-core` suite запускался и упал на remote-инфраструктуре, подробности зафиксированы выше |
| Миграционная сверка после обновления upstream 2026-06-19 | ручная сверка выполнена; тестовое покрытие fallback-контракта `agent_name` усилено; команды проверок должен запустить основной агент |
