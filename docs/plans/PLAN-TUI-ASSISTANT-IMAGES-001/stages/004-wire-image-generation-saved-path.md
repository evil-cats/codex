# 004: ImageGeneration saved_path как image preview

## Статус

`proposed`

## Кратко

| Поле | Значение |
| --- | --- |
| Цель этапа | Решить, должен ли `ImageGeneration.saved_path` отправлять `AppEvent::InsertLocalImage { path, caption }`, и подключить его, если контракт подходит. |
| Уже сделано / решено | Stage 003 подключил `ThreadItem::ImageView` / `view_image` как первый production caller. |
| Открыто / отложено / не сделано | Нужно проверить origin/lifetime `saved_path`, caption contract и поведение, когда `saved_path` отсутствует. |
| Следующий шаг | Найти producer `ImageGeneration.saved_path` и подтвердить, что TUI может безопасно использовать локальный файл как preview source. |
| Детали | [details:architecture], [details:scope], [details:actions], [details:criteria], [details:questions] |

## Зачем этап нужен

`ImageGeneration` уже несет `saved_path: Option<AbsolutePathBuf>` в protocol
item. Это похожий на `view_image` controlled path, но его origin, cleanup и
поведение без сохраненного файла нужно проверить отдельно, чтобы не смешивать
два разных source contract в stage 003.

## Связанные элементы архитектуры

| Элемент | Ссылка |
| --- | --- |
| `ChatWidget::on_image_generation_end` | [code:tool-lifecycle] |
| `AppEvent::InsertLocalImage` | [code:app-event] |
| Dispatch app events | [code:event-dispatch] |
| `LocalImageHistoryCell` | [code:local-image-cell] |
| Фича image preview | [feature:tui-history-image-previews] |

## Объем работ

- Найти, где создается и сохраняется `ImageGeneration.saved_path`.
- Решить, является ли `saved_path` достаточно controlled source для terminal preview.
- Если да, отправлять `AppEvent::InsertLocalImage { path, caption }` для `saved_path`.
- Если `saved_path` отсутствует, сохранить текущую текстовую history cell.
- Добавить точечные тесты на `saved_path` и no-path fallback.

## Вне объема работ

- Не менять `view_image` path, закрытый stage 003.
- Не менять MCP/base64 image output.
- Не добавлять managed storage или resume-stable ownership.
- Не менять resize/reflow/replay behavior.

## План действий

- [ ] Найти producer `ImageGeneration.saved_path`.
- [ ] Проверить lifetime и доступность файла на стороне TUI.
- [ ] Зафиксировать caption contract.
- [ ] Подключить `saved_path` к `InsertLocalImage` или явно оставить text-only.
- [ ] Обновить feature card и план по фактическому решению.

## Критерии готовности

- Принято явное решение по `ImageGeneration.saved_path`.
- Если path подключен, TUI не создает duplicate legacy text-only cell рядом с bitmap preview.
- Если path не подключен, причина зафиксирована, а текущий text-only fallback остается понятным.
- Тесты покрывают выбранный happy path и fallback/no-path behavior.

## Открытые вопросы

| ID | Вопрос | Статус | Решение / итог | Детали |
| --- | --- | --- | --- | --- |
| [Q-001] | Достаточно ли controlled origin у `saved_path`? | `open` | TBD | [details][Q-001] |
| [Q-002] | Что показывать, если `saved_path` отсутствует? | `open` | TBD | [details][Q-002] |

### Q-001: Origin saved_path

Статус: `open`.

Нужно проверить, кто записывает локальный файл, когда он удаляется и может ли
TUI повторно открыть его для validation.

### Q-002: No-path fallback

Статус: `open`.

Если `ImageGeneration` завершился без `saved_path`, текущая текстовая history
cell с prompt/call id, вероятно, должна остаться без bitmap marker.

[code:app-event]: ../../../../codex-rs/tui/src/app_event.rs
[code:event-dispatch]: ../../../../codex-rs/tui/src/app/event_dispatch.rs
[code:local-image-cell]: ../../../../codex-rs/tui/src/history_cell/local_image.rs
[code:tool-lifecycle]: ../../../../codex-rs/tui/src/chatwidget/tool_lifecycle.rs
[details:actions]: #план-действий
[details:architecture]: #связанные-элементы-архитектуры
[details:criteria]: #критерии-готовности
[details:questions]: #открытые-вопросы
[details:scope]: #объем-работ
[feature:tui-history-image-previews]: ../../../architecture/features/tui-history-image-previews.md
[Q-001]: #q-001-origin-saved_path
[Q-002]: #q-002-no-path-fallback
