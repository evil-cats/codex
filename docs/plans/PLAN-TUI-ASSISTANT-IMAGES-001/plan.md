# PLAN-TUI-ASSISTANT-IMAGES-001: превью изображений ассистента в истории TUI

## Статус

`active`

## Кратко

| Поле | Значение |
| --- | --- |
| Цель | Изображения из ответов ассистента/инструментов должны попадать в историю TUI как терминальные превью через контролируемый путь с исходным файлом. |
| Уже сделано / решено | Превью пользовательских вложений уже идут через `HistoryCellDisplayItem::LocalImage`, `HistoryInsertItem::Image` и подготовку terminal image в стиле `/pets`. |
| Открыто / отложено / не сделано | Нет `LocalImageHistoryCell`, нет структурированного источника изображений ассистента/инструментов, повторная эмиссия bitmap-превью при resize/reflow/replay отложена. |
| Следующий шаг | Реализовать [stage:PLAN-TUI-ASSISTANT-IMAGES-001:002]. |
| Детали | [details:architecture], [details:state], [details:stages], [details:follow-ups] |

## Цель

Сделать так, чтобы ассистентские ответы и контролируемые пути инструментов/workflow
могли вставлять локальные изображения в историю TUI как терминальные image
previews, а не только как текстовые ссылки или превью пользовательских
вложений.

План относится к локальной Hermione-доработке Codex TUI и не является
пользовательской документацией продукта.

## Архитектурные ссылки

| Тема | Документ |
| --- | --- |
| Локальные image previews в истории TUI | [feature:tui-history-image-previews] |
| Источник маркеров локальных изображений | [code:history-cell] |
| Путь превью пользовательских вложений | [code:history-cell-messages] |
| Подготовка payload для image protocol | [code:pets-mod] |
| Helpers для Kitty / Sixel | [code:image-protocol] |
| Вставка в историю терминала | [code:insert-history] |
| Replay при resize/reflow | [code:resize-reflow] |
| Путь вставки через app event | [code:app-event] |

## Текущее состояние

- Уже есть общий путь вставки в scrollback для `HistoryInsertItem::Image`.
- `UserHistoryCell.local_image_paths` умеет отдавать `HistoryCellDisplayItem::LocalImage`.
- `App::prepare_history_insert_items` конвертирует маркеры локальных
  изображений в terminal image payload через detection в стиле `/pets`.
- `pets::prepare_history_image` поддерживает Kitty, KittyLocalFile и Sixel;
  пути Kitty нормализуют входные изображения в PNG-preview cache.
- Реализованные commits: `96feb7e0d Add terminal image previews to TUI history`,
  `483c08245 Normalize history images for Kitty previews`.
- Сейчас preview доступен для пользовательских локальных image attachments, но
  нет отдельной ассистентской cell с исходным путем и нет согласованного
  источника image item со стороны ответа ассистента или вывода tool.

## Этапы

| Этап | Статус | Документ | Результат |
| --- | --- | --- | --- |
| 001 | `completed` | [stage:PLAN-TUI-ASSISTANT-IMAGES-001:001] | Границы MVP |
| 002 | `proposed` | [stage:PLAN-TUI-ASSISTANT-IMAGES-001:002] | Источник изображений ассистента/инструментов |
| 003 | `proposed` | TBD | Подключить контролируемый источник ассистентских image items |
| 004 | `proposed` | TBD | Улучшить replay/resize и ручную проверку в реальных терминалах |

## Следующий шаг

Следующий шаг: реализовать этап 002:

- `LocalImageHistoryCell`;
- структурированное событие `AppEvent::InsertLocalImage { path, caption }`;
- handler валидации;
- точечные тесты.

## Отложенные работы

| ID | Статус | Когда вернуться |
| --- | --- | --- |
| [follow-up:FU-2026-001] | `accepted` | Перед расширением истории изображений за пределы обычной вставки |
| [follow-up:FU-2026-002] | `accepted` | При старте превью изображений ассистента/инструментов с исходным файлом |

[code:app-event]: ../../../codex-rs/tui/src/app_event.rs
[code:history-cell]: ../../../codex-rs/tui/src/history_cell/mod.rs
[code:history-cell-messages]: ../../../codex-rs/tui/src/history_cell/messages.rs
[code:image-protocol]: ../../../codex-rs/tui/src/pets/image_protocol.rs
[code:insert-history]: ../../../codex-rs/tui/src/insert_history.rs
[code:pets-mod]: ../../../codex-rs/tui/src/pets/mod.rs
[code:resize-reflow]: ../../../codex-rs/tui/src/app/resize_reflow.rs
[details:architecture]: #архитектурные-ссылки
[details:follow-ups]: #отложенные-работы
[details:stages]: #этапы
[details:state]: #текущее-состояние
[feature:tui-history-image-previews]: ../../architecture/features/tui-history-image-previews.md
[follow-up:FU-2026-001]: ../../follow-ups/FU-2026-001-tui-history-image-reflow-reemit.md
[follow-up:FU-2026-002]: ../../follow-ups/FU-2026-002-tui-assistant-tool-image-source.md
[stage:PLAN-TUI-ASSISTANT-IMAGES-001:001]: stages/001-open-questions-and-mvp.md
[stage:PLAN-TUI-ASSISTANT-IMAGES-001:002]: stages/002-local-image-history-cell.md
