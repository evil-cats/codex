---
id: fork-core-system-time-tool
status: active
created: 2026-06-09
updated: 2026-08-31
---

# Утилитарный core tool `get_system_time`

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая добавляет
`get_system_time`: встроенный core tool для получения текущего времени host без
запуска shell-команды `date`.

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

## Карта файлов

| Файл | Ответственность |
| --- | --- |
| `codex-rs/core/src/tools/handlers/system_time.rs` | Runtime-обработчик: разбор аргументов, выбор `local`/`utc`/fixed offset, форматирование времени, короткий и полный ответ, ошибки для модели |
| `codex-rs/core/src/tools/handlers/system_time_spec.rs` | Описание Responses API tool: имя, описание, input schema, `oneOf` output schema для short/full форм |
| `codex-rs/core/src/tools/handlers/system_time_tests.rs` | Unit tests для runtime-контракта: default short output, full metadata, strftime, offset parsing, IANA rejection, invalid format |
| `codex-rs/core/tests/suite/system_time.rs` | Интеграционные тесты Responses API: успешный вызов настоящего `SystemTimeHandler` и ошибка неизвестного поля для модели |
| `codex-rs/core/src/tools/handlers/system_time_spec_tests.rs` | Unit tests для spec-контракта: описанные defaults и short/full output schema |
| `codex-rs/core/src/tools/handlers/mod.rs` | Подключает `system_time` и `system_time_spec`, экспортирует `SystemTimeHandler` |
| `codex-rs/config/src/config_toml.rs` | Объявляет включённый по умолчанию config gate `[tools.get_system_time].enabled` по форме соседнего статического `update_plan` |
| `codex-rs/core/src/config/mod.rs` | Преобразует config gate в `Config::get_system_time_enabled`, сохраняя обычное значение `true` |
| `codex-rs/core/config.schema.json` | Содержит сгенерированную схему конфигурации для `[tools.get_system_time]` |
| `codex-rs/core/src/tools/spec_plan.rs` | Добавляет `SystemTimeHandler` в `add_core_utility_tools(...)`, только если разрешён `get_system_time_enabled` |
| `codex-rs/core/src/tools/spec_plan_tests.rs` | Проверяет, что config gate одновременно управляет видимой модели и зарегистрированной поверхностями tool |
| `codex-rs/core/tests/suite/mod.rs` | Подключает интеграционный модуль `system_time` |
| `codex-rs/core/tests/suite/prompt_caching.rs` | Обновляет ожидаемый список prompt tools, чтобы cache-sensitive тест видел новый tool |
| `codex-rs/tui/src/temporary_structured_request.rs` | Явно отключает `get_system_time` в fail-closed temporary structured thread |
| `codex-rs/tui/src/app/tests/recap_generation_tests.rs` | Upstream-регрессия подтверждает, что structured recap отправляет `tools: []` |
| `codex-rs/thread-manager-sample/src/main.rs` | Сохраняет обычный default `true` в ручной конструкции `Config` |
| `docs/fork/core-system-time-tool.md` | Владеющий handoff-артефакт: контракт, перенос, проверки и ограничения fork-доработки |

Намеренно не менялись:

| Зона | Почему не меняется |
| --- | --- |
| `Cargo.toml` и `Cargo.lock` | Новые зависимости не нужны: `codex-core` уже использует `chrono` |
| App-server protocol | Внешний app-server API не меняется |
| Пользовательская TUI | Отдельная видимая поверхность не нужна: меняется только изоляция внутреннего temporary thread |
| Prompt text | Новые prompt fragments не добавляются; меняется только список доступных tools |
| `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR` и `CODEX_SANDBOX_ENV_VAR` | Эти зоны запрещены локальным `AGENTS.md` и не относятся к времени |

## Итоговый контракт

### Доступность tool

В обычной сессии `get_system_time` включён по умолчанию. Config gate
`[tools.get_system_time].enabled` повторяет актуальный upstream-механизм для
статического `update_plan`: отсутствие секции или поля означает `true`, а
`enabled = false` запрещает и model-visible spec, и runtime-регистрацию handler.

Пустой `dynamic_tools` отключает только динамически переданные tools и не влияет
на регистрацию статических core tools. Поэтому fail-closed temporary structured
thread обязан дополнительно передать
`tools.get_system_time.enabled = false`. Это сохраняет `get_system_time` в
обычных Responses-запросах и cached tool set, но не даёт structured recap
получить инструментальную поверхность.

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

Реализация `ToolExecutor<ToolInvocation>::handle` должна повторять явную
сигнатуру времени жизни из trait:

```rust
fn handle<'a>(&'a self, invocation: ToolInvocation) -> ToolExecutorFuture<'a>
where
    ToolInvocation: 'a,
```

Сокращённая сигнатура с `ToolExecutorFuture<'_>` больше не совпадает с trait и
не проходит проверку компиляции, хотя само runtime-поведение обработчика не
меняется.

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

### Примеры поведения

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

Основная реализация поэтому ограничена приватными модулями handler/spec и одной
условной точкой регистрации в `add_core_utility_tools(...)`. Небольшой config
gate нужен не как пользовательская настройка времени, а как upstream-совместимый
механизм изоляции внутренних temporary threads.

### Почему затронут upstream temporary thread

`codex-rs/tui/src/temporary_structured_request.rs` владеет fail-closed
конфигурацией structured recap и других внутренних краткоживущих запросов. Это
чужая относительно runtime tool реализация, но именно она обязана перечислить
каждый default-enabled статический core tool, который нужно отключить. Изменение
в ней ограничено одним config override для `get_system_time`; prompt, сбор
recap, permissions, MCP shutdown и обработка результата не меняются.
Другие статические core tools остаются областью их собственных fork-карточек.

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
7. Добавить `[tools.get_system_time].enabled` со значением `true` по умолчанию в `ToolsToml`,
   разрешить его в `Config::get_system_time_enabled` и перегенерировать config
   schema через skill-owned generator gate.
8. Добавить условный `registry.add(SystemTimeHandler);` в
   `add_core_utility_tools(...)` сразу после `PlanHandler`, чтобы tool входил в
   обычный набор core utility tools, но уважал config gate.
9. Добавить `tools.get_system_time.enabled = false` в config overrides
   `start_temporary_thread(...)`; не заменять этим пустые `dynamic_tools` и не
   ослаблять upstream assertion `tools: []`.
10. Обновить `codex-rs/core/tests/suite/prompt_caching.rs`: добавить
    `"get_system_time"` в `expected_tools_names`.
11. Добавить соседние test files с `#[path = "..._tests.rs"]`, а не inline tests:
   `system_time_tests.rs` и `system_time_spec_tests.rs`.
12. Добавить `codex-rs/core/tests/suite/system_time.rs` и подключить его через
    `mod system_time;` в `codex-rs/core/tests/suite/mod.rs`.
13. Покрыть тестами следующие контракты: встроенный короткий ответ, `full: true`,
    форматирование strftime, регистронезависимый разбор `utc`, пробельный
    `offset` после `trim()`, допустимые границы и ошибочные формы fixed offset,
    отклонение `Europe/Moscow`, неизвестные JSON-поля, ошибочный формат strftime,
    схемы short/full, config gate, сквозной вызов Responses API настоящего
    `SystemTimeHandler` и отсутствие tools в structured recap.

## Проверки

Исполняемая карта card-level regression tests:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "формат, границы offset, полный ответ, ошибки, config gate и Responses-вызов get_system_time",
      "argv": ["just", "test", "-p", "codex-core", "system_time"]
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
      "purpose": "fail-closed structured recap отправляет Responses-запрос без get_system_time и других tools",
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

- `local` зависит от timezone host system. Это ожидаемый контракт, а не
  побочный эффект: tool отвечает за время текущего host.
- Fixed offset parser намеренно строгий: принимает только `+HH:MM` и `-HH:MM`.
- `utc` разбирается без учета регистра, но label в full response нормализуется
  в `utc`.
- Пустой `offset` после trim считается `local`; если будущий API захочет
  считать пустую строку ошибкой, это будет breaking change runtime-контракта.
- `full: true` добавляет timestamps и не должен использоваться для обычного
  вопроса времени без причины.
- Каждый новый default-enabled статический core tool обязан иметь собственный
  config gate и явное отключение в fail-closed temporary structured thread;
  пустой `dynamic_tools` этого не обеспечивает.
- Если будущий Responses API schema validator перестанет принимать `oneOf` в
  output schema, нужно заменить schema shape, сохранив runtime default:
  `{"formatted":"..."}`.
- Если понадобится IANA timezone support, нельзя расширять `offset` молча:
  нужна отдельная доработка с timezone database owner, tests и обновленной
  карточкой.
