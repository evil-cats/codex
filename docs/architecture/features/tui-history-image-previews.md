# Фича: локальные превью изображений в истории TUI

## Статус

`implemented`

## Кратко

| Поле | Значение |
| --- | --- |
| Назначение | Локальные user attachments рендерятся в TUI history как terminal image previews с текстовым fallback `[Image #n]`. |
| Текущее состояние | Реализовано для user attachments и normal history insertion. |
| Готово / реализовано | `LocalImage` display item, подготовка image через `/pets`, вставка в terminal scrollback, PNG-normalization для Kitty, targeted tests. |
| Открыто / отложено | Assistant/tool source path, bitmap re-emission при resize/reflow/replay и managed resume-stable image ownership. |
| Следующий шаг | Продолжить [plan:PLAN-TUI-ASSISTANT-IMAGES-001], особенно [stage:PLAN-TUI-ASSISTANT-IMAGES-001:002]. |

## Карта деталей

| Деталь | Где |
| --- | --- |
| Текущее устройство | [details:current-shape] |
| Карта кода | [details:code-map] |
| Поток | [details:flow] |
| Контракты | [details:contracts] |
| Инварианты | [details:invariants] |
| Runtime-заметки | [details:runtime-notes] |
| Проверки | [details:verification] |
| Связи | [details:links] |

## Назначение

Показывать локальные изображения из пользовательских attachments в TUI history
как terminal image previews, сохраняя текстовый fallback `[Image #n]` для raw
mode, copy, unsupported terminals, resize/reflow и replay paths.

Фича относится к локальной Hermione-ветке Codex и не является официальной
пользовательской документацией upstream.

## Текущее устройство

Локальный путь изображения остается source-backed частью `UserHistoryCell`.
В rich history insertion cell отдает обычные текстовые строки и дополнительный
маркер `HistoryCellDisplayItem::LocalImage(path)`. App layer превращает marker
в `HistoryInsertItem::Image`, используя тот же terminal-image protocol stack,
что и `/pets`.

Для Kitty и KittyLocalFile изображения сначала нормализуются в PNG-preview в
`CODEX_HOME/cache/tui-history-images`, потому что Kitty payload объявляется как
`f=100`. Для Sixel создается sixel cache asset. Если terminal image protocol
недоступен или подготовка asset падает, история остается читаемой через
текстовый fallback.

## Карта кода

- `codex-rs/tui/src/history_cell/mod.rs`
  - `HistoryCellDisplayItem::LocalImage`: marker для terminal-backed bitmap
    preview вне ratatui `Line`.
  - `HistoryCell::display_items_for_mode`: rich mode может вернуть строки и
    image markers.
- `codex-rs/tui/src/history_cell/messages.rs`
  - `UserHistoryCell.local_image_paths`: source локальных attachment paths и
    fallback labels `[Image #n]`.
- `codex-rs/tui/src/app/resize_reflow.rs`
  - `App::prepare_history_insert_items`: превращает `LocalImage` marker в
    `HistoryInsertItem::Image` best-effort.
- `codex-rs/tui/src/pets/mod.rs`
  - `prepare_history_image`: готовит Kitty, KittyLocalFile или Sixel payload и
    размер preview.
- `codex-rs/tui/src/pets/image_protocol.rs`
  - `png_frame`, `sixel_frame`: создают cache assets для terminal protocols.
- `codex-rs/tui/src/insert_history.rs`
  - `HistoryInsertItem::Image`: резервирует строки scrollback и пишет image
    payload напрямую в terminal writer.
- `codex-rs/tui/src/tui.rs`
  - `insert_history_items_with_wrap_policy`: TUI boundary для вставки mixed
    line/image history items.

## Поток

```text
User local image attachment
  |
  v
UserHistoryCell.local_image_paths
  |
  v
Rich mode display_items_for_mode(width)
  |
  +-- Line("[Image #n]") fallback
  |
  +-- HistoryCellDisplayItem::LocalImage(path)
          |
          v
     App::prepare_history_insert_items
          |
          v
     pets::prepare_history_image
          |
          +-- Kitty / KittyLocalFile: PNG preview cache
          +-- Sixel: sixel preview cache
          |
          v
     HistoryInsertItem::Image
          |
          v
     insert_history_items_with_wrap_policy
          |
          v
     terminal scrollback
```

```mermaid
flowchart TD
    Attachment["User local image attachment"]
    UserCell["UserHistoryCell.local_image_paths"]
    Display["display_items_for_mode(width, Rich)"]
    Fallback["Line: [Image #n]"]
    Marker["HistoryCellDisplayItem::LocalImage(path)"]
    Prepare["App::prepare_history_insert_items"]
    Pets["pets::prepare_history_image"]
    Cache["CODEX_HOME/cache/tui-history-images"]
    Item["HistoryInsertItem::Image"]
    Writer["insert_history_items_with_wrap_policy"]
    Scrollback["terminal scrollback"]

    Attachment --> UserCell
    UserCell --> Display
    Display --> Fallback
    Display --> Marker
    Marker --> Prepare
    Prepare --> Pets
    Pets --> Cache
    Pets --> Item
    Item --> Writer
    Fallback --> Writer
    Writer --> Scrollback
```

## Контракты

- `HistoryCell::display_items_for_mode(width, HistoryRenderMode::Rich)` может
  вернуть `HistoryCellDisplayItem::LocalImage(path)` только рядом с текстовым
  fallback line.
- `HistoryRenderMode::Raw` остается line-only. Raw/copy-friendly история не
  содержит terminal image payload.
- `App::prepare_history_insert_items` является best-effort boundary: при
  unsupported terminal или ошибке asset preparation marker пропускается, а
  fallback line остается в истории.
- `pets::prepare_history_image` возвращает `TerminalHistoryImage` с координатой
  `x = 2`, размером в строках/колонках и payload типа `Text` или `Bytes`.
- `insert_history_items_with_wrap_policy` резервирует rows для
  `HistoryInsertItem::Image` так же, как считает wrapped rows для text lines.

## Инварианты

- Не встраивать raw terminal escape payload в ratatui `Line`.
- Не читать локальные пути из произвольного Markdown/plain text как trusted
  image source.
- Всегда сохранять `[Image #n]` fallback рядом с bitmap preview.
- Для Kitty и KittyLocalFile history previews сначала делать PNG-preview cache,
  потому что payload отправляется как PNG (`f=100`).
- `tmux` и `zellij` остаются fallback-only через текущий
  `detect_pet_image_support` policy.

## Runtime-заметки

- Поддерживаемые protocol paths: Kitty inline data, Kitty local file graphics и
  Sixel.
- Cache root: `CODEX_HOME/cache/tui-history-images`.
- Target preview height: `HISTORY_IMAGE_TARGET_ROWS = 12`; width ограничивается
  текущей terminal width через `max_columns = width - 4`.
- Normal history insertion может вставить bitmap preview. Resize reflow,
  initial replay и overlay-deferred paths сейчас остаются line-oriented и
  сохраняют только fallback labels.
- Source image ownership остается у исходного attachment path; cache содержит
  derived preview assets, а не managed копию оригинала.

## Проверки

- Cell marker emission:
  `cargo test -p codex-tui user_history_cell_emits_local_image_items_for_terminal_history`

  Proves: `UserHistoryCell` в rich mode отдает `LocalImage` marker и fallback
  label.

- Terminal image row handling:
  `cargo test -p codex-tui history_image_restores_cursor_and_reserves_rows`

  Proves: `insert_history` резервирует rows и восстанавливает cursor при image
  payload.

- Kitty PNG normalization:
  `cargo test -p codex-tui history_image_prepare_kitty_payload_converts_jpeg_to_png`

  Proves: Kitty history preview получает PNG payload даже из JPEG source.

- Width clamp:
  `cargo test -p codex-tui history_image_size_clamps_wide_images_to_available_columns`

  Proves: preview не выходит за доступную ширину history.

- Full TUI crate:
  `cargo test -p codex-tui`

  Proves: regression coverage для TUI paths, если нужен полный локальный
  прогон.

## Связи

- План: [plan:PLAN-TUI-ASSISTANT-IMAGES-001]
- Этап 002: [stage:PLAN-TUI-ASSISTANT-IMAGES-001:002]
- Отложенная работа по resize/reflow: [follow-up:FU-2026-001]
- Отложенная работа по assistant/tool source: [follow-up:FU-2026-002]
- Отложенные работы: [follow-ups:image-history]
- Коммиты реализации: `96feb7e0d Add terminal image previews to TUI history`,
  `483c08245 Normalize history images for Kitty previews`

[details:code-map]: #карта-кода
[details:contracts]: #контракты
[details:current-shape]: #текущее-устройство
[details:flow]: #поток
[details:invariants]: #инварианты
[details:links]: #связи
[details:runtime-notes]: #runtime-заметки
[details:verification]: #проверки
[follow-up:FU-2026-001]: ../../follow-ups/FU-2026-001-tui-history-image-reflow-reemit.md
[follow-up:FU-2026-002]: ../../follow-ups/FU-2026-002-tui-assistant-tool-image-source.md
[follow-ups:image-history]: ../../follow-ups/README.md
[plan:PLAN-TUI-ASSISTANT-IMAGES-001]: ../../plans/PLAN-TUI-ASSISTANT-IMAGES-001/plan.md
[stage:PLAN-TUI-ASSISTANT-IMAGES-001:002]: ../../plans/PLAN-TUI-ASSISTANT-IMAGES-001/stages/002-local-image-history-cell.md
