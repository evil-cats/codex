# 002: LocalImageHistoryCell для локальных изображений

## Статус

`completed`

## Кратко

| Поле | Значение |
| --- | --- |
| Цель этапа | Реализовать минимальный контролируемый путь от `AppEvent::InsertLocalImage` до `HistoryCellDisplayItem::LocalImage`. |
| Уже сделано / решено | Добавлены `LocalImageHistoryCell`, `AppEvent::InsertLocalImage`, validation в app layer, warning fallback и тесты. |
| Открыто / отложено / не сделано | Вызывающий production-код еще не подключен; resize/reflow/resume остаются вне scope. |
| Следующий шаг | Реализовать [stage:PLAN-TUI-ASSISTANT-IMAGES-001:003]. |
| Детали | [details:architecture], [details:scope], [details:actions], [details:criteria], [details:questions], [details:verification], [details:follow-ups] |

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

- [x] Выбрать место для `LocalImageHistoryCell`: отдельный файл
  `history_cell/local_image.rs`.
- [x] Реализовать `display_lines`, `raw_lines` и `display_items_for_mode`.
- [x] Добавить `AppEvent::InsertLocalImage { path, caption }`.
- [x] Добавить validation helper в app layer, чтобы вызывающий код не создавал
  `LocalImageHistoryCell` напрямую.
- [x] Подключить event dispatch к существующему пути `InsertHistoryCell`/transcript.
- [x] Добавить unit-тесты для поведения cell.
- [x] Добавить тесты для успешной и неуспешной validation.
- [x] Добавить regression tests, что Markdown/plain text не порождают `LocalImage`.
- [x] Запустить точечный `cargo test -p codex-tui local_image`.
- [x] Запустить `just fmt`; если `just fmt` снова упадёт на Python `uv`,
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
- Точечные тесты проходят или их невозможность явно задокументирована.

## Открытые вопросы

| ID | Вопрос | Статус | Решение / итог | Детали |
| --- | --- | --- | --- | --- |
| [Q-001] | Где разместить `LocalImageHistoryCell`? | `decided` | Отдельный модуль `history_cell/local_image.rs` | [details][Q-001] |
| [Q-002] | Как назвать warning/fallback? | `decided` | Валидный image event: `[Image]` / `[Image: <caption>]`; invalid event: warning cell без bitmap marker | [details][Q-002] |

### Q-001: Размещение LocalImageHistoryCell

Статус: `decided`.

Решение: отдельный приватный модуль `history_cell/local_image.rs`, чтобы не
раздувать существующие центральные модули и держать поведение cell рядом с
другими реализациями history cell.

### Q-002: Название warning/fallback

Статус: `decided`.

Решение: валидная image cell показывает fallback `[Image]` или
`[Image: <caption>]` и в `Rich` добавляет `HistoryCellDisplayItem::LocalImage`.
Invalid structured event превращается в warning cell без bitmap marker, чтобы
ошибка была видна и не пыталась вставить terminal payload.

## Проверки

- `cargo test -p codex-tui local_image` - passed.
- `cargo test -p codex-tui` - failed on unrelated
  `app::tests::discard_side_thread_keeps_local_state_when_server_close_fails`
  with stack overflow; isolated rerun reproduces the same failure.
- `just fix -p codex-tui` - passed; unrelated clippy auto-fixes were reverted
  from this diff.
- `just fmt` - `cargo fmt` completed, then recipe failed on Python `uv` because
  `openai-codex-cli-bin==0.131.0a4` has no compatible wheel for current
  manylinux.

## Найденные отложенные работы

- [follow-up:FU-2026-001]: resize/reflow и replay на уровне items для bitmap-превью.
- [follow-up:FU-2026-002]: закрыто stage 002; вызывающий production-код вынесен в [stage:PLAN-TUI-ASSISTANT-IMAGES-001:003].

[code:app-event]: ../../../../../../codex-rs/tui/src/app_event.rs
[code:event-dispatch]: ../../../../../../codex-rs/tui/src/app/event_dispatch.rs
[code:history-cell]: ../../../../../../codex-rs/tui/src/history_cell/mod.rs
[code:history-cell-base]: ../../../../../../codex-rs/tui/src/history_cell/base.rs
[code:pets-mod]: ../../../../../../codex-rs/tui/src/pets/mod.rs
[code:resize-reflow]: ../../../../../../codex-rs/tui/src/app/resize_reflow.rs
[details:actions]: #план-действий
[details:architecture]: #связанные-элементы-архитектуры
[details:criteria]: #критерии-готовности
[details:follow-ups]: #найденные-отложенные-работы
[details:questions]: #открытые-вопросы
[details:scope]: #объем-работ
[details:verification]: #проверки
[feature:tui-history-image-previews]: ../../../../../architecture/features/tui-history-image-previews.md
[follow-up:FU-2026-001]: ../../../../../follow-ups/archive/2026/FU-2026-001-tui-history-image-reflow-reemit.md
[follow-up:FU-2026-002]: ../../../../../follow-ups/archive/2026/FU-2026-002-tui-assistant-tool-image-source.md
[Q-001]: #q-001-размещение-localimagehistorycell
[Q-002]: #q-002-название-warningfallback
[stage:PLAN-TUI-ASSISTANT-IMAGES-001:003]: 003-wire-assistant-image-source.md
