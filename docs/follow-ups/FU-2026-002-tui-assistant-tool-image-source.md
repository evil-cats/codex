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
  - docs/architecture/features/tui-history-image-previews.md#contracts
invalid_if:
  - assistant and tool outputs remain text-only and do not render local bitmap previews in TUI history
---

# FU-2026-002: Controlled assistant/tool image source path

## Кратко

| Поле | Значение |
| --- | --- |
| Статус | `accepted` |
| Summary | Добавить controlled source-backed path для assistant/tool generated local images без Markdown auto-rendering. |
| Почему важно | Без structured boundary легко смешать model text, shell output, local file reads и trusted UI events. |
| Когда вернуться | При старте stage 002 или другого плана, который добавляет assistant/tool image previews. |
| Когда закрыть | Если assistant/tool outputs остаются text-only или image output переедет в отдельный UI surface. |
| Следующий шаг | Реализовать `AppEvent::InsertLocalImage { path, caption }`, validation и regression tests против Markdown/plain text. |
| Связи | [plan:PLAN-TUI-ASSISTANT-IMAGES-001], [stage:PLAN-TUI-ASSISTANT-IMAGES-001:002], [feature:tui-history-image-previews] |

## Наблюдение

Current implementation supports local image previews for user attachments via
`UserHistoryCell.local_image_paths`. Assistant/tool generated local images still
need a controlled source-backed path; arbitrary Markdown image syntax or shell
output must not become trusted local file input.

## Почему это важно

Without a structured source boundary, image rendering can accidentally blur
model text, shell output, local file reads and trusted UI events. That would
make the feature harder to review and riskier to extend.

## Что нужно сделать

- Define the MVP source path, for example `AppEvent::InsertLocalImage { path,
  caption }` plus app-layer validation.
- Add a dedicated cell or equivalent source-backed representation for
  assistant/tool image outputs.
- Validate that `path` exists, is a regular file and can be decoded before
  producing a bitmap marker.
- Add regression tests proving that Markdown/plain text does not auto-render
  local images.

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
