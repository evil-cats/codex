# 002: LocalImageHistoryCell для локальных изображений

## Статус

`proposed`

## Кратко

| Поле | Значение |
| --- | --- |
| Цель этапа | Реализовать минимальный контролируемый путь от `AppEvent::InsertLocalImage` до `HistoryCellDisplayItem::LocalImage`. |
| Уже сделано / решено | Этап 001 зафиксировал trust boundary, fallback policy, требования validation и items вне объема. |
| Открыто / отложено / не сделано | Нужно выбрать точное место `LocalImageHistoryCell` и имя warning/fallback; resize/reflow/resume остаются вне scope. |
| Следующий шаг | Начать с размещения `LocalImageHistoryCell`, затем добавить structured event, validation и точечные тесты. |
| Детали | [details:architecture], [details:scope], [details:actions], [details:criteria], [details:questions], [details:follow-ups] |

## Зачем этап нужен

Этап 001 зафиксировал границы первого вертикального среза: ассистентские
изображения должны попадать в историю TUI только через контролируемый
structured event, а не через Markdown, shell output или произвольный текст
модели.

Этот этап реализует минимальный путь с исходным файлом от structured event до
терминального image preview:

```text
AppEvent::InsertLocalImage { path, caption }
  -> validation в app layer
  -> LocalImageHistoryCell
  -> HistoryCellDisplayItem::LocalImage
  -> HistoryInsertItem::Image
```

## Связанные элементы архитектуры

| Элемент | Ссылка |
| --- | --- |
| `HistoryCell` / `HistoryCellDisplayItem` | [code:history-cell] |
| Фича image preview | [feature:tui-history-image-previews] |
| Базовые history cells | [code:history-cell-base] |
| `AppEvent` | [code:app-event] |
| Dispatch app events | [code:event-dispatch] |
| Подготовка вставки в историю | [code:resize-reflow] |
| Подготовка terminal image | [code:pets-mod] |

## Объем работ

- Добавить `LocalImageHistoryCell` для ассистентских и созданных инструментами
  локальных изображений.
- Добавить structured event:
  `AppEvent::InsertLocalImage { path: PathBuf, caption: Option<String> }`.
- Реализовать validation в app layer для structured event:
  - `path` существует;
  - `path` указывает на regular file;
  - файл декодируется через `image` crate.
- При успешной validation вставлять `LocalImageHistoryCell` через существующий
  путь вставки в историю.
- При невалидном event вставлять warning/fallback cell, не пытаясь создать bitmap payload.
- Оставить unsupported terminal и failures в `prepare_history_image` как
  best-effort runtime fallback: текстовая строка остаётся, ошибка логируется
  debug-level.

## Вне объема работ

- Не парсить `![alt](path)` из ассистентского Markdown как terminal image.
- Не превращать shell/plain text output в доверенный image source.
- Не скачивать remote URLs.
- Не менять поведение resize/reflow и resume.
- Не добавлять поддержку bitmap preview внутри `tmux`/`zellij`.
- Не вводить managed хранилище оригинальных изображений.

## План действий

- [ ] Выбрать место для `LocalImageHistoryCell`: новый приватный модуль рядом с
  `history_cell/base.rs` или отдельный файл в `history_cell/`.
- [ ] Реализовать `display_lines`, `raw_lines` и `display_items_for_mode`.
- [ ] Добавить `AppEvent::InsertLocalImage { path, caption }`.
- [ ] Добавить validation helper в app layer, чтобы вызывающий код не создавал
  `LocalImageHistoryCell` напрямую.
- [ ] Подключить event dispatch к существующему пути `InsertHistoryCell`/transcript.
- [ ] Добавить unit-тесты для поведения cell.
- [ ] Добавить tests для успешной и неуспешной validation.
- [ ] Добавить regression tests, что Markdown/plain text не порождают `LocalImage`.
- [ ] Запустить точечный `cargo test -p codex-tui <filters>`.
- [ ] Запустить `just fmt`; если `just fmt` снова упадёт на Python `uv`,
  зафиксировать причину.

## Критерии готовности

- `LocalImageHistoryCell` всегда показывает fallback line.
- В `Rich` mode cell отдаёт fallback line и
  `HistoryCellDisplayItem::LocalImage(path)`.
- В `Raw` mode cell отдаёт только fallback/copy-friendly lines.
- `AppEvent::InsertLocalImage` является единственным MVP-путём создания
  ассистентской cell истории изображений.
- Невалидный structured event создаёт warning/fallback cell и не создаёт bitmap payload.
- Markdown/plain text не создаёт `HistoryCellDisplayItem::LocalImage`.
- Точечные tests проходят или их невозможность явно задокументирована.

## Открытые вопросы

| ID | Вопрос | Статус | Решение / итог | Детали |
| --- | --- | --- | --- | --- |
| [Q-001] | Где разместить `LocalImageHistoryCell`? | `open` | Новый приватный модуль в `history_cell/` | [details][Q-001] |
| [Q-002] | Как назвать warning/fallback? | `open` | Переиспользовать warning cell или `[Image: <caption>]` | [details][Q-002] |

### Q-001: Размещение LocalImageHistoryCell

Статус: `open`.

Предварительное решение: новый приватный модуль в `history_cell/`, чтобы не
раздувать существующие центральные модули и держать поведение cell рядом с
другими реализациями history cell.

### Q-002: Название warning/fallback

Статус: `open`.

Предварительное решение: переиспользовать warning cell или текстовый fallback
`[Image: <caption>]`, но точную форму выбрать при реализации validation path,
чтобы поведение raw/copy осталось понятным.

## Найденные отложенные работы

- [follow-up:FU-2026-001]: resize/reflow и replay на уровне items для bitmap-превью.
- [follow-up:FU-2026-002]: контролируемый исходный путь для изображений ассистента/инструментов.

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
