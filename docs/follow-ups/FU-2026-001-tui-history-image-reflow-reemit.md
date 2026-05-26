---
id: FU-2026-001
status: accepted
priority: medium
kind: enhancement
tags: [tui, images, resize-reflow, replay]
created: 2026-05-26
updated: 2026-05-26
review_at: PLAN-TUI-ASSISTANT-IMAGES-001/stages/002-local-image-history-cell.md
owner_plan: PLAN-TUI-ASSISTANT-IMAGES-001
owner_stage: 002-local-image-history-cell
architecture_refs:
  - docs/architecture/features/tui-history-image-previews.md#runtime-заметки
invalid_if:
  - terminal history image previews remain intentionally fallback-only outside normal insertion
---

# FU-2026-001: переэмиссия bitmap previews при resize/reflow/replay

## Кратко

| Поле | Значение |
| --- | --- |
| Статус | принят (`accepted`) |
| Суть | Решить, должны ли bitmap previews переэмититься при resize/reflow, initial replay и resume. |
| Почему важно | Сейчас вне normal insertion остаются только `[Image #n]` fallback labels; ожидания пользователя могут стать выше. |
| Когда вернуться | Перед расширением image history beyond normal insertion или перед resume-stable image assets. |
| Когда закрыть | Если проектное решение закрепит fallback-only behavior для local images вне normal insertion. |
| Следующий шаг | При планировании replay/reflow решить, нужен ли item-oriented replay, и добавить focused tests. |
| Связи | [plan:PLAN-TUI-ASSISTANT-IMAGES-001], [stage:PLAN-TUI-ASSISTANT-IMAGES-001:002], [feature:tui-history-image-previews] |

## Наблюдение

Normal history insertion умеет писать `HistoryInsertItem::Image` в terminal
scrollback, но resize reflow, initial replay и overlay-deferred paths сейчас
остаются line-oriented и сохраняют только текстовые `[Image #n]` fallback
labels.

## Почему это важно

Если image history станет важной частью workflow, пользователь может ожидать,
что bitmap preview переживет resize, thread replay или resume так же, как
обычный текстовый transcript. Сейчас это не invariant фичи, а осознанное
ограничение.

## Что нужно сделать

- Решить, должен ли replay оставаться fallback-only или стать item-oriented.
- Если нужен bitmap replay, провести `HistoryCellDisplayItem::LocalImage`
  через resize/reflow и replay buffers без потери row cap semantics.
- Добавить focused tests на reflow/replay behavior для local images.
- Провести ручную проверку в Kitty-compatible terminal и fallback проверку в
  unsupported terminal/multiplexer.

## Когда вернуться

Перед расширением image history beyond normal insertion, особенно перед
assistant/tool image source path или resume-stable image assets.

## Когда закрыть как неактуальное

Если проектное решение закрепит, что resize/reflow/replay для local images
намеренно остаются fallback-only.

## Связи

- План: [plan:PLAN-TUI-ASSISTANT-IMAGES-001]
- Этап: [stage:PLAN-TUI-ASSISTANT-IMAGES-001:002]
- Архитектура: [feature:tui-history-image-previews]

[feature:tui-history-image-previews]: ../architecture/features/tui-history-image-previews.md
[plan:PLAN-TUI-ASSISTANT-IMAGES-001]: ../plans/PLAN-TUI-ASSISTANT-IMAGES-001/plan.md
[stage:PLAN-TUI-ASSISTANT-IMAGES-001:002]: ../plans/PLAN-TUI-ASSISTANT-IMAGES-001/stages/002-local-image-history-cell.md
