---
id: fork-tui-history-image-previews
status: active
created: 2026-06-08
updated: 2026-08-13
---

# TUI history image previews

## Обзор

Эта карточка фиксирует крупную fork-доработку Hermione: TUI history умеет
показывать локальные изображения как terminal previews с текстовым fallback.

Карточка предназначена именно для переноса fork patch на новый upstream: она
сохраняет полный feature chain, кодовые границы, config/API surface, проверки,
ограничения и известные риски.

## Зачем это нужно

До доработки изображения в TUI history оставались текстовыми ссылками или
fallback labels. Hermione fork добавляет rich terminal preview для локальных
изображений, но сохраняет читаемый текст там, где bitmap невозможен.

Поддерживаемые источники:

- пользовательские local image attachments;
- `view_image`;
- `ImageGeneration.saved_path`;
- replay/reflow paths, включая resize reflow, initial replay,
  thread-switch tail replay и overlay-deferred history.

Fallback остается рядом:

- `[Image #n]` для пользовательских вложений;
- `[Image]`;
- `[Image: <caption>]`.

## Карта файлов

### Ячейки истории TUI и слой текстовых строк

| Файл | Роль |
| --- | --- |
| `codex-rs/tui/src/history_cell/mod.rs` | `HistoryCellDisplayItem::Line(HyperlinkLine)`, `HistoryCellDisplayItem::LocalImage`, `display_items_for_mode`, вспомогательные функции преобразования |
| `codex-rs/tui/src/history_cell/messages.rs` | `UserHistoryCell.local_image_paths`, fallback labels и default `ImagePreviewSize::Normal` |
| `codex-rs/tui/src/history_cell/local_image.rs` | `LocalImageHistoryCell` для controlled assistant/tool images |
| `codex-rs/tui/src/history_cell/tests.rs` | Tests for marker emission, Raw/Rich behavior, Markdown non-trust |
| `codex-rs/tui/src/terminal_hyperlinks.rs` | `HyperlinkLine` хранит видимый `Line` и метаданные терминальных ссылок отдельно, чтобы байты OSC 8 не влияли на геометрию |

### TUI event and callers

| Файл | Роль |
| --- | --- |
| `codex-rs/tui/src/app_event.rs` | `AppEvent::InsertLocalImage { path, caption, preview_size }` |
| `codex-rs/tui/src/app/event_dispatch.rs` | Validation: regular file и decode через `image` crate |
| `codex-rs/tui/src/chatwidget/tool_lifecycle.rs` | `on_view_image_tool_call`, `on_image_generation_end`, `insert_local_image_history` |
| `codex-rs/tui/src/chatwidget/replay.rs` | Replay `ThreadItem::ImageView` back into `on_view_image_tool_call` |
| `codex-rs/tui/src/session_log.rs` | Сохранение image generation event details, если path есть |

### TUI terminal insertion

| Файл | Роль |
| --- | --- |
| `codex-rs/tui/src/app/resize_reflow.rs` | `App::prepare_history_insert_items`, rows/config/protocol resolution |
| `codex-rs/tui/src/app/resize_reflow_tests.rs` | Проверки upstream-контрактов ограничения строк, уведомления о пагинации и initial replay для типизированных display items |
| `codex-rs/tui/src/insert_history.rs` | `HistoryInsertItem::Line(HyperlinkLine)`, `HistoryInsertItem::Image`, подсчёт строк, граница записи в терминал |
| `codex-rs/tui/src/tui.rs` | `insert_history_items_with_wrap_policy` boundary |
| `codex-rs/tui/src/custom_terminal.rs` | Kitty history image tracking and cleanup |

### Terminal image protocol

| Файл | Роль |
| --- | --- |
| `codex-rs/tui/src/pets/mod.rs` | `prepare_history_image`, size calculation, cache path, protocol branching |
| `codex-rs/tui/src/pets/image_protocol.rs` | PNG/Sixel frames, Kitty virtual placement commands |
| `codex-rs/tui/src/kitty_placeholder.rs` | `U+10EEEE` placeholder grid for Kitty scrollback anchoring |

### Protocol, tool and config surface

| Файл | Роль |
| --- | --- |
| `codex-rs/protocol/src/items.rs` | `ImagePreviewSize`, `ImageViewItem.path` как `PathUri`, `ImageViewItem.preview_size`, `ImageGenerationItem.saved_path` |
| `codex-rs/protocol/src/protocol.rs` | Legacy `ViewImageToolCallEvent.preview_size` |
| `codex-rs/core/src/tools/handlers/view_image.rs` | Parses `preview_size`, rejects invalid values |
| `codex-rs/core/src/tools/handlers/view_image_spec.rs` | Exposes `preview_size`, not `preview_rows` |
| `codex-rs/config/src/types.rs` | `[tui.history_image_preview]` defaults |
| `codex-rs/core/src/config/mod.rs` | Runtime `HistoryImagePreviewConfig::rows_for` |
| `codex-rs/core/config.schema.json` | Config schema |
| `codex-rs/app-server-protocol/src/protocol/v2/item.rs` | App-server v2 `ThreadItem::ImageView.path` как `LegacyAppPathString`, `previewSize` |
| `codex-rs/app-server-protocol/schema/typescript/v2/ThreadItem.ts` | Generated TS union для `ThreadItem::ImageView.path` и `previewSize` |
| `codex-rs/app-server-protocol/schema/typescript/ImagePreviewSize.ts` | Generated TS enum |

## Итоговый контракт

### Rendering contract

1. `HistoryCell::display_items_for_mode(width, HistoryRenderMode::Rich)` может
   возвращать `HistoryCellDisplayItem::LocalImage { path, preview_size }`.
2. Bitmap marker всегда идёт рядом с текстовой fallback line.
3. `HistoryRenderMode::Raw` остается line-only.
4. Произвольный Markdown/plain text не должен создавать `LocalImage`.
5. Raw terminal image payload не хранится в ratatui `Line`.
6. Terminal payload создаётся только в `App::prepare_history_insert_items`.
7. If terminal protocol unsupported или asset preparation fails, marker
   пропускается best-effort, fallback остается.
8. После миграции upstream на `HyperlinkLine` обычные строки терминальной
   истории должны сохранять путь `HistoryCellDisplayItem::Line(HyperlinkLine)`
   -> `HistoryInsertItem::Line(HyperlinkLine)`.
9. Не возвращать пути scrollback и reflow к старому `Line<'static>` как к
   основному типу строк. `HistoryCellDisplayItem::line()` допустим только для
   потребителей обычных видимых строк и намеренно отбрасывает метаданные
   ссылок.
10. Уведомление upstream о неполной или пагинированной истории терминала
    добавляется как `HistoryCellDisplayItem::Line`; оно не должно понижать
    соседние маркеры изображений до `HyperlinkLine` или обычного текста.

### Source contract

1. Пользовательские local attachments входят через `UserHistoryCell.local_image_paths`.
2. Assistant/tool-generated images входят только через trusted structured path:
   `AppEvent::InsertLocalImage`.
3. `view_image` создаёт `ThreadItem::ImageView`, а TUI caller отправляет
   `InsertLocalImage`.
4. `ImageGeneration.saved_path` является controlled source, если path есть.
5. Если `ImageGeneration.saved_path` отсутствует, TUI оставляет text-only
   `Generated Image` history cell.

### Preview size contract

1. `ImagePreviewSize` имеет значения `Small`, `Normal`, `Large`, serializes as
   `small`, `normal`, `large`.
2. Default: `normal`.
3. `view_image.preview_size` принимает только `small`, `normal`, `large`.
4. Missing `preview_size` равен `normal`.
5. Invalid value returns model-visible error:

   ```text
   view_image.preview_size only supports `small`, `normal`, or `large`; omit `preview_size` for default normal preview, got `<value>`
   ```

6. Numeric `preview_rows` не входит в model-visible tool API.
7. `[tui.history_image_preview]` задает row counts:
   - `small_rows = 8`;
   - `normal_rows = 12`;
   - `large_rows = 20`.
8. Runtime `HistoryImagePreviewConfig::rows_for` clamps to at least `1`.

### Kitty contract

1. Kitty and KittyLocalFile history previews normalize source into PNG cache
   under `CODEX_HOME/cache/tui-history-images`.
2. Kitty payload uses PNG format `f=100`.
3. History image uses virtual placement `U=1`, not floating screen placement.
4. Scrollback anchor is text grid of Unicode placeholder cells `U+10EEEE` with
   row/column diacritics.
5. `CustomTerminal` tracks history rows and deletes Kitty image ids after the
   anchored image scrolls out of visible history.
6. Sixel path remains bytes payload.

## Архитектурное решение

History cells возвращают типизированные display items и не хранят terminal
bytes в `Line`. `App` преобразует только доверенные `LocalImage` markers на
границе вставки истории, protocol modules владеют кодированием и Kitty
placement, а `CustomTerminal` — очисткой terminal resources. Config и protocol
surfaces передают только ограниченный enum размера preview.

## Порядок повторения при переносе

### 1. Ввести typed display item

В `history_cell/mod.rs` расширить display item enum:

```rust
HistoryCellDisplayItem::LocalImage {
    path: PathBuf,
    preview_size: ImagePreviewSize,
}
```

Все code paths, которым нужны plain lines, должны явно фильтровать markers.
Для этого используется вспомогательная функция:

```rust
pub(crate) fn line(self) -> Option<Line<'static>>
```

Важно: эта функция не является общим мостом миграции для терминальной истории.
Она нужна только потребителям обычных видимых строк. Пути scrollback,
resize reflow, initial replay, thread-switch tail replay и overlay-deferred
history должны переносить обычные строки как `HyperlinkLine`, чтобы не потерять
метаданные терминальных ссылок.

### 2. Добавить fallback plus marker в user attachments

В `UserHistoryCell` оставить fallback `[Image #n]`, а в Rich mode добавить
`LocalImage { path, preview_size: ImagePreviewSize::Normal }`.

### 3. Добавить controlled local image cell

Создать `LocalImageHistoryCell`:

- fields: `path`, `caption`, `preview_size`;
- fallback:
  - no caption -> `[Image]`;
  - caption -> `[Image: <caption>]`;
- Rich mode -> fallback line plus `LocalImage`;
- Raw mode -> fallback only.

### 4. Добавить `AppEvent::InsertLocalImage`

Event fields:

```rust
InsertLocalImage {
    path: PathBuf,
    caption: Option<String>,
    preview_size: ImagePreviewSize,
}
```

В dispatch:

- проверить, что path is regular file;
- проверить, что `image` crate can decode;
- on success создать `LocalImageHistoryCell`;
- on failure добавить warning/fallback, не создавать bitmap marker.

### 5. Подключить production callers

`ChatWidget::on_view_image_tool_call`:

- получает path и `preview_size`;
- строит caption через display path относительно cwd;
- отправляет `AppEvent::InsertLocalImage`.

`ChatWidget::on_image_generation_end`:

- если `saved_path` есть, отправляет `InsertLocalImage`;
- caption из непустого `revised_prompt`, иначе из `call_id`;
- если path нет, сохраняет text-only cell.

### 6. Сохранить markers through replay/reflow

Resize reflow, initial replay, thread-switch tail replay и overlay-deferred
history должны работать на `HistoryCellDisplayItem`, а не терять image marker
при ранней конвертации в `Line`.

При совмещении с ограничением строк и пагинацией upstream нужно сохранять
`InitialHistoryReplayBuffer.retained_items`, `was_truncated`, уведомление о
неполном transcript и запрос дозагрузки старой истории. Уведомление вставляется
в типизированную последовательность элементов до `prepare_history_insert_items`,
поэтому следующие за ним маркеры `LocalImage` остаются типизированными.

`ChatWidget` replay должен восстанавливать `ThreadItem::ImageView` через тот же
`on_view_image_tool_call` path.

### 7. Подготовить terminal payload at boundary

В `App::prepare_history_insert_items`:

- пройти по display items;
- `HistoryCellDisplayItem::Line(HyperlinkLine)` превращать в
  `HistoryInsertItem::Line(HyperlinkLine)` без промежуточного понижения до
  `Line<'static>`;
- `LocalImage` обрабатывать best-effort:
  - detect terminal image support;
  - взять target rows через `config.history_image_preview.rows_for(preview_size)`;
  - ограничить columns доступной шириной терминала с учётом protocol padding;
  - вызвать `pets::prepare_history_image`;
  - при success создать `HistoryInsertItem::Image`;
  - при error логировать и пропустить marker.

### 8. Реализовать Kitty anchoring

В `pets/image_protocol.rs`:

- `png_frame` converts image to PNG preview cache;
- `kitty_transmit_png_with_virtual_placement`;
- `kitty_transmit_png_file_with_virtual_placement`.

Команда должна передавать image data и создавать virtual placement `U=1`.

В `kitty_placeholder.rs` печатать `U+10EEEE` grid как реальные scrollback text
rows. Cursor movement внутри одной строки недостаточен: Kitty покажет только
одну полосу image.

В `custom_terminal.rs` хранить image id и удалять его, когда anchored rows ушли
из visible history.

### 9. Добавить protocol and config plumbing

- `ImagePreviewSize` в protocol items;
- `ImageViewItem.preview_size`;
- legacy `ViewImageToolCallEvent.preview_size`, если legacy event ещё есть;
- app-server v2 `previewSize`;
- generated TS `ImagePreviewSize.ts`;
- JSON schema fixtures;
- `[tui.history_image_preview]` config;
- `HistoryImagePreviewConfig::rows_for`.

При разрешении upstream-конфликтов сохранять актуальные upstream-типы и
накладывать только fork-поля и typed markers:

- путь `ImageView` остаётся в актуальной upstream-модели
  `PathUri`/`LegacyAppPathString`; fork добавляет `preview_size`, но не возвращает
  устаревшие feature branches;
- `TurnItem::ImageView`, legacy event, app-server v2, generated TypeScript и TUI
  replay передают один `ImagePreviewSize` без числового `preview_rows`;
- generated `ThreadItem.ts` сохраняет все upstream variants, а вариант
  `imageView` дополнительно содержит `previewSize`;
- `InitialHistoryReplayBuffer`, `ReflowRenderResult` и terminal insertion
  хранят `items`, а не пониженные `lines`; обычные строки остаются
  `HyperlinkLine`, изображения — отдельным `HistoryInsertItem::Image`;
- `TerminalWidth` из текущего upstream, уведомление об усечении и дозагрузка
  пагинированной истории сохраняются вокруг типизированных `items`, а
  `resize_reflow_tests.rs` проверяет тот же контракт через
  `items`/`retained_items`;
- `CustomTerminal` инициализирует и очищает Kitty bindings вместе с остальным
  состоянием истории;
- конфликты соседних lifecycle features не включаются в эту карточку только из-за
  общего файла.

## Проверки

Исполняемая карта card-level regression tests:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "trusted ImageView path и model-visible результат view_image",
      "argv": ["just", "test", "-p", "codex-core", "view_image"]
    },
    {
      "purpose": "rich image history, replay, reflow и terminal insertion в TUI",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-tui",
        "--",
        "--skip",
        "ide_context::ipc::tests::fetch_ide_context_uses_unregistered_request_route"
      ]
    },
    {
      "purpose": "app-server wire contract для image preview items и previewSize",
      "argv": ["just", "test", "-p", "codex-app-server-protocol"]
    },
    {
      "purpose": "protocol types для preview size и сохранённого image path",
      "argv": ["just", "test", "-p", "codex-protocol"]
    },
    {
      "purpose": "отсутствие непреднамеренных изменений TUI snapshots",
      "argv": [
        "cargo",
        "insta",
        "pending-snapshots",
        "--manifest-path",
        "codex-rs/tui/Cargo.toml"
      ]
    }
  ]
}
```

Дополнительно обязателен `fork generators`, если перенос меняет `ImagePreviewSize`, `[tui.history_image_preview]` или app-server `previewSize`.

`manual-required`: в Kitty нужно проверить показ PNG, `view_image` в истории, сохранение preview после resize и text fallback при слишком узком окне.

## Риски и ограничения

### Ограничения

- Managed ownership original images after resume intentionally not implemented.
  Если source path disappears, bitmap payload cannot be regenerated; fallback
  remains.
- Unsupported terminal protocol drops bitmap marker at
  `prepare_history_insert_items`; fallback remains.
- `tmux` and `zellij` currently work only through fallback under current
  `detect_pet_image_support` policy.
- Extreme narrow terminal geometry can make preview not fit.
- Настройки строк ограничены типом `u16` и нижним значением `1`, но не имеют
  отдельного верхнего runtime-ограничения; чрезмерные значения могут сделать
  подготовку preview дорогой до применения ограничения ширины терминала.
- Do not expose numeric `preview_rows` to the model-visible `view_image` API.
- Do not parse local image paths from arbitrary Markdown/plain text as trusted
  sources.

### Риски

- Upstream changes in history rendering can silently convert typed markers back
  to lines. This breaks replay/reflow bitmap preservation.
- Изменения upstream во вставке терминальной истории могут так же незаметно
  понизить `HyperlinkLine` обратно до `Line<'static>`. Это теряет метаданные
  терминальных ссылок и было одним из реальных сценариев отказа при миграции
  на `0.137.0`.
- Kitty screen placement `a=T` works for ambient `/pets`, but not for scrollback
  history. History must use virtual placement plus placeholders.
- Architecture docs in `docs/architecture` may drift from code. For example,
  test names listed in docs must be rechecked against live code before being
  cited as executed tests.
- App-server schema fixtures are part of the public-ish protocol surface. If
  `ImagePreviewSize` changes, generated JSON/TS fixtures must move with it.
