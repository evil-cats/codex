# 002: LocalImageHistoryCell для локальных изображений

## Статус

`proposed`

## Кратко

| Поле | Значение |
| --- | --- |
| Цель этапа | Реализовать минимальный controlled path от `AppEvent::InsertLocalImage` до `HistoryCellDisplayItem::LocalImage`. |
| Уже сделано / решено | Этап 001 зафиксировал trust boundary, fallback policy, validation requirements и out-of-scope items. |
| Открыто / отложено / не сделано | Нужно выбрать точное место `LocalImageHistoryCell` и имя warning/fallback; resize/reflow/resume остаются вне scope. |
| Следующий шаг | Начать с размещения `LocalImageHistoryCell`, затем добавить structured event, validation и targeted tests. |
| Детали | [details:architecture], [details:scope], [details:actions], [details:criteria], [details:questions], [details:follow-ups] |

## Зачем этап нужен

Этап 001 зафиксировал границы первого вертикального среза: ассистентские
изображения должны попадать в TUI history только через controlled structured
event, а не через Markdown, shell output или произвольный текст модели.

Этот этап реализует минимальный source-backed путь от structured event до
terminal image preview:

```text
AppEvent::InsertLocalImage { path, caption }
  -> app-layer validation
  -> LocalImageHistoryCell
  -> HistoryCellDisplayItem::LocalImage
  -> HistoryInsertItem::Image
```

## Связанные элементы архитектуры

| Элемент | Ссылка |
| --- | --- |
| `HistoryCell` / `HistoryCellDisplayItem` | [code:history-cell] |
| Image preview feature | [feature:tui-history-image-previews] |
| Base history cells | [code:history-cell-base] |
| `AppEvent` | [code:app-event] |
| App event dispatch | [code:event-dispatch] |
| History insertion preparation | [code:resize-reflow] |
| Terminal image preparation | [code:pets-mod] |

## Объем работ

- Добавить `LocalImageHistoryCell` для ассистентских и tool-generated
  локальных изображений.
- Добавить structured event:
  `AppEvent::InsertLocalImage { path: PathBuf, caption: Option<String> }`.
- Реализовать app-layer validation для structured event:
  - `path` существует;
  - `path` указывает на regular file;
  - файл декодируется через `image` crate.
- При успешной validation вставлять `LocalImageHistoryCell` через существующий
  history insertion path.
- При невалидном event вставлять warning/fallback cell, не пытаясь создать bitmap payload.
- Оставить unsupported terminal и `prepare_history_image` failures как
  best-effort runtime fallback: текстовая строка остаётся, ошибка логируется
  debug-level.

## Вне объема работ

- Не парсить `![alt](path)` из ассистентского Markdown как terminal image.
- Не превращать shell/plain text output в trusted image source.
- Не скачивать remote URLs.
- Не менять resize/reflow и resume behavior.
- Не добавлять поддержку bitmap preview внутри `tmux`/`zellij`.
- Не вводить managed хранилище оригинальных изображений.

## План действий

- [ ] Выбрать место для `LocalImageHistoryCell`: новый приватный модуль рядом с
  `history_cell/base.rs` или отдельный файл в `history_cell/`.
- [ ] Реализовать `display_lines`, `raw_lines` и `display_items_for_mode`.
- [ ] Добавить `AppEvent::InsertLocalImage { path, caption }`.
- [ ] Добавить validation helper в app layer, чтобы callers не создавали
  `LocalImageHistoryCell` напрямую.
- [ ] Подключить event dispatch к существующему `InsertHistoryCell`/transcript path.
- [ ] Добавить unit tests для cell behavior.
- [ ] Добавить tests для validation success/failure.
- [ ] Добавить regression tests, что Markdown/plain text не порождают `LocalImage`.
- [ ] Запустить targeted `cargo test -p codex-tui <filters>`.
- [ ] Запустить `just fmt`; если `just fmt` снова упадёт на Python `uv`,
  зафиксировать причину.

## Критерии готовности

- `LocalImageHistoryCell` всегда показывает fallback line.
- В `Rich` mode cell отдаёт fallback line и
  `HistoryCellDisplayItem::LocalImage(path)`.
- В `Raw` mode cell отдаёт только fallback/copy-friendly lines.
- `AppEvent::InsertLocalImage` является единственным MVP-путём создания
  ассистентского image history cell.
- Невалидный structured event создаёт warning/fallback cell и не создаёт bitmap payload.
- Markdown/plain text не создаёт `HistoryCellDisplayItem::LocalImage`.
- Targeted tests проходят или их невозможность явно задокументирована.

## Открытые вопросы

| ID | Вопрос | Статус | Решение / итог | Детали |
| --- | --- | --- | --- | --- |
| [Q-001] | Где разместить `LocalImageHistoryCell`? | `open` | Новый приватный модуль в `history_cell/` | [details][Q-001] |
| [Q-002] | Как назвать warning/fallback? | `open` | Reuse warning cell или `[Image: <caption>]` | [details][Q-002] |

### Q-001: Размещение LocalImageHistoryCell

Статус: `open`.

Предварительное решение: новый приватный модуль в `history_cell/`, чтобы не
раздувать существующие central modules и держать cell behavior рядом с другими
history cell implementations.

### Q-002: Название warning/fallback

Статус: `open`.

Предварительное решение: переиспользовать warning cell или текстовый fallback
`[Image: <caption>]`, но точную форму выбрать при реализации validation path,
чтобы raw/copy behavior остался понятным.

## Найденные отложенные работы

- [follow-up:FU-2026-001]: item-oriented resize/reflow и replay для bitmap previews.
- [follow-up:FU-2026-002]: controlled assistant/tool image source path.

[code:app-event]: ../../../../codex-rs/tui/src/app_event.rs
[code:event-dispatch]: ../../../../codex-rs/tui/src/app/event_dispatch.rs
[code:history-cell]: ../../../../codex-rs/tui/src/history_cell/mod.rs
[code:history-cell-base]: ../../../../codex-rs/tui/src/history_cell/base.rs
[code:pets-mod]: ../../../../codex-rs/tui/src/pets/mod.rs
[code:resize-reflow]: ../../../../codex-rs/tui/src/app/resize_reflow.rs
[details:actions]: #план-действий
[details:architecture]: #связанные-элементы-архитектуры
[details:criteria]: #критерии-готовности
[details:follow-ups]: #найденные-отложенные-работы
[details:questions]: #открытые-вопросы
[details:scope]: #объем-работ
[feature:tui-history-image-previews]: ../../../architecture/features/tui-history-image-previews.md
[follow-up:FU-2026-001]: ../../../follow-ups/FU-2026-001-tui-history-image-reflow-reemit.md
[follow-up:FU-2026-002]: ../../../follow-ups/FU-2026-002-tui-assistant-tool-image-source.md
[Q-001]: #q-001-размещение-localimagehistorycell
[Q-002]: #q-002-название-warningfallback
