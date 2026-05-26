# Итог: PLAN-TUI-ASSISTANT-IMAGES-001

## Кратко

| Поле | Значение |
| --- | --- |
| Итог | Controlled local image previews для TUI history реализованы для пользовательских вложений, `view_image`, `ImageGeneration.saved_path` и replay/reflow paths. |
| Изменилось | План расширился от MVP пользовательских вложений до ассистентских/tool-generated sources и item-level replay. |
| Осталось | Managed ownership оригинальных изображений после resume не реализован; при недоступном source path остается fallback. При экстремально узком окне preview может не поместиться. |
| Проверки | Узкие `cargo test -p codex-tui ...`, work-tracking check, `git diff --check`, manual Kitty smoke. |
| Дальше | Дальнейшего плана не требуется. |

## Итог

План завершен. TUI history теперь умеет показывать локальные изображения через
структурированный и controlled path:

- пользовательские local image attachments;
- `AppEvent::InsertLocalImage { path, caption }`;
- `ThreadItem::ImageView` / `view_image`;
- `ImageGeneration.saved_path`;
- resize reflow, initial replay, thread-switch tail replay и overlay-deferred
  history paths.

Bitmap marker сохраняется как `HistoryCellDisplayItem::LocalImage` до границы
`App::prepare_history_insert_items`, где он best-effort превращается в
`HistoryInsertItem::Image`. Текстовый fallback остается рядом с preview.

## Что изменилось относительно исходного плана

Первоначальный MVP был осторожнее и допускал fallback-only поведение для
resize/reflow/replay. По итогам этапа 005 replay переведен на item-level path,
а manual smoke в Kitty подтвердил, что preview появляется в TUI и переживает
обычный resize.

## Ключевые решения

- Не рендерить произвольный Markdown/plain text как trusted local image source.
- Использовать controlled app-layer event для ассистентских и tool-generated
  local images.
- Считать `ImageGeneration.saved_path` controlled source, но сохранять no-path
  fallback как text-only history cell.
- Для replay/reflow хранить display-items, а не только `Line`.
- Не вводить managed storage оригинальных изображений после resume в этом
  плане.

## Измененные архитектурные элементы

- Feature card: [feature:tui-history-image-previews].
- Follow-up replay/reflow: [follow-up:FU-2026-001] закрыт как реализованный.
- Follow-up controlled source: [follow-up:FU-2026-002] закрыт ранее stage 002.

## Что осталось

Осталось только осознанное ограничение, не требующее нового плана:

- если исходный файл после resume недоступен, bitmap payload не готовится, но
  fallback остается;
- если окно экстремально уже preview, картинка может не поместиться.

## Проверки

- `env RUST_MIN_STACK=8388608 cargo test -p codex-tui resize_reflow` - passed.
- `env RUST_MIN_STACK=8388608 cargo test -p codex-tui insert_history_items_with_wrap_policy_counts_image_rows` - passed.
- `env RUST_MIN_STACK=8388608 cargo test -p codex-tui image_generation_call` - passed.
- `python3 -B /home/slader/.codex/skills/work-tracking/scripts/check_work_tracking.py /mnt/ml/Projects/evilcats/codex` - passed.
- `git diff --check` - passed.
- Manual Kitty smoke - passed: прямой Kitty graphics command показал PNG, TUI
  показал картинку через `view_image`, preview пережил обычный resize.

Полный `cargo test -p codex-tui` запускался и упал на уже известные
environment-dependent failures: snapshot drift от temp project name и Unix
socket `Operation not permitted` в sandbox.

## Что дальше

Дальнейшего плана не требуется.

[feature:tui-history-image-previews]: ../../../../architecture/features/tui-history-image-previews.md
[follow-up:FU-2026-001]: ../../../../follow-ups/archive/2026/FU-2026-001-tui-history-image-reflow-reemit.md
[follow-up:FU-2026-002]: ../../../../follow-ups/archive/2026/FU-2026-002-tui-assistant-tool-image-source.md
