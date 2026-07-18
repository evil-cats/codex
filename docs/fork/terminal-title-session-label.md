---
id: fork-terminal-title-session-label
status: active
created: 2026-06-08
updated: 2026-07-18
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
| `codex-rs/tui/src/terminal_title.rs` | Санитизирует итоговый title и безопасно пишет или очищает OSC title |
| `codex-rs/tui/src/chatwidget/tests/terminal_title.rs` | Проверяет terminal title с configured session label |
| `codex-rs/tui/src/app.rs` | Очищает управляемый Codex title при завершении `App`; предыдущий title терминала не восстанавливается |
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
11. Итоговая строка terminal title перед OSC-записью проходит централизованную
    санитизацию в `codex-rs/tui/src/terminal_title.rs`:
    - управляющие символы и невидимые/bidi форматирующие codepoints удаляются;
    - последовательности пробельных символов сворачиваются в один пробел;
    - итог ограничивается 240 символами;
    - OSC 0 завершается через `BEL`.
12. Если настроенные items не дают видимого текста, ранее записанный Codex title
    очищается; успешное значение кэшируется, чтобы не повторять одинаковые
    OSC-записи.
13. Item должен быть доступен в terminal title selector snapshots.
14. Низкоуровневые set/clear операции пишут OSC только когда `stdout` является
    terminal; Windows использует ANSI-реализацию `SetWindowTitle`.
15. При завершении `App::drop` очищает последний управляемый Codex title.
    Предыдущий title shell/terminal не читается и не восстанавливается, потому
    что переносимого механизма для этого нет.

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

### 7. Сохранить общий lifecycle и escaping terminal title

`session-label` является текстом из config, то есть недоверенным вводом. Не
добавлять для него отдельную OSC-запись или обход общего пути
`set_terminal_title`.

`ChatWidget::refresh_terminal_title_from_selections` должен:

- пропускать отсутствующее значение `terminal_title_label` как недоступный
  сегмент;
- собирать доступные сегменты в настроенном порядке;
- очищать ранее записанный Codex title, если список items пуст или итоговая
  строка отсутствует;
- передавать непустую строку в `set_terminal_title`;
- кэшировать title только после результата `Applied`;
- очищать ранее управляемый title при `NoVisibleContent`;
- не выполнять повторную OSC-запись, если вычисленное значение не изменилось.

Низкоуровневый `set_terminal_title` владеет санитизацией всей итоговой строки,
включая значение `session-label`. Этот слой удаляет управляющие и
невидимые/bidi форматирующие codepoints, нормализует пробельные символы,
применяет общий лимит длины и кодирует OSC 0 с terminator `BEL`.

## Проверки

### Смысловое покрытие

Проверочное покрытие этой карточки должно подтверждать:

- Слой config/TOML принимает опциональный ключ `[tui].terminal_title_label`.
- Effective `Config` сохраняет значение как `tui_terminal_title_label`; это
  отдельный обязательный слой, потому что при merge `0.137.0` поле однажды
  потерялось именно там.
- Тест `load_config_resolves_tui_terminal_title_label` проверяет, что значение
  `[tui].terminal_title_label` доходит до effective `Config`.
- Элемент `session-label` для terminal title доступен в selector, preview model и
  runtime-рендеринге.
- Если значение config отсутствует, сегмент не выводится; если значение есть, строка
  обрезается через `ChatWidget::truncate_terminal_title_part(..., 24)`.
- Значение label из config не обходит общий безопасный путь terminal title:
  итоговая строка санитизируется непосредственно перед OSC-записью.
- Lifecycle terminal title сохраняет cache/clear contract:
  - одинаковый title повторно не записывается;
  - пустой настроенный список очищает ранее управляемый title;
  - `NoVisibleContent` после санитизации также очищает ранее управляемый title;
  - cache обновляется только после успешного результата `Applied`.
- Тест `terminal_title_can_include_configured_session_label`:
  - создаёт `ChatWidget`;
  - ставит `chat.config.tui_terminal_title_label = Some("hermione")`;
  - ставит `tui_terminal_title = ["session-label", "project-name", "run-state"]`;
  - вызывает `refresh_terminal_title`;
  - ожидает:

    ```text
     hermione | project | Ready
     ```

- Тест `terminal_title_omits_absent_session_label_and_truncates_configured_value`
  проверяет, что отсутствующий label пропускается, а настроенное значение
  ограничивается 24 символами.
- Тест `empty_terminal_title_selection_clears_cached_title` проверяет очистку
  кэша ранее управляемого title при пустом списке configured items.
- Snapshot-тесты показывают новый item `session-label` в selector.
- Unit tests в `codex-rs/tui/src/terminal_title.rs` проверяют удаление
  управляющих и невидимых/bidi codepoints, ограничение длины и OSC 0 с
  terminator `BEL`.
- `set_terminal_title` и `clear_terminal_title` проверяют terminal support через
  `stdout().is_terminal()`; `SetWindowTitle` объявляет ANSI support на Windows.
- `App::drop` вызывает `clear_managed_terminal_title`; lifecycle намеренно
  очищает управляемый title, но не пытается восстановить предыдущий.
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
      "purpose": "config terminal title label",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "load_config_resolves_tui_terminal_title_label"
      ]
    },
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
- После merge `rust-v0.144.5` статическая сверка подтвердила, что этот test
  target включает проверку контракта настроенного session label и
  низкоуровневые unit tests санитизации и кодирования OSC в
  `terminal_title.rs`.
- После merge `rust-v0.144.6` статическая сверка подтвердила сохранность config
  plumbing, derivation и truncation `session-label`, общего безопасного OSC
  path, terminal support, cache/clear lifecycle, очистки на `App::drop` и пяти
  selector snapshots. Upstream diff `rust-v0.144.5..rust-v0.144.6` не меняет
  owner-файлы этой карточки.
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
- В one-card проходе после merge `rust-v0.144.5` project-level проверки не
  запускались по контракту подагента; их должен выполнить общий проверочный
  проход через skill-owned `fork tests` и при необходимости `fork generators`.
- В one-card проходе после merge `rust-v0.144.6` tests, build, generators,
  format, fix и markdownlint не запускались по явному ограничению задачи; их
  должен выполнить общий проверочный проход.

## Ограничения

- `session-label` должен быть optional item: если config value absent, segment
  omitted, а не rendered as empty string.
- Не добавлять label в default terminal title автоматически. Пользователь сам
  включает item через `tui.terminal_title`.
- Не путать `terminal_title_label` с thread title или session id: это статичный
  profile/session label.
- Не считать ограничение label до 24 символов достаточной защитой OSC: escaping
  и удаление управляющих/bidi символов принадлежат общему
  `terminal_title::set_terminal_title`.

## Риски

- Проверка только `config/src/types.rs` недостаточна. При merge `0.137.0`
  `Tui.terminal_title_label` существовал, но effective `Config` потерял поле.
- Selector snapshots легко устаревают, потому что добавление item меняет список
  в нескольких popup variants.
- Long labels должны truncate'иться, иначе terminal title становится шумным.
- Значение label из config может содержать OSC terminators, управляющие или
  bidi codepoints; перенос не должен обходить централизованную санитизацию
  `terminal_title.rs`.
- Изменение clear/cache lifecycle может оставить stale title или вызвать
  повторные OSC-записи, даже если сам `SessionLabel` продолжает компилироваться.

## Проверка покрытия

| Пункт | Статус | Где отражено |
| --- | --- | --- |
| Добавить `[tui].terminal_title_label` | перенесено в карточку | `Итоговый контракт`, `Пошаговое воспроизведение`, `Проверки` |
| Добавить item `session-label` | перенесено в карточку | `Итоговый контракт`, `Пошаговое воспроизведение`, `Проверки` |
| Прокинуть значение в effective `Config` | перенесено в карточку | `Итоговый контракт`, `Проверки`, `Риски` |
| Проверить рендеринг `hermione`, `project`, `Ready` | перенесено в карточку | `Проверки` |
| Сохранить общий escaping значения label из config перед OSC | перенесено в карточку | `Итоговый контракт`, `Пошаговое воспроизведение`, `Проверки`, `Ограничения` |
| Сохранить clear/cache lifecycle terminal title | перенесено в карточку | `Итоговый контракт`, `Пошаговое воспроизведение`, `Проверки`, `Риски` |
| Сохранить terminal support и очистку при завершении `App` без ложного обещания restore | перенесено в карточку | `Карта файлов`, `Итоговый контракт`, `Проверки`, `Исторические результаты` |
| Сохранить владельца `fork tests` и блок `fork-tests.v1` | перенесено в карточку | `Проверки` |
| Сохранить исторические команды и результаты проверок | перенесено в карточку | `Проверки` |
| Зафиксировать migration gotcha из `0.137.0` | перенесено в карточку | `Пошаговое воспроизведение`, `Проверки`, `Риски` |
