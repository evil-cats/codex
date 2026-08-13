---
id: fork-core-read-file-tool
status: active
created: 2026-07-03
updated: 2026-08-13
source_scope: discussion-2026-07-03..discussion-2026-08-12
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

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Имя tool | `read_file` |
| Человеческое имя фичи | `ReadFile` |
| Crate | `codex-core` |
| Основной обработчик | `codex-rs/core/src/tools/handlers/read_file.rs` |
| Описание tool | `codex-rs/core/src/tools/handlers/read_file_spec.rs` |
| Регистрация | `codex-rs/core/src/tools/spec_plan.rs` |
| Экспозиция | `DirectModelOnly`: прямой model-visible tool без вложения в code-mode `exec` |
| Диапазон строк | опциональные `start_line` и `end_line`, только вместе |
| Номера строк | включены по умолчанию, отключаются через `line_numbers=false` |
| Лимит содержимого | только из config, не из аргументов tool call |
| Встроенный лимит содержимого | `10_000` approximate tokens |
| Header metadata | не входит в лимит содержимого |
| Усечение | только справа и только целыми строками |
| Слишком длинная первая строка | blocker/error, строка не режется |
| Контекстная дедупликация | реализована в `codex-core` |
| Условие ссылки | один прежний content-bearing output полностью содержит новый диапазон |
| Граница доверия | coverage действует только внутри значения `Session::current_window_id()` вида `<thread_id>:<window_number>`; смена окна безусловно его аннулирует |
| Неполное пересечение | возвращается весь запрошенный диапазон или файл |
| Source of truth | эта карточка и owner-файлы ниже |

Главный контракт:

- модель вызывает `read_file` как прямой верхнеуровневый
  `FunctionCall(name=read_file)`; tool не публикуется внутри code-mode
  `functions.exec` и не требует дополнительного слоя оркестрации;
- tool всегда сначала вычисляет `total_lines`;
- затем нормализует запрос к диапазону строк: явно переданному диапазону или
  полному диапазону `1-total_lines`;
- content token limit применяется только к содержимому файла, без header
  metadata и без номеров строк как служебного представления;
- если запрошенный диапазон не помещается, tool удаляет строки с конца
  результата целиком, пока возвращаемый текст не поместится;
- если первая строка запрошенного диапазона сама не помещается в лимит, tool
  возвращает ошибку/blocker и не возвращает частично обрезанную строку;
- `complete=no` запрещает агенту считать запрошенный диапазон полностью
  прочитанным;
- повторный вызов может вернуть короткий `Status: already_in_context` только
  тогда, когда один прежний output с фактическим содержимым целиком покрывает
  новый диапазон, всё ещё присутствует в фактической model-visible history и
  относится к тому же `window_id`;
- compaction является жёсткой границей: после смены `window_id` прежнее
  coverage не используется, даже если replacement-history содержит старую
  typed call/output пару;
- частичное пересечение никогда не приводит к сборке содержимого из нескольких
  мест контекста: tool возвращает обычный полный результат для нового запроса.

Контекстная дедупликация реализована 2026-08-12. Отдельным открытым хвостом
остается поведенческое тестирование:
после него нужно решить, нужны ли prompt/system/developer instructions помимо
model-visible описания `read_file`, чтобы модель выбирала этот tool вместо
shell-команд чтения.

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
| `codex-rs/core/src/tools/spec_plan.rs` | Регистрирует `ReadFileHandler` рядом с core utility tools |
| `codex-rs/core/src/tools/spec_plan_tests.rs` | Проверяет visibility для environment-backed tools и `DirectModelOnly`: `read_file` остается прямым при code mode и не входит в описание nested `exec` |
| `codex-rs/config/src/config_toml.rs` | Добавляет TOML config `[tools.read_file].content_max_tokens` |
| `codex-rs/core/src/config/mod.rs` | Добавляет effective config field, default `10_000` и resolver для лимита `read_file` |
| `codex-rs/core/src/config/config_tests.rs` | Проверяет deserialization, default и rejection невалидного лимита |
| `codex-rs/core/tests/suite/tools.rs` | Интеграционное покрытие: tool доступен при local environment, отсутствует без environment, следует environment выбранного шага и возвращает line metadata для UTF-8 fixture |
| `codex-rs/core/tests/suite/code_mode.rs` | Интеграционно проверяет фактический Responses request: `read_file` остается отдельным tool в code-mode-only surface и отсутствует в описании `exec` |
| `codex-rs/core/src/session/mod.rs` | Передаёт handler-у ключ `Session::current_window_id()` и восстанавливает оконный provenance после rollout replay |
| `codex-rs/core/src/session/rollout_reconstruction.rs` | Отделяет call IDs из replacement-history последней сохранившейся compaction от вызовов, записанных в хвосте текущего окна |
| `codex-rs/core/src/session/rollout_reconstruction_tests.rs` | Проверяет маркировку вызовов, принесённых replacement-history, при resume-реконструкции |
| `codex-rs/core/config.schema.json` | Regenerated schema для `[tools.read_file].content_max_tokens` |
| `docs/fork/core-read-file-tool.md` | Владеющий handoff-артефакт: контракт, перенос, проверки и ограничения fork-доработки |

Owner-файлы контекстной дедупликации:

| Файл | Статус | Ответственность |
| --- | --- | --- |
| `codex-rs/core/src/tools/handlers/read_file_context.rs` | `implemented` | Приватный helper: bounded provenance index для одного `window_id`, resolved `environment_id + PathUri`, консервативное восстановление после resume, поиск одного полностью покрывающего output и короткая ссылка |
| `codex-rs/core/src/tools/handlers/read_file_context_tests.rs` | `implemented` | Unit tests typed call/output, полного вложения, raw/numbered content, несовпадений, смены окна, compaction-replacement exclusion, cold-resume fallback и запрета reference chaining |
| `codex-rs/core/tests/suite/read_file_context.rs` | `implemented` | Integration coverage обычного результата, `already_in_context`, overlap/union, изменения файла, смены primary cwd, rollback, явной compaction и cold resume |
| `codex-rs/core/tests/suite/mod.rs` | `implemented extension` | Регистрирует отдельный integration test module контекстной дедупликации на non-Windows targets |

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
- по умолчанию равен `10_000` approximate tokens;
- настраивается только через config.

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

Согласованные решения:

| Решение | Статус | Обоснование |
| --- | --- | --- |
| Делать отдельный `read_file`, а не дисциплинировать использование `sed` | принято | Нужны отдельные лимиты, metadata полноты и tool identity без эвристик shell parser |
| Не проектировать это как внешний JSON-протокол | принято | Для агента это typed tool call, как `get_thread_info` и `get_system_time`; schema остается внутренней деталью Responses API |
| Публиковать `read_file` как `DirectModelOnly` | принято | Tool нужен модели только для загрузки текста в context; прямой call сохраняет typed history, а исключение из nested `exec` убирает лишний слой оркестрации |
| Всегда сначала вычислять `total_lines` | принято | Полный размер файла известен до нормализации диапазона и до усечения вывода |
| Если диапазон не передан, использовать `1-total_lines` | принято | У tool нет отдельного режима "читать весь файл"; есть только нормализованный диапазон |
| Диапазон задается парой `start_line`, `end_line` | принято | Оба края должны быть явными; варианты "только start" и "только end" не входят в MVP |
| `line_numbers=true` по умолчанию | принято | Номера строк помогают ссылаться на фрагменты, продолжать чтение и проверять coverage |
| Поддержать `line_numbers=false` | принято | Нужен raw-вывод для точного копирования, форматирования и сравнения |
| Не передавать token limit в tool call | принято | Лимит является настройкой config, чтобы агент не подгонял бюджет вручную |
| Встроенный лимит содержимого равен `10_000` approximate tokens | принято | Это отдельный лимит для file-read content, больше текущего inline exec excerpt |
| Точное имя config key | принято | Используется `[tools.read_file].content_max_tokens = 10000` |
| Header metadata не входит в token limit | принято | Лимит защищает именно содержимое файла, а служебная координатная информация должна быть стабильной |
| Номера строк не съедают content token budget | принято | Они являются представлением, а не содержимым файла |
| Усечение выполняется только справа целыми строками | принято | Модель никогда не получает поврежденную строку и не принимает обрубок за текст файла |
| Первая строка, превышающая лимит, является blocker/error | принято | Нельзя безопасно вернуть частичную строку; нужна явная ошибка |
| Не выводить `Truncated`/`next` в обычном header | принято | `requested`, `returned` и `complete` уже полностью задают остаток чтения |
| Ключи header писать по-английски | принято | Tool output должен быть стабильным, коротким и grep-friendly |
| Возвращать текстовый output с согласованным header, а не JSON | принято | Header уже спроектирован как readable contract; JSON добавил бы экранирование содержимого и не нужен для чтения файлов |
| MVP читает только обычные текстовые UTF-8 файлы | принято | Binary, non-UTF-8 и неподдерживаемые файлы должны давать понятную ошибку |
| Model-visible description должен направлять агента к `read_file` вместо shell-команд чтения | принято | Иначе модель может продолжить выбирать `cat`, `sed -n`, `nl`, `head` и `tail` по привычке |
| Отдельные prompt/system/developer instructions пересмотреть после поведенческого тестирования | принято | Для MVP достаточно model-visible description; усиление зависит от поведения модели |
| Решение о повторной выдаче принимает host-side runtime | принято | Модель не может надежно доказать, что точный прежний output доступен в текущем prompt; runtime проверяет историю и `window_id` |
| Считать смену `window_id` безусловной границей | принято | Compaction может оставить только summary либо иначе переписать history; старое coverage нельзя переносить в новое окно |
| При resume исключать call IDs из compaction replacement-history | принято | Это сохраняет дедупликацию для вызовов, реально выполненных после compaction, но не доверяет старым парам, перенесённым в replacement-history |
| Ссылаться только на один content-bearing output с полным вложением диапазона | принято | Модели не приходится собирать содержимое из разных мест истории |
| При неполном пересечении возвращать весь новый диапазон или файл | принято | Простота и локальность результата важнее экономии токенов на общей части диапазонов |
| Не объединять несколько прежних outputs | принято | Union-покрытие потребовало бы от модели восстанавливать единый текст из нескольких удаленных фрагментов контекста |
| Не добавлять `delivery=content` или другой force-флаг | принято | Host уже проверяет фактический prompt; escape hatch вернул бы бесконтрольное дублирование |
| Хранить resolved source identity отдельно от текста в живой session | принято | Bounded provenance index хранит только `environment_id + PathUri` и не заменяет проверку активного output; после cold resume runtime использует более консервативный fallback по совпавшим typed-аргументам и точному актуальному тексту |

Отклоненные альтернативы:

| Альтернатива | Почему не выбрана |
| --- | --- |
| Продолжать использовать `sed -n` маленькими диапазонами | Это остается shell stdout без metadata полноты и отдельного content limit |
| Публиковать `read_file` внутри code-mode `functions.exec` | В rollout остаётся только внешний `custom_tool_call(exec)`, поэтому отдельная typed call/output пара `read_file` недоступна host-side matcher-у; дополнительная оркестрация здесь не даёт пользы |
| Использовать exec spill-log как способ "прочитать весь файл" | Spill-log является артефактом большого вывода команды, а не typed file-read контрактом |
| Добавить per-call `token_limit` | Агент начнет подгонять лимиты вручную; лимит должен быть config-only |
| Резать текст по токенам внутри строки | Это возвращает поврежденный текст и создает риск ложного чтения |
| Добавить `Truncated` и `next` в header | Эти поля дублируют `requested`, `returned` и `complete` |
| Возвращать файл как JSON вместо текстового header/output | JSON усложняет чтение и экранирует содержимое, а header уже задает нужную metadata полноты |
| Полагаться на память или самооценку модели | Модель не знает, сохранился ли точный tool output после compaction или другой переписи истории |
| Считать compaction summary доказательством наличия файла | Summary может сохранить только вывод или упоминание и не гарантирует наличие точных строк |
| Доверять старой typed call/output паре после смены `window_id` | Наличие пары в replacement-history не доказывает, что compaction сохранила прежний model context без потерь |
| Хранить независимый cache прочитанных файлов между окнами истории | Cache способен пережить исчезновение текста из model-visible context и дать ложный `already_in_context` |
| Возвращать только непокрытые части пересекающихся диапазонов | Модели пришлось бы собирать единый запрос из старого и нового outputs |
| Объединять coverage нескольких вызовов | Покрытие было бы host-side полным, но model-side работа оставалась бы распределенной по нескольким местам контекста |
| Разрешить модели принудительно запросить повторный текст | Такой флаг подрывает детерминированную дедупликацию и воспроизводит исходную проблему |

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
15. Добавить unit и integration coverage прямой model exposure без nested
    `exec`, повторного вызова, вложенного диапазона, изменения файла,
    безусловной инвалидации после compaction, rollback, cold resume,
    `line_numbers`, environment и неполного пересечения.
16. После поведенческого тестирования решить, нужны ли отдельные
    prompt/system/developer instructions помимо model-visible `read_file`
    description для выбора tool вместо shell readers.
17. Обновить эту карточку: указать фактический статус реализации, owner-файлы и
    результаты проверок.

## Проверки

### Смысловое покрытие

| Контракт | Обязательность | Где покрывается |
| --- | --- | --- |
| Чтение маленького файла без диапазона возвращает `requested=1-total`, `returned=1-total`, `complete=yes` | `required` | `read_file` runtime tests |
| Чтение большого файла без диапазона возвращает префикс по строкам и `complete=no` | `required` | `read_file` runtime tests |
| Чтение заданного диапазона возвращает только запрошенные строки | `required` | `read_file` runtime tests |
| Overflow возвращает только префикс диапазона и только полные строки | `required` | `read_file` runtime tests |
| Первая строка диапазона больше лимита дает error/blocker | `required` | `read_file` runtime tests |
| Header metadata и `line_numbers=true` не уменьшают content budget | `required` | `read_file` runtime tests |
| `line_numbers=false` возвращает raw content без префиксов строк | `required` | `read_file` runtime tests |
| Пустой файл возвращает `total=0`, `requested=empty`, `returned=empty`, `complete=yes` | `required` | `read_file` runtime tests |
| Невалидные диапазоны дают понятные ошибки | `required` | `read_file` runtime tests |
| Config default равен `10_000`, key равен `[tools.read_file].content_max_tokens`, нулевой лимит отклоняется | `required` | config tests |
| Binary, non-UTF-8 и неподдерживаемые файлы дают понятную ошибку | `required` | `read_file` runtime и integration tests |
| Output остается текстовым и содержит согласованный header | `required` | runtime и integration tests |
| Tool description направляет агента к `read_file` вместо shell-команд чтения для выбранных файлов и диапазонов | `required` | spec tests |
| `read_file` является environment-backed: скрыт без environment и получает `environment_id` при multiple environments | `required` | tool visibility tests |
| В code mode `read_file` остаётся отдельным прямым tool и не публикуется внутри `exec` | `required, implemented` | spec-plan test `read_file_stays_direct_and_outside_code_mode` и integration test `code_mode_only_keeps_read_file_direct_and_outside_exec` |
| Tool реально вызывается через mocked Responses flow и возвращает line metadata для UTF-8 fixture | `required` | integration test |
| `read_file` разрешает `path` через environment выбранного шага без преобразования его `cwd` в путь локального хоста | `required` | integration test и общий remote-environment gate |
| Повторный запрос того же диапазона возвращает один `already_in_context` без текста | `required, implemented` | `read_file` context unit tests и integration test |
| Прежний полный файл или больший диапазон покрывает вложенный новый диапазон одним `CoveredBy` | `required, implemented` | context unit tests и integration test |
| Частично пересекающийся, больший или полный новый запрос возвращается целиком | `required, implemented` | context unit tests и integration test |
| Несколько прежних outputs не объединяются для покрытия одного запроса | `required, implemented` | context unit test и integration full-file-after-ranges scenario |
| Изменившееся содержимое, другой `line_numbers`, environment или `PathUri` отключают ссылку | `required, implemented` | context unit tests и integration tests изменения файла и смены primary cwd |
| Смена `window_id` после compaction безусловно запрещает прежний `CoveredBy`, даже если старая пара сохранена | `required, implemented` | unit tests `read_file_context_window_change_invalidates_retained_output`, `read_file_context_resume_excludes_compaction_replacement_calls` и integration test `read_file_context_compaction_requires_fresh_content` |
| Rollback без прежнего content-bearing output повторно возвращает содержимое | `required, implemented` | context unit test отсутствующего output и integration rollback test |
| Resume-реконструкция отличает call IDs из compaction replacement-history от хвоста текущего окна | `required, implemented` | `reconstruct_history_marks_read_file_calls_from_compaction_replacement` |
| Cold resume с сохранённой прямой typed call/output парой текущего окна восстанавливает `already_in_context` без долговечного file cache | `required, implemented` | context unit tests resume provenance и integration test `read_file_context_dedup_survives_cold_resume` |
| Reference output не может стать `CoveredBy` для следующего вызова | `required, implemented` | context unit tests |
| Поиск покрытия использует prompt-equivalent history, а не rollout или summary | `required, implemented` | context unit tests и integration assertion исходящего request body |
| Prompt/system/developer instructions сверх description решаются после поведенческого тестирования | `deferred` | open question |

### Владелец исполняемой карты

| Owner | Label | Статус |
| --- | --- | --- |
| `fork tests` | `fork-core-read-file-tool` | `required` |

Исполняемая карта card-level проверок хранится в блоке `fork-tests.v1`, который
читает `fork tests`. Карточка не является runbook запуска проверок.

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "runtime, direct exposure, and active-context dedup contract",
      "argv": ["just", "test", "-p", "codex-core", "read_file"]
    },
    {
      "purpose": "tool visibility",
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

### Дополнительные gates

| Gate | Когда нужен | Статус |
| --- | --- | --- |
| `fork generators` | Изменяется `ConfigToml` и schema для `[tools.read_file].content_max_tokens` | `required` |
| `fork build-fast` | Нужен финальный migration/build gate перед переносом или установкой binary | `passed` для `0.146.0` |

### Исторические результаты

| Проверка | Результат | Примечание |
| --- | --- | --- |
| `rust-v0.142.5` one-card migration audit | `доработано` | Разрешен конфликт слияния в `codex-rs/core/src/config/mod.rs` вокруг `resolve_read_file_content_max_tokens` и upstream `resolve_orchestrator_feature_enabled`; снят конфликтный import в `codex-rs/core/src/config/config_tests.rs`. Проверки не запускались: их выполняет родительский агент после прохода по карточкам |
| `rust-v0.143.0` one-card migration audit | `доработано` | Разрешен конфликт слияния в `codex-rs/core/src/tools/spec_plan_tests.rs`: ожидания видимости при нескольких окружениях сохраняют `read_file`, `view_image` и upstream `request_permissions`. Проверки не запускались: их выполняет родительский агент после прохода по карточкам |
| `rust-v0.144.4` one-card migration audit | `доработано` | `ReadFileHandler` переведен с устаревшего `turn.environments` на выбранный `step_context.environments`; `path` теперь разрешается через `PathUri` без преобразования `cwd` в путь локального хоста. Добавлен интеграционный тест выбора environment в `step_context`. Проверки не запускались: их выполняет родительский агент после прохода по карточкам |
| `rust-v0.144.5` one-card migration audit | `без доработки` | Контракт `read_file`, owner-файлы, config/schema, регистрация, visibility и integration coverage сохранились после merge; card-scoped конфликтов нет. Проверки не запускались: их выполняет родительский агент после прохода по карточкам |
| `rust-v0.144.6` one-card migration audit | `доработано` | Контракт `read_file`, owner-файлы, config/schema, регистрация, visibility и integration coverage сохранились после merge; card-scoped конфликтов нет. Добавлен integration regression test фактических handler error branches: понятный отказ для non-UTF-8 файла и directory/non-regular path. Test target в `fork-tests.v1` не изменился и включает новый тест по фильтру `read_file`. Проверки не запускались: их выполняет родительский агент после прохода по карточкам |
| `rust-v0.145.0` one-card migration audit | `без доработки` | Полный контракт `read_file`, owner-файлы, config/schema, регистрация, environment-backed visibility и integration coverage сохранились после merge. Конфликты в `codex-rs/core/src/config/mod.rs` и `codex-rs/core/src/tools/spec_plan.rs` не затрагивают card-owned участки и оставлены родительскому агенту. Проверки не запускались: их выполняет родительский агент после прохода по карточкам |
| `rust-v0.146.0` one-card migration audit | `доработано` | Разрешены card-owned конфликты в `codex-rs/config/src/config_toml.rs`, `codex-rs/core/src/config/mod.rs`, `codex-rs/core/config.schema.json` и `codex-rs/core/src/tools/spec_plan.rs`: контракт `read_file` сохранен вместе с upstream-настройкой `update_plan`. Card-owned config test адаптирован к новому полю `ToolsToml`; исполняемая карта тестов не изменилась. Проверки не запускались: их выполняет родительский агент после прохода по карточкам |
| Контекстная дедупликация, 2026-08-12 | `реализовано` | Handler всегда читает актуальный файл, затем ищет один content-bearing output в prompt-equivalent history; source provenance хранит только resolved `environment_id + PathUri`, а overlap, union, изменившийся файл, смена source и исчезнувший output приводят к полной выдаче |
| Прямая экспозиция и cold resume, 2026-08-12 | `реализовано` | `ReadFileHandler` использует `DirectModelOnly`, поэтому Responses API видит отдельный `FunctionCall(read_file)`, а code-mode `exec` больше не публикует nested `tools.read_file`; пустой после resume provenance index восстанавливает coverage из typed history и точного актуального текста |
| Первый прогон cold-resume regression, 2026-08-12 | `исправлено` | Resume test builder создавал новый temporary cwd, поэтому fixture отсутствовал. Тест теперь явно использует persisted cwd исходной session и удерживает его `TempDir` до конца проверки |
| Второй прогон cold-resume regression, 2026-08-12 | `исправлено` | Persisted `FunctionCallOutputPayload` теряет внутреннее поле `success` при deserialize. Matcher по-прежнему отвергает `Some(false)`, а `None` принимает только после разбора content-bearing `ReadFile` output и точного сравнения с текущим файлом |
| Первоначальный compaction regression test, 2026-08-12 | `ошибочный вывод, исправлено` | Вывод, будто local compaction сохранила content-bearing output и разрешает ссылку, был неверной интерпретацией неудачного теста. Точная причина прежнего результата не была доказана; сценарий `ThreadRollback` не заменяет обязательную проверку compaction |
| Оконная инвалидация после compaction, 2026-08-12 | `реализовано и проверено` | `ReadFileContextIndex` теперь привязан к `window_id`; смена окна очищает provenance, resume исключает call IDs из compaction replacement-history, а явный integration test дожидается завершения compaction и получает полный текст при следующем чтении |
| `fork tests --mode cards --card fork-core-read-file-tool`, 2026-08-12 | `passed` | Прошли runtime/active-context contract и tool visibility targets, включая unit, spec и integration coverage новой дедупликации |
| `fork format --fix`, 2026-08-12 | `passed` | Отформатированы Rust changes и вынесенный integration module |
| `fork format --check`, 2026-08-12 | `passed` | После scoped fix форматирование осталось чистым |
| `fork fix --package codex-core`, 2026-08-12 | `passed` | Scoped Clippy/fix gate завершился без card-owned исправлений |
| `fork build-fast`, 2026-08-12 | `passed` | На ветке `hermione-0.146.0` прошли branch check, migration map, release-fast build, binary metadata и version check |
| `fork tests --mode list --card fork-core-read-file-tool --card fork-tui-core-tool-activity` после direct-exposure доработки | `passed` | Исполняемая карта включает новые core direct/resume regressions и TUI lifecycle regression без добавления обходных команд |
| `fork tests --mode cards --card fork-core-read-file-tool --version 0.146.0` после исправления cold resume | `passed` | Оба core targets прошли, включая `42` tests по фильтру `read_file`, фактический code-mode Responses surface и cold resume |
| `fork fix --package codex-core --package codex-tui` после direct-exposure доработки | `passed` | Scoped Rust lint/fix завершился успешно; единственный несвязанный auto-fix `needless_borrow` в `config_tests.rs` исключён из card-owned diff |
| `fork format --check` и `fork cards validate` после direct-exposure доработки | `passed` | Форматирование чистое; проверены `27` карточек, `card_errors: 0` |
| `fork build-fast --version 0.146.0` после direct-exposure доработки | `passed` | Прошли branch check, migration map, release-fast build, binary metadata и version check; артефакт собран в `codex-rs/target/release-fast/codex` |
| `fork tests --mode cards --card fork-core-read-file-tool --version 0.146.0` после оконной инвалидации | `passed` | Прошли `46` tests по фильтру `read_file` и visibility target; integration test `read_file_context_compaction_requires_fresh_content` подтвердил полный post-compaction output и смену `x-codex-window-id` |
| `fork fix --package codex-core`, `fork format --check`, `fork cards validate` после оконной инвалидации | `passed` | Scoped lint завершился успешно; форматирование чистое; проверены `27` карточек, `card_errors: 0` |
| `fork build-fast --version 0.146.0` после оконной инвалидации, 2026-08-13 | `passed` | Прошли branch check, migration map, release-fast build, binary metadata и version check |
| `fork install`, 2026-08-13 | `passed` | Установлен `/home/slader/.local/bin/codex-hermione` версии `codex-cli 0.146.0+hermione`, размером `374974600` bytes; SHA-256 источника и установленного binary совпал: `9ab0cbb6d00228d2083500e25d58e9ebd3b18b73de8a6962720cedbe591c93f3` |
| `fork install`, 2026-08-12 | `passed` | Установлен `/home/slader/.local/bin/codex-hermione` версии `codex-cli 0.146.0+hermione`; SHA-256 источника и установленного binary совпал: `67974837a85f3d010bb98377aa7ca20e8c25a7ccebaffdacfc509cecb65165df` |
| `cargo check -p codex-core` | `passed` | Прошел до финальной правки `Error:` header; после финальной правки crate был снова проверен через Clippy |
| `just fmt` | `passed` | Прошел после финальных code changes |
| `just write-config-schema` | `passed` | Обновил `codex-rs/core/config.schema.json` |
| `just test -p codex-core read_file` | `passed` | Прошло `17` tests |
| `just test -p codex-core environment_count_controls_environment_backed_tools` | `passed` | Проверена visibility логика для environment-backed tools |
| `fork build-fast --version 0.141.0 --skip-branch-check` | `passed` | Собран binary `codex-rs/target/release-fast/codex` |
| `codex-hermione --version` после установки | `passed` | Установленный binary вернул `codex-cli 0.141.0+hermione`; предупреждение про PATH aliases связано с read-only filesystem и не заблокировало запуск |
| `just fix -p codex-core` | `passed` | Успешно проверил crate через Clippy после финальной правки |
| `just test -p codex-core` | `failed` | Широкий crate run упал на `6` tests вне `read_file` coverage; новый integration test в этом запуске прошел |

### Известные падения и пропуски

- Широкий crate test run падал на внешних для этой карточки tests:
  `config::config_loader_tests::codex_home_is_not_loaded_as_project_layer_from_home_dir`,
  `config::config_loader_tests::project_layers_disabled_when_untrusted_or_unknown`,
  `git_info_tests::resolve_root_git_project_for_trust_returns_none_outside_repo`,
  `realtime_context::tests::recent_work_section_groups_threads_by_cwd`,
  `realtime_context::tests::workspace_section_requires_meaningful_structure`,
  `suite::user_shell_cmd::user_shell_command_does_not_set_network_sandbox_env_var`.
- Полный workspace test suite не запускался: правила проекта требуют отдельного
  решения пользователя перед complete suite.
- Behavioral testing фактического выбора `read_file` моделью после появления
  tool description еще не выполнен; от него зависит решение про отдельную
  prompt/system/developer instructions.

## Runtime, сборка и установка

Текущая реализация direct exposure, контекстной дедупликации и безусловной
оконной инвалидации проверена 2026-08-12 в checkout
`/home/slader/Projects/evilcats/codex` на ветке `hermione-0.146.0`.
Card-scoped tests, scoped fix, format/card validation и `fork build-fast`
прошли; config/schema/dependency surface не менялся. `fork build-fast` и
`fork install` повторно выполнены 2026-08-13. Новый binary собран в
`codex-rs/target/release-fast/codex` и установлен в
`/home/slader/.local/bin/codex-hermione`; установленная версия —
`codex-cli 0.146.0+hermione`, размер — `374974600` bytes, SHA-256 —
`9ab0cbb6d00228d2083500e25d58e9ebd3b18b73de8a6962720cedbe591c93f3`.

Поведение базового tool подтверждено существующими unit/spec/config tests и
integration test `suite::tools::read_file_tool_reads_utf8_file_with_line_metadata`.
Контекстная дедупликация дополнительно подтверждена unit tests matcher-а и
отдельным integration module `suite::read_file_context`, который проходит через
mocked Responses flow и проверяет фактическую model-visible history.

В первоначальной реализации schema/generator change выполнялся через schema
generator. Текущая контекстная доработка не меняет schema или зависимости;
Bazel lock update не требуется. `codex-rs/core/BUILD.bazel` использует
`compile_data = glob(...)`, поэтому отдельное перечисление новых Rust modules
не требуется.

Исторический `fork build-fast --version 0.141.0 --skip-branch-check` прошел и
подтвердил:

- `release-fast build`;
- metadata binary;
- binary version.

Исторически собранный binary: `codex-rs/target/release-fast/codex`.

Установка версии `0.141.0` была выполнена атомарно через временный файл:

1. `codex-rs/target/release-fast/codex` скопирован в
   `/home/slader/.local/bin/codex-hermione.new`;
2. временная копия проверена через `--version`, `ls -l` и `file`;
3. `/home/slader/.local/bin/codex-hermione.new` заменил установленный
   `/home/slader/.local/bin/codex-hermione`.

Исторически установленный binary:

| Поле | Значение |
| --- | --- |
| Путь | `/home/slader/.local/bin/codex-hermione` |
| Версия | `codex-cli 0.141.0+hermione` |
| Размер | `324159496` bytes |
| BuildID | `f800a2568624fa83ccf94cae9f637340e6195e36` |
| Strip status | `stripped` |

## Риски и ограничения

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

## Проверка покрытия

| Смысловой пункт | Статус | Где покрыто |
| --- | --- | --- |
| Отдельный `read_file` вместо shell-чтения | `перенесено в карточку` | `Зачем это нужно`, `Архитектурное решение` |
| Вызов как typed tool, а не внешний JSON-протокол | `перенесено в карточку` | `Итоговый контракт`, `Архитектурное решение` |
| Прямая `DirectModelOnly` экспозиция без вложения в code-mode `exec` | `перенесено в карточку` | `Обзор`, `Карта файлов`, `Архитектурное решение`, `Проверки` |
| `total_lines` вычисляется всегда первым | `перенесено в карточку` | `Обзор`, `Итоговый контракт` |
| Диапазон отсутствует -> `1-total_lines` | `перенесено в карточку` | `Итоговый контракт` |
| Диапазон задается `start_line` + `end_line` | `перенесено в карточку` | `Итоговый контракт` |
| `line_numbers=true` по умолчанию | `перенесено в карточку` | `Обзор`, `Итоговый контракт` |
| `line_numbers=false` для raw content | `перенесено в карточку` | `Итоговый контракт` |
| Token limit только в config | `перенесено в карточку` | `Обзор`, `Итоговый контракт`, `Риски и ограничения` |
| Default token limit `10_000` | `перенесено в карточку` | `Обзор`, `Итоговый контракт` |
| Config key `[tools.read_file].content_max_tokens` | `перенесено в карточку` | `Итоговый контракт`, `Карта файлов` |
| Header metadata не входит в лимит | `перенесено в карточку` | `Обзор`, `Итоговый контракт` |
| Номера строк не входят в content budget | `перенесено в карточку` | `Итоговый контракт`, `Проверки` |
| Обрезка справа только целыми строками | `перенесено в карточку` | `Обзор`, `Итоговый контракт` |
| Одна строка больше лимита -> blocker/error | `перенесено в карточку` | `Обзор`, `Итоговый контракт` |
| Header без `Truncated`/`next` | `перенесено в карточку` | `Итоговый контракт`, `Архитектурное решение` |
| Ключи header на английском | `перенесено в карточку` | `Итоговый контракт`, `Архитектурное решение` |
| Текстовый output с согласованным header, не JSON | `перенесено в карточку` | `Итоговый контракт`, `Архитектурное решение` |
| MVP только для обычных текстовых UTF-8 файлов | `перенесено в карточку` | `Карта файлов`, `Риски и ограничения` |
| Model-visible description направляет к `read_file` вместо shell-команд чтения | `перенесено в карточку` | `Итоговый контракт`, `Проверки` |
| Данные исполняемой карты перенесены в блок `fork-tests.v1`; `fork tests` владеет запуском | `перенесено в карточку` | `Проверки` |
| Вернуться к отдельным prompt/system/developer instructions после поведенческого тестирования | `перенесено в карточку` | `Обзор`, `Проверки`, `Риски и ограничения` |
| Host-side решение по фактической model-visible history | `перенесено в карточку` | `Повторные чтения и активный контекст`, `Архитектурное решение` |
| Ссылка только на один content-bearing output с полным вложением диапазона | `перенесено в карточку` | `Повторные чтения и активный контекст`, `Проверки` |
| Полная выдача при частичном пересечении, большем диапазоне или полном файле | `перенесено в карточку` | `Повторные чтения и активный контекст`, `Архитектурное решение` |
| Запрет union нескольких outputs и reference chaining | `перенесено в карточку` | `Повторные чтения и активный контекст`, `Проверки` |
| Безусловная повторная выдача после смены `window_id` при compaction | `перенесено в карточку` | `Повторные чтения и активный контекст`, `Архитектурное решение`, `Проверки`, `Риски и ограничения` |
| Повторная выдача после rollback при исчезновении content-bearing output | `перенесено в карточку` | `Повторные чтения и активный контекст`, `Проверки` |
| Восстановление дедупликации после cold resume только для хвоста текущего окна | `перенесено в карточку` | `Карта файлов`, `Повторные чтения и активный контекст`, `Проверки`, `Риски и ограничения` |
| Отсутствие `delivery=content` и долговечного cache доступности | `перенесено в карточку` | `Архитектурное решение`, `Риски и ограничения` |
| Новая реализация и regression tests | `перенесено в карточку` | `Карта файлов`, `Проверки`, `Исторические результаты` |

## Открытые вопросы

- Нужно ли добавлять отдельные prompt/system/developer instructions помимо
  model-visible `read_file` description, если поведенческое тестирование
  покажет, что модель продолжает выбирать shell-команды чтения для уже
  известных файлов. Этот вопрос относится только к выбору tool: дедупликация
  повторного `read_file` является host-side контрактом и от prompt не зависит.
