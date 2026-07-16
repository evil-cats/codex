---
id: fork-mcp-stderr-thread-logs
status: active
created: 2026-07-09
updated: 2026-07-16
source_scope: investigation-019f41cd-da28-7dc3-b2a5-9438bd0e8bdd
---

# Thread-attributed stderr-логи MCP stdio

## Обзор

Эта карточка фиксирует handoff по багу диагностики MCP stdio: Codex должен
сохранять stderr MCP-сервера в локальной log DB так, чтобы запись можно было
найти по concrete `thread_id` и имени MCP-сервера. В расследовании вокруг
`codebase-memory-mcp` было подтверждено, что сам сервер пишет startup log в
stderr, но в логах текущей Codex-сессии не находится ни `mem.init`, ни реальная
строка `MCP server stderr` для этого сервера.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Пользовательская цель | Отличить падение MCP-сервера от обрыва транспорта и найти stderr конкретного MCP по thread |
| Наблюдаемый сервер | `codebase-memory-mcp` |
| Транспорт | MCP stdio через stdin/stdout, stderr читается отдельно |
| Основной симптом | Сервер пишет `mem.init` в stderr, но Codex не сохраняет эту строку как thread-attributed log |
| Главное owner-место | `codex-rs/rmcp-client/src/stdio_server_launcher.rs` |
| Связанный owner логов | `codex-rs/state/src/log_db.rs`, `codex-rs/state/src/runtime/logs.rs` |
| Не является целью этой карточки | Автоматическое восстановление stale MCP transport после `Transport closed` |

Ключевой вывод расследования: проблема была не в том, что MCP-сервер вообще не
запускается или не пишет stderr. Минимальный direct smoke показал, что
`codebase-memory-mcp` на startup пишет строку вида
`level=info msg=mem.init ...` в stderr. Потеря происходила на стороне Codex
logging/attribution: stderr-reader запускался как detached task, а startup MCP
path не гарантировал `thread_id` в span, который попадет в log DB.

## Зачем это нужно

Когда MCP tool call падает с ошибкой:

```text
Transport closed
```

без stderr-следа нельзя понять, что произошло:

- MCP-процесс завершился сам и оставил ошибку в stderr;
- transport внутри текущей Codex-сессии оборвался при живом или уже умершем
  child process;
- parent session потерял свой MCP transport, а subagent успешно работает с
  другим MCP child process;
- stderr был записан как threadless log и быстро вычищен retention-ограничением.

Для пользовательской диагностики нужен нормальный лог, который отвечает на
вопросы:

- какой `thread_id` породил MCP startup или tool call;
- какой `server_name` писал stderr;
- какой child command/program писал строку;
- какая именно stderr-строка пришла;
- была ли ошибка чтения stderr pipe.

Это особенно важно для multi-session и subagent сценариев: успешный MCP в
subagent не доказывает, что transport родительской сессии жив, потому что
subagent может иметь собственный MCP child process и собственный thread log.

## Подтвержденные наблюдения

Наблюдения ниже являются смысловым итогом расследования. Они не являются
каноническим runbook и не хранят локальные пути к временным логам.

| Наблюдение | Статус | Значение |
| --- | --- | --- |
| `codebase-memory-mcp` пишет startup log в stderr | подтверждено | Direct smoke с закрытым stdin показал `level=info msg=mem.init ...` в stderr |
| Строка заканчивается newline | подтверждено | Line-oriented reader в Codex должен уметь прочитать такую строку |
| Hermione config не отключал CBM logs | подтверждено локально | В конфиге был задан command для `codebase-memory-mcp`, без env override вроде `CBM_LOG_LEVEL=none` |
| В retained Codex logs не нашлось реального forwarded `mem.init` | подтверждено локально | Поиск находил только self-noise от поисковых запросов, а не строку MCP stderr |
| До правки stderr-reader local stdio запускался detached `tokio::spawn` без span | подтверждено кодом | `stdio_server_launcher.rs` читал stderr в отдельной задаче без явного `.instrument(...)`; теперь reader future instrument-ирован текущим span |
| Log DB умеет брать `thread_id` из event field или span scope | подтверждено кодом | `log_db.rs` читает `thread_id` из `MessageVisitor` или `event_thread_id(...)` |
| До правки startup MCP path не гарантировал `thread_id` в span | подтверждено кодом | `session_init.mcp_manager_init` теперь несет `thread_id`, а `AsyncManagedClient::new` сохраняет server span |
| Threadless logs имеют жесткий retention bucket | подтверждено кодом | Threadless partition ограничен `LOG_PARTITION_ROW_LIMIT = 1_000` и 10 MiB |
| Log DB queue неблокирующая | подтверждено кодом | `LogDbLayer::try_send` использует bounded queue и молча дропает entry при переполнении |

## Карта файлов

| Файл | Роль в правке |
| --- | --- |
| `codex-rs/rmcp-client/src/stdio_server_launcher.rs` | Local stdio launcher: `StdioServerCommand` несет `server_name`; detached stderr-reader instrument-ирован текущим span; stderr events стали structured |
| `codex-rs/rmcp-client/src/executor_process_transport.rs` | Remote/executor stdio path: transport хранит `server_name`; stderr chunks логируются теми же structured fields |
| `codex-rs/rmcp-client/src/rmcp_client.rs` | Создает `TransportRecipe::Stdio` и `StdioServerCommand`, принимает `server_name` в `new_stdio_client(...)` |
| `codex-rs/rmcp-client/tests/foreign_stdio_cwd.rs` | Обновляет direct stdio-client test call site после добавления `server_name` |
| `codex-rs/rmcp-client/tests/process_group_cleanup.rs` | Обновляет local stdio integration test call sites после добавления `server_name` |
| `codex-rs/rmcp-client/tests/resources.rs` | Обновляет resource integration test call site после добавления `server_name` |
| `codex-rs/codex-mcp/src/rmcp_client.rs` | Знает `server_name`, создает stdio client и передает `server_name` в rmcp-client |
| `codex-rs/codex-mcp/src/connection_manager.rs` | Создает per-server startup futures и background startup summary; смысловой owner span propagation, кодовых правок не потребовал |
| `codex-rs/core/src/session/session.rs` | Знает concrete `thread_id` во время session init; `session_init.mcp_manager_init` теперь несет `thread_id` |
| `codex-rs/state/src/log_db.rs` | Log DB tracing layer: добавлен regression test для spawned stderr event с `thread_id` и structured fields |
| `codex-rs/state/src/runtime.rs` | Владеет лимитами retained log partitions: 10 MiB и 1 000 rows |
| `codex-rs/state/src/runtime/logs.rs` | Вставляет logs, считает estimated bytes, pruning и query filters для feedback/query logs |
| `docs/fork/mcp-stderr-thread-logs.md` | Владеющий handoff-артефакт этой fork-доработки |

Намеренно не менять в рамках минимального фикса:

| Зона | Почему не менять |
| --- | --- |
| `codebase-memory-mcp` | Сервер уже пишет startup stderr; текущая проблема на стороне Codex logging/attribution |
| MCP JSON-RPC stdin/stdout protocol | Stderr не является protocol stream и должен оставаться диагностическим side channel |
| Recovery после `Transport closed` | Это отдельная поведенческая задача: эта карточка только делает причину видимой |
| Общий redesign log retention | Retention объясняет исчезновение threadless startup rows, но минимальный фикс должен сначала перестать писать их как threadless |
| Per-session отдельные MCP log-файлы | Log DB уже является owner локальных логов; плодить второй источник истины не нужно без отдельного дизайна |

## Итоговый контракт

### Log attribution

Каждая stderr-строка MCP stdio child process должна попадать в log DB как
событие, которое можно связать с конкретным Codex thread.

Контракт для persisted row:

- `thread_id` в таблице `logs` должен быть concrete `ThreadId` сессии, которая
  запустила MCP startup или выполняла reconnect/tool-call work;
- stderr local stdio не должен уходить только в threadless partition;
- `server_name` должен присутствовать в structured fields или span fields
  так, чтобы строку можно было найти по имени MCP-сервера;
- `program` или другое поле command identity должно сохранять child command;
- stderr payload должен быть отдельным field, например `stderr_line`, а не
  только частью formatted message;
- ошибка чтения stderr pipe должна быть `warn` с `server_name`, `program` и
  `error`.

Рекомендуемая форма event message:

```text
MCP server stderr
```

Рекомендуемые fields:

```text
server_name=<mcp server name>
program=<resolved or configured program label>
stderr_line=<one line from stderr>
```

Для ошибки чтения:

```text
Failed to read MCP server stderr
```

с fields:

```text
server_name=<mcp server name>
program=<resolved or configured program label>
error=<io error>
```

### Startup path

Startup stderr должен быть thread-attributed уже при первом запуске MCP-сервера
во время session init. Недостаточно полагаться на будущий tool-call span: строка
`mem.init` появляется до первого tool call.

Будущая реализация должна обеспечить оба условия:

- startup future MCP-сервера выполняется под span, в scope которого есть
  `thread_id`;
- detached stderr-reader task наследует текущий span через явную
  instrumentation.

### Local и remote stdio

Local stdio и executor-backed stdio должны иметь одинаковую диагностическую
форму:

- одинаковый message для обычной stderr-строки;
- одинаковые field names для `server_name`, `program`, stderr payload и error;
- одинаковое правило attribution по `thread_id`.

Если remote path не может получить `thread_id` через тот же span route, это
нужно зафиксировать как отдельный open question или добавить локальный контекст
в executor transport path. Нельзя оставить remote stderr в старом формате
молча, потому что future debugging снова разделится на два несовместимых
маршрута.

### Model-visible поведение

Эта доработка не должна добавлять stderr MCP-сервера в model context, tool
output или TUI history. Цель - локальные logs/feedback diagnostics, а не
новый model-visible artifact.

Существующий риск приватности stderr не должен увеличиваться за счет новых
публичных поверхностей. Structured fields делают уже сохраняемую строку легче
найти в log DB, но не должны отправлять ее модели автоматически.

## Архитектурное решение

Рекомендуемый минимальный подход:

1. `server_name` добавлен в stdio command/log context.

   `codex-mcp` уже знает `server_name` в
   `codex-rs/codex-mcp/src/rmcp_client.rs::make_rmcp_client`. Идентификатор
   передается в `RmcpClient::new_stdio_client(...)`, затем в
   `StdioServerCommand`, чтобы low-level `rmcp-client` мог логировать
   `server_name` без парсинга command path.

2. `thread_id` добавлен в startup tracing scope, а не протаскивается как
   обычный параметр в transport library.

   `thread_id` является Codex session concept, а `rmcp-client` должен оставаться
   транспортным слоем. Поэтому предпочтительный route - span attribution:
   `codex-rs/core/src/session/session.rs` знает concrete `thread_id` перед
   вызовом `McpConnectionManager::new(...)`; span `session_init.mcp_manager_init`
   расширен field `thread_id`.

3. Span сохраняется в detached stderr-reader.

   В `LocalStdioServerLauncher::launch_server(...)` перед `tokio::spawn`
   захватывается `tracing::Span::current()`, а async reader future
   instrument-ируется этим span. Без этого даже правильно созданный startup span
   не дошел бы до stderr events.

4. Events структурированы вместо форматированной строки.

   Текущий local формат:

   ```text
   MCP server stderr ({program_name}): {line}
   ```

   заменен на stable message и fields:

   ```rust
   info!(
       server_name = %server_name,
       program = %program_name,
       stderr_line = %line,
       "MCP server stderr"
   );
   ```

   Remote executor path использует ту же схему fields.

5. Explicit `thread_id` event field оставлен как fallback, а не как первый
   выбор.

   `LogDbLayer` уже умеет читать `thread_id` из event fields, но протаскивание
   `thread_id` до `rmcp-client` расширяет API transport слоя Codex-specific
   знанием. Если тесты покажут, что startup/reconnect span route недостаточен,
   допустимый fallback - маленький Codex-owned context struct выше transport
   слоя, но не ad hoc параметр в каждом низкоуровневом вызове.

Отклоненные альтернативы:

| Альтернатива | Почему отклонена |
| --- | --- |
| Искать stderr только в threadless logs по `process_uuid` | Threadless bucket ограничен 1 000 rows и быстро теряет startup строки в шумных сессиях |
| Делать отдельный log file на каждый MCP child | Появится второй источник истины рядом с SQLite logs и feedback pipeline |
| Логировать только `program_name` без `server_name` | Один command может обслуживать несколько server configs; имя сервера является пользовательским ключом поиска |
| Считать успех subagent MCP доказательством живого parent MCP | Subagent может стартовать собственный MCP child process и писать в другой thread |
| Чинить `Transport closed` recovery в этой же правке | Сначала нужен достоверный stderr/log signal; recovery является отдельным behavior change |

## Порядок повторения при переносе

1. Проверить текущую форму `StdioServerCommand`, `RmcpClient::new_stdio_client`
   и `make_rmcp_client`: если upstream уже несет `server_name` или log context,
   не дублировать новый параметр.
2. Проверить startup span chain: `Session::new`, `McpConnectionManager::new`,
   `AsyncManagedClient::new`, `ManagedClientStartup::start` и per-server spawned
   tasks должны сохранять scope с `thread_id`.
3. В local stdio launcher instrument-ировать detached stderr-reader текущим
   span before spawn.
4. Перевести local stderr events на stable message plus structured fields.
5. Синхронизировать executor-backed stderr events с той же structured схемой.
6. Добавить regression coverage на thread attribution и searchable
   `server_name`.
7. При переносе сохранить active-карточку вместе с реальным `fork-tests.v1`
   блоком или новым owner-gate, который запускает эти проверки.
8. Не смешивать эту правку с recovery/reconnect policy после `Transport closed`,
   кроме узкой проверки, что reconnect stderr тоже не становится threadless.

## Проверки

### Смысловое покрытие

| Контракт | Обязательность | Где должно покрываться |
| --- | --- | --- |
| Local MCP stderr event сохраняется с `thread_id` сессии | `required` | `spawned_mcp_stderr_event_keeps_thread_id_and_fields` |
| Startup stderr до первого tool call получает `thread_id` | `required` | `session_init.mcp_manager_init` содержит `thread_id`; regression проверяет spawned task span attribution |
| `server_name` присутствует в searchable log body/fields | `required` | `spawned_mcp_stderr_event_keeps_thread_id_and_fields`; compile coverage stdio call chain |
| Stderr payload логируется отдельным field | `required` | `spawned_mcp_stderr_event_keeps_thread_id_and_fields` проверяет `stderr_line` |
| Error reading stderr pipe сохраняет `server_name`, `program`, `error` | `required` | Source audit `stdio_server_launcher.rs`; отдельного fault-injection test пока нет |
| Executor-backed stdio не остается в старом формате | `required` | Source audit `executor_process_transport.rs`; compile coverage `codex-rmcp-client` |
| Log queue drop behavior не маскируется как гарантированная доставка | `required` для документации риска | Existing `LogDbLayer` drop test |

### Владелец исполняемой карты

Card-level проверки запускает skill-owned command `fork tests`. Карточка не
является runbook запуска внутренних команд: конкретные argv хранятся только в
машинно-читаемом блоке `fork-tests.v1`, который читает `fork tests`.

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "mcp stderr log attribution",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-state",
        "spawned_mcp_stderr_event_keeps_thread_id_and_fields"
      ]
    },
    {
      "purpose": "rmcp stdio call chain",
      "argv": ["just", "test", "-p", "codex-rmcp-client", "resources"]
    }
  ]
}
```

### Дополнительные gates

| Gate | Когда нужен | Статус |
| --- | --- | --- |
| `fork cards validate` | После добавления или изменения этой карточки | `required` |
| `fork tests` list map | После добавления active card-level test map | `required` |
| `fork format` | После кодовых правок Rust | `required` |
| `fork build-fast` | Если нужен installed/runtime smoke | `manual-required` при live-проверке установленного бинаря |
| `fork generators` | Если реализация не меняет config/schema/API artifacts | `not-applicable` |

### Исторические результаты

| Проверка или источник | Результат | Примечание |
| --- | --- | --- |
| Direct smoke MCP binary | `passed` | `codebase-memory-mcp` пишет `level=info msg=mem.init ...` в stderr при закрытом stdin |
| Проверка текущей Codex log DB | `not-found` | Реальный forwarded `mem.init` не был найден; найденные `MCP server stderr` совпадения были self-noise поисковых запросов |
| Initial source audit `stdio_server_launcher.rs` | `found` | До правки local stderr-reader запускался через detached `tokio::spawn` без явного span instrumentation |
| Source audit `log_db.rs` | `found` | Log DB берет `thread_id` из event field или span scope |
| Initial source audit startup MCP path | `found` | До правки startup spans имели server context, но route не гарантировал `thread_id` для MCP stderr |
| Source audit retention | `found` | Threadless logs capped per `process_uuid`; startup rows могут исчезать под шумом |
| `.codex/skills/fork/scripts/fork format --fix` | `passed` | Применено Rust/doc formatting через skill-owned workflow |
| `.codex/skills/fork/scripts/fork cards validate` | `passed` | Active карточка валидна: `card_errors: 0` |
| `.codex/skills/fork/scripts/fork tests --mode list --card fork-mcp-stderr-thread-logs` | `passed` | Test map содержит `codex-state` attribution test и `codex-rmcp-client` stdio call chain |
| `.codex/skills/fork/scripts/fork tests --mode cards --card fork-mcp-stderr-thread-logs` | `passed` | Прошли `mcp stderr log attribution` и `rmcp stdio call chain` |
| `.codex/skills/fork/scripts/fork build-fast` | `passed` | Release-fast binary собран и прошел metadata/version checks |
| `.codex/skills/fork/scripts/fork install` | `passed` | Собранный binary установлен как `${HOME}/.local/bin/codex-hermione` и прошел metadata/version checks |
| Проверка исходного кода после слияния на `hermione-0.144.5` | `passed` | Сохранены startup span с `thread_id`, явное инструментирование local stderr-reader, одинаковые структурированные поля local/executor stdio, передача `server_name` и регрессионный тест в `codex-state`; проверки уровня проекта оставлены общему проверочному проходу |

### Миграция на `rust-v0.144.5`

После слияния `rust-v0.144.5` контракт карточки сохранился без дополнительных
кодовых правок. Проверены thread attribution, поля `server_name`, `program`,
`stderr_line` и `error`, маршруты local/executor stdio, место вызова ресурсов и
тесты очистки жизненного цикла.

В `codex-rs/rmcp-client/src/rmcp_client.rs` уже находился отдельный
diff диагностики восстановления MCP в области другой карточки. Он совместим с
этой доработкой, использует общий `StdioServerDiagnosticState` и намеренно
оставлен без изменений. Тесты уровня карточки, форматирование, сборка и другие
проверки уровня проекта в one-card проходе не запускались; их результаты должен
зафиксировать родительский общий проверочный проход.

### Известные падения и пропуски

- `Transport closed` остается отдельной runtime-проблемой. Эта карточка только
  делает будущие причины видимыми в logs.
- Текущая log DB queue может дропать новые записи при переполнении; это уже
  подтвержденное поведение `LogDbLayer::try_send`. Если новый тест станет
  flaky под нагрузкой, нужно либо flush/контролировать очередь, либо отдельно
  улучшать observability dropped logs.
- Background reconnect для Codex Apps MCP может требовать отдельной проверки
  span preservation, потому что reconnect запускается из background task.

## Runtime, сборка и установка

В проходе первоначальной реализации бинарный файл был собран через
`fork build-fast` из
`codex-rs/target/release-fast/codex` и установлен через `fork install` как
`${HOME}/.local/bin/codex-hermione`. Проверки оберток сборки и установки
подтвердили метаданные и источник версии временного и установленного бинарных
файлов.

Для будущей реализации полезен runtime smoke на установленном fork-бинаре:

- запустить сессию с stdio MCP server fixture, который пишет startup stderr;
- получить concrete `thread_id` текущей сессии;
- убедиться, что в local log DB есть строка `MCP server stderr` с этим
  `thread_id` и нужным `server_name`;
- повторить для subagent, если правка затрагивает spawn/subagent startup route.

Этот smoke не заменяет automated regression coverage, но хорошо подтверждает
исходный пользовательский сценарий.

## Риски и ограничения

- Stderr MCP-сервера может содержать чувствительные данные. Эта карточка не
  предлагает новую model-visible поверхность, но сопровождение правки должно
  помнить, что structured fields облегчают поиск уже сохраненной строки.
- `thread_id` не протащен как обычный параметр в `rmcp-client`: transport crate
  не знает о Codex session model, attribution идет через span route.
- Если починить только local stdio, executor-backed stdio останется в старом
  формате и remote debugging снова будет отличаться.
- Если добавить `server_name` только в span, но не в event fields, строку все
  еще можно будет искать в formatted feedback body, но structured event будет
  менее явным. Лучше сохранить `server_name` как event field.
- Retention и queue drop означают, что logs не являются абсолютной гарантией
  доставки. Цель минимального фикса - убрать лишнюю потерю attribution, а не
  обещать durable audit log.

## Проверка покрытия

| Смысловой пункт | Статус | Где покрыто |
| --- | --- | --- |
| Сервер пишет stderr, но Codex не показывает его как thread log | `перенесено в карточку` | "Обзор", "Подтвержденные наблюдения" |
| Различить падение сервера и обрыв transport | `перенесено в карточку` | "Зачем это нужно", "Не является целью этой карточки" |
| Subagent MCP может быть отдельным child process | `перенесено в карточку` | "Зачем это нужно", "Отклоненные альтернативы" |
| Local stderr-reader теряет span из-за detached task | `перенесено в карточку` | "Подтвержденные наблюдения", "Архитектурное решение" |
| Log DB умеет брать `thread_id` из span или event field | `перенесено в карточку` | "Подтвержденные наблюдения", "Карта файлов" |
| Startup MCP path должен получить `thread_id` до первого tool call | `перенесено в карточку` | "Итоговый контракт", "Архитектурное решение" |
| `server_name` должен быть structured/searchable | `перенесено в карточку` | "Итоговый контракт", "Архитектурное решение" |
| Remote/executor stderr нужно синхронизировать | `перенесено в карточку` | "Итоговый контракт", "Карта файлов" |
| `Transport closed` recovery не входит в scope | `перенесено в карточку` | "Обзор", "Известные падения и пропуски" |
| Реальный card-level test добавлен после реализации | `перенесено в карточку` | "Проверки" |

## Открытые вопросы

- Нужно ли добавлять отдельный counter/warn для dropped log entries в
  `LogDbLayer::try_send`, или оставить это отдельной observability карточкой?
- Должен ли background reconnect сохранять исходный thread attribution или
  получать новый current thread context при каждом reconnect?
