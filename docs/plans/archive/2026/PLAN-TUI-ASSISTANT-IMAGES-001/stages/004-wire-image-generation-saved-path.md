# 004: ImageGeneration saved_path как image preview

## Статус

`completed`

## Кратко

| Поле | Значение |
| --- | --- |
| Цель этапа | Решить, должен ли `ImageGeneration.saved_path` отправлять `AppEvent::InsertLocalImage { path, caption }`, и подключить его, если контракт подходит. |
| Уже сделано / решено | `ImageGeneration.saved_path` признан controlled source: core сохраняет artifact в `CODEX_HOME/generated_images/...`, TUI отправляет `AppEvent::InsertLocalImage`, app layer сохраняет validation. |
| Открыто / отложено / не сделано | Replay/resize/resume остались вне scope и перенесены в [stage:PLAN-TUI-ASSISTANT-IMAGES-001:005]. |
| Следующий шаг | Перейти к [stage:PLAN-TUI-ASSISTANT-IMAGES-001:005]: replay/resize и ручная проверка в реальных терминалах. |
| Детали | [details:architecture], [details:scope], [details:actions], [details:criteria], [details:questions], [details:verification] |

## Зачем этап нужен

`ImageGeneration` уже несет `saved_path: Option<AbsolutePathBuf>` в protocol
item. Это похожий на `view_image` controlled path, но его origin, cleanup и
поведение без сохраненного файла нужно проверить отдельно, чтобы не смешивать
два разных source contract в stage 003.

## Связанные элементы архитектуры

| Элемент | Ссылка |
| --- | --- |
| `ChatWidget::on_image_generation_end` | [code:tool-lifecycle] |
| `AppEvent::InsertLocalImage` | [code:app-event] |
| Dispatch app events | [code:event-dispatch] |
| `LocalImageHistoryCell` | [code:local-image-cell] |
| Фича image preview | [feature:tui-history-image-previews] |

## Объем работ

- Найти, где создается и сохраняется `ImageGeneration.saved_path`.
- Решить, является ли `saved_path` достаточно controlled source для terminal preview.
- Если да, отправлять `AppEvent::InsertLocalImage { path, caption }` для `saved_path`.
- Если `saved_path` отсутствует, сохранить текущую текстовую history cell.
- Добавить точечные тесты на `saved_path` и no-path fallback.

## Вне объема работ

- Не менять `view_image` path, закрытый stage 003.
- Не менять MCP/base64 image output.
- Не добавлять managed storage или resume-stable ownership.
- Не менять resize/reflow/replay behavior.

## План действий

- [x] Найти producer `ImageGeneration.saved_path`.
- [x] Проверить lifetime и доступность файла на стороне TUI.
- [x] Зафиксировать caption contract.
- [x] Подключить `saved_path` к `InsertLocalImage`; no-path fallback оставить text-only.
- [x] Обновить feature card и план по фактическому решению.

## Критерии готовности

- Принято явное решение по `ImageGeneration.saved_path`.
- Если path подключен, TUI не создает duplicate legacy text-only cell рядом с bitmap preview.
- Если path не подключен, причина зафиксирована, а текущий text-only fallback остается понятным.
- Тесты покрывают выбранный happy path и fallback/no-path behavior.

## Открытые вопросы

| ID | Вопрос | Статус | Решение / итог | Детали |
| --- | --- | --- | --- | --- |
| [Q-001] | Достаточно ли controlled origin у `saved_path`? | `decided` | Да: core сохраняет artifact под `CODEX_HOME/generated_images/...`, а TUI дополнительно валидирует local image event. | [details][Q-001] |
| [Q-002] | Что показывать, если `saved_path` отсутствует? | `decided` | Оставлять текущую text-only history cell с prompt/call id без bitmap marker. | [details][Q-002] |

### Q-001: Origin saved_path

Статус: `decided`.

Решение: `saved_path` является controlled source для текущего stage. Core
сохраняет completed `image_generation_call` result в
`CODEX_HOME/generated_images/<session>/<call>.png`, записывает путь в
`ImageGeneration.saved_path`, а TUI отправляет этот путь через
`AppEvent::InsertLocalImage`. App layer по-прежнему проверяет `regular file` и
decode через `image` crate перед созданием bitmap marker.

Caption contract: если `revised_prompt` непустой, fallback caption равен ему;
иначе используется `call_id`.

### Q-002: No-path fallback

Статус: `decided`.

Если `ImageGeneration` завершился без `saved_path`, текущая текстовая history
cell с prompt/call id остается без bitmap marker. Это сохраняет понятный
fallback для случаев, где сохранение результата не удалось или payload не был
доступен.

## Проверки

- `env RUST_MIN_STACK=8388608 cargo test -p codex-tui image_generation_call` -
  passed.
- `just fmt` - `cargo fmt` completed, затем recipe снова упал на Python `uv`:
  `openai-codex-cli-bin==0.131.0a4` не имеет совместимого wheel для текущей
  Linux platform.

[code:app-event]: ../../../../../../codex-rs/tui/src/app_event.rs
[code:event-dispatch]: ../../../../../../codex-rs/tui/src/app/event_dispatch.rs
[code:local-image-cell]: ../../../../../../codex-rs/tui/src/history_cell/local_image.rs
[code:tool-lifecycle]: ../../../../../../codex-rs/tui/src/chatwidget/tool_lifecycle.rs
[details:actions]: #план-действий
[details:architecture]: #связанные-элементы-архитектуры
[details:criteria]: #критерии-готовности
[details:questions]: #открытые-вопросы
[details:scope]: #объем-работ
[details:verification]: #проверки
[feature:tui-history-image-previews]: ../../../../../architecture/features/tui-history-image-previews.md
[Q-001]: #q-001-origin-saved_path
[Q-002]: #q-002-no-path-fallback
[stage:PLAN-TUI-ASSISTANT-IMAGES-001:005]: 005-replay-resize-and-terminal-verification.md
