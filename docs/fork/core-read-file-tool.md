---
id: fork-core-read-file-tool
status: active
created: 2026-07-03
updated: 2026-07-18
source_scope: discussion-2026-07-03
---

# Утилитарный core tool `read_file`

## Обзор

Эта карточка владеет fork-доработкой Hermione, которая добавляет `read_file`:
встроенный core tool для чтения известного текстового UTF-8 файла целиком или
по диапазону строк без запуска shell-команд чтения вроде `cat`, `sed -n`,
`nl`, `head` или `tail`.

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
| Диапазон строк | опциональные `start_line` и `end_line`, только вместе |
| Номера строк | включены по умолчанию, отключаются через `line_numbers=false` |
| Лимит содержимого | только из config, не из аргументов tool call |
| Встроенный лимит содержимого | `10_000` approximate tokens |
| Header metadata | не входит в лимит содержимого |
| Усечение | только справа и только целыми строками |
| Слишком длинная первая строка | blocker/error, строка не режется |
| Source of truth | эта карточка и owner-файлы ниже |

Главный контракт:

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
  прочитанным.

Открытый хвост после MVP: после поведенческого тестирования решить, нужны ли
отдельные prompt/system/developer instructions помимо model-visible описания
`read_file`.

## Зачем это нужно

Codex до этой доработки читает файлы через shell-команды. Для модели это имеет
несколько плохих свойств:

- `cat`, `sed -n`, `nl`, `head` и `tail` являются обычным stdout, а не
  типизированным чтением файла;
- большой stdout попадает в общий механизм excerpt/spill-log, после чего модель
  видит только обрывок вывода и путь к сохраненному логу;
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

## Карта файлов

Owner-файлы реализации:

| Файл | Ответственность |
| --- | --- |
| `codex-rs/core/src/tools/handlers/read_file.rs` | Runtime: path, sandbox/read permissions, строки, диапазоны, content token budget, line-number rendering и ошибки |
| `codex-rs/core/src/tools/handlers/read_file_spec.rs` | Spec Responses API tool: имя `read_file`, аргументы, model-visible description и текстовый output contract с согласованным header |
| `codex-rs/core/src/tools/handlers/read_file_tests.rs` | Unit tests runtime-контракта: диапазоны, right-tail line trimming, long line, пустой файл и `line_numbers=false` |
| `codex-rs/core/src/tools/handlers/read_file_spec_tests.rs` | Tests spec-контракта: имя tool, default `line_numbers`, отсутствие argument для token limit и описание поведения `complete=no` |
| `codex-rs/core/src/tools/handlers/mod.rs` | Подключает handler и spec-модуль |
| `codex-rs/core/src/tools/spec_plan.rs` | Регистрирует `ReadFileHandler` рядом с core utility tools |
| `codex-rs/core/src/tools/spec_plan_tests.rs` | Проверяет visibility для environment-backed tools: скрытие без environment, видимость с environment и `environment_id` |
| `codex-rs/config/src/config_toml.rs` | Добавляет TOML config `[tools.read_file].content_max_tokens` |
| `codex-rs/core/src/config/mod.rs` | Добавляет effective config field, default `10_000` и resolver для лимита `read_file` |
| `codex-rs/core/src/config/config_tests.rs` | Проверяет deserialization, default и rejection невалидного лимита |
| `codex-rs/core/tests/suite/tools.rs` | Интеграционное покрытие: tool доступен при local environment, отсутствует без environment, следует environment выбранного шага и возвращает line metadata для UTF-8 fixture |
| `codex-rs/core/config.schema.json` | Regenerated schema для `[tools.read_file].content_max_tokens` |
| `docs/fork/core-read-file-tool.md` | Владеющий handoff-артефакт: контракт, перенос, проверки и ограничения fork-доработки |

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
- произвольный per-call token limit.

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
before treating the requested content as fully read.
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

## Архитектурное решение

`read_file` является отдельным core utility tool, а не shell-wrapper.

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

Отклоненные альтернативы:

| Альтернатива | Почему не выбрана |
| --- | --- |
| Продолжать использовать `sed -n` маленькими диапазонами | Это остается shell stdout без metadata полноты и отдельного content limit |
| Использовать exec spill-log как способ "прочитать весь файл" | Spill-log является артефактом большого вывода команды, а не typed file-read контрактом |
| Добавить per-call `token_limit` | Агент начнет подгонять лимиты вручную; лимит должен быть config-only |
| Резать текст по токенам внутри строки | Это возвращает поврежденный текст и создает риск ложного чтения |
| Добавить `Truncated` и `next` в header | Эти поля дублируют `requested`, `returned` и `complete` |
| Возвращать файл как JSON вместо текстового header/output | JSON усложняет чтение и экранирует содержимое, а header уже задает нужную metadata полноты |

## Порядок повторения при переносе

Для нового upstream checkout:

1. Найти текущую архитектуру core utility tools: handlers, spec modules,
   registry в `spec_plan.rs`, prompt tool coverage tests.
2. Добавить `read_file` handler и spec по локальным паттернам соседних tools.
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
12. После поведенческого тестирования решить, нужны ли отдельные
    prompt/system/developer instructions помимо model-visible `read_file`
    description.
13. Обновить эту карточку: указать фактический статус, owner-файлы и
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
| Tool реально вызывается через mocked Responses flow и возвращает line metadata для UTF-8 fixture | `required` | integration test |
| `read_file` разрешает `path` через environment выбранного шага без преобразования его `cwd` в путь локального хоста | `required` | integration test и общий remote-environment gate |
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
      "purpose": "runtime contract",
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
| `fork build-fast` | Нужен финальный migration/build gate перед переносом или установкой binary | `passed` |

### Исторические результаты

| Проверка | Результат | Примечание |
| --- | --- | --- |
| `rust-v0.142.5` one-card migration audit | `доработано` | Разрешен конфликт слияния в `codex-rs/core/src/config/mod.rs` вокруг `resolve_read_file_content_max_tokens` и upstream `resolve_orchestrator_feature_enabled`; снят конфликтный import в `codex-rs/core/src/config/config_tests.rs`. Проверки не запускались: их выполняет родительский агент после прохода по карточкам |
| `rust-v0.143.0` one-card migration audit | `доработано` | Разрешен конфликт слияния в `codex-rs/core/src/tools/spec_plan_tests.rs`: ожидания видимости при нескольких окружениях сохраняют `read_file`, `view_image` и upstream `request_permissions`. Проверки не запускались: их выполняет родительский агент после прохода по карточкам |
| `rust-v0.144.4` one-card migration audit | `доработано` | `ReadFileHandler` переведен с устаревшего `turn.environments` на выбранный `step_context.environments`; `path` теперь разрешается через `PathUri` без преобразования `cwd` в путь локального хоста. Добавлен интеграционный тест выбора environment в `step_context`. Проверки не запускались: их выполняет родительский агент после прохода по карточкам |
| `rust-v0.144.5` one-card migration audit | `без доработки` | Контракт `read_file`, owner-файлы, config/schema, регистрация, visibility и integration coverage сохранились после merge; card-scoped конфликтов нет. Проверки не запускались: их выполняет родительский агент после прохода по карточкам |
| `rust-v0.144.6` one-card migration audit | `доработано` | Контракт `read_file`, owner-файлы, config/schema, регистрация, visibility и integration coverage сохранились после merge; card-scoped конфликтов нет. Добавлен integration regression test фактических handler error branches: понятный отказ для non-UTF-8 файла и directory/non-regular path. Test target в `fork-tests.v1` не изменился и включает новый тест по фильтру `read_file`. Проверки не запускались: их выполняет родительский агент после прохода по карточкам |
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

Проверки выполнялись локально в `/data/Projects/codex/codex-rs` и из repo root
`/data/Projects/codex` для обновления config schema.

Поведение подтверждено unit/spec/config tests в `codex-core` и integration test
`suite::tools::read_file_tool_reads_utf8_file_with_line_metadata`, который
вызывает `read_file` через mocked Responses flow и проверяет фактический tool
output.

Schema/generator change выполнен через schema generator; dependency changes и
Bazel lock updates не требовались. `codex-rs/core/BUILD.bazel` использует
`compile_data = glob(...)`, поэтому отдельное перечисление новых Rust modules
там не потребовалось.

`fork build-fast --version 0.141.0 --skip-branch-check` прошел и подтвердил:

- `release-fast build`;
- metadata binary;
- binary version.

Собранный binary: `codex-rs/target/release-fast/codex`.

Установка выполнена атомарно через временный файл:

1. `codex-rs/target/release-fast/codex` скопирован в
   `/home/slader/.local/bin/codex-hermione.new`;
2. временная копия проверена через `--version`, `ls -l` и `file`;
3. `/home/slader/.local/bin/codex-hermione.new` заменил установленный
   `/home/slader/.local/bin/codex-hermione`.

Установленный binary:

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

## Проверка покрытия

| Смысловой пункт | Статус | Где покрыто |
| --- | --- | --- |
| Отдельный `read_file` вместо shell-чтения | `перенесено в карточку` | `Зачем это нужно`, `Архитектурное решение` |
| Вызов как typed tool, а не внешний JSON-протокол | `перенесено в карточку` | `Итоговый контракт`, `Архитектурное решение` |
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

## Открытые вопросы

- Нужно ли добавлять отдельные prompt/system/developer instructions помимо
  model-visible `read_file` description, если поведенческое тестирование
  покажет, что модель продолжает выбирать shell-команды чтения для уже
  известных файлов.
