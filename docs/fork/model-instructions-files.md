---
id: fork-model-instructions-files
status: active
created: 2026-07-24
updated: 2026-07-24
source_scope: hermione-0.145.0..HEAD
---

# Model instructions files

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая добавляет top-level
config key `model_instructions_files`. Он позволяет разделить базовые инструкции
модели между несколькими Markdown-файлами и собрать их в одно значение
Responses API `instructions`.

Доработка реализована и проверена в рабочей копии ветки
`hermione-0.145.0`, зафиксирована отдельным feature commit `258ffab29` поверх
опубликованного migration commit `38b2a768b` и отправлена в
`origin/hermione-0.145.0`.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Статус реализации | `implemented` |
| Проверочный статус | `verified` |
| Основной commit | `258ffab29 Support multiple model instruction files` |
| Граница изменения | Отдельный feature commit после migration commit |
| Текущая база | `0.145.0`, ветка `hermione-0.145.0` |
| Новый config key | `model_instructions_files` |
| Тип | `Vec<AbsolutePathBuf>` |
| Существующий config key | `model_instructions_file` |
| Разделитель документов | `\n\n` |
| Release-fast binary | Собран и проверен |
| Локальная установка | `/home/slader/.local/bin/codex-hermione`, проверена |
| Установка на `f-ms-dev` | `/home/slader/.local/bin/codex-hermione`, проверена |

## Зачем это нужно

Одиночный `model_instructions_file` вынуждает хранить весь базовый prompt модели
в одном документе. Для большого профиля это затрудняет редактирование, review,
повторное использование независимых разделов и контроль их порядка.

Реализованное поведение:

- поддержка существующего `model_instructions_file` сохранена;
- добавлен упорядоченный список `model_instructions_files`;
- файлы читаются строго в порядке config-массива;
- каждый документ обрабатывается через `trim()` и соединяется с соседним через
  `\n\n`;
- объединённый текст передаётся как одно значение базовых инструкций;
- дополнительные сообщения `input` или элементы с ролью `developer` не
  создаются;
- одновременное использование одиночного и множественного ключей возвращает
  ошибку конфигурации;
- пустой, отсутствующий или нечитаемый файл возвращает ошибку конфигурации;
- встроенные инструкции модели остаются fallback-значением, если ни один
  файловый override не задан.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/config/src/config_toml.rs` | Содержит `model_instructions_files` в `ConfigToml` |
| `codex-rs/config/src/profile_toml.rs` | Содержит новый ключ в profile config |
| `codex-rs/config/src/loader/mod.rs` | Нормализует каждый относительный путь массива |
| `codex-rs/core/src/config/mod.rs` | Проверяет конфликт ключей, читает файлы и собирает `base_instructions` |
| `codex-rs/core/src/config/config_tests.rs` | Покрывает parsing, порядок, ошибки и precedence |
| `codex-rs/core/src/config/config_loader_tests.rs` | Покрывает config layers и CLI config override |
| `codex-rs/core/tests/suite/client.rs` | Проверяет исходящее поле Responses API `instructions` |
| `codex-rs/core/src/session/config_lock.rs` | Исключает файловый startup-only override из thread config lock |
| `codex-rs/core/config.schema.json` | Описывает новый config key в сгенерированной schema |

## Итоговый контракт

### Формат конфигурации

Новая настройка является top-level config key:

```toml
model_instructions_files = [
    "instructions/base.md",
    "instructions/tools.md",
    "instructions/workflow.md",
]
```

Она также разрешена в отдельном profile config-файле.

`ConfigToml` и `ConfigProfile` содержат поле:

```rust
#[serde(default)]
pub model_instructions_files: Vec<AbsolutePathBuf>,
```

Относительные пути разрешаются относительно каталога config-файла, в котором
они заданы. Порядок элементов после нормализации не меняется.

### Сборка базовых инструкций

Каждый файл:

1. читается как UTF-8 через `ExecutorFileSystem`;
2. обрезается снаружи через `trim()`;
3. проверяется на непустое содержимое;
4. добавляется в результирующий список без сортировки.

Секции соединяются через `\n\n`. Для файлов:

```text
base.md       -> "\nBase instructions.\n"
tools.md      -> "Tools instructions.\n\n"
workflow.md   -> "Workflow instructions."
```

результат должен быть точным:

```text
Base instructions.

Tools instructions.

Workflow instructions.
```

Итог становится одним `Config.base_instructions`. На уровне Responses API это
одно верхнеуровневое поле:

```json
{
  "instructions": "Base instructions.\n\nTools instructions.\n\nWorkflow instructions."
}
```

Доработка не создаёт несколько `instructions`, не добавляет элементы в
`input` и не меняет `developer_instructions_files`.

### Совместимость и взаимоисключение

Существующий ключ сохраняется:

```toml
model_instructions_file = "instructions.md"
```

Если одновременно заданы непустой `model_instructions_files` и
`model_instructions_file`, загрузка завершается `InvalidInput` с сообщением:

```text
`model_instructions_file` and `model_instructions_files` cannot both be set
```

Пустой массив считается отсутствием множественного override:

```toml
model_instructions_files = []
```

Поэтому пустой массив не конфликтует с `model_instructions_file` и сам по себе
не заменяет встроенные инструкции модели.

Не допускается молча выбирать один из двух ключей или неявно добавлять одиночный
файл к массиву: это скрывает ошибку конфигурации и делает порядок документов
неочевидным.

### Ошибки файлов

Пустой или состоящий только из whitespace файл завершает загрузку с
`InvalidData`. Сообщение содержит контекст и путь:

```text
model instructions file is empty: <path>
```

Отсутствующий или нечитаемый файл сохраняет исходный `std::io::ErrorKind`, а
сообщение содержит путь:

```text
failed to read model instructions file <path>: <source error>
```

Пустой файл намеренно не пропускается с warning. В отличие от дополнительного
developer-слоя, каждый документ массива составляет часть базового prompt;
молчаливое выпадение файла может удалить идентичность агента или обязательный
раздел его рабочего контракта.

### Precedence

Effective base instructions выбираются в следующем порядке:

1. runtime `ConfigOverrides.base_instructions`;
2. объединённое содержимое `model_instructions_files`;
3. содержимое `model_instructions_file`;
4. inline `instructions`;
5. встроенные инструкции выбранной модели на более позднем этапе создания
   сессии, если `Config.base_instructions` остаётся `None`.

Поскольку непустой `model_instructions_files` и `model_instructions_file`
взаимоисключающие, между ними нет молчаливого выбора.

Runtime override сохраняет наивысший приоритет. Эта карточка не меняет общий
контракт `ConfigOverrides` и не меняет источник встроенных инструкций модели.

## Архитектурное решение

### Выбранный вариант

Множественная настройка является альтернативой одиночной. Она собирается на
этапе загрузки `Config`, поэтому дальнейший runtime продолжает работать с одним
`Option<String>` и не получает новый тип prompt.

Преимущества:

- Responses API surface не меняется;
- downstream-код не должен знать о количестве исходных файлов;
- порядок и ошибки проверяются в одном месте;
- старый config остаётся совместимым;
- schema явно описывает новый список путей.

### Отклонённые альтернативы

Не выбраны:

- автоматическое объединение `model_instructions_file` с
  `model_instructions_files`, потому что порядок двух источников неочевиден;
- молчаливое предпочтение нового ключа, потому что опечатка или забытый старый
  ключ останутся незаметными;
- alias, который десериализует одиночный и множественный ключи в одно поле,
  потому что невозможно выдать точную ошибку при одновременном использовании;
- несколько API-сообщений, потому что Responses API ожидает единое поле
  `instructions`, а не ordered message list для базового prompt;
- warning и пропуск пустого файла, потому что это может молча удалить
  обязательную часть базовых инструкций.

## Порядок повторения при переносе

Доработка уже реализована. При переносе на следующий upstream её нужно
воспроизвести в следующем порядке:

1. найти `ConfigToml.model_instructions_file`, `ConfigProfile` и обработку
   `base_instructions`;
2. добавить множественное поле рядом с одиночным во всех config types;
3. сохранить `#[serde(default)]`, чтобы отсутствие ключа давало пустой список;
4. убедиться, что loader round-trip нормализует каждый `AbsolutePathBuf`;
5. добавить конфликт одиночного и непустого множественного ключей;
6. прочитать файлы через существующий abstraction `ExecutorFileSystem`;
7. trim'ить и проверять каждый файл, сохраняя порядок;
8. соединить секции через `\n\n` до вычисления effective
   `base_instructions`;
9. исключить startup-only список из thread config lock рядом с одиночным ключом;
10. обновить config schema через skill-owned generator;
11. восстановить unit, loader и сквозное Responses API покрытие;
12. сверить эту карточку с итоговым diff и обновить результаты проверок.

## Проверки

### Смысловое покрытие

Реализованное проверочное покрытие подтверждает:

- `model_instructions_files_parse_from_toml` — TOML parsing массива в
  `Vec<AbsolutePathBuf>`;
- `model_instructions_files_resolve_relative_to_config_directory` —
  нормализацию каждого относительного пути без изменения порядка;
- `cli_override_model_instructions_files_sets_base_instructions` — передачу
  массива через CLI config override и сборку effective `base_instructions`;
- `model_instructions_files_are_joined_in_order` — точное объединение
  обработанных через `trim()` документов с разделителем `\n\n`;
- `empty_model_instructions_files_preserve_inline_instructions` — отсутствие
  файлового override для пустого массива;
- существующее покрытие `model_instructions_file` — совместимость одиночного
  ключа;
- `model_instructions_files_conflict_with_model_instructions_file` — ошибку при
  одновременном использовании одиночного и непустого множественного ключей;
- `model_instructions_files_reject_empty_file` — `InvalidData` и путь пустого
  файла;
- `model_instructions_files_reject_missing_file` — сохранение `NotFound` и путь
  отсутствующего файла;
- `base_instructions_override_skips_model_instructions_files` — наивысший
  приоритет runtime `base_instructions`;
- skill-owned генераторы — наличие массива в config schema с default `[]`;
- `model_instructions_files_are_loaded_into_request_in_order` — сквозной путь
  от нескольких реальных файлов до одного исходящего Responses API поля
  `instructions`.

### Владелец исполняемой карты

Проверки уровня карточки запускает skill-owned command `fork tests`. Внутренний
argv хранится как данные `fork-tests.v1`, а не как пользовательский runbook.

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "model instructions files",
      "argv": ["just", "test", "-p", "codex-core", "model_instructions_files"]
    }
  ]
}
```

### Дополнительные gates

- `fork generators` обновил и проверил `codex-rs/core/config.schema.json`; этот
  gate остаётся владельцем schema при будущих изменениях.
- `fork format` применил и проверил обязательное форматирование Rust-кода.
- `fork cards validate` подтвердил форму карточки и связь с исполняемой картой.
- Режим списка `fork tests` подтвердил обнаружение блока `fork-tests.v1`.
- `fork build-fast` собрал и проверил fork-бинарник.
- Полный test suite не является автоматическим требованием этой узкой
  доработки и требует отдельного решения пользователя.

### Исторические результаты

Карточка создана 2026-07-24 после завершённой и отправленной миграции `0.145.0`.
Реализация зафиксирована отдельным feature commit `258ffab29` поверх
опубликованного migration commit `38b2a768b`.

В реализации 2026-07-24:

- skill-owned генераторы обновили config schema и завершились с результатом
  `OK`; app-server schemas остались без diff;
- форматирование в режимах исправления и проверки завершилось с результатом
  `OK`;
- `fork cards validate` проверил 25 карточек без ошибок;
- режим списка `fork tests` обнаружил исполняемую карту
  `fork-model-instructions-files`;
- card-level проход выполнил девять тестов нового config/runtime/API-контракта
  с итогом `OK`;
- fast build собрал `codex-rs/target/release-fast/codex`, проверил metadata и
  версию бинарника с итогом `OK`;
- feature commit `258ffab29` отправлен в `origin/hermione-0.145.0`;
- skill-owned локальная установка атомарно заменила
  `/home/slader/.local/bin/codex-hermione`;
- тот же проверенный бинарник скопирован на `f-ms-dev`, запущен из временного
  пути и только после успешной проверки атомарно установлен в
  `/home/slader/.local/bin/codex-hermione`;
- обе установленные копии возвращают `codex-cli 0.145.0+hermione`, имеют размер
  `372607112` bytes, mode `755` и SHA-256
  `02fb5f8bd562da66a9ed776080f4ebcc6132467fbf580ff1e5c67bf69f698636`.

Первый card-level проход выявил некорректный выбор внешнего CLI fixture:
`codex_utils_cargo_bin::cargo_bin("codex")` разрешил ранее собранный бинарник,
который ещё не знал новый config key. Семь тестов текущего `codex-core` прошли,
а внешний тест увидел встроенные инструкции. Сквозное покрытие перенесено в
существующий in-process `test_codex` pattern, который загружает текущий
`config.toml` и проверяет точное поле исходящего запроса. Повторный проход
завершился успешно.

### Известные падения и пропуски

- Известных падений после повторного card-level прохода нет.
- Полный workspace test suite не запускался: для узкой карточки выполнены
  целевые tests, а полный проход требует отдельного решения пользователя.
- Общий предел размера базового prompt не вводится этой карточкой. Он должен
  проектироваться как отдельный контракт model-visible context, а не как
  локальное усечение отдельных документов.

## Runtime, сборка и установка

Карточка меняет startup config и формирование базовых инструкций, но не меняет
формат release-бинарника. В ходе реализации `fork build-fast` успешно собрал
и проверил `codex-rs/target/release-fast/codex`.

Skill-owned workflow установил бинарник локально в
`/home/slader/.local/bin/codex-hermione`. Тот же артефакт после проверки
архитектуры `x86_64`, запуска из временного пути и контрольной суммы атомарно
установлен на `f-ms-dev` в `/home/slader/.local/bin/codex-hermione`.

## Риски и ограничения

- Порядок массива является частью prompt-контракта; сортировка запрещена.
- Порядок и точный разделитель защищены регрессионными тестами.
- `trim()` удаляет внешние пустые строки каждого документа.
- Разделитель всегда равен `\n\n`; содержимое внутри документа сохраняется.
- Файлы читаются целиком; отдельный предел размера в этой доработке отсутствует.
- Молчаливое усечение или пропуск файла недопустимы.
- Новый ключ является fork-specific и должен переноситься при следующих
  upstream migration.
- Schema обновлена вместе с реализацией. При будущих изменениях она должна
  оставаться синхронизированной с config types.

## Проверка покрытия

| Согласованный пункт | Статус | Где отражено |
| --- | --- | --- |
| Добавить `model_instructions_files` | реализовано и отражено в карточке | «Формат конфигурации» |
| Читать файлы по порядку | реализовано и проверено | «Сборка базовых инструкций», «Смысловое покрытие» |
| Соединять документы через `\n\n` | реализовано и проверено | «Сборка базовых инструкций», «Смысловое покрытие» |
| Передавать одно поле API `instructions` | реализовано и проверено | «Сборка базовых инструкций», «Смысловое покрытие» |
| Сохранить `model_instructions_file` | реализовано и проверено | «Совместимость и взаимоисключение» |
| Запретить одновременное использование двух ключей | реализовано и проверено | «Совместимость и взаимоисключение», «Смысловое покрытие» |
| Пустой массив считать отсутствием override | реализовано и проверено | «Совместимость и взаимоисключение», «Смысловое покрытие» |
| Пустой файл считать ошибкой | реализовано и проверено | «Ошибки файлов», «Смысловое покрытие» |
| Missing/unreadable файл считать ошибкой | реализовано и проверено | «Ошибки файлов», «Смысловое покрытие» |
| Сохранить runtime precedence | реализовано и проверено | «Precedence», «Смысловое покрытие» |
| Обновить profile config и schema | реализовано и проверено | «Карта файлов», «Проверки» |
| Не менять developer-инструкции и skill | намеренно не изменено | «Сборка базовых инструкций», scope карточки |
| Добавить сквозное API-покрытие | реализовано и проверено | «Смысловое покрытие» |
| Ввести предел размера prompt | оставлено как open question | «Известные падения и пропуски», «Риски и ограничения» |
