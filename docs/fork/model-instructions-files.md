---
id: fork-model-instructions-files
status: active
created: 2026-07-24
updated: 2026-08-30
---

# Model instructions files

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая добавляет top-level
config key `model_instructions_files`. Он позволяет разделить базовые инструкции
модели между несколькими Markdown-файлами и собрать их в одно значение
`Config.base_instructions`. Обычный Responses transport передаёт это значение
через top-level поле `instructions`; Responses Lite использует свой upstream
wire contract с одним developer item.

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
- объединённый текст ограничен `10_000` оценочных токенов и при превышении
  предела отклоняется целиком без усечения;
- загрузка файлов сама не создаёт дополнительные сообщения `input` или элементы
  с ролью `developer`; стандартный Responses transport использует top-level
  `instructions`, а Responses Lite применяет общий upstream wire adapter;
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
| `codex-rs/core/src/config/config_tests.rs` | Покрывает разбор, порядок, предел размера, ошибки и приоритет |
| `codex-rs/core/src/config/config_loader_tests.rs` | Покрывает config layers и CLI config override |
| `codex-rs/core/tests/suite/client.rs` | Проверяет обычное поле `instructions` и раздельные developer items в Responses Lite |
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

Итог становится одним `Config.base_instructions`. В стандартном Responses wire
mode это одно верхнеуровневое поле:

```json
{
  "instructions": "Base instructions.\n\nTools instructions.\n\nWorkflow instructions."
}
```

Доработка не создаёт несколько `instructions`, сама не добавляет элементы в
`input` и не меняет `developer_instructions_files`.

Поскольку объединённый текст становится одним model-visible элементом, его
размер не может превышать `10_000` оценочных токенов. Предел вычисляется по
байтовой оценке после `trim()` каждой секции и добавления разделителей. При
превышении загрузка завершается `InvalidInput` с сообщением:

```text
`model_instructions_files` exceeds the model-context limit of 10000 estimated tokens (<actual>)
```

Текст не усекается: частичная базовая инструкция могла бы незаметно удалить
обязательный раздел контракта модели.

В Responses Lite upstream wire-адаптер намеренно не сериализует верхнеуровневое
`instructions` и преобразует единое `Config.base_instructions` через
`BaseInstructionsFragment` в отдельный developer item перед основным input. Это
преобразование не зависит от числа исходных файлов и не является дополнительной
логикой этой fork-доработки. Если одновременно заданы
`developer_instructions_files`, их итоговое значение остаётся отдельной точной
content-секцией в обычном агрегированном developer message: два слоя не
объединяются и не заменяют друг друга.

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

### Граница runtime-конфигурации

Исходные пути `model_instructions_files` используются только при загрузке
`Config`; дальнейший runtime получает уже собранное значение
`base_instructions`. Механизм снимка или фиксации эффективной конфигурации
потока должен сохранять итоговые `base_instructions`, а не исходные пути
файлов.

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

## Порядок повторения при переносе

При переносе на следующий upstream доработку нужно воспроизвести в следующем
порядке:

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
9. отклонить объединённый текст больше `10_000` оценочных токенов без усечения;
10. обновить config schema через skill-owned generator;
11. восстановить unit, loader и сквозное покрытие обычного Responses и
    Responses Lite.

## Проверки

Исполняемая карта card-level regression tests:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "порядок загрузки, жёсткий предел размера, приоритет, взаимоисключение, ошибки и отдельная доставка Responses Lite",
      "argv": ["just", "test", "-p", "codex-core", "model_instructions_files"]
    }
  ]
}
```

Дополнительно обязателен `fork generators`, поскольку доработка меняет config
types и schema.

## Риски и ограничения

- Порядок массива является частью prompt-контракта; сортировка запрещена.
- Порядок и точный разделитель защищены регрессионными тестами.
- `trim()` удаляет внешние пустые строки каждого документа.
- Разделитель всегда равен `\n\n`; содержимое внутри документа сохраняется.
- Файлы читаются целиком, но объединённый model-visible текст ограничен `10_000`
  оценочных токенов.
- Молчаливое усечение или пропуск файла недопустимы; превышение предела всегда
  является ошибкой конфигурации.
- Новый ключ является fork-specific и должен переноситься при следующих
  upstream migration.
- Schema обновлена вместе с реализацией. При будущих изменениях она должна
  оставаться синхронизированной с config types.
