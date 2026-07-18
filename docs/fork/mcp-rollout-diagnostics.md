---
id: fork-mcp-rollout-diagnostics
status: active
created: 2026-07-09
updated: 2026-07-18
source_scope: discussion-2026-07-09-mcp-transport-closed
---

# Rollout-диагностика падений MCP transport

## Обзор

Эта карточка фиксирует fork-доработку Hermione для диагностики и восстановления
MCP stdio transport после `Transport closed`. Codex должен сохранять
ограниченную диагностику времени выполнения в rollout сессии как служебный
`RolloutItem`, который не попадает в model context, но переживает resume и
позволяет разобрать, что сказал MCP-сервер перед закрытием транспорта.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Пользовательская цель | По `thread_id`, `server_name` и `call_id` восстановить причину `Transport closed` без зависимости от SQLite retention |
| Базовое допущение | MCP-вызовы считаются идемпотентными, поэтому retry после recovery допустим |
| Наблюдаемый runtime-симптом | `codebase-memory-mcp/search_graph` возвращает `Transport closed`, а stderr перед смертью не находится достоверно |
| Новый сохраняемый элемент | `RolloutItem::McpDiagnostic(McpDiagnosticItem)` |
| Главное model-visible правило | MCP stderr и recovery diagnostics не должны становиться `ResponseItem` или tool output |
| Связанная карточка | `docs/fork/mcp-stderr-thread-logs.md` |
| Не входит в границы задачи | Полное перепроектирование `rollout-trace`, перенос всей SQLite log DB в rollout, unbounded stdout/stderr capture |

`docs/fork/mcp-stderr-thread-logs.md` владеет thread-attributed stderr в
tracing/SQLite log DB. Эта карточка владеет другим уровнем: служебная
диагностика должна сохраняться рядом с rollout сессии, чтобы ошибка MCP
оставалась расследуемой даже если log DB уже вытеснила нужные строки.

## Зачем это нужно

В live-сессии `019f4317-0598-74c3-8f0a-38593b0e5e8a` был пойман свежий сбой:

```text
tool call failed for `codebase-memory-mcp/search_graph`

Caused by:
    Transport closed
```

SQLite log DB сохранила строку tool-call error, но в этом же `thread_id` не было
строк `MCP server stderr`. После сбоя `list_all_tools` продолжал показывать
`codebase-memory-mcp tool_count=14`, что не доказывает живость MCP-процесса:
менеджер может отдавать сохраненный список tools, пока следующий реальный
запрос снова не ударится в закрытый transport.

Нужная диагностика должна отвечать без внешней стенограммы:

- какой `thread_id` и `turn_id` выполняли MCP-вызов;
- какой `call_id`, `server_name`, `tool_name` и `launch_id` связаны со сбоем;
- какое событие transport произошло: `transport_closed`, `broken_pipe`,
  `process_exited`, `recovery_started`, `recovery_succeeded` или
  `recovery_failed`;
- какой bounded stderr-tail был накоплен перед сбоем;
- был ли выполнен restart/reinitialize;
- был ли исходный MCP-вызов повторен и чем закончился повтор;
- почему diagnostic item не попал в model context.

## Согласованные решения

| Пункт | Решение | Причина |
| --- | --- | --- |
| Идемпотентность MCP | Считать MCP-вызовы идемпотентными | После recovery можно безопасно повторить исходную operation один раз |
| Место записи diagnostics | Писать bounded diagnostics в основной rollout сессии как служебный item | Rollout переживает resume и уже содержит служебные элементы, не попадающие в model context |
| Форма записи | Добавить новый `RolloutItem::McpDiagnostic` | Не смешивать диагностику времени выполнения с UI/protocol `EventMsg` и model-visible `ResponseItem` |
| Stderr payload | Писать не весь поток, а bounded tail при значимом событии | Не превращать rollout в unbounded log и сохранить последние строки перед смертью |
| Transport recovery | При `TransportClosed`/`BrokenPipe` reinitialize transport и retry operation один раз | Recovery должен жить на уровне `RmcpClient`, где есть `transport_recipe` и `session_recovery_lock` |
| Потеря child process | `process_exited` помечает текущий `launch_id` непригодным; следующий реальный MCP-вызов делает recovery до отправки запроса | Смерть процесса тоже должна вести в общий recovery path, но без фонового crash loop |
| Child liveness | Использовать process liveness только как дополнительный сигнал | `process alive` слабее, чем `MCP usable`; transport может быть закрыт при живом процессе |
| Rollout trace | Не делать `rollout-trace` обязательным для MVP | Для этой задачи достаточно служебного rollout item; trace bundle может быть будущим усилением |

## Карта файлов

| Файл | Роль в будущей правке |
| --- | --- |
| `codex-rs/protocol/src/protocol.rs` | Добавить `RolloutItem::McpDiagnostic` и типы `McpDiagnosticItem`/`McpDiagnosticEvent` |
| `codex-rs/core/src/session/rollout_reconstruction.rs` | Явно игнорировать `McpDiagnostic` при восстановлении model history и world-state replay |
| `codex-rs/core/src/mcp_tool_call.rs` | Связать `call_id`, `thread_id`, `turn_id`, `server_name`, `tool_name` и diagnostic persistence для tool-call failure |
| `codex-rs/codex-mcp/src/connection_manager.rs` | Сохранить server-level context и не терять `server_name` при маршрутизации `call_tool` |
| `codex-rs/rmcp-client/src/rmcp_client.rs` | Обнаруживать closed transport или непригодный launch, reinitialize через `transport_recipe`, retry operation один раз и эмитить recovery diagnostics |
| `codex-rs/rmcp-client/src/stdio_server_launcher.rs` | Накапливать bounded stderr tail per stdio launch, связывать его с `launch_id` и передавать lifecycle signal о завершении процесса |
| `codex-rs/rmcp-client/src/executor_process_transport.rs` | Для executor stdio использовать lifecycle events `Exited`/`Closed`/`Failed` и stderr chunks как источник diagnostics и dead-launch signal |
| `codex-rs/tools/src/tool_output.rs` | Не менять model-visible MCP output format ради stderr diagnostics |
| `docs/fork/mcp-stderr-thread-logs.md` | Связанная карточка для SQLite/tracing stderr attribution; не является owner recovery rollout item |
| `docs/fork/mcp-rollout-diagnostics.md` | Владеющий handoff-артефакт этой fork-доработки |

Намеренно не менять в рамках MVP:

| Зона | Почему не менять |
| --- | --- |
| MCP stdout protocol stream | stdout содержит JSON-RPC protocol data и tool results; raw stdout не должен попадать в diagnostics без отдельного дизайна приватности |
| Полный `rollout-trace` reducer | Текущий MVP решает расследуемость через основной rollout; trace graph можно расширить позже |
| SQLite retention policy | Новая диагностика не должна зависеть от того, успела ли log DB вытеснить stderr rows |
| Model-visible tool output | Stderr процесса не является результатом MCP tool call |

## Итоговый контракт

### Сохраняемый rollout item

Добавить служебный persisted item, который сериализуется в rollout JSONL, но не
материализуется в model context:

```rust
RolloutItem::McpDiagnostic(McpDiagnosticItem)
```

Минимальные поля:

```rust
pub struct McpDiagnosticItem {
    pub thread_id: String,
    pub turn_id: Option<String>,
    pub call_id: Option<String>,
    pub server_name: String,
    pub tool_name: Option<String>,
    pub launch_id: String,
    pub old_launch_id: Option<String>,
    pub new_launch_id: Option<String>,
    pub event: McpDiagnosticEvent,
    pub timestamp_ms: i64,
    pub error: Option<String>,
    pub stderr_tail: Option<String>,
    pub stderr_truncated: bool,
}
```

`timestamp_ms` нужен для человеческой диагностики и сортировки событий внутри
одного rollout. Он не должен участвовать в model context.

Реализация хранит `old_launch_id` и `new_launch_id` отдельными optional-полями.
Это оставляет `launch_id` локальным для события и упрощает разбор перехода между
старым и новым stdio launch при recovery.

### Диагностические события

События MVP:

| Event | Когда писать | Обязательные связи |
| --- | --- | --- |
| `process_started` | MCP stdio launch создан | `thread_id`, `server_name`, `launch_id` |
| `process_exited` | child process или executor process сообщил terminal state; текущий launch помечается непригодным | `thread_id`, `server_name`, `launch_id`, `stderr_tail` |
| `transport_closed` | rmcp вернул `ServiceError::TransportClosed` | `thread_id`, `server_name`, `launch_id`, `call_id`, `tool_name`, `stderr_tail` |
| `transport_broken_pipe` | send/write вернул broken pipe, `stdin closed` или `unknown process` | те же поля, что и `transport_closed` |
| `recovery_started` | Codex начал reinitialize после transport failure или перед operation для непригодного launch | `thread_id`, `server_name`, old `launch_id`, `call_id` если есть |
| `recovery_succeeded` | Новый transport инициализирован и operation будет повторена | old/new `launch_id`, `call_id` |
| `recovery_failed` | Reinitialize или replay завершился ошибкой | old/new `launch_id` если есть, `error`, `stderr_tail` |

`old_launch_id` и `new_launch_id` заполняются для recovery-событий, где нужно
связать старый launch с новым. Для событий одного launch достаточно
`launch_id`.

### Ограниченный stderr tail

Stderr MCP-процесса нужно держать в bounded ring buffer per `launch_id`.

Требования:

- tail должен включать последние полные строки stderr;
- partial line при закрытии процесса должен быть сброшен в tail;
- размер tail должен иметь жесткий лимит, например в bytes или chars;
- при превышении лимита выставлять `stderr_truncated = true`;
- diagnostics должны хранить tail только при значимых событиях, а не каждую
  строку stderr отдельным `RolloutItem`.

### Восстановление и повтор

При закрытии transport во время operation:

1. `RmcpClient` обнаруживает `ServiceError::TransportClosed` или retryable
   broken-pipe transport send error.
2. Записывает `McpDiagnosticEvent::TransportClosed` или
   `McpDiagnosticEvent::TransportBrokenPipe` в rollout.
3. Под `session_recovery_lock` проверяет, что текущий `service` всё еще тот же
   failed service.
4. Создает новый transport через сохраненный `transport_recipe`.
5. Выполняет initialize с сохраненным `initialize_context`.
6. Заменяет `ClientState::Ready`.
7. Записывает `recovery_succeeded` или `recovery_failed`.
8. Повторяет исходную operation один раз.

При потере child process без активной operation:

1. Process lifecycle watcher получает `process_exited`, `process_failed` или
   `process_closed`.
2. Codex записывает `McpDiagnosticEvent::ProcessExited` с `thread_id`,
   `server_name`, `launch_id` и stderr tail.
3. Текущий `launch_id` помечается непригодным для новых backend operations.
4. Codex не запускает бесконечный фоновый restart-loop сразу после смерти
   процесса.
5. Следующая реальная MCP operation перед отправкой запроса видит непригодный
   launch, входит в тот же recovery helper, создает новый transport, выполняет
   initialize и затем выполняет operation.

Если child process умер во время in-flight operation, итог должен сходиться с
веткой закрытого transport: текущая operation получает общий recovery/retry, а
diagnostics связывают `process_exited`, `transport_closed` если он произошел, и
последующий `recovery_started`.

Поскольку MCP-вызовы считаются идемпотентными, retry допускается и для
`tools/call`, а не только для `tools/list` или resource reads. Бесконечный retry
запрещен: если повтор исходной operation после успешного recovery снова
завершается `TransportClosed`, Codex записывает ровно один
`McpDiagnosticEvent::RecoveryFailed`, возвращает tool error наружу и не запускает
вторую recovery loop.

### Model-visible поведение

`McpDiagnostic` не должен становиться:

- `ResponseItem`;
- MCP `CallToolResult`;
- function/tool output для модели;
- TUI history item, если UI явно не добавит отдельный diagnostic viewer.

При resume `rollout_reconstruction` обязан игнорировать `McpDiagnostic` для
`ContextManager::record_items(...)`. Это аналогично уже существующему поведению,
где служебные `EventMsg(_)`, `TurnContext(_)`, `WorldState(_)` и `SessionMeta(_)`
не материализуются как model-visible history.

## Архитектурное решение

### Почему rollout сессии подходит

Основной rollout уже хранит не только model-visible items. В
`RolloutItem` существуют служебные варианты `SessionMeta`, `TurnContext`,
`WorldState`, `Compacted`, `EventMsg` и `InterAgentCommunicationMetadata`.
`rollout_reconstruction` использует часть из них для resume metadata и
игнорирует остальное при построении model history.

Поэтому правильная граница не "stderr нельзя писать в rollout", а:

- нельзя писать stderr как `ResponseItem` или tool output;
- можно писать ограниченную диагностику времени выполнения как отдельный
  служебный `RolloutItem`;
- reconstruction должен явно доказать, что этот item не попадает в model
  context.

### Почему не только child process liveness

Проверка `process alive` полезна как ранний signal, но не доказывает `MCP
usable`. Процесс может быть жив, но закрыть stdout, перестать читать stdin,
зависнуть, писать невалидный JSON-RPC или умереть сразу после проверки.

Поэтому recovery должен учитывать и ошибку operation, и lifecycle-состояние:

```text
operation fails with closed transport
  -> persist diagnostic
  -> reinitialize transport
  -> replay idempotent operation once

process exits while idle
  -> persist diagnostic
  -> mark launch dead
  -> next operation reinitializes before request send
```

Process lifecycle watcher нужен для лучшего `process_exited` diagnostic item,
и для ранней invalidation текущего launch, но не должен быть единственным
механизмом восстановления.

### Почему не `EventMsg`

`EventMsg` связан с protocol/UI event stream. MCP diagnostics являются persisted
runtime evidence, а не пользовательским событием по умолчанию. Новый
`RolloutItem::McpDiagnostic` точнее задает owner semantics и уменьшает риск
случайно показать stderr пользователю или модели.

## Порядок повторения при переносе

1. Проверить текущий `RolloutItem` enum: если upstream уже имеет service
   diagnostic item, переиспользовать или расширить его вместо нового варианта.
2. Добавить `McpDiagnosticItem` и `McpDiagnosticEvent` в protocol crate.
3. Обновить `rollout_reconstruction`: новый item должен быть явно игнорируемым
   при model history replay и world-state replay.
4. Добавить helper для записи diagnostics из MCP runtime к
   `Session::persist_rollout_items` или существующему session-owned приемнику,
   не протаскивая `Session` глубоко в transport layer без владельца.
5. Ввести `launch_id` для stdio MCP launch и bounded stderr tail owner.
6. В `rmcp_client.rs` расширить `run_service_operation` recovery path:
   `TransportClosed`, broken pipe и непригодный launch должны идти через единый
   reinitialize helper.
7. Добавить rollout diagnostics на `transport_closed`, `process_exited`,
   `recovery_started`, `recovery_succeeded` и `recovery_failed`.
8. Сохранить существующее model-visible MCP output поведение.
9. Синхронизировать `fork-tests.v1` с фактическими tests после реализации.

## Проверки

### Смысловое покрытие

| Контракт | Обязательность | Где должно покрываться |
| --- | --- | --- |
| `McpDiagnostic` сериализуется в rollout и десериализуется при resume | `required` | protocol/rollout round-trip test |
| `McpDiagnostic` не попадает в model history после reconstruction | `required` | core rollout reconstruction test |
| `TransportClosed` persist-ит diagnostic item с `thread_id`, `server_name`, `call_id`, `launch_id` и stderr tail | `required` | core/rmcp integration test |
| Closed transport вызывает reinitialize и one-shot retry для idempotent `tools/call` | `required` | rmcp-client или core suite test |
| `process_exited` помечает launch dead, а следующий MCP-вызов выполняет recovery до send | `required` | rmcp-client или core suite lifecycle test |
| Повторный `TransportClosed` после replay записывает ровно один `RecoveryFailed` и не запускает вторую recovery loop | `required` | failure-path test с проверкой количества `RecoveryFailed` и `RecoveryStarted` |
| Bounded stderr tail flush-ит partial line при process close | `required` | rmcp-client unit/integration test |
| Process liveness не является единственным recovery условием | `required` для review | source audit recovery path |
| MCP stderr diagnostics не становятся `ResponseItem`/tool output | `required` | reconstruction and output tests |

### Владелец исполняемой карты

Проверки уровня карточки запускает skill-owned command `fork tests`. Карточка
хранит машиночитаемый блок `fork-tests.v1`; внутренние `argv` ниже являются
данными для `fork tests`, а не пользовательским runbook прямого запуска.

Первый шаг исполняемой карты собирает тестовый бинарник `test_stdio_server` из
пакета `codex-rmcp-client`. Это обязательное предусловие для последующих тестов
`codex-core`: запуск с фильтром пакета `codex-core` не собирает бинарник
другого пакета, а `cargo_bin("test_stdio_server")` ожидает резервный путь
`codex-rs/target/debug/test_stdio_server`, если Cargo не передал
`CARGO_BIN_EXE_test_stdio_server`.

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "mcp stdio fixture binary",
      "argv": [
        "cargo",
        "build",
        "--manifest-path",
        "codex-rs/Cargo.toml",
        "-p",
        "codex-rmcp-client",
        "--bin",
        "test_stdio_server"
      ]
    },
    {
      "purpose": "mcp rollout diagnostic reconstruction",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "mcp_diagnostic_rollout_item_is_not_replayed_to_model_history"
      ]
    },
    {
      "purpose": "mcp transport closed recovery and diagnostic",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "mcp_transport_closed_persists_diagnostic_and_retries_once"
      ]
    },
    {
      "purpose": "mcp transport replay failure stops after one retry",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "mcp_transport_closed_replay_failure_is_not_retried_again"
      ]
    },
    {
      "purpose": "mcp process exit lazy recovery",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "mcp_process_exit_marks_launch_dead_and_recovers_on_next_call"
      ]
    },
    {
      "purpose": "rmcp stderr tail and process lifecycle",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-rmcp-client",
        "mcp_stderr_tail_flushes_on_transport_close"
      ]
    }
  ]
}
```

### Дополнительные gates

| Gate | Когда нужен | Статус |
| --- | --- | --- |
| Проверка формы fork-карточек | После добавления или изменения этой карточки | `required` |
| Печать исполняемой карты card-level проверок | После добавления active card-level test map | `required` |
| Форматирование Rust-кода | После Rust-кодовых правок | `required` |
| Генераторы schema/TS artifacts | Если `RolloutItem` schema/TS artifacts требуют regeneration | `manual-required` после реализации |
| Быстрая fork-сборка | После реализации recovery path | `manual-required` в общем fork-проходе |

### Исторические результаты

| Проверка или источник | Результат | Примечание |
| --- | --- | --- |
| SQLite check по live thread `019f4317-0598-74c3-8f0a-38593b0e5e8a` | `found` | Найден `MCP tool call error` для `codebase-memory-mcp/search_graph` с `Transport closed` |
| SQLite stderr check по тому же thread | `not-found` | `MCP server stderr` rows для target thread отсутствовали в retained window |
| Source audit `RolloutItem` | `found` | Rollout уже содержит служебные элементы, не являющиеся `ResponseItem` |
| Source audit `rollout_reconstruction` | `found` | Служебные rollout items могут быть явно проигнорированы при model history replay |
| Runtime discussion | `accepted` | MCP-вызовы считаются идемпотентными для recovery/retry policy |
| `fork tests --mode list --card docs/fork/mcp-rollout-diagnostics.md` | `ok` | Исполняемая карта содержит подготовку `test_stdio_server` и 4 проверки |
| `fork tests --mode cards --card docs/fork/mcp-rollout-diagnostics.md` | `ok` | Прошли reconstruction, transport recovery, lazy process recovery и stderr-tail tests |
| `fork cards validate` | `ok` | `cards_checked: 22`, `card_errors: 0` |
| `fork format --check` | `ok` | Rust formatting gate прошел после реализации |
| `fork generators` | `ok` | Config/app-server schema generators прошли; ожидаемые generated paths проверены |
| `fork build-fast` | `ok` | `release-fast` build прошел после обновления exhaustive matches |
| `git diff --check` | `ok` | Whitespace diff check прошел |
| `fork install` | `ok` | Установлен release-fast fork-бинарник |
| Source audit после merge `rust-v0.144.5` | `needs-checks` | Persistence/replay/recovery contracts сохранились; добавлено отсутствовавшее failure-path покрытие повторного `TransportClosed`, общий проверочный проход не запускался подагентом |
| Source audit после merge `rust-v0.144.6` | `needs-checks` | Diagnostic schema, launch IDs, bounded stderr tail, rollout persistence, reconstruction filtering и one-shot recovery сохранились; failure-path test усилен проверкой ровно одного `RecoveryStarted`, общий проверочный проход не запускался подагентом |

### Известные падения и пропуски

- Runtime smoke на установленном fork-бинаре отдельно не выполнялся. Покрытие
  recovery path подтверждено card-level integration tests.
- При миграции на `rust-v0.144.4` проверка уровня карточки остановилась до теста
  восстановления: запуск с фильтром пакета `codex-core` не собрал
  `test_stdio_server`, поэтому `cargo_bin("test_stdio_server")` не нашёл бинарник
  по резервному пути. Исполняемая карта теперь готовит тестовый бинарник
  отдельным шагом с остановкой при ошибке перед тестами `codex-core`; проверку
  уровня карточки нужно повторить в общем проверочном проходе.
- После merge `rust-v0.144.5` source audit подтвердил сохранность persisted
  `McpDiagnostic`, исключения из model replay, one-shot recovery и lazy recovery
  после `process_exited`. Обнаруженное отсутствие обязательного теста повторного
  `TransportClosed` устранено; обновлённую карту проверок нужно выполнить в общем
  проверочном проходе.
- После merge `rust-v0.144.6` source audit подтвердил сохранность diagnostic
  schema, launch IDs, bounded stderr tail, rollout persistence и recovery/replay
  path. Failure-path test теперь отдельно доказывает, что повторный
  `TransportClosed` после replay создаёт ровно один `RecoveryFailed` и не
  запускает вторую recovery loop; карту проверок нужно выполнить в общем
  проверочном проходе.
- Текущая установленная версия может продолжать терять stderr в SQLite
  retention; эта карточка не исправляет уже произошедшие rollouts.
- Если future implementation решит хранить полный stderr artifact рядом с
  rollout, карточку нужно обновить: текущий MVP требует bounded tail в rollout.

## Runtime, сборка и установка

Кодовая реализация выполнена в текущем проходе. Быстрая fork-сборка прошла через
`fork build-fast`, новый release-fast fork-бинарник установлен через
skill-owned install flow.

Card-level integration coverage проверяет:

- MCP fixture пишет stderr и закрывает stdio transport во время `tools/call`;
- rollout сессии получает `McpDiagnostic` с `transport_closed` и stderr tail;
- recovery поднимает новый transport;
- исходный idempotent MCP call повторяется не более одного раза, а повторный
  `TransportClosed` сохраняет `recovery_failed` и возвращается наружу;
- отдельный сценарий завершает MCP process в idle-состоянии и подтверждает, что
  следующий MCP call восстанавливает transport до send;
- следующий resume не добавляет diagnostic item в model history.

Runtime smoke-проверка на установленном fork-бинаре остаётся полезной перед
ручным переключением или релизом, но не заменяет card-level regression coverage
из исполняемой карты проверок.

## Риски и ограничения

- Stderr MCP-сервера может содержать чувствительные данные. Поэтому MVP хранит
  bounded tail только при значимом diagnostic event.
- Rollout-файл станет источником диагностики времени выполнения. Нужно сохранять
  жесткие лимиты, чтобы шумный MCP не раздул session history.
- Однократный retry допустим только из-за принятого допущения идемпотентности
  MCP-вызовов. Если позже появятся non-idempotent MCP tools, recovery policy
  придется расширить.
- Eager background restart после `process_exited` не входит в MVP: без backoff и
  лимитов он может превратиться в crash loop.
- Новый `RolloutItem` требует аккуратного update всех exhaustive matches по
  `RolloutItem`, особенно в reconstruction, truncation и agent control code.
- Нельзя считать `list_all_tools` health check: cached tools после startup не
  доказывают, что stdio transport жив.

## Проверка покрытия

| Смысловой пункт | Статус | Где покрыто |
| --- | --- | --- |
| Нужно расследовать `Transport closed` без зависимости от SQLite retention | `реализовано` | `McpDiagnosticItem`, card-level tests |
| `stderr` нельзя писать как model-visible `ResponseItem` | `реализовано` | `rollout_reconstruction`, memory/search/list filters |
| Rollout уже имеет служебные элементы, не попадающие в model context | `реализовано` | `RolloutItem::McpDiagnostic` как служебный persisted item |
| Добавить `RolloutItem::McpDiagnostic` | `реализовано` | `codex-rs/protocol/src/protocol.rs` |
| Хранить bounded stderr tail, а не весь поток | `реализовано` | `codex-rs/rmcp-client/src/stdio_diagnostics.rs` |
| MCP-вызовы считаются идемпотентными | `реализовано` | One-shot retry в `RmcpClient` |
| При closed transport нужен reinitialize и one-shot retry | `реализовано` | `mcp_transport_closed_persists_diagnostic_and_retries_once` |
| Повторный `TransportClosed` после replay записывает ровно один `RecoveryFailed` и не запускает вторую recovery loop | `реализовано` | `mcp_transport_closed_replay_failure_is_not_retried_again` проверяет ровно один `RecoveryFailed` и один `RecoveryStarted` |
| Потеря child process делает launch непригодным и восстанавливается на следующем вызове | `реализовано` | `mcp_process_exit_marks_launch_dead_and_recovers_on_next_call` |
| Child process liveness полезен, но недостаточен | `реализовано` | Dead-launch signal ведёт в recovery до следующего operation |
| Новая карточка не дублирует `mcp-stderr-thread-logs.md` | `перенесено в карточку` | "Обзор", "Карта файлов" |
| Требуемая карта regression tests зафиксирована и пройдена | `реализовано` | `fork-tests.v1`, card-level tests |

## Открытые вопросы

- Нужен ли отдельный persisted artifact для полного stderr при debug mode, или
  bounded tail в rollout должен остаться единственным MVP-механизмом?
- Нужно ли показывать `McpDiagnostic` в TUI/debug view, или достаточно поиска по
  rollout JSONL?
