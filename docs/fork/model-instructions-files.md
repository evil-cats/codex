---
id: fork-model-instructions-files
status: active
created: 2026-07-24
updated: 2026-08-13
---

# Model instructions files

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая добавляет top-level
config key `model_instructions_files`. Он позволяет разделить базовые инструкции
модели между несколькими Markdown-файлами и собрать их в одно значение
Responses API `instructions`.

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
9. исключить startup-only список из thread config lock рядом с одиночным ключом;
10. обновить config schema через skill-owned generator;
11. восстановить unit, loader и сквозное Responses API покрытие.

## Проверки

Исполняемая карта card-level regression tests:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "ordered loading, precedence, mutual exclusion и ошибки instruction files",
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
- Файлы читаются целиком; отдельный предел размера в этой доработке отсутствует.
- Молчаливое усечение или пропуск файла недопустимы.
- Новый ключ является fork-specific и должен переноситься при следующих
  upstream migration.
- Schema обновлена вместе с реализацией. При будущих изменениях она должна
  оставаться синхронизированной с config types.
