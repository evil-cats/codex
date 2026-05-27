---
id: FU-2026-006
status: done
priority: medium
kind: enhancement
tags: [codex, tui, images, view-image, config, mermaid]
created: 2026-05-27
updated: 2026-05-27
review_at: реализация structured preview_size для TUI history image previews
architecture_refs:
  - docs/architecture/features/tui-history-image-previews.md
  - codex-rs/core/src/tools/handlers/view_image.rs
  - codex-rs/core/src/tools/handlers/view_image_spec.rs
  - codex-rs/config/src/types.rs
  - codex-rs/tui/src/history_cell/mod.rs
  - codex-rs/tui/src/app/resize_reflow.rs
  - codex-rs/tui/src/pets/mod.rs
invalid_if:
  - preview_size удален из trusted image path
  - TUI history image previews снова используют только fixed-size rows без config
---

# FU-2026-006: настраиваемые размеры TUI history image preview

## Кратко

| Поле | Значение |
| --- | --- |
| Статус | `done` |
| Суть | Добавить структурный `preview_size` со значениями `small`, `normal`, `large` для локальных image previews и вынести rows для этих размеров в config. |
| Почему важно | Mermaid-диаграммы и другие wide/diagram images лучше читать крупным preview, но обычные картинки должны сохранить текущий дефолт `normal = 12 rows`. |
| Когда вернулись | При реализации structured `preview_size` для `view_image` и TUI history previews. |
| Итог | Реализованы `preview_size = small | normal | large`, config rows, schema fixtures, app-server v2 propagation и TUI sizing по resolved rows. |
| Следующий шаг | Нет; карточка закрыта. |
| Связи | [feature:tui-history-image-previews], [code:view-image-handler], [code:config-types], [code:pets-mod] |

## Наблюдение

При ручном тесте Mermaid PNG в Kitty стало ясно, что размер preview удобнее
задавать не глобально и не через парсинг текста, а как управляемый
per-image hint. Пользователь предложил форму вроде
`[Image:large: /tmp/codex-mermaid-test.png]`, но локальные пути и размер нельзя
извлекать из произвольного Markdown/plain text как trusted source. Такой size
hint также не должен попадать в видимый fallback: пользователю консоли нужен
обычный текст вроде `[Image: /tmp/codex-mermaid-test.png]`, а не служебный тег.

Нужен структурный путь: `view_image` или другой controlled image source
передает размер preview вместе с уже проверенным image event, а видимый fallback
показывает только человекочитаемую метку path/caption без `preview_size`.

## Почему это важно

Текущий history preview использует fixed target rows (`normal = 12`). Это
нормально для обычных изображений и совместимо с уже реализованным поведением,
но для диаграмм, особенно Mermaid, часто нужен крупный режим. Если rows будут
жестко зашиты в коде, пользователю придется пересобирать Codex для настройки
размера под терминал.

Числовой `preview_rows` не стоит делать model-visible аргументом `view_image`:
это деталь renderer'а, зависящая от терминала и пользовательского config.
Внешний контракт должен выражать намерение (`small`, `normal`, `large`), а
конкретные rows должны резолвиться внутри TUI из config.

## Что нужно сделать

- [x] Добавить enum размера preview: `small`, `normal`, `large`; если
  `preview_size` не задан, использовать `normal`.
- [x] Провести size hint через trusted path:
  `view_image(..., preview_size = "large")` -> `ImageViewItem` ->
  `AppEvent::InsertLocalImage` -> `LocalImageHistoryCell` ->
  `HistoryCellDisplayItem::LocalImage`.
- [x] Не парсить локальные пути или размер из произвольного текста вида
  `[Image:large: /tmp/file.png]`.
- [x] Не показывать size hint в fallback label. Видимый fallback должен оставаться
  обычным `[Image]`, `[Image: <caption>]` или `[Image: <path>]`, чтобы
  служебный `preview_size` не торчал в консольной истории, Raw/copy и логах.
- [x] Не добавлять `preview_rows` в model-visible `view_image` API. Числовые rows
  должны быть только config/input для вычисления размера renderer'ом.
- [x] Вынести rows в config с дефолтами:

  ```toml
  [tui.history_image_preview]
  small_rows = 8
  normal_rows = 12
  large_rows = 20
  ```

- [x] Разрешать отсутствие секции config: дефолты должны давать текущее поведение
  `normal = 12 rows`.
- [x] Передавать resolved rows в `pets::prepare_history_image` вместо fixed
  `HISTORY_IMAGE_TARGET_ROWS`.
- [x] Обновить `ConfigToml` / runtime `Config` / schema fixtures.
- [x] Добавить тесты:
  - parsing/defaults для `[tui.history_image_preview]`;
  - `view_image` принимает и валидирует `preview_size`;
  - `view_image` schema не включает публичный `preview_rows`;
  - `ImageViewItem`/legacy event несет size hint;
  - fallback label не содержит `preview_size` / `small` / `large`;
  - TUI sizing использует `large` rows и сохраняет `normal` по умолчанию;
  - invalid size дает понятную ошибку.

## Итог

Закрыто реализацией structured preview sizing:

- `ImagePreviewSize` добавлен в protocol items, `ImageViewItem`, legacy
  `ViewImageToolCallEvent` и app-server v2 `ThreadItem::ImageView`.
- `view_image` принимает опциональный `preview_size` (`small`, `normal`,
  `large`) и отвергает другие значения с понятной ошибкой.
- `view_image` schema показывает `preview_size`, но не раскрывает renderer-only
  `preview_rows`.
- `LocalImageHistoryCell`, `AppEvent::InsertLocalImage` и
  `HistoryCellDisplayItem::LocalImage` несут size hint как trusted data, не
  через текст fallback.
- `[tui.history_image_preview]` задает `small_rows`, `normal_rows`,
  `large_rows`; runtime `Config` резолвит rows через `rows_for`.
- `pets::prepare_history_image` больше не использует fixed target rows:
  `resize_reflow` передает rows, выбранные по `preview_size`.
- Config schema и app-server schema fixtures обновлены.

## Когда вернуться

Карточка закрыта после реализации structured `preview_size` и config rows.

## Когда закрыть как неактуальное

Карточка закрыта как реализованная. Она стала бы неактуальной, если бы проект
закрепил fixed-size previews или перенес Mermaid/diagram output в отдельный
viewer path без history image preview.

## Связи

- Архитектура: [feature:tui-history-image-previews]
- `view_image` handler: [code:view-image-handler]
- `view_image` tool schema: [code:view-image-spec]
- Config TOML types: [code:config-types]
- Protocol item: [code:protocol-items]
- TUI image marker: [code:history-cell]
- History insert preparation: [code:resize-reflow]
- Image sizing/preparation: [code:pets-mod]

[code:config-types]: ../../../../codex-rs/config/src/types.rs
[code:history-cell]: ../../../../codex-rs/tui/src/history_cell/mod.rs
[code:pets-mod]: ../../../../codex-rs/tui/src/pets/mod.rs
[code:protocol-items]: ../../../../codex-rs/protocol/src/items.rs
[code:resize-reflow]: ../../../../codex-rs/tui/src/app/resize_reflow.rs
[code:view-image-handler]: ../../../../codex-rs/core/src/tools/handlers/view_image.rs
[code:view-image-spec]: ../../../../codex-rs/core/src/tools/handlers/view_image_spec.rs
[feature:tui-history-image-previews]: ../../../architecture/features/tui-history-image-previews.md
