---
id: fork-tui-core-tool-activity
status: active
created: 2026-07-04
updated: 2026-07-18
source_scope: working-tree
---

# Видимость core tools в TUI

## Обзор

Эта карточка владеет fork-доработкой Hermione со статусом `active`: выбранные
core function tools видимы в TUI как понятные пользовательские действия, а не
остаются скрытыми `FunctionCallOutput` и не отображаются только как внутреннее
имя tool.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Основной элемент protocol | `CoreToolActivity` |
| Файл core-логики | `codex-rs/core/src/tools/core_tool_activity.rs` |
| Renderer истории TUI | `codex-rs/tui/src/history_cell/core_tool_activity.rs` |
| Метка `fork tests` | `fork-tui-core-tool-activity` |

Первая область:

| Tool | Группа в TUI | Действие в TUI |
| --- | --- | --- |
| `read_file` | `Exploring` / `Explored` | `File` |
| `get_thread_info` | `Inspecting` / `Inspected` | `Thread info` |
| `get_system_time` | `Inspecting` / `Inspected` | `System time` |

Главный UI-контракт:

```text
• Exploring
  └ File core-read-file-tool.md
```

```text
• Explored
  └ File core-read-file-tool.md
```

Последовательные чтения файлов через `read_file` должны коалеситься в один
компактный блок истории по аналогии со старым renderer-ом
`Exploring/Explored -> Read` для shell-команд:

```text
• Explored
  └ File fork_cli.py, checks-and-gates.md, shell.md
```

Core `File` должен коалеситься не только с соседними core `File`, но и с
активным exploration-блоком shell-команд. Если последним незавершенным
history cell является `Exploring` или `Explored` с `Search`, `List` или `Read`,
новый `read_file` добавляется в него отдельной строкой `File`:

```text
• Explored
  └ Search as_any in history_cell
  └ File insert_history.rs
```

То же правило действует для активного состояния:

```text
• Exploring
  └ Search as_any in history_cell
  └ File insert_history.rs
```

Если `read_file` стартовал чуть раньше shell exploration-команды из-за
параллельного запуска tools, его active `Exploring -> File` не должен
закрепляться отдельной history-карточкой. При старте `Search`, `List` или
`Read` такой pending `File` переносится в новый exploration-блок и дальше
завершается там же:

```text
• Exploring
  └ Search impl in tui
  └ File model.rs
```

Если сохраненная thread history восстанавливает уже завершенные
`CoreToolActivity` items без отдельной live-фазы `InProgress`, соседние
completed-only `File` items тоже должны коалеситься. Два чтения разных диапазонов
одного файла не должны превращаться в две одинаковые строки:

```text
• Explored
  └ File HANDOFF.md
```

Если завершение несвязанной `exec`-команды приходит между параллельными
`read_file` calls, команда отображается отдельной history-строкой, но активный
`Exploring -> File` не сбрасывается и после завершения чтений остается одним
сгруппированным блоком:

```text
• Ran wc -l tasks tests/test_tasks_cli.py

• Explored
  └ File HANDOFF.md
```

```text
• Inspecting
  └ Thread info current
```

```text
• Inspected
  └ System time local
```

Внутренние имена `read_file`, `get_thread_info` и `get_system_time` остаются
доступны для диагностики, raw transcript, structured payloads и проверок, но не
должны быть основным названием действия в обычной истории TUI.

## Зачем это нужно

После добавления core function tools у агента появляется более точный путь для
простых операций без shell-команд: чтение файла, получение metadata текущего
thread и чтение текущего времени host. Но если эти вызовы не видны в TUI,
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
- `get_thread_info` и `get_system_time` также являются core function tools и
  могут оставаться невидимыми для пользователя;
- сырые подписи вроде `Tool read_file` или `Function get_system_time` раскрывают
  внутренний API вместо пользовательского действия;
- отсутствие snapshot-контракта делает будущую регрессию UI незаметной.

## Карта файлов

Владеющие файлы реализации:

| Файл или зона | Ответственность |
| --- | --- |
| `codex-rs/protocol/src/items.rs` | Добавляет `TurnItem::CoreToolActivity`, `CoreToolActivityItem`, `CoreToolActivityKind` и `CoreToolActivityStatus` как ограниченную структурированную поверхность activity |
| `codex-rs/protocol/src/legacy_events.rs` | Старый слой совместимости с legacy-событиями явно не материализует `CoreToolActivity` в `EventMsg`, чтобы новая UI-поверхность activity не меняла legacy/model-visible поток |
| `codex-rs/core/src/tools/core_tool_activity.rs` | Определяет сопоставление выбранных function tools с activity item, компактный `detail`, включая разрешение пути `read_file` через выбранную step environment, raw `arguments`, lifecycle started/completed и status |
| `codex-rs/core/src/tools/core_tool_activity_tests.rs` | Проверяет, что `read_file detail` использует `environment_id` и path convention выбранного foreign `PathUri`, а не primary cwd текущего host |
| `codex-rs/core/src/tools/registry.rs` | Оборачивает выполнение подходящих core function tools событиями `emit_turn_item_started` и `emit_turn_item_completed` без изменения model-visible `FunctionCallOutput` |
| `codex-rs/core/src/tools/mod.rs` | Подключает модуль `core_tool_activity` |
| `codex-rs/app-server-protocol/src/protocol/v2/item.rs` | Экспортирует v2 `ThreadItem::CoreToolActivity`, wire enums, `id()` и conversion из core `TurnItem` |
| `codex-rs/app-server-protocol/src/protocol/thread_history.rs` | Восстанавливает `CoreToolActivity` из `ItemStarted`/`ItemCompleted` при replay сохраненной thread history |
| `codex-rs/analytics/src/reducer.rs` | Явно игнорирует `CoreToolActivity` в analytics reducer, чтобы новый UI/history item не расширял telemetry contract этой карточкой |
| `codex-rs/tui/src/history_cell/core_tool_activity.rs` | Рисует человекочитаемые строки `Exploring/Explored -> File` и `Inspecting/Inspected -> Thread info/System time`; для `File` показывает короткие имена, убирает диапазоны строк и дедуплицирует повторы |
| `codex-rs/tui/src/exec_cell/model.rs` | Хранит core `File` как отдельную запись внутри существующего exploration cell, не маскируя `read_file` под shell command |
| `codex-rs/tui/src/exec_cell/render.rs` | Рисует смешанные exploration-блоки `Search`/`List`/`Read` + `File`, включая active `Exploring` и completed `Explored` |
| `codex-rs/tui/src/history_cell/mod.rs` | Экспортирует новый renderer history cell |
| `codex-rs/tui/src/history_cell/tests.rs` | Содержит `insta` snapshot-покрытие для active `read_file` и completed inspect tools |
| `codex-rs/tui/src/chatwidget/protocol.rs` | Направляет live `ItemStarted` для core activity в TUI lifecycle |
| `codex-rs/tui/src/chatwidget/replay.rs` | Восстанавливает active/completed core activity при replay turn items |
| `codex-rs/tui/src/chatwidget/command_lifecycle.rs` | При старте shell exploration-команды переносит уже активный core `File` в новый `ExecCell`, а при несвязанном завершении `exec` не сбрасывает активный in-progress `File` |
| `codex-rs/tui/src/chatwidget/tool_lifecycle.rs` | Управляет active cell, completion и резервным путем для completed activity, пришедшей не по порядку; коалесит последовательные `File` activity, completed-only replay и смешанные `Search`/`File` exploration-блоки в одну ячейку |
| `codex-rs/tui/src/thread_transcript.rs` | Рендерит persisted `CoreToolActivity` в transcript/history view |
| `codex-rs/tui/src/app/agent_status_feed.rs` | Показывает bounded summary `File`, `Thread info` или `System time` в `/agent` preview |
| `.codex/skills/fork/scripts/fork_cli.py` | Читает блоки `fork-tests.v1` из карточек и исполняет проверки через `fork tests` |
| `docs/fork/tui-core-tool-activity.md` | Владеющий handoff-артефакт: UI-контракт, перенос, проверки и ограничения fork-доработки |

Намеренно несемантический touch:

| Файл | Причина |
| --- | --- |
| `codex-rs/core/src/tools/handlers/read_file.rs` | Только форматирование существующего `collapsible_if`; runtime-контракт `read_file` не меняется |

Связанные карточки:

| Карточка | Связь |
| --- | --- |
| `docs/fork/core-read-file-tool.md` | Владеет runtime/model-visible контрактом `read_file`; эта карточка владеет только TUI activity surface |
| `docs/fork/core-thread-info-tool.md` | Владеет runtime/model-visible контрактом `get_thread_info`; эта карточка владеет новым TUI activity surface и не меняет аргументы tool |
| `docs/fork/core-system-time-tool.md` | Владеет runtime/model-visible контрактом `get_system_time`; эта карточка владеет новым TUI activity surface и не меняет аргументы tool |

Намеренно не входит в эту карточку:

- изменение runtime-контрактов `read_file`, `get_thread_info` или
  `get_system_time`;
- переименование самих tools;
- изменение model-visible descriptions этих tools;
- изменение telemetry contract или добавление отдельного analytics event для
  core tool activity;
- принудительный перевод всех function tools в видимые TUI items;
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

### `get_system_time`

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

### Raw и диагностика

Обычная история TUI использует labels `File`, `Thread info` и `System time`.
Технические имена tools должны оставаться доступны в structured payloads,
debug/raw transcript или раскрываемых details:

```text
read_file
get_thread_info
get_system_time
```

Это важно для диагностики, проверок, потребителей app-server и будущей отладки.
Но эти имена не должны становиться основной человекочитаемой подписью в compact
history.

## Архитектурное решение

Доработка должна добавить отдельную поверхность UI activity для выбранных core
function tools, а не притворяться shell execution.

Принятый подход:

- не создавать поддельный `ParsedCommand::Read` для `read_file`;
- не отправлять синтетическую shell-команду ради TUI;
- не показывать обобщенный `Tool read_file` как основной label;
- добавить структурированное сопоставление выбранного core tool call с
  пользовательской activity;
- для `read_file` повторить компактность старого shell `Read`: короткие имена
  файлов, группировка соседних чтений и дедупликация повторов;
- сохранять grouping при replay completed-only `CoreToolActivity` и при
  interleaving с завершением несвязанной `exec`-команды;
- оставить старую shell-модель `Exploring/Explored -> Read/List/Search` как
  самостоятельный renderer для exploration через exec.

Ожидаемая модель:

```text
FunctionCall(read_file args) -> CoreToolActivity(kind=File, group=Explore)
FunctionCall(get_thread_info args) -> CoreToolActivity(kind=ThreadInfo, group=Inspect)
FunctionCall(get_system_time args) -> CoreToolActivity(kind=SystemTime, group=Inspect)
```

Выбранная форма реализации:

| Уровень | Форма |
| --- | --- |
| Core protocol | `TurnItem::CoreToolActivity(CoreToolActivityItem)` |
| App-server v2 | `ThreadItem::CoreToolActivity` |
| Kind | `File`, `ThreadInfo`, `SystemTime` |
| Status | `InProgress`, `Completed`, `Failed` |
| Detail | Короткая строка, вычисленная из аргументов: basename файла для `read_file`, разрешенный через выбранную step environment, `current`, `local`, `utc` или фиксированный offset |
| Raw diagnostics | `tool_name` и `arguments` сохраняются в structured payload |

Главный инвариант: model-visible output остается `FunctionCallOutput`, а TUI
получает отдельный ограниченный по размеру user-facing activity item.

Отклоненные альтернативы:

| Альтернатива | Почему не выбрана |
| --- | --- |
| Показывать `Tool read_file` / `Tool get_system_time` | Слишком близко к внутреннему API и хуже текущего `Explored -> Read` UX |
| Маппить `read_file` в старый `Read` | Пользовательский контракт выбран как `Exploring/Explored -> File`; старый `Read` остается для shell-read |
| Притворяться shell-командой | Смешивает typed core tool с exec path и делает provenance ложным |
| Делать видимыми все function tools сразу | Слишком широкая область влияния; первая область ограничена тремя согласованными tools |
| Скрывать вызовы, полагаясь только на финальный ответ агента | Пользователь не видит, когда агент читает файл, thread metadata или время |
| Показывать workspace-relative путь и диапазоны строк в компактной истории | При нескольких чтениях экран превращается в журнал обращений; короткие имена файлов дают ту же наблюдаемость без лишнего шума |

## Порядок повторения при переносе

1. Проверить текущий upstream-путь TUI для shell exploration:
   `Exploring/Explored -> Read/List/Search` в renderer-е истории exec.
2. Проверить текущий путь обычных function tools:
   `FunctionCall` -> tool dispatch -> `FunctionCallOutput`.
3. Выбрать или перенести минимальную структурированную поверхность для выбранной
   core tool activity; текущая реализация использует `CoreToolActivity` item.
4. Реализовать mapping только для `read_file`, `get_thread_info` и
   `get_system_time`.
5. Сохранить model-visible outputs без изменений: результат tool call по-прежнему
   возвращается модели как `FunctionCallOutput`.
6. Добавить TUI rendering с точными labels из этой карточки.
7. Для `read_file` в компактной истории показывать короткое имя файла без
   диапазонов строк; коалесить соседние `File` activity в один блок без
   повторов и приклеивать `File` к текущему exploration-блоку shell
   `Search`/`List`/`Read`, если он еще активен в transcript tail. Если указан
   `environment_id`, разрешать путь через cwd той же step environment, которую
   выбирает handler, чтобы не интерпретировать foreign path по правилам primary
   host.
8. Поддержать completed-only replay: если сохраненная thread history уже
   схлопнула `ItemStarted`/`ItemCompleted` в completed item, соседние `File`
   entries должны группироваться так же, как live start/completion flow.
9. Защитить live interleaving: завершение несвязанной `exec`-команды не должно
   сбрасывать активный in-progress `File` и разделять группу параллельных
   чтений.
10. Добавить snapshot-покрытие для active и completed состояний:
    `Exploring/Explored -> File`, `Inspecting/Inspected -> Thread info`,
    `Inspecting/Inspected -> System time`, а также для сгруппированной
    `File` activity и смешанного `Search`/`File` exploration-блока.
11. Поддержать app-server v2 conversion и thread history replay, если переносимый
    upstream еще не знает `CoreToolActivity`.
12. Синхронизировать блок `fork-tests.v1`, schema artifacts и исторические результаты
    карточки через skill-owned workflow.

## Проверки

Карточка имеет статус `active`; исполняемая карта регрессионного покрытия
карточки живет в `fork tests` под меткой `fork-tui-core-tool-activity`.

### Смысловое покрытие

| Контракт | Обязательность | Где покрывается |
| --- | --- | --- |
| `read_file` отображается как `Exploring/Explored -> File` | `required` | TUI snapshot `active_core_tool_activity_read_file_snapshot` |
| `read_file` в компактной истории показывает короткое имя файла без префикса пути | `required` | TUI snapshots `active_core_tool_activity_read_file_snapshot` и сгруппированная `File` activity |
| `read_file` с явным `environment_id` берет basename по path convention выбранной step environment, включая foreign Windows `PathUri` | `required` | core test `read_file_detail_uses_selected_environment_path_convention` |
| `read_file` в компактной истории не показывает `:start-end` диапазоны строк | `required` | TUI snapshots для одиночной и сгруппированной `File` activity |
| Последовательные `read_file` calls коалессятся в один блок `File` | `required` | TUI lifecycle test `sequential_core_tool_activity_files_coalesce_in_active_cell` |
| `read_file` коалесится с текущим shell exploration-блоком `Search`/`List`/`Read` и для `Exploring`, и для `Explored` | `required` | TUI lifecycle test `core_tool_activity_file_coalesces_with_exec_exploration_cell` |
| Pending `read_file`, стартовавший до shell `Search`/`List`/`Read`, переносится в новый exploration-блок без отдельного stale `Exploring -> File` | `required` | TUI lifecycle test `core_tool_activity_file_started_before_exec_exploration_is_adopted` |
| Completed-only replay соседних `read_file` items коалесится в один `File` блок | `required` | TUI lifecycle test `completed_only_core_tool_activity_file_replay_coalesces_file_items` |
| Завершение несвязанной `exec`-команды между параллельными `read_file` calls не разделяет `File` группу | `required` | TUI lifecycle test `core_tool_activity_file_group_survives_exec_completion_between_parallel_reads` |
| Повторные чтения одного файла не повторяют имя в сгруппированном блоке `File` | `required` | TUI snapshot для сгруппированной `File` activity |
| `get_thread_info` отображается как `Inspecting/Inspected -> Thread info` | `required` | TUI snapshot `completed_core_tool_activity_inspect_tools_snapshot` |
| `get_system_time` отображается как `Inspecting/Inspected -> System time` | `required` | TUI snapshot `completed_core_tool_activity_inspect_tools_snapshot` |
| Обычный user-facing label не равен `read_file`, `get_thread_info` или `get_system_time` | `required` | TUI snapshots и renderer `CoreToolActivityCell` |
| Техническое имя tool сохраняется в structured payload, raw transcript или диагностике | `required` | `CoreToolActivityItem.tool_name`, `arguments`, `raw_lines` и app-server conversion |
| Старый shell-read renderer не переименован из `Read` в `File` без отдельного решения | `required` | существующие exec snapshots |
| Model-visible `FunctionCallOutput` для tools не меняется | `required` | mapping добавлен вокруг `ToolRegistry` lifecycle; handlers core tools не меняют output contract |
| Persisted thread history восстанавливает completed activity | `required` | app-server protocol test `rebuilds_core_tool_activity_from_item_lifecycle_events` |
| Analytics telemetry surface не расширяется этой UI-карточкой | `required` | `codex-analytics` явно игнорирует `CoreToolActivity` как отдельный telemetry item |

### Владелец исполняемой карты

| Владелец | Метка | Статус |
| --- | --- | --- |
| `fork tests` | `fork-tui-core-tool-activity` | `required`: исполняемая карта содержит core activity path detail, TUI snapshots, app-server replay, protocol item model, analytics reducer и pending snapshots |

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "core activity path detail",
      "argv": ["just", "test", "-p", "codex-core", "core_tool_activity"]
    },
    {
      "purpose": "tui snapshots and lifecycle",
      "argv": ["just", "test", "-p", "codex-tui", "core_tool_activity"]
    },
    {
      "purpose": "thread history replay",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-app-server-protocol",
        "core_tool_activity"
      ]
    },
    {
      "purpose": "protocol item model",
      "argv": ["just", "test", "-p", "codex-protocol"]
    },
    {
      "purpose": "analytics reducer",
      "argv": ["just", "test", "-p", "codex-analytics"]
    },
    {
      "purpose": "pending snapshots",
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

### Дополнительные gates

| Gate | Когда нужен | Статус |
| --- | --- | --- |
| `fork generators` | Реализация добавляет app-server v2 `ThreadItem::CoreToolActivity` и wire enums | `passed-current-pass` |
| `fork build-fast` | Нужен перед переносом или установкой binary с реализованным UI-поведением | `passed-current-pass` |

### Исторические результаты

| Проверка | Результат | Примечание |
| --- | --- | --- |
| Просмотр кода текущего пути `read_file` | `observed` | `read_file` является Responses function tool и возвращает `FunctionCallOutput`, отдельного TUI item нет |
| Просмотр кода текущего shell exploration renderer | `observed` | shell `cat`/`sed -n` отображаются через `Exploring/Explored -> Read` |
| Реализация `CoreToolActivity` в рабочем дереве | `implemented` | Новый item, сопоставление, TUI renderer, replay и snapshots добавлены в текущем diff |
| `.codex/skills/fork/scripts/fork format --fix` | `ok` | Форматирование применено |
| `.codex/skills/fork/scripts/fork format --check` | `ok` | Проверка форматирования прошла |
| `.codex/skills/fork/scripts/fork generators` | `ok` | Schema artifacts обновлены |
| `.codex/skills/fork/scripts/fork tests --mode list --card fork-tui-core-tool-activity` | `ok` | `fork-tests.v1` печатает TUI snapshots, app-server replay, protocol item model, analytics reducer и pending snapshots |
| `.codex/skills/fork/scripts/fork tests --mode cards --card fork-tui-core-tool-activity` | `ok` | Все пять card-level checks прошли |
| Дополнение 2026-07-05: компактная сгруппированная история `File` | `implemented` | `read_file` показывает короткие имена без диапазонов строк, группирует соседние `File` activity и дедуплицирует повторные имена |
| `.codex/skills/fork/scripts/fork format --fix` после дополнения 2026-07-05 | `ok` | Форматирование применено |
| `.codex/skills/fork/scripts/fork tests --mode list --card fork-tui-core-tool-activity` после дополнения 2026-07-05 | `ok` | `fork-tests.v1` по-прежнему печатает TUI snapshots, app-server replay, protocol item model, analytics reducer и pending snapshots |
| `.codex/skills/fork/scripts/fork tests --mode cards --card fork-tui-core-tool-activity` после дополнения 2026-07-05 | `ok` | Все пять card-level checks прошли |
| `.codex/skills/fork/scripts/fork format --check` после дополнения 2026-07-05 | `ok` | Проверка форматирования прошла; перед commit повторно пройдена |
| Дополнение 2026-07-05: смешанный exploration-блок `Search`/`File` | `implemented` | `read_file` добавляется в текущий `ExecCell` exploration tail, поэтому completed-only `File` после `Search` и active `Exploring` остаются в одном блоке |
| `.codex/skills/fork/scripts/fork format --fix` после смешанного `Search`/`File` | `ok` | Форматирование применено |
| `.codex/skills/fork/scripts/fork tests --mode list --card fork-tui-core-tool-activity` после смешанного `Search`/`File` | `ok` | `fork-tests.v1` печатает TUI snapshots, app-server replay, protocol item model, analytics reducer и pending snapshots |
| `.codex/skills/fork/scripts/fork tests --mode cards --card fork-tui-core-tool-activity` после смешанного `Search`/`File` | `ok` | Все пять card-level checks прошли |
| `.codex/skills/fork/scripts/fork format --check` после смешанного `Search`/`File` | `ok` | Проверка форматирования прошла |
| `.codex/skills/fork/scripts/fork build-fast` после смешанного `Search`/`File` | `ok` | Release-fast binary собран и проверен |
| `.codex/skills/fork/scripts/fork install` после смешанного `Search`/`File` | `ok` | Установлен `/home/slader/.local/bin/codex-hermione` |
| Дополнение 2026-07-05: порядок `File start -> Search start` | `implemented` | Pending core `File` переносится из active `CoreToolActivityCell` в новый `ExecCell`, поэтому stale `Exploring -> File` не остается отдельной history-карточкой |
| `.codex/skills/fork/scripts/fork format --fix` после порядка `File start -> Search start` | `ok` | Форматирование применено |
| `.codex/skills/fork/scripts/fork tests --mode list --card fork-tui-core-tool-activity` после порядка `File start -> Search start` | `ok` | `fork-tests.v1` печатает TUI snapshots, app-server replay, protocol item model, analytics reducer и pending snapshots |
| `.codex/skills/fork/scripts/fork tests --mode cards --card fork-tui-core-tool-activity` после порядка `File start -> Search start` | `ok` | Все пять card-level checks прошли |
| `.codex/skills/fork/scripts/fork format --check` после порядка `File start -> Search start` | `ok` | Проверка форматирования прошла |
| `.codex/skills/fork/scripts/fork build-fast` после порядка `File start -> Search start` | `ok` | Release-fast binary собран и проверен |
| `.codex/skills/fork/scripts/fork install` после порядка `File start -> Search start` | `ok` | Установлен `/home/slader/.local/bin/codex-hermione` |
| Live TUI smoke после перезапуска 2026-07-05 | `ok` | Подтверждены отдельный одиночный `File`, последующая grouped-строка, short names, отсутствие line ranges и dedupe повторного файла |
| Дополнение 2026-07-06: completed-only replay и interleaving с `exec` completion | `implemented` | Два completed-only `File` items с разными диапазонами одного файла коалессятся; несвязанное завершение `exec` между параллельными `read_file` calls не flush-ит in-progress `File` |
| `.codex/skills/fork/scripts/fork format --fix` после исправления completed-only replay | `ok` | Форматирование применено |
| `.codex/skills/fork/scripts/fork tests --mode list --card fork-tui-core-tool-activity` после исправления completed-only replay | `ok` | `fork-tests.v1` печатает TUI snapshots and lifecycle, app-server replay, protocol item model, analytics reducer и pending snapshots |
| `.codex/skills/fork/scripts/fork tests --mode cards --card fork-tui-core-tool-activity` после исправления completed-only replay | `ok` | Все пять card-level checks прошли |
| `.codex/skills/fork/scripts/fork format --check` после исправления completed-only replay | `ok` | Проверка форматирования прошла |
| `.codex/skills/fork/scripts/fork build-fast` после исправления completed-only replay | `ok` | Release-fast binary собран и проверен |
| `.codex/skills/fork/scripts/fork install` после исправления completed-only replay | `ok` | Установлен `${HOME}/.local/bin/codex-hermione` |
| Миграционный проход `rust-v0.142.5`: сгенерированные артефакты схем v2 | `resolved-current-pass` | Убраны маркеры конфликтов в `ThreadItem.ts` и файлах JSON Schema; сохранены обе сгенерированные записи `definitions`: `ImagePreviewSize`, `LegacyAppPathString` и `McpToolCallAppContext` |
| Миграционный проход `rust-v0.143.0`: lifecycle и слой legacy-событий для `CoreToolActivity` | `resolved-current-pass` | `thread_history` объединяет upstream materialized lifecycle с `CoreToolActivity`; `legacy_events.rs` явно игнорирует `CoreToolActivity`, чтобы не добавлять legacy-событие или менять model-visible поток |
| Миграционный проход `rust-v0.143.0`: проверки подагента | `not-run-current-pass` | По явному ограничению текущего subagent-запуска проверки, генераторы, форматирование, сборка, `fork tests --mode list` и `fork cards validate` не запускались |
| Миграционный проход `rust-v0.144.1`: конфликты live/replay/history для `CoreToolActivity` | `resolved-current-pass` | Разрешены конфликты в `chatwidget/protocol.rs`, `chatwidget/replay.rs` и `thread_history.rs`: сохранен fork lifecycle `CoreToolActivity` и объединены upstream-изменения `WebSearch(item)`, `Extension(ImageGeneration)` и materialized review/extension items |
| Миграционный проход `rust-v0.144.1`: проверки подагента | `not-run-current-pass` | По явному ограничению текущего subagent-запуска проверки, генераторы, форматирование, сборка, `fork tests --mode list` и `fork cards validate` не запускались |
| `.codex/skills/fork/scripts/fork cards validate` после исправления completed-only replay | `ok` | Проверка карточек прошла: `cards_checked: 19`, `card_errors: 0` |
| `.codex/skills/fork/scripts/fork build-fast` | `ok` | Release-fast binary собран и проверен |
| `.codex/skills/fork/scripts/fork install` | `ok` | Установлен `/home/slader/.local/bin/codex-hermione` |
| Миграционный проход `rust-v0.144.4`: environment-aware `read_file detail` | `resolved-current-pass` | Activity выбирает ту же step environment по `environment_id`, что и handler, и получает basename через ее `PathUri`; добавлен regression test с foreign Windows cwd |
| Миграционный проход `rust-v0.144.4`: проверки подагента | `not-run-current-pass` | По ограничению subagent one-card flow форматирование, card-level tests, generators и сборка переданы parent-agent |
| Миграционный проход `rust-v0.144.5`: статическая сверка owner-файлов | `preserved-current-pass` | Upstream diff `rust-v0.144.4..rust-v0.144.5` не затрагивает owner-файлы карточки; модель событий core activity, model-visible output, распространение через protocol/app-server/schema/analytics, TUI lifecycle/render/replay/transcript и заявленное тестовое и snapshot-покрытие сохранены без правок кода |
| Миграционный проход `rust-v0.144.5`: проверки подагента | `not-run-current-pass` | По ограничению subagent one-card flow проверки уровня проекта, принятие snapshots, форматирование, генераторы и сборка не запускались; проверки карточки и общие gates переданы parent-agent |
| Миграционный проход `rust-v0.144.6`: статическая сверка owner-файлов | `preserved-current-pass` | Upstream diff `rust-v0.144.5..rust-v0.144.6` не затрагивает owner-файлы карточки; после merge сохранены begin/end lifecycle core activity, model-visible `FunctionCallOutput`, protocol/app-server item и history replay, analytics ignore, TUI live/replay lifecycle, grouping/rendering/transcript и заявленное test/snapshot coverage |
| Миграционный проход `rust-v0.144.6`: проверки подагента | `not-run-current-pass` | По ограничению subagent one-card flow tests, build, generators, format/fix, markdownlint и snapshot acceptance не запускались; card-level проверки и общие gates переданы parent-agent |

### Известные падения и пропуски

- До текущего миграционного прохода актуальных падений card-level checks,
  проверки карточек, форматирования и быстрой сборки не было.
- В миграционном проходе `rust-v0.144.6` правки кода не потребовались, а
  проверки уровня проекта не запускались по ограничению subagent one-card flow;
  их выполняет parent-agent в общем проверочном проходе.
- В ходе реализации уже исправлены промежуточные падения: отсутствующий
  `CoreToolActivity` в app-server thread history, exhaustive match в
  `codex-analytics`, неверный TUI test filter и внешний `.snap.new` вместо
  inline snapshot.
- Старые карточки `core-thread-info-tool.md` и `core-system-time-tool.md` пока
  содержат прежнюю формулировку о том, что отдельная TUI-поверхность не нужна.
  Эта карточка является owner artifact нового TUI-решения; старые карточки
  нужно синхронизировать отдельным рефакторингом, не смешивая его с этой
  реализацией.

## Runtime, сборка и установка

Runtime-проверка этой карточки должна подтверждать видимые пользователю строки,
а не только успешный вызов core tools. Минимальный автоматический слой находится
в TUI snapshot coverage; ручная проверка установленного бинарника нужна перед
установкой или release-fast переносом.

В последнем полном проверочном проходе release-fast binary был собран и проверен
wrapper-ом `fork build-fast`: `codex-rs/target/release-fast/codex`. Бинарник был
установлен wrapper-ом `fork install` в
`/home/slader/.local/bin/codex-hermione`. Миграционный subagent-проход
`rust-v0.144.6` эти gates не повторял.

## Риски и ограничения

- Нужно не смешать user-facing activity item с model-visible
  `FunctionCallOutput`: первое нужно пользователю, второе нужно модели.
- Нельзя подменять provenance: `read_file` не должен выглядеть как shell command.
- Компактная история намеренно не показывает точный диапазон строк; точный
  `path` и диапазоны строк нужно искать в structured payload или diagnostics, а
  не в обычной строке истории.
- Replay сохраненной thread history может видеть только completed item без
  отдельного start item; TUI grouping обязан учитывать этот persisted shape, а
  не только live lifecycle.
- Завершения `exec` могут приходить рядом с active core `File`; TUI не должен
  выводить из такого завершения, что текущий `File` поток уже можно flush-ить.
- Выбран item уровня protocol, поэтому нужно поддерживать app-server v2 schema,
  replay/history и обратную совместимость.
- Динамическое значение времени может сделать snapshots нестабильными. Базовый snapshot
  должен проверять label `System time` и детерминированный detail из аргументов,
  а не реальное текущее время.
- Старые карточки для `get_thread_info` и `get_system_time` имеют прежнее решение
  "TUI не меняется"; эта карточка уже владеет новым TUI surface, а старые
  карточки требуют отдельной синхронизации.
- Видимость не должна раскрывать лишний полный path, если текущий TUI обычно
  показывает workspace-relative или compact path.
- `read_file detail` нельзя вычислять через primary cwd, если handler выбрал
  другую environment: host-native разбор foreign path может показать соседний
  сегмент вместо basename реально прочитанного файла.

## Проверка покрытия

| Смысловой пункт | Статус | Где покрыто |
| --- | --- | --- |
| Новая доработка оформлена отдельно от runtime-контрактов core tools | `перенесено в карточку` | `Обзор`, `Карта файлов`, `Архитектурное решение` |
| `read_file` должен отображаться как `Exploring/Explored -> File` | `перенесено в карточку` | `Обзор`, `Итоговый контракт`, `Проверки` |
| `read_file` должен показывать в компактной истории короткие имена файлов без префикса пути | `перенесено в карточку` | `Обзор`, `Итоговый контракт`, `Проверки` |
| `read_file detail` должен использовать `environment_id` и path convention выбранной step environment | `перенесено в карточку` | `Итоговый контракт`, `Архитектурное решение`, `Проверки`, `Риски и ограничения` |
| `read_file` не должен показывать номера строк в компактной истории | `перенесено в карточку` | `Итоговый контракт`, `Проверки`, `Риски и ограничения` |
| Последовательные `read_file` calls должны коалеситься в один блок `File` | `перенесено в карточку` | `Обзор`, `Итоговый контракт`, `Архитектурное решение`, `Проверки` |
| `read_file` должен коалеситься с текущим shell exploration-блоком `Search`/`List`/`Read` для `Exploring` и `Explored` | `перенесено в карточку` | `Обзор`, `Итоговый контракт`, `Карта файлов`, `Проверки` |
| Pending `read_file`, стартовавший до shell exploration-команды, не должен оставлять отдельный stale `Exploring -> File` | `перенесено в карточку` | `Обзор`, `Итоговый контракт`, `Карта файлов`, `Проверки` |
| Completed-only replay соседних `read_file` items должен коалеситься в один `File` блок | `перенесено в карточку` | `Обзор`, `Итоговый контракт`, `Проверки`, `Риски и ограничения` |
| Несвязанное завершение `exec` между параллельными `read_file` calls не должно разделять `File` группу | `перенесено в карточку` | `Обзор`, `Итоговый контракт`, `Карта файлов`, `Проверки`, `Риски и ограничения` |
| Повторные чтения одного файла не должны повторять имя в сгруппированном блоке `File` | `перенесено в карточку` | `Итоговый контракт`, `Проверки` |
| `get_thread_info` должен отображаться как `Inspecting/Inspected -> Thread info` | `перенесено в карточку` | `Обзор`, `Итоговый контракт`, `Проверки` |
| `get_system_time` должен отображаться как `Inspecting/Inspected -> System time` | `перенесено в карточку` | `Обзор`, `Итоговый контракт`, `Проверки` |
| Основной TUI label не должен быть сырым function tool name | `перенесено в карточку` | `Итоговый контракт`, `Архитектурное решение` |
| Старый shell `Read` не переименовывается этой карточкой | `перенесено в карточку` | `Карта файлов`, `Проверки`, `Риски и ограничения` |
| Model-visible output core tools не меняется | `перенесено в карточку` | `Карта файлов`, `Итоговый контракт`, `Архитектурное решение` |
| Telemetry contract не расширяется этой UI-карточкой | `перенесено в карточку` | `Карта файлов`, `Итоговый контракт`, `Проверки` |
| `fork tests` должен владеть card-level запуском, а блок `fork-tests.v1` - данными запуска | `перенесено в карточку` | `Проверки`, `Владелец исполняемой карты` |
| Старые tool-карточки не правятся в этом diff | `перенесено в карточку` | `Карта файлов`, `Известные падения и пропуски`, `Риски и ограничения` |

## Открытые вопросы

- Вопрос о коалесинге последовательных core file activity calls закрыт
  2026-07-05: `read_file` должен группироваться по аналогии с exec exploration
  reads, но оставаться действием `File`, а не старым shell `Read`.
- Вопрос о коалесинге смешанного `Search`/`File` tail закрыт 2026-07-05:
  core `File` должен приклеиваться к последнему active exploration cell, если
  другой видимый history item еще не прервал поток.
- Вопрос о порядке `File start -> Search start` закрыт 2026-07-05: pending
  `File` переносится в новый shell exploration cell и не остается отдельной
  stale history-карточкой.
- Вопрос о completed-only replay и interleaving с завершением несвязанной
  `exec`-команды закрыт 2026-07-06: оба сценария сохраняют единый
  `Explored -> File` блок.
