---
id: fork-memory-read-template-path
status: active
created: 2026-06-08
updated: 2026-06-20
source_scope: rust-v0.141.0..HEAD
---

# Memory read template: `[memories].read_template_path`

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая позволяет профилю
переопределять Markdown-шаблон read-path memory prompt через
`[memories].read_template_path`.

Карточка нужна как самостоятельный handoff: следующий агент должен понять, что
именно было изменено, почему, как повторить перенос на новый upstream и какие
проверки нужны, не читая transcript.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Основной commit | `9c9af8853 Make memory read template configurable` |
| Текущая база проверки | `rust-v0.141.0..HEAD`, ветка `hermione-0.141.0` |
| Главный config key | `[memories].read_template_path` |
| Runtime-владелец после merge `rust-v0.141.0` | `codex-rs/ext/memories/src/prompts.rs` |
| Исторический владелец до merge | `codex-rs/memories/read/src/prompts.rs` |
| Checkpoint перед карточкой | Пропущен по явному разрешению пользователя от 2026-06-08 |

## Зачем это нужно

Upstream Codex держит memory read-path developer prompt как embedded template.
Для Hermione-профиля нужно менять форму memory-инструкций без patching upstream
шаблона и без вписывания большого текста прямо в `config.toml`.

Доработка вводит профильный путь к Markdown-файлу. Если путь задан, prompt
строится из этого файла; если путь не задан, сохраняется штатный embedded
template.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/config/src/types.rs` | Добавляет `read_template_path` в `MemoriesToml` и `MemoriesConfig` |
| `codex-rs/core/config.schema.json` | Экспортирует поле в JSON schema config |
| `codex-rs/core/src/config/config_tests.rs` | Проверяет TOML parsing и effective `MemoriesConfig` |
| `codex-rs/ext/memories/src/extension.rs` | Прокидывает путь из `Config.memories` в memory extension context |
| `codex-rs/ext/memories/src/prompts.rs` | Загружает embedded или configured template и рендерит prompt |
| `codex-rs/ext/memories/src/prompts_tests.rs` | Проверяет embedded template и configured template |
| `codex-rs/ext/memories/src/tests.rs` | Обновляет extension test config новым полем |
| `codex-rs/memories/README.md` | Документирует override, placeholders и фактический путь к runtime-шаблону |

## Итоговый контракт

1. `MemoriesToml` получает поле:

   ```rust
   pub read_template_path: Option<AbsolutePathBuf>,
   ```

2. `MemoriesConfig` получает такое же поле. Значение по умолчанию: `None`.
3. `From<MemoriesToml> for MemoriesConfig` переносит `read_template_path` без
   дополнительной нормализации: `AbsolutePathBuf` уже пришёл из config loader.
4. `MemoriesExtensionConfig::from_config` сохраняет
   `config.memories.read_template_path.clone()`.
5. `ContextContributor for MemoriesExtension` вызывает:

   ```rust
   build_memory_tool_developer_instructions(
       &config.codex_home,
       config.read_template_path.as_ref(),
   )
   ```

6. Если `read_template_path` отсутствует, используется embedded template
   `templates/memories/read_path.md`.
7. Если `read_template_path` задан, файл читается асинхронно через `tokio::fs`.
8. Если configured template нельзя прочитать или нельзя распарсить, memory
   developer prompt становится недоступным (`None`), а не падает весь config
   load.
9. Разрешены только placeholders `{{ base_path }}` и `{{ memory_summary }}`.
10. Любой неизвестный placeholder делает prompt unavailable (`None`).
11. `memory_summary.md` по-прежнему читается из `${codex_home}/memories`,
    trim'ится и обрезается по
    `MEMORY_TOOL_DEVELOPER_INSTRUCTIONS_SUMMARY_TOKEN_LIMIT`.
12. Если summary пустая или отсутствует, prompt не добавляется.

## Пошаговое воспроизведение

### 1. Добавить config surface

В `codex-rs/config/src/types.rs` найти `MemoriesToml` и добавить:

```rust
/// Optional Markdown template used to render memory usage instructions into developer prompts.
pub read_template_path: Option<AbsolutePathBuf>,
```

В `MemoriesConfig` добавить:

```rust
pub read_template_path: Option<AbsolutePathBuf>,
```

В `Default for MemoriesConfig` поставить:

```rust
read_template_path: None,
```

В `From<MemoriesToml> for MemoriesConfig` перенести:

```rust
read_template_path: toml.read_template_path,
```

### 2. Обновить schema

Если меняется `ConfigToml` или nested config type, проектное правило требует
обновить `codex-rs/core/config.schema.json` через `just write-config-schema`.
Для этой fork-доработки schema должна содержать поле `read_template_path` внутри
definitions для memories config.

Важно: в текущем workflow Rust/Cargo/`just` запускаются только на
`f-ms-dev:/home/slader/Projects/codex`, если пользователь не разрешил иначе.

### 3. Прокинуть путь в extension config

В `codex-rs/ext/memories/src/extension.rs` расширить
`MemoriesExtensionConfig`:

```rust
pub(crate) read_template_path: Option<AbsolutePathBuf>,
```

В `from_config` добавить:

```rust
read_template_path: config.memories.read_template_path.clone(),
```

В `contribute` передать `config.read_template_path.as_ref()` в builder
developer instructions.

### 4. Обновить prompt builder

В текущей проверке после merge `rust-v0.141.0` владелец находится в
`codex-rs/ext/memories/src/prompts.rs`. В более старых ветках этот код мог жить
в `codex-rs/memories/read/src/prompts.rs`; при переносе на новый upstream нужно
сначала найти живой вызов `build_memory_tool_developer_instructions`.

Сигнатура должна стать:

```rust
pub(crate) async fn build_memory_tool_developer_instructions(
    codex_home: &AbsolutePathBuf,
    read_template_path: Option<&AbsolutePathBuf>,
) -> Option<String>
```

Добавить загрузку template:

```rust
async fn load_memory_tool_developer_instructions_template(
    read_template_path: Option<&AbsolutePathBuf>,
) -> Option<Template> {
    let Some(read_template_path) = read_template_path else {
        return Some(MEMORY_TOOL_DEVELOPER_INSTRUCTIONS_TEMPLATE.clone());
    };

    let template = fs::read_to_string(read_template_path).await.ok()?;
    Template::parse(&template).ok()
}
```

Добавить рендер с whitelist placeholders:

```rust
fn render_memory_tool_developer_instructions_template(
    template: &Template,
    base_path: &str,
    memory_summary: &str,
) -> Option<String> {
    let mut variables = Vec::new();
    for placeholder in template.placeholders() {
        match placeholder {
            "base_path" => variables.push(("base_path", base_path)),
            "memory_summary" => variables.push(("memory_summary", memory_summary)),
            _ => return None,
        }
    }
    template.render(variables).ok()
}
```

### 5. Обновить docs

В `codex-rs/memories/README.md` добавить, что read-path template можно
переопределить через `[memories].read_template_path`, а configured template
рендерится с `{{ base_path }}` и `{{ memory_summary }}`. Отдельно указать, что
unknown placeholders делают read-path prompt unavailable.

После merge `rust-v0.141.0` README также должен указывать фактический
канонический путь к runtime-шаблону:
`codex-rs/ext/memories/templates/memories/read_path.md`, а не исторический путь
в `codex-rs/memories/read`.

## Регрессионное покрытие

Покрытие, которое должно присутствовать в diff:

- `codex-rs/core/src/config/config_tests.rs`:
  - `test_toml_parsing` проверяет `[memories].read_template_path`;
  - effective `MemoriesConfig` содержит `read_template_path`.
- `codex-rs/ext/memories/src/prompts_tests.rs`:
  - `build_memory_tool_developer_instructions_renders_embedded_template`
    вызывает builder с `None`;
  - `build_memory_tool_developer_instructions_uses_configured_template`
    создаёт temp template, использующий `{{ base_path }}` и
    `{{ memory_summary }}`, и проверяет точный результат.
- `codex-rs/ext/memories/src/tests.rs`:
  - test configs явно задают `read_template_path: None`, если создают
    `MemoriesExtensionConfig` напрямую.

Полезный дополнительный тест при будущей правке: configured template с
unknown placeholder должен возвращать `None`.

## Проверки

Историческая карточка не утверждает, что проверки были запущены в текущем turn.
Для повторения доработки разумные проверки:

Исполняемая карта `fork tests`:

Данные ниже являются текущим блоком `fork-tests.v1`, который читает
`fork tests`.

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "stdio fixture binary",
      "argv": [
        "cargo",
        "build",
        "--manifest-path",
        "codex-rs/Cargo.toml",
        "-p",
        "codex-rmcp-client",
        "--bin",
        "test_stdio_server"
      ]
    },
    {
      "purpose": "core config",
      "argv": ["just", "test", "-p", "codex-core", "config"]
    },
    {
      "purpose": "memories extension",
      "argv": ["just", "test", "-p", "codex-memories-extension"]
    }
  ]
}
```

1. На `f-ms-dev:/home/slader/Projects/codex`:
   - `just write-config-schema`;
   - целевые тесты для `codex-core` config и `codex-ext-memories`, если
     пользователь разрешил тесты.
2. Локально без Rust/Cargo:
   - `git diff --check`;
   - `rg -n "read_template_path|build_memory_tool_developer_instructions" codex-rs`.

## Ограничения

- Не менять пользовательский `hermione.config.toml` в рамках этой doработки.
- Не писать сам template в repo, если задача только добавляет config surface.
- Не использовать unknown placeholders как частичный render: это должно быть
  fail-closed через `None`.
- Не запускать Rust/Cargo/`just` локально в текущем workflow; использовать
  `f-ms-dev`, если пользователь разрешил нужную проверку.

## Риски

- Upstream может переносить memory extension code между crates. При rebase
  искать живой owner по `build_memory_tool_developer_instructions`, а не
  полагаться на старый путь.
- Silent `None` при unreadable template означает, что профиль может лишиться
  memory instructions без config-load ошибки. Это осознанная деградация, но при
  диагностике prompt нужно проверять путь и placeholders.
- Если schema не обновить, config key будет работать в Rust type, но tooling и
  редакторские подсказки будут устаревшими.

## Сводка покрытия

| Пункт | Статус | Где отражено |
| --- | --- | --- |
| Добавить `[memories].read_template_path` | перенесено | "Итоговый контракт", "Пошаговое воспроизведение" |
| Сохранить embedded template как default | перенесено | "Итоговый контракт" |
| Поддержать только `base_path` и `memory_summary` | перенесено | "Итоговый контракт", "Пошаговое воспроизведение" |
| Учесть перенос owner crate после upstream merge | перенесено | "Пошаговое воспроизведение", "Риски" |
| Синхронизировать README с фактическим путём к runtime-шаблону | перенесено | "Карта файлов", "Пошаговое воспроизведение" |
| Зафиксировать тесты и проверки | перенесено | "Регрессионное покрытие", "Проверки" |
