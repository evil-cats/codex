---
id: fork-tui-core-tool-activity
status: active
created: 2026-07-04
updated: 2026-09-02
---

# Видимость core tools в TUI

## Обзор

Эта карточка владеет fork-доработкой Hermione, которая показывает выбранные
core function tools в TUI как понятные пользовательские действия, а не только
как внутренние имена tools или скрытые `FunctionCallOutput`.

## Зачем это нужно

После добавления core function tools у агента появляется более точный путь для
простых операций без shell-команд: чтение файла, получение metadata текущего
thread и чтение текущего времени host через fork-инструмент `get_system_time`
либо upstream-инструмент `clock/curr_time`. Но если эти вызовы не видны в TUI,
пользователь теряет наблюдаемость работы агента.

Старое shell-чтение уже имеет понятную визуальную модель:

```text
• Explored
  └ Read foo.txt
```

Новые core tools не должны ухудшать этот UX. Пользователь должен видеть, что
агент прочитал файл или проверил служебный контекст, даже если операция прошла
через Responses function tool, а не через `sed`, `cat` или `date`.

Проблемы, которые решает доработка:

- `read_file` возвращает результат модели как `FunctionCallOutput`, но не
  создает отдельное видимое действие TUI;
- `get_thread_info`, `get_system_time` и upstream-инструмент `clock/curr_time`
  из namespace `clock` также могут оставаться невидимыми для пользователя;
- сырые подписи вроде `Tool read_file` или `Function get_system_time` раскрывают
  внутренний API вместо пользовательского действия;
- отсутствие snapshot-контракта делает будущую регрессию UI незаметной.

## Карта файлов

Владеющие файлы реализации:

| Файл или зона | Ответственность |
| --- | --- |
| `codex-rs/protocol/src/items.rs` | Добавляет `TurnItem::CoreToolActivity`, `CoreToolActivityItem`, `CoreToolActivityKind` и `CoreToolActivityStatus` как ограниченную структурированную поверхность activity |
| `codex-rs/protocol/src/legacy_events.rs` | Старый слой совместимости с legacy-событиями явно не материализует `CoreToolActivity` в `EventMsg`, чтобы новая UI-поверхность activity не меняла legacy/model-visible поток |
| `codex-rs/core/src/tools/core_tool_activity.rs` | Определяет сопоставление выбранных function tools из default namespace и точного upstream-инструмента `clock/curr_time` с activity item, компактный `detail`, включая разрешение пути `read_file` через выбранную step environment, raw `arguments`, lifecycle started/completed и status |
| `codex-rs/core/src/tools/core_tool_activity_tests.rs` | Проверяет нормализованный default namespace, включение точной пары `clock/curr_time`, исключение остальных инструментов из других namespaces, `clock/curr_time -> System time utc` и выбор `environment_id`/path convention для `read_file detail`; тестовые данные оставляют разрешённые `workspace_roots` пустыми, чтобы изолировать разрешение пути через выбранный `cwd` |
| `codex-rs/core/tests/suite/core_tool_activity.rs` | Через настоящий function call Responses API проверяет согласованную пару `ItemStarted`/`ItemCompleted` для успешного `read_file` и итоговый `CoreToolActivityStatus::Completed` |
| `codex-rs/core/tests/suite/mod.rs` | Подключает интеграционный тест core tool activity к общему core test binary |
| `codex-rs/core/src/tools/registry.rs` | Оборачивает текущий путь выполнения, возвращающий `AnyToolResult`, событиями `emit_turn_item_started` и `emit_turn_item_completed` для подходящих core function tools без отдельной ячейки передачи результата и без изменения model-visible `FunctionCallOutput` |
| `codex-rs/core/src/tools/mod.rs` | Подключает модуль `core_tool_activity` |
| `codex-rs/app-server-protocol/src/protocol/v2/item.rs` | Экспортирует v2 `ThreadItem::CoreToolActivity`, wire enums, `id()` и conversion из core `TurnItem` |
| `codex-rs/app-server-protocol/src/protocol/thread_history.rs` | Восстанавливает `CoreToolActivity` из `ItemStarted`/`ItemCompleted` при replay сохраненной thread history |
| `codex-rs/app-server-protocol/schema/` | Хранит сгенерированные JSON, TypeScript и precomputed schema artifacts для v2 `CoreToolActivity` |
| `codex-rs/analytics/src/reducer.rs` | Явно игнорирует `CoreToolActivity` в analytics reducer, чтобы новый UI/history item не расширял telemetry contract этой карточкой |
| `codex-rs/rollout/src/persistence_metrics.rs` | Классифицирует вложенный `CoreToolActivity` как отдельный тип сохраняемого item без изменения решения о сохранении legacy history |
| `codex-rs/rollout/src/persistence_metrics_tests.rs` | Проверяет тип вложенного completed item и сохранение действующего решения о фильтрации |
| `codex-rs/thread-store/src/local/thread_history.rs` | Не использует UI-only activity для вычисления заголовка thread history |
| `codex-rs/thread-store/src/local/thread_history/search.rs` | Не добавляет UI-only activity в searchable text thread history |
| `codex-rs/tui/src/history_cell/core_tool_activity.rs` | Рисует человекочитаемые строки `Exploring/Explored -> File` и `Inspecting/Inspected -> Thread info/System time`; для `File` показывает короткие имена, убирает диапазоны строк и дедуплицирует повторы; при очистке переводит незавершённые записи в `Failed` |
| `codex-rs/tui/src/exec_cell/model.rs` | Хранит core `File` как отдельную запись внутри существующего exploration cell, не маскируя `read_file` под shell command |
| `codex-rs/tui/src/exec_cell/render.rs` | Рисует смешанные exploration-блоки `Search`/`List`/`Read` + `File`, включая active `Exploring` и completed `Explored` |
| `codex-rs/tui/src/history_cell/mod.rs` | Экспортирует новый renderer history cell |
| `codex-rs/tui/src/history_cell/tests.rs` | Содержит `insta` snapshot-покрытие для active `read_file` и completed inspect tools |
| `codex-rs/tui/src/chatwidget/tests/exec_flow.rs` | Проверяет группировку последовательных `File`, явный `Search -> direct File -> Search`, перенос pending `File`, completed-only replay, interleaving с завершением `exec` и очистку без `ItemCompleted` |
| `codex-rs/tui/src/chatwidget/protocol.rs` | Направляет live `ItemStarted` для core activity в TUI lifecycle |
| `codex-rs/tui/src/chatwidget/replay.rs` | Восстанавливает active/completed core activity при replay turn items |
| `codex-rs/tui/src/chatwidget/command_lifecycle.rs` | При старте shell exploration-команды переносит уже активный core `File` в новый `ExecCell`, а при несвязанном завершении `exec` не сбрасывает активный in-progress `File` |
| `codex-rs/tui/src/chatwidget/tool_lifecycle.rs` | Управляет active cell, completion и резервным путем для completed activity, пришедшей не по порядку; коалесит последовательные `File` activity, completed-only replay и смешанные `Search`/`File` exploration-блоки в одну ячейку |
| `codex-rs/tui/src/chatwidget.rs` | При общей очистке turn переводит оставшиеся `InProgress` записи `CoreToolActivityCell` в `Failed` до переноса ячейки в историю |
| `codex-rs/tui/src/chatwidget/turn_runtime.rs` | На обычном завершении turn финализирует оставшуюся активную `CoreToolActivity`, даже если соответствующий `ItemCompleted` не дошёл до TUI |
| `codex-rs/tui/src/thread_transcript.rs` | Рендерит persisted `CoreToolActivity` в transcript/history view |
| `codex-rs/tui/src/app/agent_status_feed.rs` | Показывает bounded summary `File`, `Thread info` или `System time` в `/agent` preview |
| `codex-rs/tui/src/dynamic_tools.rs` | Сохраняет ограниченный structured `CoreToolActivity` в thread summary, распознаёт его как latest tool marker и не подменяет им Responses `FunctionCallOutput` |
| `codex-rs/tui/src/dynamic_tools_tests.rs` | Проверяет structured thread summary для `CoreToolActivity` вместе с независимыми метаданными upstream и Responses `FunctionCallOutput` |

Намеренно не входит в эту карточку:

- изменение runtime-контрактов `read_file`, `get_thread_info` или
  `get_system_time`, а также upstream `clock/curr_time` и `clock/sleep`;
- переименование самих tools;
- изменение model-visible descriptions этих tools;
- изменение telemetry contract или добавление отдельного analytics event для
  core tool activity;
- принудительный перевод всех function tools в видимые TUI items;
- отображение `clock/sleep` как `System time`: согласованное исключение
  относится только к чтению времени через `clock/curr_time`;
- переименование старого shell-based `Read` для `cat`, `sed -n`, `nl`, `head`
  или `tail`.

## Итоговый контракт

### Подписи в TUI

TUI должен показывать выбранные core tool calls через человекочитаемую пару
группы и действия.

| Tool | Заголовок во время выполнения | Заголовок после завершения | Подпись действия | Деталь |
| --- | --- | --- | --- | --- |
| `read_file` | `Exploring` | `Explored` | `File` | Короткие имена прочитанных файлов без диапазонов строк |
| `get_thread_info` | `Inspecting` | `Inspected` | `Thread info` | `current` без аргумента или компактный явный `thread_id` |
| `get_system_time` | `Inspecting` | `Inspected` | `System time` | `local`, `utc`, фиксированный offset или другой краткий detail из аргументов |
| `clock/curr_time` | `Inspecting` | `Inspected` | `System time` | Всегда `utc` |

`read_file` намеренно получает подпись действия `File`, а не `Read`. Старый
shell-read renderer может продолжать писать `Read`, потому что эта карточка
владеет только новым core function tool surface.

### `read_file`

Минимальный вид вызова во время выполнения:

```text
• Exploring
  └ File core-read-file-tool.md
```

Минимальный вид завершенного вызова:

```text
• Explored
  └ File core-read-file-tool.md
```

Если задан диапазон строк, компактная история все равно показывает только имя
файла. Диапазон остается в structured `arguments` и диагностике, но не должен
забивать обычную историю TUI:

```text
• Explored
  └ File core-read-file-tool.md
```

Если подряд прочитано несколько файлов, TUI должен группировать их в одну строку
без повторов:

```text
• Explored
  └ File fork_cli.py, checks-and-gates.md, shell.md
```

Правила:

- путь берется из аргумента `path`;
- при явном `environment_id` путь для `detail` разрешается относительно cwd
  выбранной step environment через `PathUri`; это сохраняет foreign path
  convention и показывает фактический basename прочитанного файла;
- обычная история TUI показывает basename разрешённого файла; если имя самого
  файла совпадает со служебным именем каталога вроде `src`, оно не отбрасывается;
- если путь нельзя разрешить через environment `PathUri`, fallback сокращает
  исходный `path` по смыслу старого shell `Read`: оставляет последний значимый
  компонент без префикса workspace/root и служебных сегментов вроде `src`,
  `build`, `dist` или `node_modules`;
- `start_line` и `end_line` не добавляются в компактную историю;
- последовательные события `read_file` activity коалессятся в один
  блок `Exploring`/`Explored`, пока поток работы не прерывается другим видимым
  history item;
- если текущий active cell уже является exploration-блоком shell-команд
  (`Search`, `List` или `Read`), `read_file` добавляется в него отдельной
  строкой `File`; это правило действует и для активного `Exploring`, и для
  завершенного, но еще не вытолкнутого в историю `Explored`;
- прямой `FunctionCall(read_file)` между двумя shell search-командами остаётся
  в том же exploration-блоке; grouping не зависит от публикации `read_file`
  внутри `exec` и не требует такой публикации;
- если `read_file` стартует раньше shell exploration-команды, новый
  `Search`/`List`/`Read` забирает pending `File` в свой `ExecCell` вместо того,
  чтобы flush-ить stale `Exploring -> File` отдельной history-карточкой;
- при replay сохраненной thread history completed-only `File` items с разными
  `call_id` должны продолжать коалеситься в активной file-ячейке, потому что
  persisted history хранит итоговый completed item, а не пару start/completion;
- несвязанное завершение `exec`-команды, пришедшее пока core `File` еще
  `InProgress`, отображается отдельной finalized command history cell и не
  flush-ит активный `Exploring -> File`;
- повторные чтения одного и того же короткого имени в таком блоке
  дедуплицируются;
- `line_numbers=false` не меняет человекочитаемую подпись действия;
- `complete=no` из результата может быть отражен в detail или вторичной строке,
  но это не должно заменять основное действие `File`.
- полный `path`, `start_line`, `end_line`, `line_numbers` и исходные аргументы
  сохраняются в structured payload; компактная история не является источником
  точного диапазона чтения.

### `get_thread_info`

Минимальный вид вызова без аргументов во время выполнения:

```text
• Inspecting
  └ Thread info current
```

Завершенный вызов:

```text
• Inspected
  └ Thread info current
```

Если передан явный `thread_id`, detail должен показывать, что проверялся не
текущий thread. Компактное отображение может использовать короткий id, но raw
detail должен сохранять полный `thread_id`:

```text
• Inspected
  └ Thread info 019f2d70
```

### Системное время

Минимальный вид вызова во время выполнения:

```text
• Inspecting
  └ System time local
```

Завершенный вызов:

```text
• Inspected
  └ System time local
```

Если вызов использует `offset: "utc"` или fixed offset, detail должен отражать
именно выбранный offset:

```text
• Inspected
  └ System time utc
```

```text
• Inspected
  └ System time +03:00
```

Поле `formatted` можно показывать как дополнительный detail, если renderer может
сделать это без нестабильных snapshot-тестов и без переполнения строки. Базовый
контракт требует видимости вызова, а не обязательного вывода текущего времени в
краткой строке.

Вызов upstream-инструмента `clock/curr_time` использует ту же подпись действия,
но всегда получает детерминированный detail `utc`:

```text
• Inspected
  └ System time utc
```

Сопоставление применяется только к точной паре namespace/name
`clock/curr_time`. `clock/sleep`, одноимённый `curr_time` из другого namespace и
инструмент `curr_time` из default namespace не должны превращаться в
`System time`.

### Raw и диагностика

Обычная история TUI использует labels `File`, `Thread info` и `System time`.
Технические имена tools должны оставаться доступны в structured payloads,
debug/raw transcript или раскрываемых details:

```text
read_file
get_thread_info
get_system_time
clock/curr_time
```

Это важно для диагностики, проверок, потребителей app-server и будущей отладки.
Но эти имена не должны становиться основной человекочитаемой подписью в compact
history.

### Lifecycle, Responses и ограниченный preview

Lifecycle одной activity состоит из трёх наблюдаемых переходов:

- `start`: `ItemStarted` с `CoreToolActivityStatus::InProgress` создаёт или
  расширяет active TUI cell;
- `update`: соответствующий `ItemCompleted` по `id` обновляет существующую
  запись до `Completed` или `Failed`, не создавая дубликат;
- `finish`: завершённая activity остаётся доступной в истории, а завершение turn
  переводит потерявшие `ItemCompleted` записи из `InProgress` в `Failed` и
  прекращает их анимацию.

Responses integration остаётся раздельной: самостоятельный `FunctionCallOutput`
может находиться рядом с `CoreToolActivity` в `Turn`, сохраняется в structured
thread summary и остаётся model-visible output. TUI replay не превращает такой
output во вторую activity cell.

Dynamic-tools preview обязан оставаться ограниченным: `thread_summary`
ограничивает поле `summary` 300 символами, `turn_summary` сохраняет не больше 20
последних `ThreadItem`, а сериализованный ответ проходит общий предел
`MAX_RESPONSE_BYTES`.
`CoreToolActivity` участвует в `latestToolMarker` с исходными `id`, `tool_name`
и текущим `status`.

## Архитектурное решение

Доработка добавляет отдельную поверхность UI activity для выбранных core
function tools и не притворяется shell execution.

Принятый подход:

- не создавать поддельный `ParsedCommand::Read` для `read_file`;
- не отправлять синтетическую shell-команду ради TUI;
- не показывать обобщенный `Tool read_file` как основной label;
- добавить структурированное сопоставление выбранного core tool call с
  пользовательской activity;
- начинать activity перед текущим future обработчика registry и завершать её по
  `Result<AnyToolResult, FunctionCallError>`; результат уже содержит
  `ToolOutput`, поэтому отдельная ячейка результата или повторно подготовленный
  `ToolInvocation` не нужны;
- считать отсутствующий, пустой и нормализованный upstream namespace `functions`
  одним default namespace через `ToolName::is_default_namespace()`, но не
  материализовать одноимённые tools из других namespaces как core activity;
- добавить единственное точное исключение для пары namespace/name
  `clock/curr_time`, которое создаёт `CoreToolActivityKind::SystemTime` с detail
  `utc`; не расширять его на `clock/sleep` или другие namespace/name без
  отдельного решения;
- принимать `read_file` как прямой `FunctionCall` с экспозицией
  `DirectModelOnly`; code-mode `exec` не должен владеть file-read lifecycle;
- для `read_file` повторить компактность старого shell `Read`: короткие имена
  файлов, группировка соседних чтений и дедупликация повторов;
- сохранять grouping при replay completed-only `CoreToolActivity` и при
  interleaving с завершением несвязанной `exec`-команды;
- при завершении turn финализировать оставшиеся `InProgress` activity как
  `Failed`, чтобы history cell не сохраняла бесконечную анимацию;
- сохранять `CoreToolActivity` и самостоятельный Responses `FunctionCallOutput`
  как разные structured items в ограниченном dynamic-tools summary;
- оставить старую shell-модель `Exploring/Explored -> Read/List/Search` как
  самостоятельный renderer для exploration через exec.

Модель событий:

```text
FunctionCall(read_file args) -> CoreToolActivity(kind=File, group=Explore)
FunctionCall(get_thread_info args) -> CoreToolActivity(kind=ThreadInfo, group=Inspect)
FunctionCall(get_system_time args) -> CoreToolActivity(kind=SystemTime, group=Inspect)
FunctionCall(clock/curr_time args) -> CoreToolActivity(kind=SystemTime, group=Inspect, detail=utc)
```

Выбранная форма реализации:

| Уровень | Форма |
| --- | --- |
| Core protocol | `TurnItem::CoreToolActivity(CoreToolActivityItem)` |
| App-server v2 | `ThreadItem::CoreToolActivity` |
| Kind | `File`, `ThreadInfo`, `SystemTime` |
| Status | `InProgress`, `Completed`, `Failed` |
| Detail | Короткая строка, вычисленная из аргументов: basename файла для `read_file`, разрешенный через выбранную step environment, `current`, `local`, `utc` или фиксированный offset; для `clock/curr_time` всегда `utc` |
| Raw diagnostics | `tool_name` и `arguments` сохраняются в structured payload |

Главный инвариант: model-visible output остается `FunctionCallOutput`, а TUI
получает отдельный ограниченный по размеру user-facing activity item.

## Порядок повторения при переносе

1. Проверить текущий upstream-путь TUI для shell exploration:
   `Exploring/Explored -> Read/List/Search` в renderer-е истории exec.
2. Проверить текущий путь обычных function tools:
   `FunctionCall` -> tool dispatch -> `FunctionCallOutput`.
   Отдельно проверить, нормализует ли upstream обычные вызовы в namespace
   `functions`: такая нормализация должна по-прежнему считаться default namespace
   и не отключать activity mapping.
3. Выбрать или перенести минимальную структурированную поверхность для выбранной
   core tool activity; текущая реализация использует `CoreToolActivity` item.
4. Реализовать сопоставление для default namespace только для `read_file`,
   `get_thread_info` и `get_system_time`, а также точное исключение
   `clock/curr_time -> SystemTime(utc)`. Не включать `clock/sleep`.
5. Подключить lifecycle к текущему API результата registry без промежуточного
   хранилища: activity создаётся перед future обработчика, а completion использует
   success/error итогового `AnyToolResult`. Сохранить model-visible outputs без
   изменений: результат tool call по-прежнему возвращается модели как
   `FunctionCallOutput`.
6. Добавить TUI rendering с точными labels из этой карточки.
7. Для `read_file` в компактной истории показывать короткое имя файла без
   диапазонов строк; коалесить соседние `File` activity в один блок без
   повторов и приклеивать прямой `FunctionCall(read_file)` к текущему
   exploration-блоку shell `Search`/`List`/`Read`, если он еще активен в
   transcript tail, включая последовательность `Search -> File -> Search`.
   Если указан `environment_id`, разрешать путь через cwd той же step
   environment, которую выбирает handler, чтобы не интерпретировать foreign
   path по правилам primary host.
8. Поддержать completed-only replay: если сохраненная thread history уже
   схлопнула `ItemStarted`/`ItemCompleted` в completed item, соседние `File`
   entries должны группироваться так же, как live start/completion flow.
9. Защитить live interleaving: завершение несвязанной `exec`-команды не должно
   сбрасывать активный in-progress `File` и разделять группу параллельных
   чтений.
10. Добавить snapshot-покрытие для active и completed состояний:
    `Exploring/Explored -> File`, `Inspecting/Inspected -> Thread info`,
    `Inspecting/Inspected -> System time` для `get_system_time` и
    `clock/curr_time`, а также для сгруппированной `File` activity и смешанного
    `Search`/`File` exploration-блока.
11. Поддержать app-server v2 conversion и thread history replay, если переносимый
    upstream еще не знает `CoreToolActivity`.
12. Проверить exhaustive matches после upstream-изменений `ThreadItem`: Responses
    `FunctionCallOutput` должен сохраняться в structured summary и обрабатываться
    собственным replay path без изменения lifecycle `CoreToolActivity`.
13. Проверить очистку turn без соответствующего `ItemCompleted`: незавершённая
    activity становится `Failed`, переносится в историю и больше не запрашивает
    `animation tick`.
14. Синхронизировать блок `fork-tests.v1` и schema artifacts через skill-owned
    workflow.

## Проверки

Исполняемая карта card-level regression tests:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "типизированная activity, lifecycle registry и безопасная detail-строка для трёх tools из default namespace и clock/curr_time",
      "argv": ["just", "test", "-p", "codex-core", "core_tool_activity"]
    },
    {
      "purpose": "группировка TUI, очистка lifecycle, ограниченная сводка dynamic tools и snapshots core tool activity",
      "argv": ["just", "test", "-p", "codex-tui", "core_tool_activity"]
    },
    {
      "purpose": "replay завершённой core tool activity из thread history",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-app-server-protocol",
        "core_tool_activity"
      ]
    },
    {
      "purpose": "protocol и app-server item model для core tool activity",
      "argv": ["just", "test", "-p", "codex-protocol"]
    },
    {
      "purpose": "analytics reducer учитывает core tool activity",
      "argv": ["just", "test", "-p", "codex-analytics"]
    },
    {
      "purpose": "метрики сохранения классифицируют вложенную core tool activity",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-rollout",
        "filtered_core_tool_activity_completion_includes_its_nested_item_type"
      ]
    },
    {
      "purpose": "отсутствие непреднамеренных изменений TUI snapshots",
      "argv": [
        "cargo",
        "insta",
        "pending-snapshots",
        "--manifest-path",
        "codex-rs/tui/Cargo.toml"
      ]
    }
  ]
}
```

Дополнительно обязателен `fork generators`, поскольку доработка меняет
app-server v2 schema и wire enums.

## Риски и ограничения

- Нужно не смешать user-facing activity item с model-visible
  `FunctionCallOutput`: первое нужно пользователю, второе нужно модели.
- Нельзя подменять provenance: `read_file` не должен выглядеть как shell command.
- TUI grouping не должен зависеть от nested code-mode executor-а: после
  `DirectModelOnly` экспозиции прямой `FunctionCall(read_file)` обязан проходить
  тот же `CoreToolActivity` lifecycle и оставаться внутри смешанного exploration
  tail.
- Компактная история намеренно не показывает точный диапазон строк; точный
  `path` и диапазоны строк нужно искать в structured payload или diagnostics, а
  не в обычной строке истории.
- Replay сохраненной thread history может видеть только completed item без
  отдельного start item; TUI grouping обязан учитывать этот persisted shape, а
  не только live lifecycle.
- Завершения `exec` могут приходить рядом с active core `File`; TUI не должен
  выводить из такого завершения, что текущий `File` поток уже можно flush-ить.
- `ItemCompleted` может не дойти до TUI при отмене future или завершении turn;
  общая очистка обязана завершить такую activity как `Failed`, иначе уже
  перенесённая в историю ячейка продолжит считаться активной и анимироваться.
- Выбран item уровня protocol, поэтому нужно поддерживать app-server v2 schema,
  replay/history и обратную совместимость.
- Динамическое значение времени может сделать snapshots нестабильными. Базовый
  snapshot должен проверять label `System time` и детерминированный detail из
  аргументов, а не реальное текущее время.
- Видимость не должна раскрывать лишний полный path, если текущий TUI обычно
  показывает workspace-relative или compact path.
- `read_file detail` нельзя вычислять через primary cwd, если handler выбрал
  другую environment: host-native разбор foreign path может показать соседний
  сегмент вместо basename реально прочитанного файла.
- Интеграционный тест registry покрывает успешный `read_file` и согласованную пару
  `ItemStarted`/`ItemCompleted`; отдельный сценарий ошибки остаётся непокрытым.
- Нельзя проверять default tool только как `namespace == None`: upstream может
  заранее нормализовать его в `functions`. При этом любой иной namespace должен
  оставаться исключённым, кроме точной пары `clock/curr_time`; это исключение не
  должно делать видимыми `clock/sleep` или одноимённый extension tool.
