---
id: fork-terminal-title-session-label
status: active
created: 2026-06-08
updated: 2026-07-05
source_scope: rust-v0.137.0..HEAD
---

# Terminal title: `session-label`

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая добавляет статический
label в terminal title TUI через config key `tui.terminal_title_label` и item
`session-label`.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Основной commit | `268bf4302 Add session label to terminal title` |
| Migration repair | `46cdb741f Fix Hermione 0.137 release-fast build` |
| Config key | `[tui].terminal_title_label` |
| Terminal title item | `session-label` |
| Пример Hermione config | `terminal_title_label = "hermione"` и `terminal_title = ["session-label", "project-name", "run-state"]` |
| Checkpoint перед карточкой | Пропущен по явному разрешению пользователя от 2026-06-08 |

## Зачем это нужно

Hermione профиль часто работает в нескольких terminal tabs и проектах. Обычные
items вроде project name, current dir или run state не всегда показывают, какой
профиль/сессия открыта. Нужен статический label, задаваемый в config, который
можно поставить в terminal title.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/config/src/types.rs` | Добавляет `terminal_title_label` в `Tui` config |
| `codex-rs/core/config.schema.json` | Экспортирует config key |
| `codex-rs/core/src/config/mod.rs` | Добавляет effective `Config.tui_terminal_title_label` и load assignment |
| `codex-rs/core/src/config/config_tests.rs` | Обновляет defaults/expected config shape |
| `codex-rs/tui/src/bottom_pane/title_setup.rs` | Добавляет terminal title item `SessionLabel` |
| `codex-rs/tui/src/bottom_pane/status_surface_preview.rs` | Добавляет preview item `SessionLabel` |
| `codex-rs/tui/src/chatwidget/status_surfaces.rs` | Рендерит label в preview и terminal title |
| `codex-rs/tui/src/chatwidget/tests/terminal_title.rs` | Проверяет terminal title с configured session label |
| TUI snapshots | Обновляют popup со строкой `session-label` |

## Итоговый контракт

1. `Tui` config получает optional field:

   ```rust
   pub terminal_title_label: Option<String>,
   ```

2. Field доступен как TOML:

   ```toml
   [tui]
   terminal_title_label = "hermione"
   terminal_title = ["session-label", "project-name", "run-state"]
   ```

3. Effective `Config` получает:

   ```rust
   pub tui_terminal_title_label: Option<String>,
   ```

4. Config loading переносит:

   ```rust
   tui_terminal_title_label: cfg
       .tui
       .as_ref()
       .and_then(|t| t.terminal_title_label.clone()),
   ```

5. Terminal title setup получает enum variant `TerminalTitleItem::SessionLabel`.
6. Label string для config item: `session-label`.
7. Description в selector: `Static session label from tui.terminal_title_label`.
8. Preview surface получает `StatusSurfacePreviewItem::SessionLabel`.
9. Preview value возвращает `self.config.tui_terminal_title_label.clone()`.
10. Runtime terminal title value:
    - если config value отсутствует, item omitted;
    - если value есть, оно truncate'ится через
      `ChatWidget::truncate_terminal_title_part(..., 24)`.
11. Item должен быть доступен в terminal title selector snapshots.

## Пошаговое воспроизведение

### 1. Добавить config field

В `codex-rs/config/src/types.rs` в `Tui`:

```rust
/// Optional static label available to terminal-title item `session-label`.
///
/// This is useful for distinguishing named local profiles in terminals with
/// multiple Codex tabs.
#[serde(default)]
pub terminal_title_label: Option<String>,
```

### 2. Добавить effective config

В `codex-rs/core/src/config/mod.rs` в `Config`:

```rust
/// Optional static label surfaced by terminal-title item `session-label`.
pub tui_terminal_title_label: Option<String>,
```

В `Config::load_from_base_config_with_overrides` заполнить значение из
`cfg.tui`.

Важно: на `0.137.0` merge это поле однажды потерялось именно на effective
`Config` слое. При будущих migrations проверять не только TOML type, но и
runtime `Config`.

### 3. Обновить terminal title item model

В `codex-rs/tui/src/bottom_pane/title_setup.rs`:

- добавить `TerminalTitleItem::SessionLabel`;
- добавить label `session-label`;
- добавить description;
- добавить mapping в preview item:

  ```rust
  TerminalTitleItem::SessionLabel => Some(StatusSurfacePreviewItem::SessionLabel)
  ```

### 4. Обновить preview model

В `codex-rs/tui/src/bottom_pane/status_surface_preview.rs` добавить
`StatusSurfacePreviewItem::SessionLabel` и label `session-label`.

В `codex-rs/tui/src/chatwidget/status_surfaces.rs`:

```rust
StatusSurfacePreviewItem::SessionLabel => {
    return self.config.tui_terminal_title_label.clone();
}
```

### 5. Обновить terminal title rendering

В `ChatWidget::terminal_title_value_for_item`:

```rust
TerminalTitleItem::SessionLabel => {
    self.config.tui_terminal_title_label.as_ref().map(|label| {
        Self::truncate_terminal_title_part(label.clone(), /*max_chars*/ 24)
    })
}
```

### 6. Обновить schema и snapshots

После изменения config type обновить `codex-rs/core/config.schema.json`.

После изменения selector UI обновить affected insta snapshots:

- `codex_tui__bottom_pane__title_setup__tests__terminal_title_setup_basic.snap`;
- `codex_tui__chatwidget__tests__terminal_title_setup_popup_hardcoded_only.snap`;
- `codex_tui__chatwidget__tests__terminal_title_setup_popup_live_only.snap`;
- `codex_tui__chatwidget__tests__terminal_title_setup_popup_mixed.snap`;
- `codex_tui__chatwidget__tests__terminal_title_setup_popup_rate_limits.snap`.

## Проверки

### Смысловое покрытие

Проверочное покрытие этой карточки должно подтверждать:

- Слой config/TOML принимает опциональный ключ `[tui].terminal_title_label`.
- Effective `Config` сохраняет значение как `tui_terminal_title_label`; это
  отдельный обязательный слой, потому что при merge `0.137.0` поле однажды
  потерялось именно там.
- Элемент `session-label` для terminal title доступен в selector, preview model и
  runtime-рендеринге.
- Если значение config отсутствует, сегмент не выводится; если значение есть, строка
  обрезается через `ChatWidget::truncate_terminal_title_part(..., 24)`.
- Тест `terminal_title_can_include_configured_session_label`:
  - создаёт `ChatWidget`;
  - ставит `chat.config.tui_terminal_title_label = Some("hermione")`;
  - ставит `tui_terminal_title = ["session-label", "project-name", "run-state"]`;
  - вызывает `refresh_terminal_title`;
  - ожидает:

    ```text
    hermione | project | Ready
    ```

- Snapshot-тесты показывают новый item `session-label` в selector.
- Config-тесты обновлены с `terminal_title_label: None` в expected defaults.
- `codex-rs/core/config.schema.json` содержит schema для
  `terminal_title_label` после обновления config types.

### Владелец исполняемой карты

Регрессионным покрытием уровня карточки владеет skill-owned command `fork tests`.
Внутренний argv для этой карточки живет в блоке `fork-tests.v1` ниже и является
данными исполняемой карты, а не нормативной командой запуска из карточки.

Идентификатор карточки для фильтрации и отчета `fork tests`:
`fork-terminal-title-session-label`.

Данные ниже являются текущим блоком `fork-tests.v1`, который читает
`fork tests`.

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "terminal title",
      "argv": ["just", "test", "-p", "codex-tui", "terminal_title"]
    }
  ]
}
```

### Дополнительные gates

- `fork generators` требуется, когда перенос этой карточки меняет config type
  или schema artifact; исторический внутренний argv для этого gate сохранен в
  `Исторические результаты`.
- Snapshot review/accept требуется, если UI-вывод в selector snapshots меняется
  намеренно. Затронутые snapshot-файлы перечислены в разделе
  `Пошаговое воспроизведение`.
- Быстрая release-сборка относится к общему fork gate после переноса карточек;
  исторический argv из старой карточки сохранен в `Исторические результаты`.
- Статический поиск `terminal_title_label|session-label|SessionLabel` и
  проверка пробелов и чистоты diff сохраняются как диагностические проверки из
  старой карточки, а не как канонический runbook текущей карточки.

### Исторические результаты

- Исторически TUI-покрытие уровня карточки фиксировалось целевыми тестами
  terminal title в `codex-tui`; текущая исполняемая карта сохраняет внутренний argv
  `["just", "test", "-p", "codex-tui", "terminal_title"]` в
  `fork-tests.v1`.
- Зафиксированное ожидаемое runtime-значение для настроенного session label:

  ```text
  hermione | project | Ready
  ```

- Исторически слой config/schema проверялся обновлением
  `codex-rs/core/config.schema.json` после изменения config type; старая карточка
  фиксировала внутренний argv `just write-config-schema`.
- Исторически config tests были обновлены с `terminal_title_label: None` в
  expected defaults.
- Исторически TUI snapshot-покрытие показывало новый item `session-label` в
  вариантах selector popup.
- Старая карточка также сохраняла локальные диагностические команды:
  `rg -n "terminal_title_label|session-label|SessionLabel" codex-rs` и
  `git diff --check`.
- Датированных логов, exit codes или сохраненных путей к логам исходная карточка
  не фиксировала.

### Известные падения и пропуски

- Checkpoint перед карточкой был пропущен по явному разрешению пользователя от
  2026-06-08; это исторический skip, а не текущий блокер.
- Известный сценарий отказа при миграции: при merge `0.137.0`
  `Tui.terminal_title_label` существовал, но effective `Config` потерял поле.
  Будущие переносы должны проверять оба слоя.
- В исходной карточке не было зафиксированных активных красных результатов для
  terminal title tests, config tests, генерации schema или snapshot review.

## Ограничения

- `session-label` должен быть optional item: если config value absent, segment
  omitted, а не rendered as empty string.
- Не добавлять label в default terminal title автоматически. Пользователь сам
  включает item через `tui.terminal_title`.
- Не путать `terminal_title_label` с thread title или session id: это статичный
  profile/session label.

## Риски

- Проверка только `config/src/types.rs` недостаточна. При merge `0.137.0`
  `Tui.terminal_title_label` существовал, но effective `Config` потерял поле.
- Selector snapshots легко устаревают, потому что добавление item меняет список
  в нескольких popup variants.
- Long labels должны truncate'иться, иначе terminal title становится шумным.

## Проверка покрытия

| Пункт | Статус | Где отражено |
| --- | --- | --- |
| Добавить `[tui].terminal_title_label` | перенесено в карточку | `Итоговый контракт`, `Пошаговое воспроизведение`, `Проверки` |
| Добавить item `session-label` | перенесено в карточку | `Итоговый контракт`, `Пошаговое воспроизведение`, `Проверки` |
| Прокинуть значение в effective `Config` | перенесено в карточку | `Итоговый контракт`, `Проверки`, `Риски` |
| Проверить рендеринг `hermione`, `project`, `Ready` | перенесено в карточку | `Проверки` |
| Сохранить владельца `fork tests` и блок `fork-tests.v1` | перенесено в карточку | `Проверки` |
| Сохранить исторические команды и результаты проверок | перенесено в карточку | `Проверки` |
| Зафиксировать migration gotcha из `0.137.0` | перенесено в карточку | `Пошаговое воспроизведение`, `Проверки`, `Риски` |
