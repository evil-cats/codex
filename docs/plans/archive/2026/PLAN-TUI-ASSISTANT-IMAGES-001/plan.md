# PLAN-TUI-ASSISTANT-IMAGES-001: превью изображений ассистента в истории TUI

## Статус

`archived`

## Кратко

| Поле | Значение |
| --- | --- |
| Цель | Изображения из ответов ассистента/инструментов должны попадать в историю TUI как терминальные превью через контролируемый путь с исходным файлом. |
| Уже сделано / решено | Пользовательские вложения, `view_image` tool path, `ImageGeneration.saved_path` и replay/reflow paths идут через controlled local image preview. |
| Открыто / отложено / не сделано | Managed ownership оригинальных изображений после resume не входит в этот план; если source path недоступен, остается fallback. При экстремально узком окне preview может не поместиться. |
| Следующий шаг | План завершен и перенесен в архив; дальнейшего плана не требуется. |
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
- Сейчас preview доступен для пользовательских локальных image attachments.
- Для ассистентских/инструментальных изображений есть `LocalImageHistoryCell`,
  `AppEvent::InsertLocalImage { path, caption }`, validation в app layer и
  fallback при invalid image event.
- `ThreadItem::ImageView` / `view_image` отправляет `InsertLocalImage` и больше
  не создает legacy text-only history cell.
- `ImageGeneration.saved_path` отправляет `InsertLocalImage`, если core сохранил
  generated image artifact; no-path fallback остается text-only history cell.
- Resize reflow, initial replay, thread-switch tail replay и overlay-deferred
  paths сохраняют `HistoryCellDisplayItem::LocalImage` до
  `prepare_history_insert_items`, поэтому bitmap preview переэмитится при
  доступном source path.
- Manual Kitty visual smoke пройден: direct graphics command показал PNG, TUI
  показал картинку через `view_image`, preview пережил resize кроме
  экстремально узкого окна меньше картинки.

## Этапы

| Этап | Статус | Документ | Результат |
| --- | --- | --- | --- |
| 001 | `completed` | [stage:PLAN-TUI-ASSISTANT-IMAGES-001:001] | Границы MVP |
| 002 | `completed` | [stage:PLAN-TUI-ASSISTANT-IMAGES-001:002] | `LocalImageHistoryCell` и `InsertLocalImage` |
| 003 | `completed` | [stage:PLAN-TUI-ASSISTANT-IMAGES-001:003] | `view_image` production caller |
| 004 | `completed` | [stage:PLAN-TUI-ASSISTANT-IMAGES-001:004] | `ImageGeneration.saved_path` подключен к `InsertLocalImage`; no-path fallback остался text-only |
| 005 | `completed` | [stage:PLAN-TUI-ASSISTANT-IMAGES-001:005] | Replay/resize переведен на item-level path; Kitty visual smoke пройден |

## Следующий шаг

План завершен и перенесен в архив. Дальнейшего плана не требуется.

## Отложенные работы

| ID | Статус | Когда вернуться |
| --- | --- | --- |
| [follow-up:FU-2026-001] | `done` | Реализовано в stage 005; карточка перенесена в архив |
| [follow-up:FU-2026-002] | `done` | Реализовано в stage 002; следующий вызывающий код идет через stage 003 |

[code:app-event]: ../../../../../codex-rs/tui/src/app_event.rs
[code:history-cell]: ../../../../../codex-rs/tui/src/history_cell/mod.rs
[code:history-cell-messages]: ../../../../../codex-rs/tui/src/history_cell/messages.rs
[code:image-protocol]: ../../../../../codex-rs/tui/src/pets/image_protocol.rs
[code:insert-history]: ../../../../../codex-rs/tui/src/insert_history.rs
[code:pets-mod]: ../../../../../codex-rs/tui/src/pets/mod.rs
[code:resize-reflow]: ../../../../../codex-rs/tui/src/app/resize_reflow.rs
[details:architecture]: #архитектурные-ссылки
[details:follow-ups]: #отложенные-работы
[details:stages]: #этапы
[details:state]: #текущее-состояние
[feature:tui-history-image-previews]: ../../../../architecture/features/tui-history-image-previews.md
[follow-up:FU-2026-001]: ../../../../follow-ups/archive/2026/FU-2026-001-tui-history-image-reflow-reemit.md
[follow-up:FU-2026-002]: ../../../../follow-ups/archive/2026/FU-2026-002-tui-assistant-tool-image-source.md
[stage:PLAN-TUI-ASSISTANT-IMAGES-001:001]: stages/001-open-questions-and-mvp.md
[stage:PLAN-TUI-ASSISTANT-IMAGES-001:002]: stages/002-local-image-history-cell.md
[stage:PLAN-TUI-ASSISTANT-IMAGES-001:003]: stages/003-wire-assistant-image-source.md
[stage:PLAN-TUI-ASSISTANT-IMAGES-001:004]: stages/004-wire-image-generation-saved-path.md
[stage:PLAN-TUI-ASSISTANT-IMAGES-001:005]: stages/005-replay-resize-and-terminal-verification.md
