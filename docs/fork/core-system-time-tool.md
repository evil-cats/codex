---
id: fork-core-system-time-tool
status: active
created: 2026-06-09
updated: 2026-07-16
source_scope: 8fd6a41731b52d7aeee0eb369603404ccbc2e2fa..HEAD
---

# Утилитарный core tool `get_system_time`

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая добавляет
`get_system_time`: встроенный core tool для получения текущего времени host без
запуска shell-команды `date`.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Имя tool | `get_system_time` |
| Crate | `codex-core` |
| Основной обработчик | `codex-rs/core/src/tools/handlers/system_time.rs` |
| Описание tool | `codex-rs/core/src/tools/handlers/system_time_spec.rs` |
| Регистрация | `codex-rs/core/src/tools/spec_plan.rs` |
| Встроенный формат | `%H:%M` |
| Встроенный offset | `local` |
| Ответ по умолчанию | только поле `formatted` |
| Полный ответ | `full: true` |
| Карта prompt tool | `codex-rs/core/tests/suite/prompt_caching.rs` |
| Удаленный host сборки | `f-ms-dev:/home/slader/Projects/codex` |

Главное runtime-поведение:

- вызов `{}` возвращает короткий JSON вида `{"formatted":"23:16"}`;
- `format` принимает синтаксис `chrono` strftime, например `%H:%M`;
- `offset` по умолчанию равен `local` и означает локальное время host system;
- `offset` также принимает `utc`, `+HH:MM` и `-HH:MM`;
- IANA timezone names вроде `Europe/Moscow` не поддерживаются;
- `full: true` добавляет формат, выбранный offset, фактический offset и
  timestamp metadata.

## Зачем это нужно

Профилю Hermione часто нужно текущее локальное время. До этой доработки точный
ответ требовал shell-команды вроде `date`. Это работало, но было хуже как
модельный контракт:

- модель должна была выбирать shell tool ради простого чтения времени;
- shell-output добавлял лишний шум в контекст;
- для обычного вопроса "сколько времени" требовался более широкий инструмент,
  чем нужно по смыслу;
- разные сценарии могли случайно разойтись в формате вывода.

`get_system_time` делает это отдельным типизированным tool. Самый частый случай
остается коротким и дешевым: получить форматированное локальное время host одной строкой.
Расширенные поля доступны, но только по явному `full: true`.

## Согласованные решения

Эта доработка выросла из обсуждения API для системного времени. В карточке
нельзя терять следующие решения, потому что именно они задают поведение tool.

| Пункт | Итоговое решение | Причина |
| --- | --- | --- |
| Путь к времени | Добавить core tool, а не использовать shell `date` | Время суток является частым малошумным запросом и должно быть доступно без shell |
| Основной режим | По умолчанию брать локальное время host | Чаще всего нужен именно ответ "сколько времени здесь", без ручного offset |
| Имя параметра зоны | Использовать `offset`, а не `timezone` | API не обещает IANA timezone database; `offset` честно описывает поддержанные значения |
| Встроенное значение `offset` | `local` | Модель может вызвать `{}` и получить локальное время host |
| Формат | `chrono` strftime string | Позволяет передать `%H:%M` и получить строку вида `23:16`; формат стандартен для Rust-кода с `chrono` |
| Встроенный формат | `%H:%M` | Самый частый пользовательский вопрос требует часы и минуты |
| Краткость ответа | `full` по умолчанию `false` | Не засорять модельный контекст timestamp-полями, когда нужна только строка времени |
| Полный ответ | `full: true` | Явный флаг нужен для диагностики, timestamp и timezone-sensitive сценариев |
| IANA зоны | Не поддерживать `Europe/Moscow` и похожие имена | Без отдельной базы timezone это был бы ложный контракт |
| Remote build | На `f-ms-dev` только сборка, исходники правятся локально | Удаленный checkout должен получать локальный patch, а не становиться вторым местом правки |

## Карта файлов

| Файл | Ответственность |
| --- | --- |
| `codex-rs/core/src/tools/handlers/system_time.rs` | Runtime-обработчик: разбор аргументов, выбор `local`/`utc`/fixed offset, форматирование времени, короткий и полный ответ, ошибки для модели |
| `codex-rs/core/src/tools/handlers/system_time_spec.rs` | Описание Responses API tool: имя, описание, input schema, `oneOf` output schema для short/full форм |
| `codex-rs/core/src/tools/handlers/system_time_tests.rs` | Unit tests для runtime-контракта: default short output, full metadata, strftime, offset parsing, IANA rejection, invalid format |
| `codex-rs/core/src/tools/handlers/system_time_spec_tests.rs` | Unit tests для spec-контракта: описанные defaults и short/full output schema |
| `codex-rs/core/src/tools/handlers/mod.rs` | Подключает `system_time` и `system_time_spec`, экспортирует `SystemTimeHandler` |
| `codex-rs/core/src/tools/spec_plan.rs` | Добавляет `SystemTimeHandler` в `add_core_utility_tools(...)` рядом с `update_plan` |
| `codex-rs/core/tests/suite/prompt_caching.rs` | Обновляет ожидаемый список prompt tools, чтобы cache-sensitive тест видел новый tool |
| `docs/fork/core-system-time-tool.md` | Владеющий handoff-артефакт: контракт, перенос, проверки и ограничения fork-доработки |

Намеренно не менялись:

| Зона | Почему не меняется |
| --- | --- |
| `Cargo.toml` и `Cargo.lock` | Новые зависимости не нужны: `codex-core` уже использует `chrono` |
| Config schema | Tool не добавляет config key и не требует пользовательской настройки |
| App-server protocol | Внешний app-server API не меняется |
| TUI | Отдельная TUI-поверхность не нужна: tool доступен через core tool planning |
| Prompt text | Новые prompt fragments не добавляются; меняется только список доступных tools |
| `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` и `CODEX_SANDBOX_ENV_VAR` | Эти зоны запрещены локальным `AGENTS.md` и не относятся к времени |

## Итоговый контракт

### Tool spec

`get_system_time` регистрируется как `ToolSpec::Function` с описанием:
получить текущее host system time без запуска shell command. `strict` остается
`false`, как у соседних tool specs в этой зоне, но runtime-args разбираются
через `serde` с `deny_unknown_fields`.

Input schema содержит три необязательных параметра:

| Параметр | Тип | Встроенное значение | Контракт |
| --- | --- | --- | --- |
| `format` | string | `%H:%M` | `chrono` strftime format string для поля `formatted` |
| `offset` | string | `local` | `local`, `utc`, `+HH:MM` или `-HH:MM`; IANA timezone names не поддерживаются |
| `full` | boolean | `false` | `false` возвращает только `formatted`, `true` возвращает полный объект metadata |

Output schema задается через `oneOf`:

- short object: только required поле `formatted`;
- full object: required поля `formatted`, `format`, `offset`,
  `resolved_offset`, `resolved_offset_seconds`, `unix_seconds`, `unix_millis`,
  `rfc3339`, `utc_rfc3339`.

### Runtime-разбор аргументов

`SystemTimeArgs` имеет `#[serde(deny_unknown_fields)]`. Это важно для будущего
переноса: не добавляй молчаливое игнорирование неизвестных ключей без
отдельного решения, потому что model-facing ошибка лучше скрытого несовпадения
контракта.

`format`:

- если отсутствует, используется `%H:%M`;
- валидируется через `chrono::format::StrftimeItems::new(...).parse_to_owned()`;
- ошибочный формат превращается в `FunctionCallError::RespondToModel` с
  сообщением, начинающимся на `invalid chrono strftime format`.

`offset`:

- если отсутствует, равен `local`;
- перед разбором обрезается по краям через `trim()`;
- `local` и пустая строка после `trim()` выбирают локальное время host;
- `utc` разбирается без учета регистра и возвращает label `utc`;
- fixed offset должен иметь ровно форму `+HH:MM` или `-HH:MM`;
- часы fixed offset должны быть `00..23`, минуты `00..59`;
- fixed offset label в полном ответе нормализуется в `+HH:MM`/`-HH:MM`;
- неподдержанное значение возвращает model-facing ошибку с подсказкой
  использовать `local`, `utc` или fixed offset.

`full`:

- если отсутствует, `false`;
- при `false` runtime сразу возвращает short response;
- при `true` runtime строит полный ответ через `build_full_response(...)`.

### Выбор времени

Для `local` handler вызывает `chrono::Local::now()`, затем фиксирует текущий
локальный offset через `local.offset().fix()`. Это дает выбранное время как
`DateTime<FixedOffset>` и одновременно сохраняет UTC-представление того же
момента.

Для `utc` и fixed offset handler вызывает `Utc::now()`, затем переводит тот же
момент в выбранный `FixedOffset`.

У этой модели есть важное следствие: `rfc3339` и `utc_rfc3339` в full response
относятся к одному sampled instant, а не к двум независимым чтениям времени.

## Примеры поведения

Значения времени ниже иллюстративные: при живом вызове меняются timestamp и
строки времени. Формы JSON и наборы полей являются частью контракта.

### Встроенный вызов

Вход:

```json
{}
```

Ответ:

```json
{"formatted":"23:16"}
```

В этом ответе нет `format`, `offset`, `unix_seconds` и других metadata-полей.
Это намеренно.

### Явный формат

Вход:

```json
{"format":"%Y-%m-%d %H:%M"}
```

Ответ:

```json
{"formatted":"2026-06-09 23:16"}
```

### UTC

Вход:

```json
{"format":"%H:%M %:z","offset":"utc"}
```

Ответ:

```json
{"formatted":"20:16 +00:00"}
```

### Fixed offset

Вход:

```json
{"format":"%Y-%m-%d %H:%M %:z","offset":"+03:00"}
```

Ответ:

```json
{"formatted":"2026-06-09 23:16 +03:00"}
```

### Полный ответ

Вход:

```json
{"format":"%H:%M","offset":"local","full":true}
```

Форма ответа:

```json
{
  "formatted": "23:16",
  "format": "%H:%M",
  "offset": "local",
  "resolved_offset": "+03:00",
  "resolved_offset_seconds": 10800,
  "unix_seconds": 1780950000,
  "unix_millis": 1780950000000,
  "rfc3339": "2026-06-09T23:16:00.000+03:00",
  "utc_rfc3339": "2026-06-09T20:16:00.000Z"
}
```

`offset` отражает запрошенный режим, а `resolved_offset` показывает фактический
offset, примененный для форматирования. Для `local` эти поля различаются по
смыслу: `offset` остается `local`, а `resolved_offset` становится конкретным
значением host на sampled instant.

### Неподдержанная IANA timezone

Вход:

```json
{"offset":"Europe/Moscow"}
```

Ошибка для модели должна содержать:

```text
invalid offset `Europe/Moscow`; use `local`, `utc`, or a fixed offset in `+HH:MM`/`-HH:MM` form
```

### Ошибочный формат

Вход:

```json
{"format":"%Q","offset":"utc"}
```

Ошибка для модели должна начинаться с:

```text
invalid chrono strftime format
```

## Архитектурное решение

### Почему все-таки `codex-core`

Локальный `AGENTS.md` справедливо требует сопротивляться раздуванию
`codex-core`. Здесь добавление в `codex-core` оправдано тем, что runtime tool
planning, `ToolExecutor`, `CoreToolRuntime`, prompt tool consistency tests и
Responses API tool spec уже живут в core. Вынос в отдельный crate потребовал бы
больше связующего кода ради маленького обработчика и не уменьшил бы реальную
область влияния.

Изменение поэтому ограничено приватными handler/spec modules и одной точкой
регистрации в `add_core_utility_tools(...)`.

### Почему `local` по умолчанию

Пользовательский запрос "сколько времени" почти всегда означает локальное время
текущего host. Модель не должна угадывать timezone или передавать offset в
обычном случае. Поэтому самый правильный вызов для частого сценария:

```json
{}
```

### Почему параметр называется `offset`

Название `timezone` обещало бы поддержку имен зон, правил перехода на летнее
время и базы IANA. Эта доработка такого owner не вводит. `offset` честно
ограничивает контракт тем, что реально реализовано: `local`, `utc` и fixed
offset.

### Почему `full`, а не `verbose`

`verbose` звучит как уровень логирования или детализации текста. `full`
описывает именно форму JSON: short default response против полного metadata
response.

### Почему short output по умолчанию

Tool будет часто вызываться ради одной строки. Возврат timestamp metadata в
каждом таком случае загрязнил бы модельный контекст и ухудшил бы читаемость.
Полный ответ остается доступен, но требует явного намерения.

### Почему нет IANA timezone names

Без зависимости на timezone database поддержка `Europe/Moscow` была бы
ненадежной. Локальное время host уже покрывается `local`, UTC покрывается
`utc`, а переносимое смещение покрывается fixed offset. Если когда-нибудь
понадобятся IANA зоны, это должна быть отдельная доработка с явным owner для
данных timezone, тестов и правил обновления.

## Отклоненные альтернативы

| Альтернатива | Почему отклонена |
| --- | --- |
| Продолжать использовать shell `date` | Слишком широкий инструмент для частого простого запроса; шумнее для model context |
| Параметр `timezone` | Обещает IANA timezone behavior, которого нет в текущей реализации |
| UTC по умолчанию | Хуже совпадает с пользовательским вопросом о текущем локальном времени |
| Всегда возвращать полный объект | Засоряет context timestamp-полями в самом частом сценарии |
| Флаг `verbose` | Не так точно описывает форму ответа, как `full` |
| Поддержать `Europe/Moscow` без timezone database | Создало бы ложную точность и неясное обслуживание DST/исторических правил |
| Добавить config key для default timezone | Не нужно для host-local helper; расширило бы schema и migration scope |
| Добавить новый crate | Для одного core tool handler это увеличило бы связующий код больше, чем уменьшило бы связанность |

## Порядок повторения при переносе

Используй этот порядок при переносе на новый upstream checkout или при
восстановлении после конфликтного merge.

1. Проверить, что `codex-core` все еще использует `chrono`. Если зависимости
   изменились, решить отдельно: вернуть `chrono`, использовать уже принятую
   альтернативу или вынести форматирование в подходящий общий слой.
2. Добавить `codex-rs/core/src/tools/handlers/system_time.rs`.
3. Добавить `codex-rs/core/src/tools/handlers/system_time_spec.rs`.
4. Подключить `mod system_time;` и `pub(crate) mod system_time_spec;` в
   `codex-rs/core/src/tools/handlers/mod.rs`.
5. Экспортировать `SystemTimeHandler` из `handlers/mod.rs`.
6. Добавить `use crate::tools::handlers::SystemTimeHandler;` в
   `codex-rs/core/src/tools/spec_plan.rs`.
7. Добавить `planned_tools.add(SystemTimeHandler);` в
   `add_core_utility_tools(...)` сразу после `PlanHandler`, чтобы tool входил в
   базовый набор core utility tools.
8. Обновить `codex-rs/core/tests/suite/prompt_caching.rs`: добавить
   `"get_system_time"` в `expected_tools_names`.
9. Добавить соседние test files с `#[path = "..._tests.rs"]`, а не inline tests:
   `system_time_tests.rs` и `system_time_spec_tests.rs`.
10. Покрыть тестами следующие контракты:
    default short output, `full: true`, strftime formatting, `utc`
    case-insensitive parsing, fixed offset parsing, rejection для
    `Europe/Moscow`, invalid strftime format, short/full output schema.
11. Создать или обновить карточку `docs/fork/core-system-time-tool.md` в том же
    commit, что и кодовая доработка.
12. Перед удаленной сборкой синхронизировать `f-ms-dev` как build host: чистый
    checkout, `pull`/`fetch`, локальный patch, затем проверка списка файлов.
13. Выполнить targeted tests, форматирование и release-fast сборку по текущему
    repo workflow.
14. После установки бинаря проверить живой tool вызовом `{}` и убедиться, что
    ответ содержит только `formatted`.

## Проверки

### Смысловое покрытие

| Согласованный или реализованный пункт | Статус | Где покрыт |
| --- | --- | --- |
| Нужен API системного времени без shell `date` | перенесено в карточку | `Зачем это нужно`, `Итоговый контракт` |
| Чаще всего нужен форматированный local time | перенесено в карточку | `Согласованные решения`, `Почему local по умолчанию` |
| `offset` по умолчанию равен `local` | перенесено в карточку | `Tool spec`, tests `defaults_to_local_offset_and_short_time_format` |
| Функция должна брать host local time, если offset не передан | перенесено в карточку | `Выбор времени`, handler `Local::now()` |
| Формат должен принимать `%H:%M` | перенесено в карточку | `Tool spec`, `Примеры поведения`, tests для strftime |
| По умолчанию ответ должен содержать только `formatted` | перенесено в карточку | `Примеры поведения`, runtime-проверка `{"formatted":"00:45"}` |
| `full: true` возвращает metadata | перенесено в карточку | `Полный ответ`, tests `full_response_includes_metadata` |
| IANA timezone names не поддерживаются | перенесено в карточку | `Неподдержанная IANA timezone`, tests `rejects_iana_timezone_names` |
| `utc` и fixed offset поддерживаются | перенесено в карточку | `UTC`, `Fixed offset`, tests `utc_offset_is_case_insensitive`, `parses_fixed_offsets` |
| Invalid format должен быть model-facing error | перенесено в карточку | `Ошибочный формат`, tests `rejects_invalid_format_strings` |
| Новый tool должен попадать в prompt tool list | перенесено в карточку | `Карта файлов`, `prompt_tools_are_consistent_across_requests` |
| Новые source-файлы должны участвовать в diff-based workflow | перенесено в карточку | `Порядок повторения при переносе`; git rulebook также уточнен вне этой карточки |
| Remote `f-ms-dev` является только build host | перенесено в карточку | `Согласованные решения`, `Исторические результаты` |
| Full `codex-core` suite не была зеленой | перенесено в карточку | `Исторические результаты`, известные remote infra/sandbox падения |
| Config schema не меняется | не применимо | Tool не добавляет config key |
| App-server API не меняется | не применимо | Tool регистрируется только как core utility tool |
| TUI не меняется | не применимо | Нет отдельной UI-поверхности |
| Открытые вопросы по текущему контракту | open question | Возможная будущая поддержка IANA zones и совместимость `oneOf` schema |

### Владелец исполняемой карты

Card-level проверки этой карточки запускает skill-owned команда `fork tests`.
Внутренние argv и назначение targeted проверок живут только в блоке
`fork-tests.v1` ниже; они не являются нормативным runbook для ручного запуска
внутренних команд.

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "system time",
      "argv": ["just", "test", "-p", "codex-core", "system_time"]
    },
    {
      "purpose": "prompt tool cache",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "prompt_tools_are_consistent_across_requests"
      ]
    }
  ]
}
```

### Дополнительные gates

Дополнительных card-specific gates поверх `fork tests` эта карточка не вводит.
В общем миграционном проходе родительский агент отвечает за строгую валидацию
карточек и любые более широкие fork gates через skill-owned workflow.

Эта доработка не меняет зависимости, config schema, app-server protocol, TUI или
snapshot-поверхности, поэтому соответствующие gates намеренно не добавляются к
исполняемой карте этой карточки.

### Исторические результаты

Проверки ниже уже выполнялись для исходной реализации этой доработки. Они
сохранены как исторический результат и не являются инструкцией запускать прямые
`just`/`cargo` команды при текущей миграции.

| Проверка | Где запускалась | Результат | Что подтверждает |
| --- | --- | --- | --- |
| `git diff --check` | local | пройдено | Diff не содержит whitespace errors |
| `cargo fmt -- --config imports_granularity=Item` | local | пройдено, stable rustfmt печатает warning про nightly-only option | Rust formatting применен к измененным Rust-файлам |
| `just fmt` | local | завершилось ошибкой из-за отсутствующего `uv` для Python SDK/scripts; Rust formatter внутри recipe отработал | Rust часть форматирования прошла, общий recipe уперся в локальную Python-зависимость |
| `just test -p codex-core system_time` | `f-ms-dev` | пройдено, 9 tests | Handler/spec tests для новой функции проходят |
| `just test -p codex-core prompt_tools_are_consistent_across_requests` | `f-ms-dev` | пройдено, 1 test | Новый tool стабилен в списке prompt tools |
| `just test -p codex-core` | `f-ms-dev` | скомпилировалось, затем завершилось ошибкой на 66 existing remote-infra/sandbox tests | Full suite был попробован, но не является сигналом regression этой доработки |
| `just build-fast-release` | `f-ms-dev` | пройдено, `codex-cli 0.137.0+hermione` | Release-fast binary собирается с новым tool |

### Миграция на `rust-v0.144.4`

Первый проход теста на уровне карточки
`prompt_tools_are_consistent_across_requests` завершился ошибкой после того, как
upstream добавил `read_file` в базовый список инструментов при наличии среды
выполнения. Оба запроса в этом тесте используют один `TestCodex` с одной средой
выполнения, а `add_core_utility_tools(...)` регистрирует `ReadFileHandler` при
`environment_mode.has_environment()`. Поэтому `read_file` должен стабильно
присутствовать в обоих запросах; ожидаемый список синхронизирован с текущей
регистрацией. Повторный проход проверки карточки остается за родительским агентом
и на момент обновления карточки еще не подтвержден.

Удаленная сборка выполнялась на `f-ms-dev` в
`/home/slader/Projects/codex`. В этом workflow remote является только host
сборки: исходники приводятся к чистой базе и получают patch из локального diff.

Release-fast artifact:

| Поле | Значение |
| --- | --- |
| Удаленный artifact | `/home/slader/Projects/codex/codex-rs/target/release-fast/codex` |
| Локальная временная копия | `/tmp/codex-hermione-release-fast` |
| Установленный binary | `/home/slader/.local/bin/codex-hermione` |
| Проверка версии | `codex-cli 0.137.0+hermione` |
| SHA256 | `4ff2589b432d352b867d81fc59c06c14cda97948b1ccc57fc9aa324fe7a3f5f4` |

Установка выполнялась атомарно: temp binary копировался как
`/home/slader/.local/bin/codex-hermione.new`, затем заменял установленный
`codex-hermione` через `mv -f`.

Живая проверка после перезапуска бинаря:

Вход:

```json
{}
```

Ответ:

```json
{"formatted":"00:45"}
```

Эта проверка подтверждает именно краткий default output. Она не проверяет все
ветки `offset` и `full`; для них есть unit tests.

### Миграция на `rust-v0.144.5`

После merge `rust-v0.144.5` owner-файлы и runtime/tool-spec контракт
`get_system_time` сохранились без изменений: handler и spec подключены через
`handlers/mod.rs`, `SystemTimeHandler` остается в базовом наборе
`add_core_utility_tools(...)`, а prompt-cache expectation содержит
`get_system_time` вместе с актуальным `read_file`.

В текущем upstream-коде также присутствует feature-gated tool
`clock.curr_time`. Он не заменяет эту fork-доработку: feature
`current_time_reminder` по умолчанию выключен, tool возвращает только UTC в
фиксированном формате и не поддерживает контракт `format`/`offset`/`full`.
Имена tools различаются, поэтому регистрационного конфликта нет.

Миграционный проход ограничен source-level сверкой owner-файлов, runtime,
unit-test и tool-spec контрактов. Targeted tests, форматирование, генераторы и
другие project-level проверки подагент не запускал; они остаются за общим
проверочным проходом родительского агента через skill-owned workflow.

### Известные падения и пропуски

- Full `codex-core` suite на `f-ms-dev` не считается зеленым результатом:
  известные падения относились к существующим remote infra/sandbox условиям,
  включая `bwrap` loopback, missing `test_stdio_server`, `tool_search` mocks и
  sandbox/permission suites. Их нельзя переписывать как regression этой
  доработки или как зеленую проверку.
- Исторический локальный форматирующий recipe завершался ошибкой из-за
  отсутствующего `uv` для Python SDK/scripts; Rust formatter внутри recipe при
  этом отработал.
- Живая runtime-проверка после установки покрывала только default-вызов `{}` и
  short output `formatted`. Ветки `offset`, `full`, invalid format и IANA
  rejection покрываются targeted tests, а не live smoke.
- Config schema, app-server API, TUI и snapshots намеренно не проверялись для
  этой доработки, потому что соответствующие поверхности не менялись.

## Риски и ограничения

- `local` зависит от timezone host system. Это ожидаемый контракт, а не
  побочный эффект: tool отвечает за время текущего host.
- Fixed offset parser намеренно строгий: принимает только `+HH:MM` и `-HH:MM`.
- `utc` разбирается без учета регистра, но label в full response нормализуется
  в `utc`.
- Пустой `offset` после trim считается `local`; если будущий API захочет
  считать пустую строку ошибкой, это будет breaking change runtime-контракта.
- `full: true` добавляет timestamps и не должен использоваться для обычного
  вопроса времени без причины.
- Если будущий Responses API schema validator перестанет принимать `oneOf` в
  output schema, нужно заменить schema shape, сохранив runtime default:
  `{"formatted":"..."}`.
- Если понадобится IANA timezone support, нельзя расширять `offset` молча:
  нужна отдельная доработка с timezone database owner, tests и обновленной
  карточкой.

## Проверка покрытия

Эта итоговая owner-card section group сохранена отдельно от раздела `Проверки`,
потому что ее наличие является структурным контрактом active fork-карточки.
Детальное смысловое покрытие также оставлено в `Проверки` ->
`Смысловое покрытие`.

| Согласованный или реализованный пункт | Статус | Где покрыт |
| --- | --- | --- |
| Нужен API системного времени без shell `date` | перенесено в карточку | `Зачем это нужно`, `Итоговый контракт` |
| Чаще всего нужен форматированный local time | перенесено в карточку | `Согласованные решения`, `Почему local по умолчанию` |
| `offset` по умолчанию равен `local` | перенесено в карточку | `Tool spec`, tests `defaults_to_local_offset_and_short_time_format` |
| Функция должна брать host local time, если offset не передан | перенесено в карточку | `Выбор времени`, handler `Local::now()` |
| Формат должен принимать `%H:%M` | перенесено в карточку | `Tool spec`, `Примеры поведения`, tests для strftime |
| По умолчанию ответ должен содержать только `formatted` | перенесено в карточку | `Примеры поведения`, runtime-проверка `{"formatted":"00:45"}` |
| `full: true` возвращает metadata | перенесено в карточку | `Полный ответ`, tests `full_response_includes_metadata` |
| IANA timezone names не поддерживаются | перенесено в карточку | `Неподдержанная IANA timezone`, tests `rejects_iana_timezone_names` |
| `utc` и fixed offset поддерживаются | перенесено в карточку | `UTC`, `Fixed offset`, tests `utc_offset_is_case_insensitive`, `parses_fixed_offsets` |
| Invalid format должен быть model-facing error | перенесено в карточку | `Ошибочный формат`, tests `rejects_invalid_format_strings` |
| Новый tool должен попадать в prompt tool list | перенесено в карточку | `Карта файлов`, `prompt_tools_are_consistent_across_requests` |
| Новые source-файлы должны участвовать в diff-based workflow | перенесено в карточку | `Порядок повторения при переносе`; git rulebook также уточнен вне этой карточки |
| Remote `f-ms-dev` является только build host | перенесено в карточку | `Согласованные решения`, `Исторические результаты` |
| Full `codex-core` suite не была зеленой | перенесено в карточку | `Исторические результаты`, известные remote infra/sandbox падения |
| Config schema не меняется | не применимо | Tool не добавляет config key |
| App-server API не меняется | не применимо | Tool регистрируется только как core utility tool |
| TUI не меняется | не применимо | Нет отдельной UI-поверхности |
| Открытые вопросы по текущему контракту | open question | Возможная будущая поддержка IANA zones и совместимость `oneOf` schema |
