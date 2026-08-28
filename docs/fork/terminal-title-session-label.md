---
id: fork-terminal-title-session-label
status: active
created: 2026-06-08
updated: 2026-08-27
---

# Terminal title: `session-label`

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая добавляет статический
label в terminal title TUI через config key `tui.terminal_title_label` и item
`session-label`.

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
| `codex-rs/thread-manager-sample/src/main.rs` | Задаёт `tui_terminal_title_label: None` при ручной инициализации `Config` |
| `codex-rs/tui/src/bottom_pane/title_setup.rs` | Добавляет terminal title item `SessionLabel` |
| `codex-rs/tui/src/bottom_pane/status_surface_preview.rs` | Добавляет preview item `SessionLabel` |
| `codex-rs/tui/src/chatwidget/status_surfaces.rs` | Рендерит label в preview и terminal title |
| `codex-rs/tui/src/chatwidget.rs` | Передаёт унаследованный кэш заголовка терминала в `ChatWidgetInit` |
| `codex-rs/tui/src/chatwidget/constructor.rs` | Заполняет кэш до первого обновления поверхностей состояния |
| `codex-rs/tui/src/terminal_title.rs` | Санитизирует итоговый title, безопасно пишет или очищает OSC title и даёт тестам task-local recorder логических I/O-запросов |
| `codex-rs/tui/src/chatwidget/tests/terminal_title.rs` | Проверяет terminal title с configured session label |
| `codex-rs/tui/src/app/session_lifecycle.rs` | Заменяет `ChatWidget` и сохраняет поздний резервный перенос кэша для остальных путей создания |
| `codex-rs/tui/src/app/tests.rs` | Проверяет общий перенос кэша, реальные пути новой, возобновлённой и ответвлённой сессии, а также однократную очистку отключённого заголовка |
| `codex-rs/tui/src/app.rs` | Перемещает текущий кэш в параметры замены до конструктора и очищает управляемый Codex заголовок при завершении `App` |
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
16. Перед созданием замещающего `ChatWidget` жизненный цикл `App` перемещает текущий
    `last_terminal_title` в `ChatWidgetInit`. Конструктор заполняет кэш до
    первого `refresh_status_surfaces`, поэтому неизменившийся OSC-заголовок не
    записывается повторно, а отключённый заголовок очищается ровно один раз.
    `App::replace_chat_widget` сохраняет поздний резервный перенос для путей,
    которые создают замену без унаследованного состояния.
17. Ручная инициализация `Config` в `codex-thread-manager-sample` задаёт
    `tui_terminal_title_label: None`, поскольку sample не получает значение
    через общий config loading.

## Архитектурное решение

Config layer владеет optional label, типизированные TUI items проводят его в
предварительный просмотр селектора и заголовок терминала, а `terminal_title.rs`
централизованно владеет санитизацией и OSC I/O. Жизненный цикл `App` передаёт
кэш в параметры замены до первого обновления поверхностей состояния и очищает
управляемый заголовок при завершении. Недоверенный текст из config никогда не
обходит общий sanitizer.

## Порядок повторения при переносе

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

В `codex-rs/thread-manager-sample/src/main.rs` ручная инициализация `Config`
должна задавать `tui_terminal_title_label: None`.

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

При замене `ChatWidget` текущий кэш извлекается в
`App::chatwidget_init_for_forked_or_resumed_thread` и передаётся конструктору
через `ChatWidgetInit.inherited_terminal_title`. Конструктор должен заполнить
`last_terminal_title` до первого `refresh_status_surfaces`: только так
неизменившийся заголовок пропускает повторную OSC-запись, а удалённый новой
конфигурацией заголовок очищается немедленно. Поздний перенос в
`App::replace_chat_widget` остаётся резервным путём для остальных мест создания.

Пути нового, возобновлённого и ответвлённого потока структурно сходятся в
`replace_chat_widget_with_app_server_thread`. Тест
`replace_chat_widget_preserves_terminal_title_cache` защищает резервный перенос
внутри вспомогательного метода; он не заменяет интеграционные проверки реального
порядка создания замены.

Низкоуровневый `set_terminal_title` владеет санитизацией всей итоговой строки,
включая значение `session-label`. Этот слой удаляет управляющие и
невидимые/bidi форматирующие codepoints, нормализует пробельные символы,
применяет общий лимит длины и кодирует OSC 0 с terminator `BEL`.

## Проверки

Исполняемая карта card-level regression tests:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "config loading и schema optional terminal session label",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "load_config_resolves_tui_terminal_title_label"
      ]
    },
    {
      "purpose": "санитизация, кэширование, new/resume/fork lifecycle и selector snapshots terminal title",
      "argv": ["just", "test", "-p", "codex-tui", "terminal_title"]
    },
    {
      "purpose": "sample Config явно инициализирует terminal title label",
      "argv": ["just", "test", "-p", "codex-thread-manager-sample", "--no-tests=pass"]
    }
  ]
}
```

Тесты `new_session_preserves_unchanged_terminal_title_cache`,
`resume_session_preserves_unchanged_terminal_title_cache` и
`fork_session_preserves_unchanged_terminal_title_cache` вызывают настоящие
`AppEvent` и проходят через `replace_chat_widget_with_app_server_thread`.
Task-local recorder наблюдает логические запросы `Set` и `Clear` до проверки
TTY; каждый сценарий требует отсутствия I/O как во время lifecycle, так и при
следующем `refresh_terminal_title`.

Тест `new_session_clears_terminal_title_disabled_by_reloaded_config` заменяет
конфигурацию на пустой список terminal title items перед `NewSession`. Он требует
ровно один запрос `Clear`, пустой кэш replacement и отсутствие повторного I/O
при следующем `refresh_terminal_title`.

Дополнительно обязателен `fork generators`. Если selector UI меняется, нужно также обновить и проверить перечисленные в порядке переноса snapshot-файлы.

## Риски и ограничения

### Ограничения

- `session-label` должен быть optional item: если config value absent, segment
  omitted, а не rendered as empty string.
- Не добавлять label в default terminal title автоматически. Пользователь сам
  включает item через `tui.terminal_title`.
- Не путать `terminal_title_label` с thread title или session id: это статичный
  profile/session label.
- Не считать ограничение label до 24 символов достаточной защитой OSC: escaping
  и удаление управляющих/bidi символов принадлежат общему
  `terminal_title::set_terminal_title`.

### Риски

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
