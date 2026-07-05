---
id: fork-developer-instructions-files
status: active
created: 2026-06-08
updated: 2026-06-19
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
| Текущая база проверки | `rust-v0.137.0..HEAD`, ветка `hermione-0.137.0` |
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

После изменения `ConfigToml` обновить `codex-rs/core/config.schema.json` через
`just write-config-schema` на `f-ms-dev`, если пользователь разрешил запуск.

Schema должна показывать `developer_instructions_files` как array path values
с default `[]`.

## Регрессионное покрытие

Покрытие, которое должно быть в diff:

- TOML parsing:
  - `developer_instructions_files = ["<abs-a>", "<abs-b>"]` десериализуется в
    `ConfigToml.developer_instructions_files`.
- Relative path normalization:
  - relative entries in array become absolute against config base dir.
- Runtime assembly:
  - `developer_instructions_files_are_appended_in_order` проверяет inline
    section, первый файл и второй файл, соединённые через `\n\n`.
  - `developer_instructions_files_skip_empty_files_with_warning` проверяет, что
    empty file пропущен, непустой файл добавлен, warning есть.
  - `developer_instructions_files_reject_missing_file` проверяет `NotFound` и
    текст ошибки.
  - `developer_instructions_override_skips_files` проверяет, что runtime
    override `developer_instructions` не читает файлы из config и возвращает
    переданное override-значение как итоговые developer instructions.

## Проверки

Исполняемая карта `fork tests`:

Данные ниже являются текущим блоком `fork-tests.v1`, который читает
`fork tests`.

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

Для повторения доработки:

1. На `f-ms-dev:/home/slader/Projects/codex`, если пользователь разрешил:
   - `just write-config-schema`;
   - targeted config tests для `codex-core`;
   - `just build-fast-release` как compile-check.
2. Локально без Rust/Cargo:
   - `git diff --check`;
   - `rg -n "developer_instructions_files" codex-rs`.

В текущем turn карточка создана без запуска тестов/debug и без локального
Rust/Cargo/`just`.

В миграционном проходе 2026-06-19 карточка сверена с текущей рабочей копией без
запуска сборки, тестов, генераторов, форматирования или `fix` по ограничению
основного агента. Для закрытия пробела покрытия добавлен тест
`developer_instructions_override_skips_files`, но он не запускался в этом
подагентском проходе.

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

## Сводка покрытия

| Пункт | Статус | Где отражено |
| --- | --- | --- |
| Добавить top-level `developer_instructions_files` | перенесено | "Итоговый контракт", "Пошаговое воспроизведение" |
| Сохранить inline `developer_instructions` первой секцией | перенесено | "Итоговый контракт" |
| Читать файлы в заданном порядке | перенесено | "Итоговый контракт", "Регрессионное покрытие" |
| Empty file как warning | перенесено | "Итоговый контракт", "Регрессионное покрытие" |
| Missing file как error | перенесено | "Итоговый контракт", "Регрессионное покрытие" |
| Не читать files при runtime override | перенесено | "Итоговый контракт", "Ограничения", "Регрессионное покрытие" |
