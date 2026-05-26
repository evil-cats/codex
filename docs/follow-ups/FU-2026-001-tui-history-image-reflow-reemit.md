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
  - превью изображений в истории терминала намеренно остаются fallback-only вне обычной вставки
---

# FU-2026-001: переэмиссия bitmap-превью при resize/reflow/replay

## Кратко

| Поле | Значение |
| --- | --- |
| Статус | `accepted` |
| Суть | Решить, должны ли bitmap-превью переэмититься при resize/reflow, initial replay и resume. |
| Почему важно | Сейчас вне обычной вставки остаются только fallback-метки `[Image #n]`; ожидания пользователя могут стать выше. |
| Когда вернуться | Перед расширением истории изображений за пределы обычной вставки или перед image assets, стабильными после resume. |
| Когда закрыть | Если проектное решение закрепит fallback-only поведение для локальных изображений вне обычной вставки. |
| Следующий шаг | При планировании replay/reflow решить, нужен ли replay на уровне items, и добавить точечные тесты. |
| Связи | [plan:PLAN-TUI-ASSISTANT-IMAGES-001], [stage:PLAN-TUI-ASSISTANT-IMAGES-001:002], [feature:tui-history-image-previews] |

## Наблюдение

Обычная вставка в историю умеет писать `HistoryInsertItem::Image` в scrollback
терминала, но resize reflow, initial replay и overlay-deferred paths сейчас
остаются line-oriented и сохраняют только текстовые fallback-метки `[Image #n]`.

## Почему это важно

Если история изображений станет важной частью workflow, пользователь может ожидать,
что bitmap preview переживет resize, thread replay или resume так же, как
обычный текстовый transcript. Сейчас это не invariant фичи, а осознанное
ограничение.

## Что нужно сделать

- Решить, должен ли replay оставаться fallback-only или перейти на уровень items.
- Если нужен bitmap replay, провести `HistoryCellDisplayItem::LocalImage`
  через resize/reflow и replay buffers без потери ограничения строк.
- Добавить точечные тесты на поведение reflow/replay для локальных изображений.
- Провести ручную проверку в Kitty-compatible terminal и fallback проверку в
  неподдерживаемом терминале/мультиплексоре.

## Когда вернуться

Перед расширением истории изображений за пределы обычной вставки, особенно
перед исходным путем изображений ассистента/инструментов или image assets,
стабильными после resume.

## Когда закрыть как неактуальное

Если проектное решение закрепит, что resize/reflow/replay для локальных изображений
намеренно остаются fallback-only.

## Связи

- План: [plan:PLAN-TUI-ASSISTANT-IMAGES-001]
- Этап: [stage:PLAN-TUI-ASSISTANT-IMAGES-001:002]
- Архитектура: [feature:tui-history-image-previews]

[feature:tui-history-image-previews]: ../architecture/features/tui-history-image-previews.md
[plan:PLAN-TUI-ASSISTANT-IMAGES-001]: ../plans/PLAN-TUI-ASSISTANT-IMAGES-001/plan.md
[stage:PLAN-TUI-ASSISTANT-IMAGES-001:002]: ../plans/PLAN-TUI-ASSISTANT-IMAGES-001/stages/002-local-image-history-cell.md
