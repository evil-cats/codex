---
id: fork-core-read-file-tool
status: active
created: 2026-07-03
updated: 2026-08-23
---

# Утилитарный core tool `read_file`

## Обзор

Эта карточка владеет fork-доработкой Hermione, которая добавляет `read_file`:
встроенный core tool для чтения известного текстового UTF-8 файла целиком или
по диапазону строк без запуска shell-команд чтения вроде `cat`, `sed -n`,
`nl`, `head` или `tail`. Карточка также владеет согласованным расширением,
которое не дублирует содержимое файла, когда один прежний успешный output уже
полностью содержит запрошенный диапазон в активном model-visible context.

Карточка нужна для переноса доработки на новый upstream checkout, проверки
контракта и восстановления причин решений без истории чата.

## Зачем это нужно

Codex до этой доработки читает файлы через shell-команды. Для модели это имеет
несколько плохих свойств:

- `cat`, `sed -n`, `nl`, `head` и `tail` являются обычным stdout, а не
  типизированным чтением файла;
- большой stdout попадает в общий механизм excerpt/spill-log, после чего модель
  видит только обрывок вывода и путь к сохраненному логу;
- вложение file-read в code-mode `exec` скрывает отдельный typed
  `FunctionCall(read_file)` внутри `custom_tool_call(exec)` и лишает runtime
  достоверной пары call/output для контекстной дедупликации;
- TUI показывает `Read` по эвристическому разбору shell-команды, а не потому,
  что был вызван отдельный file-read tool;
- при чтении документов и fork-карточек легко ошибочно принять неполный excerpt
  за прочитанный документ;
- shell-команды не дают стабильного встроенного признака, какие строки были
  запрошены, какие реально возвращены и полон ли результат.

`read_file` нужен как отдельный агентский tool с собственным лимитом и явным
контрактом полноты чтения. Он должен стать стандартным путем для чтения уже
выбранного файла или диапазона файла. Поиск файлов и мест в коде остается за
`rg`, `rg --files`, `git grep` и похожими поисковыми командами.

После первого чтения текст файла уже занимает model context. Повторная выдача
того же содержимого увеличивает историю без добавления сведений. Модель при
этом не может надежно определить, пережил ли прежний tool output compaction:
summary может упомянуть файл, но не гарантирует наличие его точного текста.

Решение должен принимать host-side runtime, который видит фактическую историю
следующего inference и идентификатор текущего context window. История сама по
себе недостаточна: любая смена `window_id` аннулирует прежнее coverage. Это
экономит model context внутри окна, но не отменяет чтение актуального файла из
filesystem: повторный вызов обязан проверить, что прежний текст не устарел.

## Карта файлов

Owner-файлы реализации:

| Файл | Ответственность |
| --- | --- |
| `codex-rs/core/src/tools/handlers/read_file.rs` | Runtime: direct-only model exposure, path, sandbox/read permissions, строки, диапазоны, content token budget, line-number rendering и ошибки |
| `codex-rs/core/src/tools/handlers/read_file_spec.rs` | Spec Responses API tool: имя `read_file`, аргументы, model-visible description и текстовый output contract с согласованным header |
| `codex-rs/core/src/tools/handlers/read_file_tests.rs` | Unit tests runtime-контракта: диапазоны, right-tail line trimming, long line, пустой файл и `line_numbers=false` |
| `codex-rs/core/src/tools/handlers/read_file_spec_tests.rs` | Tests spec-контракта: имя tool, default `line_numbers`, отсутствие argument для token limit и описание поведения `complete=no` |
| `codex-rs/core/src/tools/handlers/mod.rs` | Подключает handler и spec-модуль |
| `codex-rs/core/src/tools/spec_plan.rs` | Регистрирует `ReadFileHandler` через текущий `ToolRegistry` рядом с core utility tools |
| `codex-rs/core/src/tools/spec_plan_tests.rs` | Проверяет visibility для environment-backed tools и `DirectModelOnly`: `read_file` остается прямым при code mode и не входит в описание nested `exec` |
| `codex-rs/config/src/config_toml.rs` | Добавляет TOML config `[tools.read_file].content_max_tokens` |
| `codex-rs/core/src/config/mod.rs` | Добавляет effective config field, default `10_000` и resolver для лимита `read_file` |
| `codex-rs/core/src/config/config_tests.rs` | Проверяет deserialization, default и rejection невалидного лимита |
| `codex-rs/core/tests/suite/tools.rs` | Интеграционное покрытие: tool доступен при local environment, отсутствует без environment, следует environment выбранного шага и возвращает line metadata для UTF-8 fixture |
| `codex-rs/core/tests/suite/code_mode.rs` | Интеграционно проверяет фактический Responses request: `read_file` остается отдельным tool в code-mode-only surface и отсутствует в описании `exec` |
| `codex-rs/core/src/session/mod.rs` | Передаёт handler-у ключ `Session::current_window_id()`, последовательно записывает пары `FunctionCall`/`FunctionCallOutput` через `ContextManager` и восстанавливает оконный provenance после rollout replay |
| `codex-rs/core/src/session/rollout_reconstruction.rs` | Отделяет call IDs из replacement-history последней сохранившейся compaction от вызовов в хвосте текущего окна и последовательно восстанавливает typed items через тот же `ContextManager`, что используется live path |
| `codex-rs/core/src/session/rollout_reconstruction_tests.rs` | Проверяет маркировку вызовов, принесённых replacement-history, и replay-stable history policy при resume-реконструкции |
| `codex-rs/core/src/context_manager/history.rs` | Применяет выбранный `ToolOutputHistoryPolicy`: сохраняет уже ограниченный `read_file` output и выполняет model-default truncation для остальных outputs |
| `codex-rs/core/src/context_manager/tool_output_history.rs` | Выбирает `ToolOutputHistoryPolicy` по typed `FunctionCall`/`FunctionCallOutput` через `call_id`, не разбирая текст output и не вводя отдельное persisted state |
| `codex-rs/core/src/context_manager/history_tests.rs` | Проверяет пару `FunctionCall(name=read_file)`/`FunctionCallOutput`, отсутствие анализа текста и сохранение общего truncation для outputs других tools |
| `codex-rs/core/config.schema.json` | Описывает `[tools.read_file].content_max_tokens` |

Owner-файлы контекстной дедупликации:

| Файл | Ответственность |
| --- | --- |
| `codex-rs/core/src/tools/handlers/read_file_context.rs` | Приватный helper: bounded provenance index для одного `window_id`, resolved `environment_id + PathUri`, консервативное восстановление после resume, поиск одного полностью покрывающего output и короткая ссылка |
| `codex-rs/core/src/tools/handlers/read_file_context_tests.rs` | Unit tests typed call/output, полного вложения, raw/numbered content, несовпадений, смены окна, compaction-replacement exclusion, cold-resume fallback и запрета reference chaining |
| `codex-rs/core/tests/suite/read_file_context.rs` | Integration coverage обычного результата, `already_in_context`, overlap/union, изменения файла, смены primary cwd, rollback, явной compaction и cold resume |
| `codex-rs/core/tests/suite/mod.rs` | Регистрирует отдельный integration test module контекстной дедупликации на non-Windows targets |

`ContextManager` и compaction-модули не должны получать долговечный file cache
только ради этой доработки. `ReadFileContextIndex`, прикреплённый к
`thread_extension_data`, хранит текущий `window_id` и bounded provenance по
`call_id`, но не содержит текст файла. В живой session смена `window_id`
сначала полностью очищает этот provenance; только новые content-bearing outputs
могут снова разрешить дедупликацию в новом окне.

После cold resume runtime восстанавливает provenance из rollout replay. Прямая
typed call/output пара из хвоста после последней сохранившейся compaction может
снова разрешить ссылку, если совпадают `path`, значение аргумента
`environment_id`, `line_numbers`, диапазон и точный актуальный текст. Call IDs,
которые входят в replacement-history самой compaction, явно исключаются:
перенос старой пары механизмом compaction не может продлить её доверие на новое
окно.
Summary или один лишь provenance ссылку также не разрешают.

Отдельная history policy для `read_file` является replay-stable и определяется
по typed связи `FunctionCall(name=read_file)` и `FunctionCallOutput` через
`call_id`. `ContextManager` ищет соответствующий call в фактической текущей
history; поэтому live-запись и последовательный rollout replay получают одно
поведение без отдельного cache. Анализ header/body вроде `ReadFile: ...` не
используется. Публичный wire-формат и persisted rollout не получают
fork-specific флаг: при resume та же связь восстанавливается из сохранённой
typed пары.

History хранит `ResponseItemEnvelope` с metadata, предназначенной только для
history. Политика `read_file` обрабатывает только вложенный `ResponseItem`,
сохраняет metadata исходного envelope без изменений и при replay передаёт
механизму сопоставления только вложенные raw items, не отбрасывая metadata из
восстанавливаемой history.

Намеренно не входит в MVP:

- замена поиска по репозиторию: для поиска остаются `rg`, `rg --files`,
  `git grep` и аналогичные команды;
- чтение списка каталога;
- чтение binary, non-UTF-8 или других неподдерживаемых файлов;
- запись файлов;
- редактирование файлов;
- чтение сохраненных exec spill-log как fallback-owner механизма;
- специальный UI renderer сверх обычного отображения tool call, если сам
  tool name уже достаточно явно показывает чтение файла;
- произвольный per-call token limit;
- сборка одного запроса из нескольких прежних outputs;
- частичный ответ только для непокрытого остатка пересекающихся диапазонов;
- принудительный аргумент вроде `delivery=content` для обхода host-side решения;
- кэш, который продолжает считать содержимое доступным после исчезновения
  исходного output из model-visible context.

## Итоговый контракт

### Вызовы и аргументы

Агентская модель вызова:

```text
read_file("path/to/file")
read_file("path/to/file", 10, 100)
read_file("path/to/file", 10, 100, line_numbers=false)
```

Концептуальные аргументы:

| Аргумент | Обязательность | Контракт |
| --- | --- | --- |
| `path` | обязателен | Путь к файлу в доступной workspace/sandbox области |
| `start_line` | опционален | 1-based начало диапазона; если задан, `end_line` тоже должен быть задан |
| `end_line` | опционален | 1-based конец диапазона включительно; если задан, `start_line` тоже должен быть задан |
| `line_numbers` | опционален | По умолчанию `true`; при `false` возвращается raw content без префиксов строк |

`token_limit`, `max_tokens`, `budget` и похожие параметры не должны быть
аргументами tool call. Лимит задается только config-ом.

### Model-visible описание

`read_file_spec.rs` должен задавать description, который прямо направляет
агента к `read_file` для чтения уже выбранных файлов и диапазонов:

```text
Read a known UTF-8 text file from the workspace, optionally by inclusive
1-based line range, without running a shell command. Use this tool to read a
selected text file or line range instead of shell readers such as cat, sed -n,
nl, head, or tail. Continue using rg/rg --files for search and discovery. The
result includes total/requested/returned line metadata and complete=yes/no;
content may be shortened only by dropping whole trailing lines to fit the
configured content token limit. If complete=no, continue with another range
before treating the requested content as fully read. A repeated read may return
Status: already_in_context with CoveredBy when one earlier content-bearing output
still present in the active model context fully covers the unchanged requested
text. Otherwise the complete newly requested file or range is returned normally;
do not assemble it from partial overlaps or multiple earlier outputs.
```

Описание аргументов:

| Аргумент | Model-visible description |
| --- | --- |
| `path` | `Path to a regular UTF-8 text file within the readable workspace/sandbox scope.` |
| `start_line` | `Optional 1-based inclusive start line. Must be provided together with end_line. Omit both start_line and end_line to request the whole file.` |
| `end_line` | `Optional 1-based inclusive end line. Must be provided together with start_line. Omit both start_line and end_line to request the whole file.` |
| `line_numbers` | `Optional. Defaults to true. When true, prefixes each returned line with its source line number. Set false only when raw file content is needed for exact copying, formatting, or comparison.` |

Эта справка не заменяет поиск по репозиторию: discovery остается за `rg`,
`rg --files`, `git grep` и аналогичными командами. Shell остается для реального
выполнения команд, metadata, проверок, сборки, тестов и операций, которые
`read_file` не поддерживает.

### Нормализация диапазона

Алгоритм:

```text
total_lines = count_lines(path)
requested_range = range.unwrap_or([1, total_lines])
returned_range = prefix_of(requested_range, content_token_limit)
```

Правила диапазона:

- `total_lines` вычисляется всегда, до нормализации диапазона;
- пустой файл имеет `total=0`, `requested=empty`, `returned=empty`,
  `complete=yes`;
- строки считаются 1-based;
- `start_line < 1` является ошибкой;
- `end_line < start_line` является ошибкой;
- `start_line > total_lines` является ошибкой для непустого файла;
- `end_line > total_lines` можно нормализовать до `total_lines`, но output
  должен честно показывать фактически запрошенный или нормализованный контракт;
- в MVP нет неявной семантики "читать до конца" через один заданный край
  диапазона.

### Лимит содержимого

Лимит:

- применяется только к содержимому файла;
- не включает header metadata;
- не включает служебные номера строк;
- сознательно допускает итоговый overhead от header и номеров строк сверх
  настроенного значения;
- по умолчанию равен `10_000` approximate tokens;
- настраивается только через config;
- является единственным владельцем усечения `read_file`: общая
  `tool_output_token_limit`/model history policy продолжает действовать для
  остальных tools, но не должна повторно изменять готовый `read_file` output.

Config key:

```toml
[tools.read_file]
content_max_tokens = 10000
```

Это утвержденное имя ключа и default для MVP.

### Усечение

Если запрошенный диапазон не помещается в лимит содержимого:

```text
while token_count(candidate_lines) > limit:
    remove last line from candidate_lines
```

Инварианты:

- возвращенный текст всегда является префиксом запрошенного диапазона по
  строкам;
- строка никогда не режется посередине;
- если даже первая строка диапазона превышает лимит, tool возвращает ошибку;
- `complete=no` означает, что агент должен продолжить чтение другим диапазоном
  или явно признать, что документ/диапазон прочитан не полностью.

### Header и output

Базовый header:

```text
ReadFile: <path>
Lines: total=<N> requested=<A-B|empty> returned=<C-D|empty> complete=<yes|no>
LineNumbers: <yes|no>

<content>
```

Полный результат:

```text
ReadFile: docs/example.md
Lines: total=240 requested=10-100 returned=10-100 complete=yes
LineNumbers: yes

10 | ...
...
100 | ...
```

Обрезанный справа результат:

```text
ReadFile: docs/example.md
Lines: total=240 requested=10-100 returned=10-87 complete=no
LineNumbers: yes

10 | ...
...
87 | ...
```

Отдельные поля `Truncated` и `next` не нужны: остаток полностью выводится из
`requested`, `returned` и `complete`.

Output должен оставаться текстовым: согласованный header и содержимое файла не
нужно заменять JSON-представлением.

`complete=yes` является end-to-end обещанием именно model-visible history, а
не только локальным результатом handler-а. После формирования такого output
никакой общий history layer не должен заменять его середину маркером
`…tokens truncated…`. Raw rollout с полным текстом не компенсирует потерю
данных в prompt следующего inference.

### Ошибки

Если первая строка диапазона превышает лимит:

```text
ReadFile: docs/example.md
Lines: total=240 requested=10-100 returned=empty complete=no
Error: line 10 exceeds ReadFile content token limit
```

Ошибки также нужны для:

- файла, который не найден;
- path, который нельзя читать из-за sandbox/permissions;
- директории вместо файла;
- binary, non-UTF-8 или другого неподдерживаемого файла;
- невалидного диапазона.

### Повторные чтения и активный контекст

Контекстная дедупликация применяется после чтения актуального файла и
нормализации нового диапазона. Она экономит model-visible output, но не является
filesystem cache и не разрешает отвечать по устаревшему снимку.

Runtime должен получить ту же нормализованную историю, которая используется
для следующего inference, и текущий `window_id`. Он сопоставляет только typed пары
`FunctionCall(name=read_file)` и content-bearing `FunctionCallOutput` по
`call_id`, всегда отвергая явно неуспешный output.
Append-only rollout, TUI history, compaction summary, пользовательское сообщение
или произвольное совпадение текста не доказывают, что содержимое файла доступно
модели.

Готовый typed output `read_file` должен попадать в prompt-equivalent history
без повторного применения model-default truncation. Это правило одинаково для
live-записи и reconstruction из persisted rollout. Общая policy сохраняется
для остальных function/custom tool outputs.

Смена значения `Session::current_window_id()` является безусловной границей
доверия. Этот ключ имеет форму `<thread_id>:<window_number>` и меняется при
переходе в новое context window. После этого ни один вызов прежнего окна не
может стать `CoveredBy`, даже если механизм compaction скопировал его typed
call/output пару в replacement-history. Первое чтение в новом окне
возвращает обычный полный результат и создаёт новое coverage только для этого
окна.

Один прежний вызов подходит для ссылки, только если одновременно выполнены все
условия:

- его content-bearing `FunctionCallOutput` всё ещё присутствует в активной
  prompt-equivalent history;
- его body присутствует полностью и не содержит результат последующего общего
  history truncation;
- вызов зарегистрирован в том же `window_id`, что и текущий запрос;
- в живой session сохранённый provenance относится к тому же выбранному
  environment и тому же разрешенному `PathUri` файла;
- после cold resume вызов находится в хвосте текущего окна, а не в
  replacement-history последней сохранившейся compaction, и совпадают
  сохранённые typed-аргументы `path` и `environment_id`;
- явно неуспешный output с `success=false` никогда не используется; после
  deserialize persisted rollout внутреннее поле `success` отсутствует, поэтому
  такой output допускается только через последующий строгий разбор формата и
  точное сравнение с актуальным файлом;
- `line_numbers` совпадает;
- прежний `returned` диапазон целиком содержит новый нормализованный
  `requested` диапазон;
- соответствующие строки прежнего output точно совпадают с актуальным
  содержимым файла;
- прежний output содержит сами строки файла, а не
  `Status: already_in_context`; цепочки ссылок запрещены.

При выполнении условий tool возвращает короткий результат без содержимого:

```text
ReadFile: docs/example.md
Status: already_in_context
Coverage: requested=40-80 available=1-240 complete=yes
LineNumbers: yes
CoveredBy: call_123
```

`available` сообщает фактический content-bearing диапазон вызова `CoveredBy`,
а `complete=yes` означает, что один этот output полностью покрывает текущий
запрос. `CoveredBy` всегда содержит ровно один `call_id`; список ссылок и
объединение диапазонов не допускаются.

Если хотя бы одно условие не выполнено, сохраняется обычный контракт
`ReadFile`: tool возвращает весь новый запрошенный диапазон либо весь файл с
обычным `Lines: ... returned=... complete=...` и применяет существующий content
token limit. В частности:

- полный прежний файл покрывает любой неизменившийся поддиапазон;
- прежний диапазон покрывает новый диапазон, целиком лежащий внутри него;
- частичное пересечение диапазонов не используется, и весь новый диапазон
  возвращается повторно;
- несколько прежних outputs не объединяются, даже если их union покрывает
  запрос;
- запрос полного файла после чтения только диапазона возвращает полный файл;
- после любой compaction содержимое возвращается заново независимо от состава
  replacement-history;
- после rollback содержимое возвращается заново, если прежний content-bearing
  output отсутствует в активной истории;
- после cold resume сохранённая прямая typed call/output пара текущего окна
  снова разрешает ссылку, если её логический source, rendering и актуальный
  текст совпадают;
- output старого binary или незавершённой реализации, в котором header
  сохранил `complete=yes`, но body был усечён общим history layer, не создаёт
  coverage и приводит к обычной полной выдаче;
- другое значение `line_numbers`, другой environment, другой файл или
  изменившиеся строки требуют обычного результата.

Model-visible description должен объяснять `Status: already_in_context` как
host-side подтверждение доступности точного текста. Отдельный аргумент для
принудительной повторной выдачи содержимого не добавляется.

## Архитектурное решение

`read_file` является отдельным прямым core utility tool, а не shell-wrapper и
не вложенным executor-ом code-mode `functions.exec`. Handler публикует
`ToolExposure::DirectModelOnly`: Responses API получает обычный
`FunctionCall(read_file)`, но code-mode namespace `tools.*` этот tool не
содержит.

Причины:

- отдельный tool может иметь собственный content token limit;
- output может стабильно сообщать `total`, `requested`, `returned`,
  `complete` и `LineNumbers`;
- TUI и model-visible trace видят реальный tool call, а не эвристически
  распознанную shell-команду;
- handler может запретить частичные строки и выдавать blocker для слишком
  длинной первой строки;
- prompt может прямо требовать использовать `read_file` для вычитки выбранных
  файлов и диапазонов.

Общий `ContextManager` truncation остаётся защитой для произвольных и
потенциально неограниченных tool outputs. Для `read_file` он не является вторым
владельцем лимита: handler уже применяет config-only content budget и возвращает
структурированный line-based результат. Header и номера строк сознательно
считаются допустимым overhead этого результата.

## Порядок повторения при переносе

Для нового upstream checkout:

1. Найти текущую архитектуру core utility tools: handlers, spec modules,
   registry в `spec_plan.rs`, prompt tool coverage tests.
2. Добавить `read_file` handler и spec по локальным паттернам соседних tools;
   задать `ToolExposure::DirectModelOnly`, чтобы tool оставался прямым в
   model-visible surface и не попадал в code-mode `exec` namespace.
3. Добавить config-only content limit
   `[tools.read_file].content_max_tokens` с default `10_000` approximate tokens.
4. Реализовать подсчет `total_lines` до нормализации диапазона и усечения.
5. Реализовать line-based right-tail trimming без частичных строк.
6. Реализовать blocker для первой строки, превышающей content limit, как
   `ReadFile` output с `returned=empty`, `complete=no` и `Error: ...`.
7. Добавить model-visible description, который направляет агента использовать
   `read_file` для чтения выбранного файла или диапазона вместо `cat`,
   `sed -n`, `nl`, `head` и `tail`, оставляя `rg`/`rg --files` для поиска.
8. Реализовать MVP только для обычных текстовых UTF-8 файлов; binary, non-UTF-8 и
   неподдерживаемые файлы должны давать понятную ошибку.
9. Обновить tool visibility expectations: `read_file` должен быть
   environment-backed и не должен появляться без environment.
10. Добавить tests по runtime, spec, config, tool visibility и integration
    flow.
11. Обновить schema для нового config key.
12. Добавить приватный helper контекстной дедупликации рядом с handler; он должен
    анализировать prompt-equivalent history, текущий `window_id` и typed пары
    `read_file` call/output, а не summary или произвольный текст. Resolved
    `environment_id + PathUri` сохранять в bounded thread-local provenance
    index, который очищается по активной истории, полностью сбрасывается при
    смене окна и не содержит file content. Для cold resume восстанавливать
    только вызовы из хвоста текущего окна, исключая call IDs из compaction
    replacement-history, и требовать совпадения typed `path`/`environment_id`
    с точным сравнением актуального текста.
13. Реализовать ссылку только на один прежний content-bearing output, который
    полностью содержит новый диапазон; запретить reference chaining, union и
    частичную выдачу пересечений.
14. Дополнить model-visible description контрактом
    `Status: already_in_context` и полным fallback-ответом при любом
    несовпадении.
15. Добавить replay-stable per-tool history policy: готовый output
    `read_file` считать `AlreadyBounded` и не применять к нему model-default
    truncation, сохраняя общую policy для остальных tools.
16. Определять policy по typed `FunctionCall(name=read_file)` и
    `FunctionCallOutput` через `call_id`, а не по header/body; не добавлять
    fork-specific поле в публичный `ResponseItem` или persisted rollout.
17. Применять одинаковое правило при live-записи history и reconstruction из
    rollout, чтобы cold resume не менял доступность точного текста.
18. Добавить unit и integration coverage прямой model exposure без nested
    `exec`, повторного вызова, вложенного диапазона, изменения файла,
    безусловной инвалидации после compaction, rollback, cold resume,
    `line_numbers`, environment, неполного пересечения и большого output выше
    общей model policy, но ниже личного content budget `read_file`.
19. После поведенческого тестирования решить, нужны ли отдельные
    prompt/system/developer instructions помимо model-visible `read_file`
    description для выбора tool вместо shell readers.

## Проверки

Исполняемая карта card-level regression tests:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "direct read_file, bounded output и дедупликация только в активном context window",
      "argv": ["just", "test", "-p", "codex-core", "read_file"]
    },
    {
      "purpose": "read_file доступен модели напрямую и не публикуется во вложенном exec namespace",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "environment_count_controls_environment_backed_tools"
      ]
    }
  ]
}
```

Дополнительно обязателен `fork generators`, поскольку доработка меняет `ConfigToml` и config schema для `[tools.read_file].content_max_tokens`.

## Риски и ограничения

- `[tools.read_file].content_max_tokens` является единственным владельцем
  усечения file content. Header metadata и номера строк сознательно допускаются
  как overhead сверх этого budget.
- Общая model/history truncation policy сохраняется для остальных tools, но
  обходит готовый typed output `read_file` одинаково при live-записи и rollout
  replay. Выбор выполняется по typed call и `call_id`, а не по текстовому
  префиксу output.
- `[tools.read_file].content_max_tokens` намеренно отделен от
  `[tools.exec].inline_output_max_tokens`: exec excerpt и file-read content
  остаются разными лимитами.
- Approximate token count реализован через
  `codex_utils_output_truncation::approx_token_count`.
- `complete=no` остается обязательным сигналом неполного чтения; model-visible
  description прямо требует продолжить другим диапазоном до вывода о полном
  чтении.
- `read_file` не заменяет поиск по репозиторию: tool description оставляет
  discovery за `rg`, `rg --files` и похожими командами.
- `DirectModelOnly` намеренно исключает `read_file` из code-mode `exec`
  namespace. Если upstream изменит смысл tool exposure, регрессия должна быть
  заметна одновременно по фактическому Responses request и по отсутствию
  `read_file` в model-visible описании `exec`.
- Sandbox/path semantics реализованы через environment filesystem,
  `FileSystemSandboxContext` и обычные permission checks. `ReadFileHandler`
  выбирает environment из `ToolInvocation.step_context.environments` и
  разрешает `path` через `PathUri`, не преобразуя `cwd` чужого environment в
  путь локального хоста. Интеграционное покрытие проверяет чтение в local
  workspace с read-only permission profile и чтение из environment выбранного
  шага; более широкие границы доступа остаются ответственностью существующей
  filesystem abstraction и общего remote-environment gate.
- MVP намеренно ограничен обычными текстовыми UTF-8 файлами; расширение на другие
  форматы должно быть отдельным решением.
- После поведенческого тестирования фичи нужно вернуться к вопросу, нужны ли
  отдельные prompt/system/developer instructions помимо model-visible `read_file`
  description.
- Контекстная дедупликация экономит model context, но намеренно не экономит
  filesystem read: актуальное содержимое проверяется до решения о ссылке.
- Источником истины внутри одного окна являются текущий `window_id` вместе с
  prompt-equivalent history. Rollout, TUI history, summary и независимый cache
  не доказывают доступность точного текста модели, а смена окна безусловно
  аннулирует прежнее coverage.
- Только один прежний content-bearing output может покрыть запрос. Частичное
  пересечение и union нескольких outputs приводят к полной выдаче нового
  диапазона или файла; это сознательный расход токенов ради локального,
  самодостаточного результата.
- `ReadFileContextIndex` хранит `window_id` и provenance call IDs текущего окна.
  Он не хранит file content, полностью очищается при смене окна и не заменяет
  проверку, что исходный output реально присутствует в текущей model-visible
  history.
- После cold resume resolved source прошлого вызова уже недоступен. Fallback
  сознательно слабее live-session provenance: он допускает только call IDs из
  хвоста текущего окна, исключает compaction replacement-history и требует
  совпадения логических `path`/`environment_id`, прежнего content-bearing output
  и точного актуального текста запрошенных строк. При любом несовпадении
  возвращается полный результат.

## Открытые вопросы

- Нужно ли добавлять отдельные prompt/system/developer instructions помимо
  model-visible `read_file` description, если поведенческое тестирование
  покажет, что модель продолжает выбирать shell-команды чтения для уже
  известных файлов. Этот вопрос относится только к выбору tool: дедупликация
  повторного `read_file` является host-side контрактом и от prompt не зависит.
