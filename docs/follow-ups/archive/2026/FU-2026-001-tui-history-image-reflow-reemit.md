---
id: FU-2026-001
status: done
priority: medium
kind: enhancement
tags: [tui, images, resize-reflow, replay]
created: 2026-05-26
updated: 2026-05-26
review_at: PLAN-TUI-ASSISTANT-IMAGES-001/stages/005-replay-resize-and-terminal-verification.md
owner_plan: PLAN-TUI-ASSISTANT-IMAGES-001
owner_stage: 005-replay-resize-and-terminal-verification
architecture_refs:
  - docs/architecture/features/tui-history-image-previews.md#runtime-заметки
invalid_if:
  - превью изображений в истории терминала намеренно остаются fallback-only вне обычной вставки
---

# FU-2026-001: переэмиссия bitmap-превью при resize/reflow/replay

## Кратко

| Поле | Значение |
| --- | --- |
| Статус | `done` |
| Суть | Решить, должны ли bitmap-превью переэмититься при resize/reflow, initial replay и resume. |
| Почему важно | Вне обычной вставки bitmap marker раньше терялся, и история сохраняла только fallback-метки `[Image #n]`. |
| Когда вернулись | В [stage:PLAN-TUI-ASSISTANT-IMAGES-001:005]. |
| Итог | Resize reflow, initial replay, thread-switch tail replay и overlay-deferred paths переведены на item-level path; Kitty visual smoke пройден. |
| Следующий шаг | Нет; карточка закрыта. Managed ownership оригиналов после resume осознанно остался вне этого follow-up. |
| Связи | [plan:PLAN-TUI-ASSISTANT-IMAGES-001], [stage:PLAN-TUI-ASSISTANT-IMAGES-001:005], [feature:tui-history-image-previews] |

## Наблюдение

Обычная вставка в историю умела писать `HistoryInsertItem::Image` в scrollback
терминала, но resize reflow, initial replay и overlay-deferred paths были
line-oriented и сохраняли только текстовые fallback-метки `[Image #n]`.

## Почему это важно

Если история изображений становится частью workflow, bitmap preview должен
переживать resize, thread replay и resume-replay при доступном source path так
же предсказуемо, как текстовый transcript.

## Что нужно сделать

- [x] Решить, должен ли replay оставаться fallback-only или перейти на уровень
  items.
- [x] Провести `HistoryCellDisplayItem::LocalImage` через resize/reflow и
  replay buffers без потери строкового fallback.
- [x] Добавить точечные тесты на поведение reflow/replay для локальных
  изображений.
- [x] Зафиксировать, что managed ownership оригиналов после resume не входит в
  этот follow-up.

## Итог

Закрыто в stage 005:

- `App::render_transcript_lines_for_reflow` возвращает display-items, а не
  только `Line`.
- `InitialHistoryReplayBuffer` хранит display-items и перед записью снова
  вызывает `prepare_history_insert_items`.
- `insert_history_items_with_wrap_policy` покрыт тестом на подсчет image rows.
- Текстовый fallback остается рядом с bitmap marker и покрывает Raw/copy,
  unsupported terminals, ошибки подготовки asset и недоступный source path.
- Пользовательский smoke в Kitty подтвердил, что TUI показывает картинку через
  `view_image` и preview переживает resize. При экстремально узком окне меньше
  картинки preview может не поместиться; это остается геометрическим
  ограничением, а не возвратом к fallback-only replay.

## Когда вернуться

Карточка была запланирована в [stage:PLAN-TUI-ASSISTANT-IMAGES-001:005] и
закрыта в этом этапе.

## Когда закрыть как неактуальное

Карточка закрыта как реализованная. Она стала бы неактуальной, если бы проект
закрепил fallback-only replay, но в stage 005 выбран item-level replay.

## Связи

- План: [plan:PLAN-TUI-ASSISTANT-IMAGES-001]
- Этап: [stage:PLAN-TUI-ASSISTANT-IMAGES-001:005]
- Архитектура: [feature:tui-history-image-previews]

[feature:tui-history-image-previews]: ../../../architecture/features/tui-history-image-previews.md
[plan:PLAN-TUI-ASSISTANT-IMAGES-001]: ../../../plans/archive/2026/PLAN-TUI-ASSISTANT-IMAGES-001/plan.md
[stage:PLAN-TUI-ASSISTANT-IMAGES-001:005]: ../../../plans/archive/2026/PLAN-TUI-ASSISTANT-IMAGES-001/stages/005-replay-resize-and-terminal-verification.md
