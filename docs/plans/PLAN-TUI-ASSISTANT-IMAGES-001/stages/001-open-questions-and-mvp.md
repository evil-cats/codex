# 001: открытые вопросы и граница MVP

## Статус

выполнен (`completed`)

## Кратко

| Поле | Значение |
| --- | --- |
| Цель этапа | Зафиксировать границы MVP для assistant/tool local image previews до реализации. |
| Уже сделано / решено | 10 вопросов решены (`decided`); выбран controlled structured path, отдельный `LocalImageHistoryCell`, fallback и validation policy. |
| Открыто / отложено / не сделано | 2 вопроса отложены (`deferred`): resize/reflow и resume bitmap replay. |
| Следующий шаг | Реализовать [stage:PLAN-TUI-ASSISTANT-IMAGES-001:002]. |
| Детали | [details:architecture], [details:scope], [details:mvp], [details:questions], [details:follow-ups] |

## Зачем этап нужен

Текущий код уже умеет вставлять локальные изображения в terminal scrollback,
но только когда `HistoryCell` отдаёт `HistoryCellDisplayItem::LocalImage`.
Для ассистентских картинок ещё не определены source, ownership, fallback,
replay и границы безопасности.

Этот этап нужен, чтобы перед реализацией выбрать минимальный путь и не смешать
три разные задачи: rendering primitive, ассистентский output source и
полноценный replay/resume behavior.

## Связанные элементы архитектуры

| Элемент | Ссылка |
| --- | --- |
| `HistoryCellDisplayItem::LocalImage` | [code:history-cell] |
| `App::prepare_history_insert_items` | [code:resize-reflow] |
| `HistoryInsertItem::Image` | [code:insert-history] |
| `pets::prepare_history_image` | [code:pets-mod] |
| `AppEvent::InsertHistoryCell` | [code:app-event] |

## Объем работ

- Зафиксировать, каким source-backed cell ассистентские изображения попадут в transcript.
- Зафиксировать, какие источники локальных путей считаются trusted.
- Выбрать fallback text для unsupported terminals, raw mode, transcript overlay и copy.
- Решить, входит ли bitmap re-emission при resize/reflow/resume в MVP.
- Описать проверки, которые должны сопровождать реализацию.

## Вне объема работ

- Не реализовывать network fetch по Markdown image URLs.
- Не включать terminal images в `tmux`/`zellij` без отдельного решения по pane-local safety.
- Не парсить произвольные пути из ассистентского Markdown как изображения.
- Не добавлять upstream/product пользовательскую документацию в `docs/`; для
  этого форка разрешена внутренняя dev-документация в `docs/architecture/`,
  `docs/plans/`, `docs/follow-ups/` и `docs/backlog/`.

## План действий

- [x] Выбрать source model: отдельный `LocalImageHistoryCell`,
  расширение `AgentMarkdownCell` или отдельный app event.
- [x] Выбрать trusted image source policy.
- [x] Выбрать fallback text и raw/copy behavior.
- [x] Выбрать MVP для resize/reflow/resume.
- [x] После решений завести этап 002 с конкретными файлами и тестами.

## Критерии готовности

- Для каждого открытого вопроса ниже выбран статус решено (`decided`) или
  явно отложено (`deferred`).
- MVP можно реализовать без чтения произвольных локальных путей из model text.
- План содержит минимум один проверяемый happy path и один fallback/error path.
- Следующий этап можно оценить по конкретным файлам и тестам.

## Граница MVP

- Добавить `LocalImageHistoryCell` для ассистентских и tool-generated
  локальных изображений.
- Добавить structured event `AppEvent::InsertLocalImage { path, caption }`.
- Handler этого event валидирует `path` и только после этого создаёт
  `LocalImageHistoryCell` или warning/fallback cell.
- Callers не должны напрямую создавать `LocalImageHistoryCell` в обход
  validation path.
- `LocalImageHistoryCell` всегда рендерит fallback line; в `Rich` добавляет
  `HistoryCellDisplayItem::LocalImage(path)`, в `Raw` оставляет только fallback.
- Первый вертикальный срез не меняет Markdown parser, remote URL handling,
  resize/reflow, resume и текущий отказ для `tmux`/`zellij`.

## Открытые вопросы

| ID | Вопрос | Статус | Итог | Готовность / связь |
| --- | --- | --- | --- | --- |
| [Q-001] | Как ассистентский ответ должен порождать image item? | решено (`decided`) | Controlled structured path через `LocalImageHistoryCell` / `AppEvent`; Markdown не source. | Готово для stage 002 |
| [Q-002] | Переиспользовать user attachments или добавить новый cell? | решено (`decided`) | Добавить отдельный `LocalImageHistoryCell`; не переиспользовать `UserHistoryCell.local_image_paths`. | Готово для stage 002 |
| [Q-003] | Можно ли автоматически превращать `![alt](path)` в preview? | решено (`decided`) | Нет, Markdown image syntax остается текстом/ссылкой. | Regression-тест в этапе 002 |
| [Q-004] | Какие local paths считать trusted? | решено (`decided`) | Только controlled structured event; перед rendering проверить regular file и decode. | Validation в stage 002 |
| [Q-005] | Что показывать при unsupported terminal или decode/cache error? | решено (`decided`) | Всегда fallback; invalid event создает warning/fallback cell, bitmap не вставляется. | Fallback/error tests |
| [Q-006] | Должны ли картинки переэмититься при resize/reflow? | отложено (`deferred`) | В MVP достаточно fallback после resize/reflow. | [follow-up:FU-2026-001] |
| [Q-007] | Должны ли картинки восстанавливаться после resume? | отложено (`deferred`) | Bitmap resume требует managed artifact ownership; пока достаточно fallback. | [follow-up:FU-2026-001] |
| [Q-008] | Где хранить preview/cache asset? | решено (`decided`) | Новое хранилище оригиналов не вводим; derived previews остаются в текущем cache. | Без нового storage в MVP |
| [Q-009] | Что делать с `tmux`/`zellij`? | решено (`decided`) | В MVP fallback-only через текущий `/pets` detection. | Ручная fallback-проверка |
| [Q-010] | Нужна ли поддержка remote URLs? | решено (`decided`) | Не входит в MVP; нужен отдельный controlled download/artifact workflow. | Out of scope |
| [Q-011] | Какой размер preview использовать? | решено (`decided`) | Переиспользовать текущую geometry: 12 rows, `x = 2`, `max_columns = width - 4`. | Настройки позже |
| [Q-012] | Какие тесты обязательны? | решено (`decided`) | Unit, validation, Markdown/plain-text regression, focused insertion и ручная Kitty-проверка. | Criteria этапа 002 |

[Q-001]: #q-001-источник-image-item-ассистента
[Q-002]: #q-002-localimagehistorycell
[Q-003]: #q-003-синтаксис-markdown-для-изображений
[Q-004]: #q-004-доверенные-local-paths
[Q-005]: #q-005-fallback-и-ошибки
[Q-006]: #q-006-resize-reflow
[Q-007]: #q-007-resume
[Q-008]: #q-008-preview-cache-asset
[Q-009]: #q-009-tmux-zellij
[Q-010]: #q-010-remote-urls
[Q-011]: #q-011-размер-preview
[Q-012]: #q-012-обязательные-тесты

### Q-001: Источник image item ассистента

Вопрос: как ассистентский ответ должен порождать image item?

Статус: решено (`decided`).

Preview изображений от ассистента создаются только через контролируемый
структурированный путь, на первом этапе через отдельный
`LocalImageHistoryCell` / `AppEvent`. Markdown-синтаксис изображений в ответах
ассистента не считается разрешением читать или рендерить локальные файлы.

### Q-002: LocalImageHistoryCell

Вопрос: переиспользовать user attachments или добавить новый cell?

Статус: решено (`decided`).

Добавить отдельный `LocalImageHistoryCell` для ассистентских и tool-generated
локальных изображений. Не переиспользовать `UserHistoryCell.local_image_paths`:
user attachments и assistant/tool image outputs имеют разные ownership,
fallback, trust boundary и future replay semantics.

### Q-003: Синтаксис Markdown для изображений

Вопрос: можно ли автоматически превращать `![alt](path)` в preview?

Статус: решено (`decided`).

Нет. TUI не превращает Markdown image syntax из ассистентского текста в
terminal image preview. Локальные пути в Markdown остаются текстом/ссылками;
для bitmap preview нужен отдельный controlled structured path.

### Q-004: Доверенные local paths

Вопрос: какие local paths считать trusted?

Статус: решено (`decided`).

В MVP trusted source только structured event из контролируемого Codex/TUI
workflow. Paths из ассистентского Markdown/plain text, shell output и remote
content не считаются trusted. Перед rendering файл всё равно должен
существовать, быть regular file и успешно декодироваться через `image` crate.

### Q-005: Fallback и ошибки

Вопрос: что показывать при unsupported terminal или decode/cache error?

Статус: решено (`decided`).

`LocalImageHistoryCell` всегда рендерит текстовый fallback. Bitmap payload
является best-effort enhancement. Unsupported terminal или failure в
`prepare_history_image` не ломает историю и логируется debug-level. Если
structured event указывает на несуществующий или недекодируемый файл, TUI
создаёт warning/fallback cell, а не пытается вставить image payload.

### Q-006: Resize reflow

Вопрос: должны ли картинки переэмититься при resize/reflow?

Статус: отложено (`deferred`).

Отложить до момента, когда базовый показ ассистентской картинки заработает.
В первом вертикальном срезе достаточно текстового fallback после
resize/reflow; item-oriented reflow вынести в `FU-2026-001`.

### Q-007: Resume

Вопрос: должны ли картинки восстанавливаться после resume?

Статус: отложено (`deferred`).

Отложить до момента, когда базовый показ ассистентской картинки заработает.
Полноценное восстановление bitmap после resume требует отдельного решения по
managed artifact ownership. Пока достаточно fallback, если source path
недоступен.

### Q-008: Preview cache asset

Вопрос: где хранить preview/cache asset?

Статус: решено (`decided`).

В первом вертикальном срезе не вводить новое managed хранилище оригинальных
изображений. Structured event передаёт source path, `LocalImageHistoryCell`
хранит source path и fallback, а derived previews продолжают использовать
существующий `CODEX_HOME/cache/tui-history-images`.

### Q-009: Tmux zellij

Вопрос: что делать с `tmux`/`zellij`?

Статус: решено (`decided`).

В MVP не поддерживать terminal image previews внутри `tmux`/`zellij`.
Сохраняем текущий отказ из `/pets` detection и показываем только fallback text.
Поддержку мультиплексоров рассматривать отдельно после ручной проверки
pane-local behavior, scrollback, resize и cleanup.

### Q-010: Remote URLs

Вопрос: нужна ли поддержка remote URLs?

Статус: решено (`decided`).

Remote URL rendering не входит в этот MVP. TUI не скачивает remote images из
Markdown/plain text; remote URLs остаются текстом/ссылками. Если позже нужен
preview remote image, он должен идти через отдельный controlled
download/artifact workflow с network approval, MIME validation, size limits и
managed cache.

### Q-011: Размер preview

Вопрос: какой размер preview использовать?

Статус: решено (`decided`).

В MVP переиспользовать текущую geometry history previews: target 12 rows,
`x = 2`, `max_columns = width - 4`, aspect ratio через
`TERMINAL_CELL_WIDTH_TO_HEIGHT`. Настройки размера, compact mode и
caption-aware layout вынести в follow-up после работающего вертикального среза.

### Q-012: Обязательные тесты

Вопрос: какие тесты обязательны?

Статус: решено (`decided`).

MVP должен иметь unit tests для `LocalImageHistoryCell`, validation tests для
structured event, regression tests против Markdown/plain-text auto-rendering и
focused insertion tests. Ручная проверка в Kitty-compatible terminal
обязательна перед завершением фичи; `tmux`/`zellij` проверяются на
fallback-only behavior.

## Найденные отложенные работы

- [follow-up:FU-2026-001]: item-oriented resize/reflow и replay для bitmap previews после
  того, как normal insertion path стабилизирован.
- [follow-up:FU-2026-002]: source-backed assistant/tool image source path без Markdown
  auto-rendering и без произвольного чтения локальных путей из текста модели.

[code:app-event]: ../../../../codex-rs/tui/src/app_event.rs
[code:history-cell]: ../../../../codex-rs/tui/src/history_cell/mod.rs
[code:insert-history]: ../../../../codex-rs/tui/src/insert_history.rs
[code:pets-mod]: ../../../../codex-rs/tui/src/pets/mod.rs
[code:resize-reflow]: ../../../../codex-rs/tui/src/app/resize_reflow.rs
[details:architecture]: #связанные-элементы-архитектуры
[details:follow-ups]: #найденные-отложенные-работы
[details:mvp]: #граница-mvp
[details:questions]: #открытые-вопросы
[details:scope]: #объем-работ
[follow-up:FU-2026-001]: ../../../follow-ups/FU-2026-001-tui-history-image-reflow-reemit.md
[follow-up:FU-2026-002]: ../../../follow-ups/FU-2026-002-tui-assistant-tool-image-source.md
[stage:PLAN-TUI-ASSISTANT-IMAGES-001:002]: 002-local-image-history-cell.md
