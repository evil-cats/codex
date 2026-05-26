# 003: источник image items ассистента/инструментов

## Статус

`completed`

## Кратко

| Поле | Значение |
| --- | --- |
| Цель этапа | Подключить первый контролируемый источник, который будет отправлять `AppEvent::InsertLocalImage { path, caption }`. |
| Уже сделано / решено | `ThreadItem::ImageView` / `view_image` выбран первым caller и отправляет `AppEvent::InsertLocalImage`. |
| Открыто / отложено / не сделано | `ImageGeneration.saved_path`, resize/reflow/resume и managed storage остаются вне scope. |
| Следующий шаг | Разобрать [stage:PLAN-TUI-ASSISTANT-IMAGES-001:004] по `ImageGeneration.saved_path`. |
| Детали | [details:architecture], [details:scope], [details:actions], [details:criteria], [details:questions], [details:verification] |

## Зачем этап нужен

Этап 002 создал безопасную границу: произвольный Markdown, shell output и
model text не превращаются в локальные previews. Теперь нужен первый реальный
вызывающий код, который уже владеет локальным файлом изображения и может явно запросить
вставку preview в историю TUI.

В этом этапе первым production caller выбран `ThreadItem::ImageView`, который
создается tool path `view_image` после core-level проверки локального файла.

## Связанные элементы архитектуры

| Элемент | Ссылка |
| --- | --- |
| `AppEvent::InsertLocalImage` | [code:app-event] |
| Dispatch app events | [code:event-dispatch] |
| `ChatWidget::on_view_image_tool_call` | [code:tool-lifecycle] |
| `LocalImageHistoryCell` | [code:local-image-cell] |
| Фича image preview | [feature:tui-history-image-previews] |

## Объем работ

- Найти первый контролируемый вызывающий код для локального файла изображения.
- Подключить вызывающий код к `AppEvent::InsertLocalImage { path, caption }`.
- Сохранить запрет на auto-render из Markdown/plain text.
- Зафиксировать caption/error contract для raw/copy и warning fallback.
- Добавить точечные тесты на выбранный вызывающий код.

## Вне объема работ

- Не парсить `![alt](path)` из Markdown как terminal image.
- Не скачивать remote URLs.
- Не менять resize/reflow/replay/resume behavior.
- Не добавлять managed storage для оригинальных изображений.
- Не подключать `ImageGeneration.saved_path` в этом этапе.

## План действий

- [x] Выбрать первый вызывающий код: `ThreadItem::ImageView` / `view_image`.
- [x] Проверить, что вызывающий код передает только локальные файлы, созданные
  контролируемым workflow.
- [x] Отправлять `AppEvent::InsertLocalImage { path, caption }` вместо
  текстового path-only сообщения там, где нужен preview.
- [x] Добавить тесты на happy path выбранного вызывающего кода.
- [x] Оставить invalid/fallback validation на app-layer тестах stage 002, потому
  что `view_image` core path уже валидирует файл до `ThreadItem::ImageView`.
- [x] Обновить архитектурную feature card и этот stage по фактическому source.

## Критерии готовности

- Есть вызывающий production-код, который создает `AppEvent::InsertLocalImage`.
- Caller не читает paths из model text, Markdown или shell output.
- Validation из stage 002 остается единственной app-layer границей создания
  `LocalImageHistoryCell`.
- Тесты подтверждают happy path и fallback path для выбранного вызывающего кода.

## Открытые вопросы

| ID | Вопрос | Статус | Решение / итог | Детали |
| --- | --- | --- | --- | --- |
| [Q-001] | Какой вызывающий код подключить первым? | `decided` | `ThreadItem::ImageView` / `view_image` | [details][Q-001] |
| [Q-002] | Как формировать caption? | `decided` | `display_path_for(path, cwd)`, например `example.png` | [details][Q-002] |

### Q-001: Первый вызывающий код

Статус: `decided`.

Решение: первым подключен `ThreadItem::ImageView`. Core `view_image` handler
получает path как аргумент tool call, резолвит его относительно cwd, проверяет,
что это файл, читает и обрабатывает image bytes, а затем публикует typed
`ImageView` item с `AbsolutePathBuf`. TUI больше не создает legacy text cell
для этого item, а отправляет `AppEvent::InsertLocalImage`.

### Q-002: Caption

Статус: `decided`.

Решение: caption для `view_image` строится через `display_path_for(path, cwd)`,
чтобы fallback был коротким и понятным: `[Image: example.png]` для файла внутри
cwd или относительный/home-relative path для других локальных файлов. Caption
не читается из Markdown, shell output или model text.

## Проверки

- `cargo test -p codex-tui view_image_tool_call_emits_local_image_event` -
  passed.
- App-layer fallback/validation остается покрыт stage 002 проверками:
  `cargo test -p codex-tui local_image`.
- Runner-проверка: `cargo test -p codex-tui local_image` - passed, 14 tests.
- Runner-проверка: `git diff --check` - passed.

[code:app-event]: ../../../../codex-rs/tui/src/app_event.rs
[code:event-dispatch]: ../../../../codex-rs/tui/src/app/event_dispatch.rs
[code:local-image-cell]: ../../../../codex-rs/tui/src/history_cell/local_image.rs
[code:tool-lifecycle]: ../../../../codex-rs/tui/src/chatwidget/tool_lifecycle.rs
[details:actions]: #план-действий
[details:architecture]: #связанные-элементы-архитектуры
[details:criteria]: #критерии-готовности
[details:questions]: #открытые-вопросы
[details:scope]: #объем-работ
[details:verification]: #проверки
[feature:tui-history-image-previews]: ../../../architecture/features/tui-history-image-previews.md
[Q-001]: #q-001-первый-вызывающий-код
[Q-002]: #q-002-caption
[stage:PLAN-TUI-ASSISTANT-IMAGES-001:004]: 004-wire-image-generation-saved-path.md
