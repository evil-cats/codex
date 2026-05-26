# 005: replay/resize и проверка terminal previews

## Статус

`completed`

## Кратко

| Поле | Значение |
| --- | --- |
| Цель этапа | Решить, должны ли bitmap-превью переэмититься при resize/reflow, initial replay и resume, и покрыть выбранное поведение тестами/ручной проверкой. |
| Уже сделано / решено | Выбран и реализован item-level replay: resize reflow, initial replay, thread-switch tail replay и overlay-deferred paths сохраняют `HistoryCellDisplayItem::LocalImage` до `prepare_history_insert_items`. |
| Открыто / отложено / не сделано | Managed storage оригинальных изображений после resume не вводился; если исходный файл недоступен, остается текстовый fallback. При экстремально узком окне, меньшем preview, bitmap может не поместиться; это ожидаемое геометрическое ограничение. |
| Следующий шаг | План готов к архивированию после проверки относительных ссылок при переносе каталога в `docs/plans/archive/2026/`. |
| Детали | [details:architecture], [details:scope], [details:actions], [details:criteria], [details:questions], [details:follow-ups], [details:verification] |

## Зачем этап нужен

Предыдущие этапы подключили controlled image sources к обычной вставке в
history scrollback. Этот этап закрепил bitmap replay как часть контракта для
resize/reflow/replay при условии, что исходный файл еще доступен.

## Связанные элементы архитектуры

| Элемент | Ссылка |
| --- | --- |
| Replay при resize/reflow | [code:resize-reflow] |
| Вставка mixed history items | [code:tui-insert-history] |
| Подготовка terminal image | [code:pets-mod] |
| Фича image preview | [feature:tui-history-image-previews] |
| Follow-up replay/reflow | [follow-up:FU-2026-001] |

## Объем работ

- Проверить текущие buffers и paths для resize reflow, initial replay и
  overlay-deferred history.
- Решить, остается ли replay fallback-only или должен перейти на
  `HistoryCellDisplayItem` / `HistoryInsertItem` level.
- Провести local image markers через нужные replay paths без нарушения
  строкового fallback.
- Добавить точечные тесты на выбранное поведение.
- Зафиксировать ручную проверку в Kitty-compatible terminal по пользовательскому
  smoke.

## Вне объема работ

- Не добавлять новые image sources.
- Не менять `view_image` и `ImageGeneration.saved_path` contracts, закрытые в
  stage 003/004.
- Не вводить managed storage оригинальных изображений после resume.

## План действий

- [x] Разобрать [follow-up:FU-2026-001] и текущие replay/reflow paths.
- [x] Выбрать fallback-only или item-level bitmap replay.
- [x] Реализовать item-level replay для resize/reflow/replay paths.
- [x] Добавить тесты на resize/reflow/replay behavior.
- [x] Обновить feature card, follow-up и план по фактическому решению.

## Критерии готовности

- Поведение bitmap previews при resize/reflow/replay описано как явный
  контракт, а не как случайный side effect.
- Текстовый fallback остается доступен для Raw/copy и unsupported terminals.
- Тесты покрывают выбранное поведение.
- Результат ручной terminal-проверки зафиксирован или невозможность проверки
  явно объяснена.

## Открытые вопросы

| ID | Вопрос | Статус | Решение / итог | Детали |
| --- | --- | --- | --- | --- |
| [Q-001] | Нужен ли item-level bitmap replay? | `decided` | Да: replay/reflow paths должны сохранять local image marker до финальной подготовки terminal image. | [details][Q-001] |
| [Q-002] | Нужно ли managed ownership оригиналов после resume? | `decided` | Нет в этом плане: source path должен оставаться доступным; при недоступном файле остается fallback. | [details][Q-002] |

### Q-001: Item-level bitmap replay

Статус: `decided`.

Решение: replay/reflow должен быть item-level. `App::render_transcript_lines_for_reflow`
теперь возвращает display-items, `InitialHistoryReplayBuffer` хранит display-items,
а финальная запись в scrollback проходит через `prepare_history_insert_items`.
Это сохраняет bitmap marker для обычного resize reflow, initial replay,
thread-switch tail replay и overlay-deferred paths.

Строковый fallback остается рядом с marker и используется для Raw/copy,
unsupported terminals и ошибок подготовки asset.

### Q-002: Managed ownership после resume

Статус: `decided`.

Решение: managed storage оригинальных изображений не входит в этот план.
`CODEX_HOME/cache/tui-history-images` содержит derived preview assets, а не
managed копию исходника. Если после resume исходный path уже недоступен,
bitmap payload не готовится, но текстовый fallback остается частью history cell.

## Найденные отложенные работы

- [follow-up:FU-2026-001] закрыт как реализованный: bitmap replay теперь идет
  через item-level path.

## Проверки

- `env RUST_MIN_STACK=8388608 cargo test -p codex-tui resize_reflow` - passed.
- `env RUST_MIN_STACK=8388608 cargo test -p codex-tui insert_history_items_with_wrap_policy_counts_image_rows` - passed.
- `env RUST_MIN_STACK=8388608 cargo test -p codex-tui image_generation_call` - passed.
- Manual Kitty smoke - passed по пользовательской проверке: direct Kitty
  graphics command показал PNG, свежий TUI показал картинку через `view_image`,
  а preview пережил resize.
- Extreme shrink note: при очень жестком resize, когда окно меньше картинки,
  preview может не поместиться. Это не ломает fallback и считается ожидаемым
  ограничением текущей геометрии preview.

[Q-001]: #q-001-item-level-bitmap-replay
[Q-002]: #q-002-managed-ownership-после-resume
[code:pets-mod]: ../../../../../../codex-rs/tui/src/pets/mod.rs
[code:resize-reflow]: ../../../../../../codex-rs/tui/src/app/resize_reflow.rs
[code:tui-insert-history]: ../../../../../../codex-rs/tui/src/tui.rs
[details:actions]: #план-действий
[details:architecture]: #связанные-элементы-архитектуры
[details:criteria]: #критерии-готовности
[details:follow-ups]: #найденные-отложенные-работы
[details:questions]: #открытые-вопросы
[details:scope]: #объем-работ
[details:verification]: #проверки
[feature:tui-history-image-previews]: ../../../../../architecture/features/tui-history-image-previews.md
[follow-up:FU-2026-001]: ../../../../../follow-ups/archive/2026/FU-2026-001-tui-history-image-reflow-reemit.md
