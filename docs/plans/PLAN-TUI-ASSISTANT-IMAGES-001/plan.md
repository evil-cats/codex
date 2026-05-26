# PLAN-TUI-ASSISTANT-IMAGES-001: превью изображений ассистента в истории TUI

## Статус

активен (`active`)

## Кратко

| Поле | Значение |
| --- | --- |
| Цель | Assistant/tool image outputs должны попадать в TUI history как terminal image previews через контролируемый source-backed путь. |
| Уже сделано / решено | User attachment previews уже идут через `HistoryCellDisplayItem::LocalImage`, `HistoryInsertItem::Image` и `/pets`-style terminal image preparation. |
| Открыто / отложено / не сделано | Нет `LocalImageHistoryCell`, нет structured assistant/tool source, resize/reflow/replay bitmap re-emission отложен. |
| Следующий шаг | Реализовать [stage:PLAN-TUI-ASSISTANT-IMAGES-001:002]. |
| Детали | [details:architecture], [details:state], [details:stages], [details:follow-ups] |

## Цель

Сделать так, чтобы ассистентские ответы и контролируемые tool/workflow paths
могли вставлять локальные изображения в TUI history как terminal image
previews, а не только как текстовые ссылки или пользовательские attachment
previews.

План относится к локальной Hermione-доработке Codex TUI и не является
пользовательской документацией продукта.

## Архитектурные ссылки

| Тема | Документ |
| --- | --- |
| TUI history local image previews | [feature:tui-history-image-previews] |
| Источник local image markers | [code:history-cell] |
| User attachment preview path | [code:history-cell-messages] |
| Подготовка image protocol payload | [code:pets-mod] |
| Kitty / Sixel helpers | [code:image-protocol] |
| Terminal scrollback insertion | [code:insert-history] |
| Resize/reflow replay | [code:resize-reflow] |
| App event insertion path | [code:app-event] |

## Текущее состояние

- Уже есть общий scrollback insertion path для `HistoryInsertItem::Image`.
- `UserHistoryCell.local_image_paths` умеет отдавать `HistoryCellDisplayItem::LocalImage`.
- `App::prepare_history_insert_items` конвертирует local image markers в
  terminal image payload через `/pets`-style detection.
- `pets::prepare_history_image` поддерживает Kitty, KittyLocalFile и Sixel;
  Kitty paths нормализуют входные изображения в PNG-preview cache.
- Реализованные commits: `96feb7e0d Add terminal image previews to TUI history`,
  `483c08245 Normalize history images for Kitty previews`.
- Сейчас preview доступен для пользовательских local image attachments, но нет
  отдельного ассистентского/source-backed cell и нет согласованного источника
  image item со стороны ответа ассистента или tool output.

## Этапы

| Этап | Статус | Документ | Результат |
| --- | --- | --- | --- |
| 001 | выполнен (`completed`) | [stage:PLAN-TUI-ASSISTANT-IMAGES-001:001] | Границы MVP |
| 002 | предложен (`proposed`) | [stage:PLAN-TUI-ASSISTANT-IMAGES-001:002] | Источник assistant/tool изображений |
| 003 | предложен (`proposed`) | TBD | Подключить контролируемый источник ассистентских image items |
| 004 | предложен (`proposed`) | TBD | Улучшить replay/resize и ручную проверку в реальных терминалах |

## Следующий шаг

Следующий шаг: реализовать этап 002:

- `LocalImageHistoryCell`;
- structured event `AppEvent::InsertLocalImage { path, caption }`;
- validation handler;
- targeted tests.

## Отложенные работы

| ID | Статус | Когда вернуться |
| --- | --- | --- |
| [follow-up:FU-2026-001] | принят (`accepted`) | Перед расширением image history beyond normal insertion |
| [follow-up:FU-2026-002] | принят (`accepted`) | При старте source-backed assistant/tool image previews |

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
