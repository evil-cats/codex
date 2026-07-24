---
id: fork-tui-history-image-previews
status: active
created: 2026-06-08
updated: 2026-07-21
source_scope: rust-v0.137.0..HEAD
---

# TUI history image previews

## Обзор

Эта карточка фиксирует крупную fork-доработку Hermione: TUI history умеет
показывать локальные изображения как terminal previews с текстовым fallback.

Карточка предназначена именно для переноса fork patch на новый upstream: она
сохраняет полный feature chain, кодовые границы, config/API surface, проверки,
ограничения и известные риски.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Основные commits | `96feb7e0d`, `483c08245`, `a041a3820`, `23726d4c6`, `bc35d2615`, `697bad938` |
| Архитектурный owner | `docs/architecture/features/tui-history-image-previews.md` |
| Главный invariant | Не встраивать terminal escape payload в ratatui `Line` |
| Инвариант после `0.137.0` | Обычные строки терминальной истории должны идти как `HyperlinkLine`, а не возвращаться к старому пути через `Line<'static>` |
| Typed marker | `HistoryCellDisplayItem::LocalImage { path, preview_size }` |
| Controlled boundary | `AppEvent::InsertLocalImage { path, caption, preview_size }` |
| Config surface | `[tui.history_image_preview]` |
| Model-visible API | `view_image.preview_size = small`, `normal`, `large` |
| Checkpoint перед карточкой | Пропущен по явному разрешению пользователя от 2026-06-08 |

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

## Commit chain

| Commit | Смысл |
| --- | --- |
| `96feb7e0d Add terminal image previews to TUI history` | Базовый marker/render path для пользовательских local images, `HistoryInsertItem::Image`, `pets::prepare_history_image` |
| `483c08245 Normalize history images for Kitty previews` | `png_frame`, нормализация non-PNG в PNG для Kitty payload `f=100` |
| `a041a3820 Wire view_image into local image history` | `AppEvent::InsertLocalImage`, validation, `LocalImageHistoryCell`, `ChatWidget::on_view_image_tool_call` |
| `23726d4c6 Preserve TUI image previews during replay` | Item-level replay/reflow, `ImageGeneration.saved_path`, snapshot update |
| `bc35d2615 Anchor Kitty history images in scrollback` | Kitty virtual placement, Unicode placeholders, scrollback anchors/deletion |
| `697bad938 Add configurable TUI image preview sizes` | `ImagePreviewSize`, `preview_size`, `[tui.history_image_preview]`, app-server/schema propagation |

Важно: текущая архитектурная карточка в `docs/architecture` исторически
упоминала только первые два commits в секции "Коммиты реализации". Для handoff
без потери смысла нужен весь chain выше.

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

## Пошаговое воспроизведение

### 1. Ввести typed display item

В `history_cell/mod.rs` расширить display item enum:

```rust
HistoryCellDisplayItem::LocalImage {
    path: PathBuf,
    preview_size: ImagePreviewSize,
}
```

Все code paths, которым нужны plain lines, должны явно фильтровать markers.
После upstream `HyperlinkLine` migration полезна вспомогательная функция:

```rust
pub(crate) fn line(self) -> Option<Line<'static>>
```

Важно: эта функция не является общим мостом миграции для терминальной истории.
Она нужна только потребителям обычных видимых строк. Пути scrollback,
resize reflow, initial replay, thread-switch tail replay и overlay-deferred
history должны переносить обычные строки как `HyperlinkLine`, чтобы не потерять
метаданные терминальных ссылок и не повторить регрессию миграции
на `0.137.0`.

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
  - ограничить columns текущей width, исторически `width - 4`;
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

Заметка для переноса на `rust-v0.142.5`: если конфликт в `view_image.rs`
появляется только на границе импортов, сохраняй `ImagePreviewSize`, но не
возвращай устаревший `codex_features::Feature`. Актуальная upstream-сторона уже
перенесла обработчик на `turn_environment.cwd().to_abs_path()`/`PathUri`,
оставила `data_url_from_bytes` без старой ветки `Feature::ResizeAllImages` и
сохранила `detail = original`; fork-добавка в этом месте ограничена разбором
`preview_size`, ошибкой для неизвестного значения и заполнением
`ImageViewItem.preview_size`.

Заметка для переноса на `rust-v0.143.0`: если upstream меняет поверхность пути
`ImageView` с `AbsolutePathBuf` на `PathUri`/`LegacyAppPathString`, сохраняй
новую upstream-модель путей и добавляй поверх нее fork-поле `preview_size`.
`TurnItem::ImageView`, legacy `ViewImageToolCallEvent`, app-server v2
`ThreadItem::ImageView`, TUI replay и `ChatWidget::on_view_image_tool_call`
должны переносить один и тот же `ImagePreviewSize` без возврата к числовому
`preview_rows`.

Заметка для переноса на `rust-v0.144.1`: если конфликт возникает в
сгенерированном TypeScript `v2/ThreadItem.ts`, сохраняй upstream-форму
`{ "type": "webSearch" } & WebSearchItem` и
`{ "type": "imageGeneration" } & ImageGenerationItem`, но добавляй fork-поле
`previewSize: ImagePreviewSize` в `imageView`. Не возвращай встроенное поле
`savedPath?: AbsolutePathBuf` в `ThreadItem.ts`: после upstream-выноса оно живет
в `ImageGenerationItem`.

Заметка для переноса на `rust-v0.144.5`: между upstream-метками
`rust-v0.144.4` и `rust-v0.144.5` owner-пути этой доработки не менялись.
Проверка текущего `HEAD` подтвердила сохранение сквозного контракта:
`view_image` возвращает модели `FunctionCallOutputContentItem::InputImage` и
отдельно создаёт `ImageViewItem` с `preview_size`; protocol/app-server передают
`ImagePreviewSize` и `previewSize`; TUI сохраняет доверенную границу
`InsertLocalImage`, `HistoryCellDisplayItem::LocalImage`, `HyperlinkLine`, пути
replay/reflow, вставку в терминальную историю, расчёт размера, Kitty virtual
placement, регрессионные тесты и snapshot-артефакты. Правки кода, схем и
snapshots для этой миграции не потребовались.

Заметка для переноса на `rust-v0.144.6`: между upstream-метками
`rust-v0.144.5` и `rust-v0.144.6` owner-пути этой доработки также не менялись.
Проверка текущего `HEAD` подтвердила тот же сквозной контракт без правок кода,
схем и snapshots:

- `view_image` разрешает путь относительно `cwd` выбранного окружения, проверяет
  доступ через его filesystem с sandbox context и принимает только обычный файл;
- TUI получает локальный путь только через структурированные `ImageView` /
  `ImageGeneration.saved_path`, повторно требует локальный обычный файл и
  успешное декодирование, а путь чужой платформы оставляет текстовым fallback;
- `ImagePreviewSize` проходит через core item, legacy event, app-server v2,
  сгенерированные TypeScript/JSON и TUI replay без потери `preview_size`;
- `HistoryCellDisplayItem::LocalImage` сохраняется в Rich history и путях
  initial replay, thread-switch replay, overlay defer и resize reflow, а terminal
  payload создаётся только на границе `prepare_history_insert_items`;
- число столбцов ограничено доступной шириной терминала и снизу значением `1`,
  а настройки строк имеют тип `u16` и ограничение снизу значением `1`;
  отдельного верхнего runtime-ограничения для `small_rows`, `normal_rows` и
  `large_rows` нет.

Заметка для переноса на `rust-v0.145.0`: upstream изменил replay/reflow и
сгенерированную форму `ThreadItem`, поэтому перенос потребовал адаптации, а не
простого сохранения прежних конфликтных сторон:

- `InitialHistoryReplayBuffer` и terminal insertion продолжают хранить
  `HistoryCellDisplayItem` / `HistoryInsertItem`, чтобы `LocalImage` переживал
  initial replay, thread-switch replay и resize reflow; при этом новая
  upstream-ветвь `render_from_transcript_tail || overlay.is_some()` сохраняется
  и планирует немедленный source-backed reflow;
- `finish_required_stream_reflow` очищает `retained_items`, а не устаревшие
  `retained_lines`, перед отложенным воспроизведением хвоста transcript;
- новые upstream-тесты capped replay в `app/tests.rs` используют
  `buffer_initial_history_replay_display_items`, `retained_items` и
  `ReflowRenderResult.items`; сам файл остаётся неразрешённым из-за чужих
  конфликтов в других участках;
- обычные строки по-прежнему проходят как `HyperlinkLine`, а изображения — как
  отдельный `HistoryInsertItem::Image`; новые upstream-изменения wrapping не
  возвращают terminal history к line-only контракту;
- сгенерированный `ThreadItem.ts` сохраняет новую upstream-форму
  `{ "type": "sleep" } & SleepItem` и одновременно fork-поле
  `previewSize: ImagePreviewSize` в `imageView`;
- конфликт `app/event_dispatch.rs`, относящийся к lifecycle runtime-потоков, не
  является частью этой карточки и намеренно оставлен владельцу соответствующей
  fork-доработки.

## Проверки

### Смысловое покрытие

Проверочное покрытие должно сохранять весь контракт TUI image preview:

- пользовательские local image attachments отображаются как fallback-строка и
  bitmap marker только в Rich mode;
- `view_image` и `ImageGeneration.saved_path` входят в историю через доверенный
  `AppEvent::InsertLocalImage`, а произвольный Markdown/plain text не создаёт
  `LocalImage`;
- пути replay/reflow, включая resize reflow, initial replay, thread-switch
  tail replay и overlay-deferred history, не теряют typed marker;
- обычные строки терминальной истории остаются `HyperlinkLine` на пути
  `HistoryCellDisplayItem::Line(HyperlinkLine)` ->
  `HistoryInsertItem::Line(HyperlinkLine)`;
- Kitty history previews используют PNG payload, virtual placement и
  placeholder anchoring вместо screen placement;
- `view_image.preview_size`, `ImagePreviewSize`, `[tui.history_image_preview]`,
  config schema и app-server protocol surfaces остаются синхронизированными;
- snapshot-покрытие сохраняет форму отрисованной истории видимой для проверки.

Покрытие, найденное в `HEAD`:

- `user_history_cell_emits_local_image_items_for_terminal_history`;
- `local_image_history_cell_emits_image_item_in_rich_mode_only`;
- `agent_markdown_image_syntax_does_not_emit_local_image_item`;
- `view_image_tool_call_emits_local_image_event`;
- `view_image_tool_call_preserves_preview_size_hint`;
- `image_generation_call_with_saved_path_emits_local_image_event`;
- `history_image_size_*`;
- `history_image_prepare_kitty_payload_*`;
- `kitty_png_virtual_placement_transmits_without_screen_placement`;
- `kitty_file_virtual_placement_transmits_without_screen_placement`;
- config-тесты для `[tui.history_image_preview]`;
- handler/spec-тесты `view_image` для `preview_size`;
- тесты app-server protocol и schema fixtures.

Snapshot-подтверждение:

- `codex-rs/tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__image_generation_call_history_snapshot.snap`

Этот snapshot подтверждает текстовый fallback для `ImageGeneration` без
`saved_path`; terminal bitmap marker намеренно не входит в ratatui snapshot.
Форма marker и его отсутствие в Raw mode проверяются обычными assertions
`user_history_cell_emits_local_image_items_for_terminal_history` и
`local_image_history_cell_emits_image_item_in_rich_mode_only`.

### Владелец исполняемой карты

Проверки уровня карточки запускает skill-owned команда `fork tests`.
Внутренние argv не являются runbook карточки: они живут в блоке `fork-tests.v1`
ниже и читаются `fork tests` для карточки `fork-tui-history-image-previews`.

Данные ниже являются текущим блоком `fork-tests.v1`.

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "core view image",
      "argv": ["just", "test", "-p", "codex-core", "view_image"]
    },
    {
      "purpose": "tui render",
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
      "purpose": "app server protocol",
      "argv": ["just", "test", "-p", "codex-app-server-protocol"]
    },
    {
      "purpose": "protocol",
      "argv": ["just", "test", "-p", "codex-protocol"]
    },
    {
      "purpose": "pending snapshots",
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

### Дополнительные gates

Для общего проверочного прохода после обработки карточек требуются не прямые
команды из карточки, а skill-owned владельцы:

- `fork generators` нужен, если diff затрагивает или подтверждает
  schema-артефакты config/app-server для `ImagePreviewSize`,
  `[tui.history_image_preview]` или `previewSize`;
- `fork build-fast` остаётся релевантным release-gate, потому что историческая
  цепочка доработки включала сборку и установку бинарника;
- ручной Kitty smoke остаётся `manual-required`: нужно подтвердить, что команда
  terminal graphics показывает PNG, TUI `view_image` показывает изображение в
  истории, preview переживает обычный resize, а крайне узкое окно отказывает без
  потери fallback;
- локальная проверка без Rust/Cargo сводится к review whitespace в diff и поиску
  ключевых идентификаторов `LocalImage`, `InsertLocalImage`, `ImagePreviewSize`,
  `preview_size`, `history_image_preview`, `KittyUnicodePlaceholder`.

### Исторические результаты

Исторические commit messages и docs фиксировали такие проверки:

- `bc35d2615`:
  - `cargo test -p codex-tui kitty`;
  - work-tracking check;
  - `git diff --check`.
- `697bad938`:
  - scoped-запуск `cargo check` для `codex-core`, `codex-tui`,
    `codex-app-server-protocol`;
  - `just write-config-schema`;
  - `just write-app-server-schema`;
  - focused-запуск `cargo test`;
  - `just fix`;
  - `just build-fast-release`;
  - установка и `--version`;
  - известные несвязанные падения full-suite.

Текущая карточка не утверждает, что эти проверки запускались в текущем запуске.
Прямые команды `just`/`cargo` выше сохранены только как историческое подтверждение,
а не как нормативный runbook.

### Известные падения и пропуски

Не копировать без перепроверки как существующие тесты:

- `insert_history_items_with_wrap_policy_counts_image_rows`;
- `kitty_placeholder_image_writes_text_anchored_cells`;
- `kitty_placeholder_image_is_deleted_after_scrolling_off_visible_history`.

Explorer нашёл эти имена только в docs, а не как тестовые функции в committed code.

Внутренний argv для `codex-tui` в `fork-tests.v1` сохраняет skip
`ide_context::ipc::tests::fetch_ide_context_uses_unregistered_request_route`.
Причина skip не подтверждена этой карточкой; это известный пропуск, который
родительский общий проверочный проход должен сохранить или пересмотреть явно.

Исторический `697bad938` фиксировал известные несвязанные падения full-suite. Эта
карточка не утверждает, что такие падения остаются актуальными после текущей
миграции.

В текущем запуске подагента проверки, сборка, генераторы, форматирование и
markdownlint не запускались по ограничению задачи. Проверки уровня проекта из
`fork-tests.v1`, gates генераторов и сборки, а также проверка pending snapshots
переданы родительскому общему проверочному проходу.

## Ограничения

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

## Риски

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

## Связанные документы

- `docs/architecture/features/tui-history-image-previews.md`
- `docs/plans/archive/2026/PLAN-TUI-ASSISTANT-IMAGES-001/summary.md`
- `docs/follow-ups/archive/2026/FU-2026-001-tui-history-image-reflow-reemit.md`
- `docs/follow-ups/archive/2026/FU-2026-002-tui-assistant-tool-image-source.md`
- `docs/follow-ups/archive/2026/FU-2026-006-tui-history-image-preview-size.md`

## Проверка покрытия

| Пункт | Статус | Где отражено |
| --- | --- | --- |
| User attachments отображаются как preview и fallback | перенесено в карточку | "Итоговый контракт", "Пошаговое воспроизведение" |
| `view_image` входит в controlled local image path | перенесено в карточку | "Source contract", "Пошаговое воспроизведение" |
| `ImageGeneration.saved_path` входит в controlled local image path | перенесено в карточку | "Source contract", "Пошаговое воспроизведение" |
| Replay/reflow сохраняет typed marker | перенесено в карточку | "Пошаговое воспроизведение", "Ограничения" |
| Контракт миграции `HyperlinkLine` сохранён | перенесено в карточку | "Итоговый контракт", "Пошаговое воспроизведение", "Риски" |
| Kitty history использует virtual placement и placeholders | перенесено в карточку | "Kitty contract", "Пошаговое воспроизведение" |
| `preview_size` и config rows остаются разными surfaces | перенесено в карточку | "Preview size contract" |
| Регрессионное покрытие, snapshot-подтверждение и test targets | перенесено в карточку | "Проверки" |
| `fork tests` владеет исполняемой картой уровня карточки | перенесено в карточку | "Проверки" |
| Исторические проверочные команды сохранены только как подтверждение | перенесено в карточку | "Проверки" |
| Риск устаревших doc/test names сохранён | перенесено в карточку | "Проверки", "Риски" |
