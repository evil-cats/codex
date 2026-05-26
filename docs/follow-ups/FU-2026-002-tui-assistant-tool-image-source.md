---
id: FU-2026-002
status: accepted
priority: high
kind: feature
tags: [tui, images, assistant-output, tool-output]
created: 2026-05-26
updated: 2026-05-26
review_at: PLAN-TUI-ASSISTANT-IMAGES-001/stages/002-local-image-history-cell.md
owner_plan: PLAN-TUI-ASSISTANT-IMAGES-001
owner_stage: 002-local-image-history-cell
architecture_refs:
  - docs/architecture/features/tui-history-image-previews.md#контракты
invalid_if:
  - вывод ассистента и инструментов остается text-only и не рендерит локальные bitmap-превью в истории TUI
---

# FU-2026-002: контролируемый исходный путь для изображений ассистента/инструментов

## Кратко

| Поле | Значение |
| --- | --- |
| Статус | `accepted` |
| Суть | Добавить контролируемый путь с исходным файлом для локальных изображений ассистента/инструментов без Markdown auto-rendering. |
| Почему важно | Без структурированной границы легко смешать текст модели, shell output, чтение локальных файлов и доверенные UI-события. |
| Когда вернуться | При старте stage 002 или другого плана, который добавляет превью изображений ассистента/инструментов. |
| Когда закрыть | Если вывод ассистента/инструментов остается text-only или image output переедет в отдельный UI surface. |
| Следующий шаг | Реализовать `AppEvent::InsertLocalImage { path, caption }`, validation и regression tests против Markdown/plain text. |
| Связи | [plan:PLAN-TUI-ASSISTANT-IMAGES-001], [stage:PLAN-TUI-ASSISTANT-IMAGES-001:002], [feature:tui-history-image-previews] |

## Наблюдение

Текущая реализация поддерживает local image previews для пользовательских
вложений через `UserHistoryCell.local_image_paths`. Для локальных изображений
ассистента/инструментов нужен контролируемый путь с исходным файлом; произвольный
Markdown image syntax или shell output не должен становиться доверенным входом
локального файла.

## Почему это важно

Без структурированной границы источника image rendering может случайно смешать
текст модели, shell output, чтение локальных файлов и доверенные UI-события. Это
усложнит review фичи и сделает расширение рискованнее.

## Что нужно сделать

- Зафиксировать исходный путь MVP, например `AppEvent::InsertLocalImage { path,
  caption }` плюс validation в app layer.
- Добавить отдельную cell или эквивалентное представление с исходным файлом для
  вывода изображений ассистента/инструментов.
- Проверять, что `path` существует, является regular file и декодируется до
  создания bitmap-маркера.
- Добавить regression tests, подтверждающие, что Markdown/plain text не
  выполняют auto-render локальных изображений.

## Когда вернуться

При старте stage 002 или другого плана, который добавляет image previews
ассистента/инструментов в историю TUI.

## Когда закрыть как неактуальное

Если вывод ассистента/инструментов окончательно остается text-only или image output
переносится в отдельный UI surface без terminal history previews.

## Связи

- План: [plan:PLAN-TUI-ASSISTANT-IMAGES-001]
- Этап: [stage:PLAN-TUI-ASSISTANT-IMAGES-001:002]
- Архитектура: [feature:tui-history-image-previews]

[feature:tui-history-image-previews]: ../architecture/features/tui-history-image-previews.md
[plan:PLAN-TUI-ASSISTANT-IMAGES-001]: ../plans/PLAN-TUI-ASSISTANT-IMAGES-001/plan.md
[stage:PLAN-TUI-ASSISTANT-IMAGES-001:002]: ../plans/PLAN-TUI-ASSISTANT-IMAGES-001/stages/002-local-image-history-cell.md
