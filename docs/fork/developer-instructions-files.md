---
id: fork-developer-instructions-files
status: active
created: 2026-06-08
updated: 2026-07-14
source_scope: rust-v0.137.0..HEAD
---

# Developer instructions files

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая добавляет top-level
config key `developer_instructions_files`. Он позволяет хранить длинные
developer instructions в отдельных Markdown-файлах и подключать их к
`developer_instructions` в заданном порядке.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Основной commit | `d976b54ed Support developer instructions files` |
| Текущая база проверки | `rust-v0.144.4`, ветка `hermione-0.144.4` |
| Config key | `developer_instructions_files` |
| Тип | `Vec<AbsolutePathBuf>` |
| Checkpoint перед карточкой | Пропущен по явному разрешению пользователя от 2026-06-08 |

## Зачем это нужно

Inline `developer_instructions` неудобен для больших profile-defining правил:
его тяжело редактировать, читать, review'ить и подключать частями. Hermione
профиль использует Markdown policy files, поэтому config должен уметь
подключать несколько файлов как единый developer block.

Нужное поведение:

- сохранить поддержку inline `developer_instructions`;
- добавить список Markdown-файлов;
- читать файлы в порядке списка;
- соединять непустые секции через пустую строку;
- пустые файлы пропускать с warning;
- missing/unreadable file считать ошибкой config load.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/config/src/config_toml.rs` | Добавляет поле `developer_instructions_files` в `ConfigToml` |
| `codex-rs/config/src/loader/mod.rs` | Нормализует relative paths в TOML config |
| `codex-rs/core/src/config/mod.rs` | Читает файлы и собирает effective `developer_instructions` |
| `codex-rs/core/src/config/config_tests.rs` | Проверяет parsing, append order, empty warnings и missing-file error |
| `codex-rs/core/config.schema.json` | Экспортирует config key в schema |

## Итоговый контракт

1. `ConfigToml` получает:

   ```rust
   /// Markdown files appended to `developer_instructions` in order.
   #[serde(default)]
   pub developer_instructions_files: Vec<AbsolutePathBuf>,
   ```

2. Значение является top-level key, не `[profile]` и не `[developer]`.
3. В `config.toml` путь может быть relative; loader должен резолвить его
   относительно base dir config-файла так же, как другие path-like fields.
4. Если runtime override `developer_instructions` передан в
   `Config::load_from_base_config_with_overrides`, файлы из config не читаются.
5. Если runtime override не передан:
   - inline `cfg.developer_instructions` trim'ится;
   - если inline значение непустое, оно становится первой секцией;
   - каждый файл читается в порядке `developer_instructions_files`;
   - каждый файл trim'ится;
   - непустые файлы добавляются как отдельные секции;
   - секции соединяются через `\n\n`.
6. Пустой файл не валит загрузку config, но добавляет startup warning:

   ```text
   developer instructions file is empty: <path>
   ```

7. Missing или unreadable file валит config load с `std::io::Error`, где text
   содержит:

   ```text
   failed to read developer instructions file <path>: <source error>
   ```

8. Если inline instructions пустые и все files пустые или список пуст,
   effective `Config.developer_instructions` остаётся `None`.

## Пошаговое воспроизведение

### 1. Добавить поле в TOML config

В `codex-rs/config/src/config_toml.rs` добавить поле рядом с
`developer_instructions`, а не рядом с `model_instructions_file`:

```rust
/// Markdown files appended to `developer_instructions` in order.
#[serde(default)]
pub developer_instructions_files: Vec<AbsolutePathBuf>,
```

### 2. Обновить relative path normalization

В `codex-rs/config/src/loader/mod.rs` убедиться, что
`resolve_relative_paths_in_config_toml` обрабатывает массив
`developer_instructions_files`.

Минимальный тестовый TOML:

```toml
model_instructions_file = "./some_file.md"
developer_instructions_files = ["./developer_a.md", "./developer_b.md"]
model = "gpt-1000"
foo = "xyzzy"
```

Ожидание:

- `model_instructions_file` становится absolute;
- каждый элемент `developer_instructions_files` становится absolute;
- обычное поле `model` сохраняется;
- неизвестное поле `foo` сохраняется на уровне raw TOML loader test.

### 3. Собрать effective developer instructions

В `codex-rs/core/src/config/mod.rs` читать files до финального вычисления
`developer_instructions`, но только если нет runtime override.

Семантический skeleton:

```rust
let file_developer_instructions = if developer_instructions.is_none() {
    let mut sections = Vec::new();
    for path in &cfg.developer_instructions_files {
        let contents = fs.read_file_text(path, /*sandbox*/ None).await?;
        let contents = contents.trim();
        if contents.is_empty() {
            startup_warnings.push(format!(
                "developer instructions file is empty: {}",
                path.display()
            ));
        } else {
            sections.push(contents.to_string());
        }
    }
    sections
} else {
    Vec::new()
};
```

Затем:

```rust
let developer_instructions = developer_instructions.or_else(|| {
    let mut sections = Vec::new();
    if let Some(inline_developer_instructions) = cfg.developer_instructions {
        let inline_developer_instructions = inline_developer_instructions.trim();
        if !inline_developer_instructions.is_empty() {
            sections.push(inline_developer_instructions.to_string());
        }
    }
    sections.extend(file_developer_instructions);
    (!sections.is_empty()).then(|| sections.join("\n\n"))
});
```

### 4. Обновить schema

После изменения `ConfigToml` обновление `codex-rs/core/config.schema.json`
принадлежит skill-owned gate `fork generators`.

Schema должна показывать `developer_instructions_files` как array со значениями
paths и default `[]`.

## Ожидаемое покрытие diff

Покрытие, которое должно быть в diff:

- Парсинг TOML:
  - `developer_instructions_files = ["<abs-a>", "<abs-b>"]` десериализуется в
    `ConfigToml.developer_instructions_files`.
- Нормализация относительных путей:
  - относительные элементы массива становятся абсолютными относительно базового
    каталога config-файла.
- Сборка runtime-значения:
  - `developer_instructions_files_are_appended_in_order` проверяет inline-секцию,
    первый файл и второй файл, соединённые через `\n\n`.
  - `developer_instructions_files_skip_empty_files_with_warning` проверяет, что
    пустой файл пропущен, непустой файл добавлен, а предупреждение запуска
    содержит путь пустого файла.
  - `developer_instructions_files_reject_missing_file` проверяет `NotFound` и
    префикс текста ошибки с путем отсутствующего файла.
  - `developer_instructions_override_skips_files` проверяет, что runtime override
    `developer_instructions` не читает файлы из config и возвращает
    переданное override-значение как итоговые developer instructions.

## Проверки

### Смысловое покрытие

Покрытие уровня карточки должно подтверждать весь контракт
`developer_instructions_files`:

- парсинг TOML: `developer_instructions_files = ["<abs-a>", "<abs-b>"]`
  десериализуется в `ConfigToml.developer_instructions_files`;
- нормализация относительных путей: относительные элементы массива становятся
  абсолютными относительно базового каталога config-файла;
- сборка runtime-значения: inline-секция, первый файл и второй файл добавляются
  в порядке списка config и соединяются через `\n\n`;
- пустой файл пропускается, непустой файл добавляется, а предупреждение запуска
  содержит путь пустого файла;
- отсутствующий файл возвращает `NotFound`, а текст ошибки содержит путь и
  исходную ошибку;
- runtime override `developer_instructions` не читает файлы из config и
  возвращает переданное override-значение;
- schema artifact показывает `developer_instructions_files` как array со
  значениями paths и default `[]`.

### Владелец исполняемой карты

Проверки уровня карточки запускает skill-owned command `fork tests`. Карточка
хранит машиночитаемый блок `fork-tests.v1`; внутренние `argv` ниже являются
данными для `fork tests`, а не пользовательским runbook прямого запуска.

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "developer instructions",
      "argv": ["just", "test", "-p", "codex-core", "developer_instructions"]
    }
  ]
}
```

### Дополнительные gates

- Обновление schema для `codex-rs/core/config.schema.json` принадлежит
  `fork generators`.
- Регрессионное покрытие уровня карточки принадлежит skill-owned владельцу
  `fork tests`; фильтр карточки и внутренние `argv` задаются skill-owned
  workflow и блоком `fork-tests.v1`.
- Проверка сборки в общем fork-проходе принадлежит `fork build-fast`.
- Проверка формы и owner artifact карточки принадлежит `fork cards validate`.
- Ручные audit-подсказки старого текста (`git diff --check` и поиск
  `developer_instructions_files` по `codex-rs`) не являются заменой
  skill-owned gates.

### Исторические результаты

В первоначальном проходе 2026-06-08 карточка создана без запуска тестов,
отладочных команд и без локального Rust/Cargo/`just`.

В миграционном проходе 2026-06-19 карточка сверена с текущей рабочей копией без
запуска сборки, тестов, генераторов, форматирования или `fix` по ограничению
основного агента. Для закрытия пробела покрытия добавлен тест
`developer_instructions_override_skips_files`, но он не запускался в этом
подагентском проходе.

В миграционном проходе 2026-07-05 для `rust-v0.142.5` карточка сверена с текущей
рабочей копией без запуска сборки, тестов, генераторов, форматирования или
`fix` по ограничению подагентского запуска. Усилены проверки в
`developer_instructions_files_skip_empty_files_with_warning` и
`developer_instructions_files_reject_missing_file`: предупреждение сверяется с
полным текстом и путем пустого файла, а ошибка отсутствующего файла - с
префиксом сообщения, включающим путь.

В миграционном проходе 2026-07-08 для `rust-v0.143.0` карточка сверена с текущей
рабочей копией без запуска сборки, тестов, генераторов, форматирования или
`fix` по ограничению подагентского запуска. Реализация в owner-файлах уже
сохраняет контракт карточки; кодовых правок не потребовалось.

В миграционном проходе 2026-07-10 для `rust-v0.144.1` карточка сверена с текущей
рабочей копией без запуска сборки, тестов, генераторов, форматирования или
`fix` по ограничению подагентского запуска. Реализация в owner-файлах уже
сохраняет контракт карточки; кодовых правок не потребовалось.

В миграционном проходе 2026-07-14 для `rust-v0.144.4` карточка сверена с текущей
рабочей копией без запуска сборки, тестов, генераторов, форматирования или
`fix` по ограничению подагентского запуска. Реализация в owner-файлах уже
сохраняет контракт карточки; кодовых правок не потребовалось.

Старый текст карточки называл прямые команды `just write-config-schema` и
`just build-fast-release` как маршрут повторения на `f-ms-dev` при разрешении
пользователя. После перехода на skill-owned workflow они сохранены только как
исторический след прежнего runbook: актуальные владельцы этих проверок -
`fork generators` и `fork build-fast`.

### Известные падения и пропуски

- Тесты, сборка, генераторы, форматирование и `fix` не запускались в текущем
  подагентском проходе.
- Тест `developer_instructions_override_skips_files`, добавленный в
  миграционном проходе 2026-06-19, не запускался в том подагентском проходе.
- Известных зафиксированных падений для этой карточки нет; оставшийся риск -
  schema artifact может устареть, если после изменения `ConfigToml` не пройти
  `fork generators`.

## Ограничения

- Не читать files, если `developer_instructions` уже передан как runtime
  override сверху. Это важно для tests, app-server или surfaces, которые
  передают developer instructions программно.
- Не делать empty file hard error: это warning, чтобы временно пустой policy file
  не ломал запуск.
- Не проглатывать missing file: это config error, иначе профиль может silently
  потерять важные правила.
- Не менять `model_instructions_file`: это отдельная base instructions
  surface, а не developer surface.

## Риски

- Порядок файлов является частью контракта. Сортировка списка или чтение через
  unordered collection сломают profile layering.
- Trim убирает внешние пустые строки в файлах. Если в будущем появится
  template, где начальная/конечная пустая строка значима, это нужно
  пересогласовать.
- Если schema не обновить, поле может работать в runtime, но быть невидимым
  для config tooling.

## Проверка покрытия

| Пункт | Статус | Где отражено |
| --- | --- | --- |
| Добавить top-level `developer_instructions_files` | перенесено | "Итоговый контракт", "Пошаговое воспроизведение" |
| Сохранить inline `developer_instructions` первой секцией | перенесено | "Итоговый контракт" |
| Читать файлы в заданном порядке | перенесено | "Итоговый контракт", "Ожидаемое покрытие diff" |
| Empty file как warning | перенесено | "Итоговый контракт", "Ожидаемое покрытие diff" |
| Missing file как error | перенесено | "Итоговый контракт", "Ожидаемое покрытие diff" |
| Не читать files при runtime override | перенесено | "Итоговый контракт", "Ограничения", "Ожидаемое покрытие diff" |
