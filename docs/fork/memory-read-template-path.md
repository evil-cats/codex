---
id: fork-memory-read-template-path
status: active
created: 2026-06-08
updated: 2026-07-05
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

Если меняется `ConfigToml` или nested config type, schema-артефакт
`codex-rs/core/config.schema.json` должен быть синхронизирован через
skill-owned владельца `fork generators`. Для этой fork-доработки schema должна
содержать поле `read_template_path` внутри definitions для memories config.

Карточка не является runbook запуска генератора. Историческое упоминание
внутреннего argv генерации сохранено ниже только как след старого формата
карточки, а не как нормативный шаг воспроизведения.

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

## Проверки

### Смысловое покрытие

Покрытие, которое должно присутствовать в diff:

- `codex-rs/config/src/types.rs`:
  - `MemoriesToml` содержит `read_template_path: Option<AbsolutePathBuf>`;
  - `MemoriesConfig` содержит такое же поле;
  - `Default for MemoriesConfig` ставит `read_template_path: None`;
  - `From<MemoriesToml> for MemoriesConfig` переносит значение без дополнительной
    нормализации.
- `codex-rs/core/config.schema.json` содержит schema-подтверждение для
  `read_template_path` внутри definitions memories config; актуализация этого
  сгенерированного артефакта принадлежит `fork generators`.
- `codex-rs/ext/memories/src/extension.rs` прокидывает
  `config.memories.read_template_path.clone()` в `MemoriesExtensionConfig` и
  передает `config.read_template_path.as_ref()` в prompt builder.
- `codex-rs/ext/memories/src/prompts.rs` выбирает embedded template при `None`,
  асинхронно читает configured template при `Some(path)`, рендерит только
  placeholders `{{ base_path }}` и `{{ memory_summary }}` и возвращает `None`
  при unreadable, unparsable или unknown-placeholder template.
- `codex-rs/memories/README.md` документирует `[memories].read_template_path`,
  разрешенные placeholders, fail-closed поведение для unknown placeholders и
  фактический путь к runtime-шаблону
  `codex-rs/ext/memories/templates/memories/read_path.md`.

Регрессионное покрытие, которое должна сохранять доработка:

- `codex-rs/core/src/config/config_tests.rs`:
  - `test_toml_parsing` проверяет `[memories].read_template_path`;
  - итоговый `MemoriesConfig` содержит `read_template_path`.
- `codex-rs/ext/memories/src/prompts_tests.rs`:
  - `build_memory_tool_developer_instructions_renders_embedded_template`
    вызывает builder с `None`;
  - `build_memory_tool_developer_instructions_uses_configured_template`
    создаёт temp template, использующий `{{ base_path }}` и
    `{{ memory_summary }}`, и проверяет точный результат.
- `codex-rs/ext/memories/src/tests.rs`:
  - тестовые конфигурации явно задают `read_template_path: None`, если создают
    `MemoriesExtensionConfig` напрямую.

Полезный дополнительный тест при будущей правке: configured template с unknown
placeholder должен возвращать `None`.

### Владелец исполняемой карты

Проверки уровня карточки запускает `fork tests`. Внутренние argv живут в блоке
`fork-tests.v1` ниже и являются данными для skill-owned владельца, а не ручным
runbook для прямого запуска `cargo` или `just`.

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

### Дополнительные gates

- Форму карточки, наличие strict-подразделов `Проверки` и связь с блоком
  `fork-tests.v1` проверяет `fork cards validate`.
- Gate schema/generator для `codex-rs/core/config.schema.json` принадлежит
  `fork generators`; прямой внутренний argv генерации не должен быть
  нормативным шагом в карточке.
- Общий проверочный проход родительского агента выбирает нужные skill-owned
  проверки после прохода по карточкам. Эта карточка не должна подменять его
  списком прямых `just`/`cargo` команд.

### Исторические результаты

- Историческая карточка не утверждает, что проверки были запущены в текущем
  turn.
- Checkpoint перед карточкой был пропущен по явному разрешению пользователя от
  2026-06-08.
- Старый формат карточки указывал `just write-config-schema` как прямой способ
  обновить `codex-rs/core/config.schema.json`. В новом формате это сохранено
  только как исторический контекст старого подтверждения schema/generator;
  активный владелец такого обновления - `fork generators`.
- Старый формат карточки также перечислял локальные ручные smoke-подсказки
  `git diff --check` и
  `rg -n "read_template_path|build_memory_tool_developer_instructions" codex-rs`.
  Они не были зафиксированы как результат текущего запуска и не являются
  нормативным runbook карточки.

### Известные падения и пропуски

- Если configured template нельзя прочитать или нельзя распарсить, memory
  developer prompt становится недоступным (`None`), а config load не падает.
- Если configured template содержит unknown placeholder, prompt становится
  недоступным (`None`); частичный render запрещен.
- Если `memory_summary.md` пустая или отсутствует, prompt не добавляется.
- Дополнительный регрессионный тест на unknown placeholder полезен при будущей
  правке, но в текущем `fork-tests.v1` отдельным argv не закреплен.

## Ограничения

- Не менять пользовательский `hermione.config.toml` в рамках этой доработки.
- Не писать сам template в repo, если задача только добавляет config surface.
- Не использовать unknown placeholders как частичный render: это должно быть
  fail-closed через `None`.
- Не запускать Rust/Cargo/`just` как нормативный шаг из карточки. Для fork
  workflow использовать skill-owned владельцев, а внутренние argv хранить только
  в `fork-tests.v1` или историческом подтверждении.

## Риски

- Upstream может переносить код memory extension между crates. При rebase
  искать живого владельца по `build_memory_tool_developer_instructions`, а не
  полагаться на старый путь.
- Тихий `None` при unreadable template означает, что профиль может лишиться
  memory instructions без ошибки загрузки config. Это осознанная деградация, но при
  диагностике prompt нужно проверять путь и placeholders.
- Если schema не обновить, config key будет работать в Rust type, но инструменты и
  редакторские подсказки будут устаревшими.

## Проверка покрытия

| Пункт | Статус | Где отражено |
| --- | --- | --- |
| Добавить `[memories].read_template_path` | перенесено | "Итоговый контракт", "Пошаговое воспроизведение" |
| Сохранить embedded template как default | перенесено | "Итоговый контракт" |
| Поддержать только `base_path` и `memory_summary` | перенесено | "Итоговый контракт", "Пошаговое воспроизведение" |
| Учесть перенос owner crate после upstream merge | перенесено | "Пошаговое воспроизведение", "Риски" |
| Синхронизировать README с фактическим путём к runtime-шаблону | перенесено | "Карта файлов", "Пошаговое воспроизведение" |
| Зафиксировать тесты, gates и владельца исполняемой карты | перенесено | "Проверки" |
| Сохранить историческое подтверждение schema/generator без нормативного прямого runbook | перенесено | "Проверки", "Ограничения" |
