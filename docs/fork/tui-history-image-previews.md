---
id: fork-tui-history-image-previews
status: active
created: 2026-06-08
updated: 2026-09-03
---

# Preview локальных изображений в истории TUI

## Обзор

Эта карточка фиксирует действующую fork-доработку Hermione: история TUI умеет
показывать локальные изображения как терминальные preview с резервным текстом.

Карточка задаёт текущую цепочку реализации, кодовые границы, поверхности
config/API, проверки, ограничения и известные риски.

## Зачем это нужно

Без этой доработки изображения в истории TUI остаются текстовыми ссылками или
резервными метками. Hermione fork добавляет графический терминальный preview для
локальных изображений, но сохраняет читаемый текст там, где растровое
изображение невозможно.

Поддерживаемые источники:

- локальные изображения, приложенные пользователем;
- `view_image`;
- `ImageGeneration.saved_path`;
- пути повторного воспроизведения и reflow, включая reflow после изменения
  размера, initial replay, thread-switch tail replay и отложенную overlay-историю.

Резервный текст остаётся рядом:

- `[Image #n]` для пользовательских вложений;
- `[Image]`;
- `[Image: <caption>]`.

## Карта файлов

### Ячейки истории TUI и слой текстовых строк

| Файл | Роль |
| --- | --- |
| `codex-rs/tui/src/history_cell/mod.rs` | `HistoryCellDisplayItem::Line(HyperlinkLine)`, `HistoryCellDisplayItem::LocalImage`, `display_items_for_mode`, вспомогательные функции преобразования |
| `codex-rs/tui/src/history_cell/messages.rs` | `UserHistoryCell.local_image_paths`, резервные метки и `ImagePreviewSize::Normal` по умолчанию |
| `codex-rs/tui/src/history_cell/local_image.rs` | `LocalImageHistoryCell` для контролируемых изображений от assistant и tool |
| `codex-rs/tui/src/history_cell/tests.rs` | Проверки выдачи маркеров, поведения `Raw`/`Rich`, запрета доверия к Markdown и сохранения `HyperlinkLine` рядом с маркером изображения |
| `codex-rs/tui/src/terminal_hyperlinks.rs` | `HyperlinkLine` хранит видимый `Line` и метаданные терминальных ссылок отдельно, чтобы байты OSC 8 не влияли на геометрию |

### Событие TUI и вызывающий код

| Файл | Роль |
| --- | --- |
| `codex-rs/tui/src/app_event.rs` | `AppEvent::InsertLocalImage { path, caption, preview_size }` |
| `codex-rs/tui/src/app/event_dispatch.rs` | Проверка обычного файла, декодирование через crate `image` и сохранение `InsertLocalImage` при replay в состоянии `offline` |
| `codex-rs/tui/src/app/tests/disconnect_tests.rs` | Регрессия обработки `InsertLocalImage` через `App::handle_event` в состоянии `offline` с резервным текстом и типизированным маркером |
| `codex-rs/tui/src/chatwidget/tool_lifecycle.rs` | `on_view_image_tool_call`, `on_image_generation_end`, `insert_local_image_history` |
| `codex-rs/tui/src/chatwidget/replay.rs` | Повторное воспроизведение `ThreadItem::ImageView` через `on_view_image_tool_call` |
| `codex-rs/tui/src/session_log.rs` | Запись варианта `InsertLocalImage` и наличия подписи в журнал сессии |

### Вставка в историю терминала

| Файл | Роль |
| --- | --- |
| `codex-rs/tui/src/app/resize_reflow.rs` | `App::prepare_history_insert_items`, выбор строк, config и протокола |
| `codex-rs/tui/src/app/resize_reflow_tests.rs` | Проверки upstream-контрактов ограничения строк, уведомления о пагинации и initial replay для типизированных элементов отображения |
| `codex-rs/tui/src/insert_history.rs` | `HistoryInsertItem::Line(HyperlinkLine)`, `HistoryInsertItem::Image`, подсчёт строк и запись в режимах `Standard`/`FullScreen` |
| `codex-rs/tui/src/tui.rs` | Граница `insert_history_items_with_wrap_policy` и очередь `PendingHistoryItems` |
| `codex-rs/tui/src/tui/history_tail.rs` | Перед заменой видимого хвоста из строк сбрасывает ожидающие `HistoryInsertItem` через общий путь вставки |
| `codex-rs/tui/src/custom_terminal.rs` | Учёт и очистка изображений истории Kitty |

### Протокол терминальных изображений

| Файл | Роль |
| --- | --- |
| `codex-rs/tui/src/pets/mod.rs` | `prepare_history_image`, расчёт размера, путь cache и выбор протокола |
| `codex-rs/tui/src/pets/image_protocol.rs` | Кадры PNG/Sixel и команды виртуального размещения Kitty |
| `codex-rs/tui/src/kitty_placeholder.rs` | Сетка заполнителей `U+10EEEE` для привязки к scrollback Kitty |

### Поверхности protocol, tool и config

| Файл | Роль |
| --- | --- |
| `codex-rs/protocol/src/items.rs` | `ImagePreviewSize`, `ImageViewItem.path` как `PathUri`, `ImageViewItem.preview_size`, `ImageGenerationItem.saved_path` |
| `codex-rs/protocol/src/protocol.rs` | Устаревающее событие `ViewImageToolCallEvent.preview_size` |
| `codex-rs/protocol/src/legacy_events.rs` | Преобразование `TurnItem::ImageView` в устаревающее событие без потери `preview_size` |
| `codex-rs/core/src/tools/handlers/view_image.rs` | Разбор `preview_size` и отклонение недопустимых значений |
| `codex-rs/core/src/tools/handlers/view_image_spec.rs` | Публикация `preview_size` без `preview_rows` |
| `codex-rs/config/src/types.rs` | Значения `[tui.history_image_preview]` по умолчанию |
| `codex-rs/core/src/config/mod.rs` | Выбор строк через `HistoryImagePreviewConfig::rows_for` во время выполнения |
| `codex-rs/core/config.schema.json` | Схема config |
| `codex-rs/app-server-protocol/src/protocol/v2/item.rs` | App-server v2 `ThreadItem::ImageView.path` как `LegacyAppPathString`, `previewSize` |
| `codex-rs/app-server-protocol/src/protocol/thread_history.rs` | Восстановление `ThreadItem::ImageView` из устаревающего события с `preview_size` |
| `codex-rs/app-server-protocol/schema/typescript/v2/ThreadItem.ts` | Сгенерированный union TS для `ThreadItem::ImageView.path` и `previewSize` |
| `codex-rs/app-server-protocol/schema/typescript/ImagePreviewSize.ts` | Сгенерированный enum TS |
| `codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.schemas.json` | Сводная сгенерированная схема с `v2/ImagePreviewSize` и `previewSize` |
| `codex-rs/app-server-protocol/schema/json/codex_app_server_protocol.v2.schemas.json` | Сгенерированная схема v2 с `ImagePreviewSize` и `previewSize` |
| `codex-rs/app-server-protocol/schema/precomputed/app-server-exports-experimental.json.zst` | Сжатая сгенерированная схема experimental API |
| `codex-rs/app-server-protocol/schema/precomputed/app-server-exports-stable.json.zst` | Сжатая сгенерированная схема stable API |

## Итоговый контракт

### Контракт отображения

1. `HistoryCell::display_items_for_mode(width, HistoryRenderMode::Rich)` может
   возвращать `HistoryCellDisplayItem::LocalImage { path, preview_size }`.
2. Маркер растрового изображения всегда идёт рядом с резервной текстовой строкой.
3. `HistoryRenderMode::Raw` возвращает только строки.
4. Произвольный Markdown или обычный текст не должен создавать `LocalImage`.
5. Необработанные данные терминального изображения не хранятся в ratatui
   `Line`.
6. Данные терминального изображения создаются только в
   `App::prepare_history_insert_items`.
7. Если терминальный протокол не поддерживается или подготовка ресурса
   завершается ошибкой, маркер пропускается, а резервная строка остаётся.
8. Обычные строки терминальной истории должны сохранять путь
   `HistoryCellDisplayItem::Line(HyperlinkLine)` ->
   `HistoryInsertItem::Line(HyperlinkLine)`.
9. Не возвращать пути scrollback и reflow к старому `Line<'static>` как к
   основному типу строк. `HistoryCellDisplayItem::line()` допустим только для
   потребителей обычных видимых строк и намеренно отбрасывает метаданные
   ссылок.
10. Уведомление о неполной или пагинированной истории терминала добавляется как
    `HistoryCellDisplayItem::Line`; оно не должно понижать соседние маркеры
    изображений до `HyperlinkLine` или обычного текста.

### Контракт источника

1. Локальные вложения пользователя входят через `UserHistoryCell.local_image_paths`.
2. Изображения от assistant или tool входят только через доверенный
   структурированный путь:
   `AppEvent::InsertLocalImage`.
3. `view_image` создаёт `ThreadItem::ImageView`, а вызывающий код TUI отправляет
   `InsertLocalImage`.
4. `ImageGeneration.saved_path` является контролируемым источником, если путь
   существует.
5. Если `ImageGeneration.saved_path` отсутствует, TUI оставляет текстовую ячейку
   истории `Generated Image`.

### Контракт размера preview

1. `ImagePreviewSize` имеет значения `Small`, `Normal`, `Large`, которые
   сериализуются как `small`, `normal`, `large`.
2. Значение по умолчанию: `normal`.
3. `view_image.preview_size` принимает только `small`, `normal`, `large`.
4. Отсутствующий `preview_size` равен `normal`.
5. Недопустимое значение возвращает видимую модели ошибку:

   ```text
   view_image.preview_size only supports `small`, `normal`, or `large`; omit `preview_size` for default normal preview, got `<value>`
   ```

6. Числовой `preview_rows` не входит в видимый модели tool API.
7. `[tui.history_image_preview]` задаёт количество строк:
   - `small_rows = 8`;
   - `normal_rows = 12`;
   - `large_rows = 20`.
8. Во время выполнения `HistoryImagePreviewConfig::rows_for` ограничивает
   значение снизу числом `1`.

### Контракт Kitty

1. Preview истории для Kitty и KittyLocalFile нормализуют источник в PNG cache
   под `CODEX_HOME/cache/tui-history-images`.
2. Payload Kitty использует формат PNG `f=100`.
3. Изображение истории использует виртуальное размещение `U=1`, а не плавающее
   размещение на экране.
4. Якорем scrollback служит текстовая сетка из заполнителей Unicode `U+10EEEE`
   с диакритиками строк и столбцов.
5. `CustomTerminal` учитывает строки истории и удаляет идентификаторы изображений
   Kitty после выхода привязанного изображения за пределы видимой истории.
6. Путь Sixel сохраняет payload в виде байтов.

## Архитектурное решение

Ячейки истории возвращают типизированные элементы отображения и не хранят байты
терминального протокола в `Line`. `App` преобразует только доверенные маркеры
`LocalImage` на границе вставки истории, модули протоколов владеют кодированием и
размещением Kitty, а `CustomTerminal` — очисткой ресурсов терминала. Очередь
`PendingHistoryItems` и оба режима вставки, `Standard` и `FullScreen`, сохраняют
изображения отдельными `HistoryInsertItem::Image`. Поверхности config и protocol
передают только ограниченный enum размера preview.

## Порядок повторения при переносе

### 1. Ввести типизированный элемент отображения

В `history_cell/mod.rs` расширить enum элементов отображения:

```rust
HistoryCellDisplayItem::LocalImage {
    path: PathBuf,
    preview_size: ImagePreviewSize,
}
```

Все пути кода, которым нужны обычные строки, должны явно фильтровать маркеры.
Для этого используется вспомогательная функция:

```rust
pub(crate) fn line(self) -> Option<Line<'static>>
```

Важно: эта функция не является общим мостом миграции для терминальной истории.
Она нужна только потребителям обычных видимых строк. Пути scrollback, reflow
после изменения размера, initial replay, thread-switch tail replay и отложенной
overlay-истории должны переносить обычные строки как `HyperlinkLine`, чтобы не
потерять метаданные терминальных ссылок.

### 2. Добавить резервный текст и маркер во вложения пользователя

В `UserHistoryCell` оставить резервный текст `[Image #n]`, а в режиме `Rich`
добавить `LocalImage { path, preview_size: ImagePreviewSize::Normal }`.

### 3. Добавить ячейку контролируемого локального изображения

Создать `LocalImageHistoryCell`:

- поля: `path`, `caption`, `preview_size`;
- резервный текст:
  - без подписи -> `[Image]`;
  - с подписью -> `[Image: <caption>]`;
- режим `Rich` -> резервная строка и `LocalImage`;
- режим `Raw` -> только резервная строка.

### 4. Добавить `AppEvent::InsertLocalImage`

Поля события:

```rust
InsertLocalImage {
    path: PathBuf,
    caption: Option<String>,
    preview_size: ImagePreviewSize,
}
```

При обработке события:

- проверить, что путь указывает на обычный файл;
- проверить, что crate `image` может его декодировать;
- не отбрасывать `InsertLocalImage` новым upstream-фильтром для состояния
  `offline`, поскольку replay-границы и обычные ячейки истории разрешены в том
  же состоянии;
- при успехе создать `LocalImageHistoryCell`;
- при ошибке добавить предупреждение с резервным текстом, не создавая маркер
  растрового изображения.

### 5. Подключить рабочий вызывающий код

`ChatWidget::on_view_image_tool_call`:

- получает путь и `preview_size`;
- строит подпись из отображаемого пути относительно cwd;
- отправляет `AppEvent::InsertLocalImage`.

`ChatWidget::on_image_generation_end`:

- если `saved_path` существует, отправляет `InsertLocalImage`;
- берёт подпись из непустого `revised_prompt`, иначе из `call_id`;
- если путь отсутствует, сохраняет только текстовую ячейку.

### 6. Сохранить маркеры при повторном воспроизведении и reflow

Reflow после изменения размера, initial replay, thread-switch tail replay и
отложенная overlay-история должны работать на `HistoryCellDisplayItem`, а не
терять маркер изображения при раннем преобразовании в `Line`.

При совмещении с ограничением строк и пагинацией upstream нужно сохранять
`InitialHistoryReplayBuffer.retained_items`, `was_truncated`, уведомление о
неполном transcript и запрос дозагрузки старой истории. Уведомление вставляется
в типизированную последовательность элементов до `prepare_history_insert_items`,
поэтому следующие за ним маркеры `LocalImage` остаются типизированными.

Повторное воспроизведение `ChatWidget` должно восстанавливать
`ThreadItem::ImageView` через тот же путь `on_view_image_tool_call`.

### 7. Подготовить терминальный payload на границе

В `App::prepare_history_insert_items`:

- пройти по элементам отображения;
- `HistoryCellDisplayItem::Line(HyperlinkLine)` превращать в
  `HistoryInsertItem::Line(HyperlinkLine)` без промежуточного понижения до
  `Line<'static>`;
- `LocalImage` обрабатывать по возможности:
  - определить поддержку изображений терминалом;
  - получить целевое число строк через
    `config.history_image_preview.rows_for(preview_size)`;
  - ограничить число столбцов доступной шириной терминала с учётом отступов
    протокола;
  - вызвать `pets::prepare_history_image`;
  - при успехе создать `HistoryInsertItem::Image`;
  - при ошибке записать её в журнал и пропустить маркер.

`insert_history_items_with_mode_and_wrap_policy` должен учитывать высоту
изображения и записывать его как в `InsertHistoryMode::Standard`, так и в
`InsertHistoryMode::FullScreen`. Понижать `HistoryInsertItem::Image` до строки в
очереди `PendingHistoryItems` нельзя.

### 8. Реализовать привязку Kitty

В `pets/image_protocol.rs`:

- `png_frame` преобразует изображение в PNG preview и помещает его в cache;
- `kitty_transmit_png_with_virtual_placement`;
- `kitty_transmit_png_file_with_virtual_placement`.

Команда должна передавать данные изображения и создавать виртуальное размещение
`U=1`.

В `kitty_placeholder.rs` печатать сетку `U+10EEEE` как реальные текстовые строки
scrollback. Перемещения курсора внутри одной строки недостаточно: Kitty покажет
только одну полосу изображения.

В `custom_terminal.rs` хранить идентификатор изображения и удалять его, когда
привязанные строки ушли из видимой истории.

### 9. Соединить protocol и config

- `ImagePreviewSize` в элементах protocol;
- `ImageViewItem.preview_size`;
- `ViewImageToolCallEvent.preview_size`, если устаревающее событие ещё существует;
- app-server v2 `previewSize`;
- сгенерированный TS `ImagePreviewSize.ts`;
- fixtures схемы JSON;
- `[tui.history_image_preview]` config;
- `HistoryImagePreviewConfig::rows_for`.

При повторном переносе сохранять актуальные upstream-типы и накладывать только
fork-поля и типизированные маркеры:

- путь `ImageView` остаётся в актуальной upstream-модели
  `PathUri`/`LegacyAppPathString`; fork добавляет `preview_size`, но не возвращает
  устаревшие ветки реализации;
- `TurnItem` и app-server v2 `ThreadItem` сохраняют все актуальные
  upstream-варианты, включая `FunctionCallOutput`; `TurnItem::id`,
  `ThreadItem::id`, `From<CoreTurnItem>` и replay TUI остаются исчерпывающими
  после добавления `ImageView`, не теряя соседние варианты;
- `TurnItem::ImageView`, устаревающее событие, app-server v2, сгенерированный
  TypeScript и повторное воспроизведение TUI передают один `ImagePreviewSize` без
  числового `preview_rows`;
- `TurnItem::as_legacy_events` в `protocol/src/legacy_events.rs` и
  `ThreadHistoryBuilder::handle_view_image_tool_call` в
  `app-server-protocol/src/protocol/thread_history.rs` переносят тот же
  `preview_size` через цепочку устаревающего события;
- сгенерированный `ThreadItem.ts` сохраняет все варианты upstream, а вариант
  `imageView` дополнительно содержит `previewSize`;
- `HyperlinkLine` сохраняет актуальную upstream-семантику графемно-корректного
  переноса и безопасной обработки `destination`; fork-доработка переносит
  метаданные ссылок как есть и не подменяет upstream-реализацию разбора ссылок;
- `InitialHistoryReplayBuffer`, `ReflowRenderResult` и вставка в историю терминала
  хранят `items`, а не пониженные `lines`; обычные строки остаются
  `HyperlinkLine`, изображения — отдельным `HistoryInsertItem::Image`;
- `TerminalWidth` из текущего upstream, уведомление об усечении и дозагрузка
  пагинированной истории сохраняются вокруг типизированных `items`, а
  `resize_reflow_tests.rs` проверяет тот же контракт через
  `items`/`retained_items`;
- `CustomTerminal` инициализирует и очищает привязки Kitty вместе с остальным
  состоянием истории;
- соседние lifecycle-функции не включаются в эту карточку только из-за общего
  файла.

## Проверки

Исполняемая карта регрессионных тестов карточки:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "типизированный ImageView с path, preview_size и результатом изображения для модели",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "view_image_tool_attaches_local_image"
      ]
    },
    {
      "purpose": "видимая модели ошибка неподдерживаемого preview_size",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "handle_rejects_unsupported_preview_size"
      ]
    },
    {
      "purpose": "схема инструмента экспортирует preview_size без preview_rows",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "view_image_schema_exposes_preview_size_but_not_preview_rows"
      ]
    },
    {
      "purpose": "графические изображения в истории, обработка в состоянии offline, replay, reflow и вставка в терминал TUI",
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
      "purpose": "wire-контракт app-server для элементов preview изображения и previewSize",
      "argv": ["just", "test", "-p", "codex-app-server-protocol"]
    },
    {
      "purpose": "типы protocol для размера preview и сохранённого пути изображения",
      "argv": ["just", "test", "-p", "codex-protocol"]
    },
    {
      "purpose": "отсутствие непреднамеренных изменений snapshot TUI",
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

Дополнительно обязателен gate `fork generators`, если перенос меняет
`ImagePreviewSize`, `[tui.history_image_preview]` или `previewSize` app-server.

Проверки `codex-core` используют точные имена принадлежащих карточке тестов.
Широкий фильтр `view_image` сюда не входит: он захватывает независимые сценарии
Code Mode и ошибочно делает host предусловием для истории TUI.

`manual-required`: в Kitty нужно проверить показ PNG, `view_image` в истории,
сохранение preview после изменения размера и резервный текст при слишком узком
окне.

## Риски и ограничения

### Ограничения

- Управляемое владение исходными изображениями после resume намеренно не
  реализовано. Если исходный путь исчезает, растровый payload нельзя создать
  повторно; резервный текст сохраняется.
- При неподдерживаемом протоколе терминала
  `prepare_history_insert_items` пропускает растровый маркер; резервный текст
  сохраняется.
- При текущей политике `detect_pet_image_support` внутри `tmux` и `zellij`
  доступен только резервный текст.
- При крайне узкой геометрии терминала preview может не поместиться.
- Настройки строк ограничены типом `u16` и нижним значением `1`, но не имеют
  отдельного верхнего ограничения во время выполнения; чрезмерные значения могут
  сделать подготовку preview дорогой до применения ограничения ширины терминала.
- Числовой `preview_rows` нельзя добавлять в видимый модели API `view_image`.
- Локальные пути изображений из произвольного Markdown или обычного текста
  нельзя считать доверенными источниками.

### Риски

- Изменения upstream в отображении истории могут незаметно преобразовать
  типизированные маркеры обратно в строки. Это нарушит сохранение растровых
  изображений при replay/reflow.
- Изменения upstream во вставке терминальной истории могут так же незаметно
  понизить `HyperlinkLine` обратно до `Line<'static>`. Это теряет метаданные
  терминальных ссылок. Оба режима вставки, `Standard` и `FullScreen`, должны
  оставаться типизированными.
- Экранное размещение Kitty `a=T` подходит для фоновых `/pets`, но не для
  истории scrollback. История должна использовать виртуальное размещение и
  заполнители.
- Fixtures схемы app-server являются частью внешней поверхности protocol. При
  изменении `ImagePreviewSize` сгенерированные fixtures JSON/TS должны меняться
  вместе с ним.
