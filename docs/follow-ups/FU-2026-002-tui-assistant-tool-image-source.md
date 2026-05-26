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
  - assistant and tool outputs remain text-only and do not render local bitmap previews in TUI history
---

# FU-2026-002: controlled source path для assistant/tool изображений

## Кратко

| Поле | Значение |
| --- | --- |
| Статус | принят (`accepted`) |
| Суть | Добавить controlled source-backed path для assistant/tool generated local images без Markdown auto-rendering. |
| Почему важно | Без structured boundary легко смешать model text, shell output, local file reads и trusted UI events. |
| Когда вернуться | При старте stage 002 или другого плана, который добавляет assistant/tool image previews. |
| Когда закрыть | Если assistant/tool outputs остаются text-only или image output переедет в отдельный UI surface. |
| Следующий шаг | Реализовать `AppEvent::InsertLocalImage { path, caption }`, validation и regression tests против Markdown/plain text. |
| Связи | [plan:PLAN-TUI-ASSISTANT-IMAGES-001], [stage:PLAN-TUI-ASSISTANT-IMAGES-001:002], [feature:tui-history-image-previews] |

## Наблюдение

Текущая реализация поддерживает local image previews для user attachments через
`UserHistoryCell.local_image_paths`. Для assistant/tool generated local images
нужен controlled source-backed path; произвольный Markdown image syntax или
shell output не должен становиться trusted local file input.

## Почему это важно

Без structured source boundary image rendering может случайно смешать model
text, shell output, local file reads и trusted UI events. Это усложнит review
фичи и сделает расширение рискованнее.

## Что нужно сделать

- Зафиксировать MVP source path, например `AppEvent::InsertLocalImage { path,
  caption }` плюс app-layer validation.
- Добавить dedicated cell или эквивалентное source-backed представление для
  assistant/tool image outputs.
- Проверять, что `path` существует, является regular file и декодируется до
  создания bitmap marker.
- Добавить regression tests, подтверждающие, что Markdown/plain text не
  auto-render local images.

## Когда вернуться

При старте stage 002 или другого плана, который добавляет assistant/tool image
previews в TUI history.

## Когда закрыть как неактуальное

Если assistant/tool outputs окончательно остаются text-only или image output
переносится в отдельный UI surface без terminal history previews.

## Связи

- План: [plan:PLAN-TUI-ASSISTANT-IMAGES-001]
- Этап: [stage:PLAN-TUI-ASSISTANT-IMAGES-001:002]
- Архитектура: [feature:tui-history-image-previews]

[feature:tui-history-image-previews]: ../architecture/features/tui-history-image-previews.md
[plan:PLAN-TUI-ASSISTANT-IMAGES-001]: ../plans/PLAN-TUI-ASSISTANT-IMAGES-001/plan.md
[stage:PLAN-TUI-ASSISTANT-IMAGES-001:002]: ../plans/PLAN-TUI-ASSISTANT-IMAGES-001/stages/002-local-image-history-cell.md
