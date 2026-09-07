---
id: fork-developer-instructions-files
status: active
created: 2026-06-08
updated: 2026-09-06
---

# Developer instructions files

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая добавляет top-level
config key `developer_instructions_files`. Он позволяет хранить длинные
developer instructions в отдельных Markdown-файлах и подключать их к
`developer_instructions` в заданном порядке. В конфиге agent role тот же список
собирается вместе с inline-инструкциями самой роли и заменяет весь
parent-derived developer block субагента.

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
- missing/unreadable file считать ошибкой config load;
- разрешить standalone agent role без inline `developer_instructions`, если она
  задаёт непустой `developer_instructions_files`;
- не добавлять role sections к унаследованным developer instructions: наличие
  inline-инструкций роли или непустого списка файлов означает полную замену.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/config/src/config_toml.rs` | Добавляет поле `developer_instructions_files` в `ConfigToml` |
| `codex-rs/config/src/loader/mod.rs` | Нормализует relative paths в TOML config |
| `codex-rs/core/src/config/instruction_files.rs` | Задаёт общий для основного config и agent role контракт чтения, trim, порядка, warnings и ошибок |
| `codex-rs/core/src/config/mod.rs` | Выбирает источники и собирает effective `developer_instructions` основного config |
| `codex-rs/core/src/config/config_tests.rs` | Проверяет parsing, append order, empty warnings и missing-file error |
| `codex-rs/agent-roles/src/agent_role_config.rs` | Принимает standalone role с inline developer instructions или непустым списком файлов |
| `codex-rs/core/src/agent/role.rs` | Собирает собственный developer block роли и заменяет parent-derived значение |
| `codex-rs/core/src/agent/role_tests.rs` | Проверяет relative paths, порядок, замену и очистку унаследованного значения |
| `codex-rs/core/config.schema.json` | Экспортирует config key в schema |
| `codex-rs/core/src/session/turn_context.rs` | Передаёт итоговые инструкции из конфигурации сессии в `TurnContext` |
| `codex-rs/core/src/session/mod.rs` | Добавляет итоговое значение в агрегированное сообщение с ролью `developer` |
| `codex-rs/core/src/context/developer_instructions.rs` | Представляет итоговый текст как `ContextualUserFragment` с ролью `developer` |
| `codex-rs/core/src/context_manager/updates.rs` | Преобразует developer sections в model-visible `ResponseItem` |
| `codex-rs/core/src/client.rs` | В Responses Lite добавляет базовые инструкции отдельным элементом с ролью `developer` перед обычным `input` |
| `codex-rs/core/tests/suite/client.rs` | Проверяет наличие `Config.developer_instructions` в developer message запроса |
| `codex-rs/core/tests/suite/subagent_notifications.rs` | Проверяет developer message реально созданного субагента |
| `codex-rs/file-system/src/lib.rs` | Предоставляет чтение UTF-8 файла; жёсткий предел для fork сейчас отсутствует |

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
9. Итоговое `Config.developer_instructions` передаётся в `TurnContext` и при
   построении начального контекста становится секцией model-visible сообщения с
   ролью `developer`.
10. Responses Lite сохраняет эту секцию отдельным точным `input_text` внутри
    обычного агрегированного developer message. Если базовые инструкции
    одновременно собраны из `model_instructions_files`, Lite adapter добавляет
    их собственным developer item перед основным input; два текста не сливаются,
    не заменяют и не дублируют друг друга.

### Agent role

В отдельном конфиге agent role разрешены оба developer-источника:

```toml
name = "researcher"
description = "Исследователь"
developer_instructions = "Сначала проверяй первичные источники."
developer_instructions_files = [
    "instructions/research.md",
    "instructions/report.md",
]
```

Контракт применения роли:

1. Относительные пути разрешаются от каталога `.toml`-файла роли.
2. Inline `developer_instructions` роли после `trim()` становится первой
   секцией, затем в заданном порядке следуют непустые файлы роли.
3. Наличие inline-секции или непустого `developer_instructions_files` включает
   role override: полученный текст целиком заменяет унаследованный developer
   block и никогда к нему не дописывается.
4. Standalone role может не иметь inline-секции, если список файлов непустой.
5. Пустой или отсутствующий список без inline-секции не является override и
   сохраняет наследование.
6. Пустые файлы по-прежнему дают warning. Если role override был включён
   непустым списком, но все его файлы пусты, effective
   `developer_instructions` становится `None`, а parent block не возвращается.
7. Missing или unreadable role file делает agent type недоступным по общему
   контракту применения agent role.

## Архитектурное решение

Config loader владеет нормализацией путей, а сборка `Config` — последовательным
чтением и объединением секций. Готовая строка оборачивается в
`DeveloperInstructions`, рендерится как фрагмент с ролью `developer` и через
общий `build_rendered_message` попадает в доступный модели `ResponseItem`, поэтому
последующие runtime-слои не знают, сколько файлов было источником. В Responses
Lite базовые инструкции добавляются перед основным `input` отдельным элементом с
ролью `developer`. Runtime override останавливает чтение файлов до I/O: явно
переданные инструкции нельзя неожиданно дополнять конфигурацией.

Agent role применяется позже, уже к parent-derived config. Поэтому её
собственные inline/file sections образуют новый developer block и заменяют даже
значение, которое родитель получил из `ConfigOverrides` или своих
`developer_instructions_files`. При cold resume роль применяется заново к
актуальному parent-derived config и перечитывает свои файлы.

## Порядок повторения при переносе

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
        let path_uri = PathUri::from_abs_path(path);
        let contents = fs
            .read_file_text(&path_uri, ReadFileOptions::default(), /*sandbox*/ None)
            .await
            .map_err(|e| {
                std::io::Error::new(
                    e.kind(),
                    format!(
                        "failed to read developer instructions file {}: {e}",
                        path.display()
                    ),
                )
            })?;
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

### 5. Применить список в agent role

В `codex-rs/core/src/agent/role.rs` нужно отличать отсутствие role override от
его effective значения. Непустой список файлов или inline-секция роли включает
замену parent-derived developer instructions, после чего общий загрузчик
собирает только role-owned sections.

Standalone discovery должен считать непустой `developer_instructions_files`
достаточным источником developer instructions и по-прежнему отклонять роль, у
которой нет ни inline-секции, ни непустого списка.

## Проверки

Исполняемая карта card-level regression tests:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "ordered merge, override precedence, warnings, errors, file-only agent role, agent-role replacement и model-visible delivery",
      "argv": ["just", "test", "-p", "codex-core", "developer_instructions"]
    }
  ]
}
```

Дополнительно обязателен `fork generators`, поскольку доработка меняет `ConfigToml` и config schema.

## Риски и ограничения

### Ограничения

- Не читать files, если `developer_instructions` уже передан как runtime
  override сверху. Это важно для tests, app-server или surfaces, которые
  передают developer instructions программно.
- Не делать empty file hard error: это warning, чтобы временно пустой policy file
  не ломал запуск.
- Не проглатывать missing file: это config error, иначе профиль может silently
  потерять важные правила.
- Не менять `model_instructions_file`: это отдельная base instructions
  surface, а не developer surface.
- Не смешивать parent и role developer sections: это нарушит обещанную замену
  и незаметно вернёт личности или политики родительского профиля.

### Риски

- Порядок файлов является частью контракта. Сортировка списка или чтение через
  unordered collection сломают profile layering.
- Trim убирает внешние пустые строки в файлах. Если в будущем появится
  template, где начальная/конечная пустая строка значима, это нужно
  пересогласовать.
- Если schema не обновить, поле может работать в runtime, но быть невидимым
  для config tooling.
- Без жёсткого предела один файл или сумма inline/file sections может создать
  model-visible item больше допустимой границы.
- Молчаливое усечение для profile-defining rules недопустимо; политику ошибки,
  разбиения или другой bounded representation нужно согласовать в owner scope
  `session/context aggregation`.
- Изменение или исчезновение файла роли между spawn/cold resume приводит к
  новому содержимому или ошибке повторного применения роли.
