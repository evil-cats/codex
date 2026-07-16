---
id: fork-release-fast-build-profile
status: active
created: 2026-06-08
updated: 2026-07-16
source_scope: rust-v0.140.0..hermione-0.140.0
---

# Release-fast build profile

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая добавляет быстрый
optimized build path: Cargo profile `release-fast` и skill-owned gate сборки
`fork build-fast`. В исходниках этому gate соответствует target
`build-fast-release` в корневом `justfile`.

Эта доработка нужна для сборок fork binary на `f-ms-dev`, когда нужен быстрый
optimized compile-check с более высокой параллельностью финальных стадий. В
`0.140.0` upstream `release` уже использует thin LTO, но остается canonical
release profile для workflow, где binary остается пригодным для symbolication
до упаковки. Hermione сохраняет отдельный named profile с
`codegen-units = 32`, без debug symbols и с явным strip, потому что этот
профиль используется для установки `codex-hermione` после сборки на `f-ms-dev`.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Основные commits | `6be4eae58`, `f2797c6e8` |
| Migration repair commits | `46cdb741f`, `671afe3ee` |
| Cargo profile | `[profile.release-fast]` |
| Just target | `build-fast-release` в root `justfile` |
| Source-of-truth build path в текущем workflow | `f-ms-dev:/home/slader/Projects/codex` |
| Артефакт установки | `codex-rs/target/release-fast/codex`, stripped |
| Checkpoint перед карточкой | Пропущен по явному разрешению пользователя от 2026-06-08 |

## Зачем это нужно

Исторически upstream `release` profile был оптимизирован под размер
поставляемого артефакта:

- `lto = "fat"`;
- `codegen-units = 1`;
- strip symbols.

В `0.140.0` upstream `release` уже перешёл на `lto = "thin"` и
`codegen-units = 4`, но также стал профилем для symbolication:

- `debug = "line-tables-only"`;
- `split-debuginfo = "off"`;
- `strip = false`.

Такой upstream release profile уместен для workflow упаковки, где symbols
архивируются или обрабатываются отдельно. Для Hermione install workflow это
лишнее: skill-owned `fork build-fast` использует target `build-fast-release` как
быстрый путь сборки на `f-ms-dev`, а результат этой сборки напрямую копируется в
`codex-hermione`.

Если `release-fast` просто наследует upstream-настройки `debug` и `strip`,
binary становится unstripped и может вырасти примерно до гигабайтного размера.
Поэтому Hermione `release-fast` обязан явно переопределять настройки символов
и давать stripped-артефакт, готовый к установке без ручного `strip` после
сборки.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/Cargo.toml` | Добавляет `[profile.release-fast]` |
| `justfile` | Добавляет `build-fast-release` |
| `codex-rs/core/src/config/mod.rs` | Migration repair: восстановлен `Config.tui_terminal_title_label` для release-fast build после merge |
| `codex-rs/tui/src/history_cell/messages.rs` | Migration repair: `Line` -> `HyperlinkLine` conversions |
| `codex-rs/tui/src/history_cell/mod.rs` | Migration repair: вспомогательная функция для line-only extraction из `HistoryCellDisplayItem` |
| `codex-rs/tui/src/insert_history.rs` | Migration repair: перевод borrowed/non-static lines в static перед hyperlink wrapping |
| `codex-rs/tui/src/app/resize_reflow.rs` | `671afe3ee`: rustfmt-style wrapping после migration |
| `codex-rs/tui/src/app_backtrack.rs` | `671afe3ee`: rustfmt-style wrapping после migration |

## Итоговый контракт

### Build profile

`codex-rs/Cargo.toml` должен содержать:

```toml
[profile.release-fast]
inherits = "release"
# Local optimized builds should keep using multiple cores during the final
# optimization stages. The canonical release profile above favors size.
lto = "thin"
codegen-units = 32
# This profile is used for Hermione's local install artifact. Upstream release
# keeps line tables for symbolication before packaging; release-fast should be
# ready to install directly after the release-fast build.
debug = "none"
strip = "symbols"
```

История:

- `6be4eae58` добавил profile с `codegen-units = 16`;
- `f2797c6e8` поднял значение до `32`;
- `0.140.0` upstream изменил `[profile.release]` на профиль для symbolication с
  `debug = "line-tables-only"` и `strip = false`;
- текущее значение для Hermione fork: `codegen-units = 32`,
  `debug = "none"` и `strip = "symbols"`.

### Just target

Корневой `justfile` должен сохранять target `build-fast-release`. Этот target
остаётся деталью реализации для skill-owned `fork build-fast`: он собирает
package `codex-cli` с Cargo profile `release-fast`, но не является нормативной
workflow-командой карточки.

Артефакт сборки:

```text
codex-rs/target/release-fast/codex
```

Этот артефакт должен быть stripped. Проверочная команда `file` не должна
показывать `with debug_info, not stripped` для результата
skill-owned build gate.

### Source-of-truth workflow

В текущих договорённостях с пользователем разработка, Rust/Cargo/`just`, сборка
и тесты для этого репозитория выполняются в source-of-truth checkout:

```text
f-ms-dev:/home/slader/Projects/codex
```

`release-fast` разрешён пользователем как compile-check и путь сборки для
установки.
Тесты и debug-команды требуют отдельного согласия, если они не были явно
оговорены в текущей задаче.

## Порядок повторения при переносе

### 1. Добавить profile

В root workspace `codex-rs/Cargo.toml` рядом с `[profile.release]` добавить
`[profile.release-fast]`, наследующий `release`.

Не менять upstream `release`: он остаётся canonical profile для upstream
workflow упаковки. Hermione `release-fast` должен переопределять только
fork-specific настройки артефакта установки.

### 2. Проверить owned target в `justfile`

В root `justfile` должен оставаться target `build-fast-release`. Он живёт рядом
с release/build targets, чтобы skill-owned `fork build-fast` имел стабильный
внутренний build target и не зависел от ручной команды в карточке.

### 3. Передать build gate родительскому проходу

Если пользователь разрешил compile-check, общий проверочный проход должен
включить skill-owned `fork build-fast` в source-of-truth checkout
`f-ms-dev:/home/slader/Projects/codex`. Ожидаемый результат: успешная optimized
сборка в `target/release-fast/codex`. Артефакт должен быть stripped без
дополнительного ручного `strip`.

### 4. При migration на новый upstream проверить не только build profile

На merge `0.137.0` простой перенос profile был недостаточен: `release-fast`
поймал regressions, которые были не видны при поверхностной проверке fork
patches.

Обязательно проверить:

- `Config.tui_terminal_title_label` присутствует в effective `Config` и
  заполняется из `cfg.tui.terminal_title_label`;
- `HistoryCellDisplayItem::Line` после upstream changes несёт актуальный тип
  (`HyperlinkLine` в `0.137.0`), и Hermione local image code конвертирует
  plain `Line` правильно;
- `insert_history_lines_with_wrap_policy` переводит non-static `Line` в static
  до `plain_hyperlink_lines(...)`, если upstream API этого требует.

## Migration repair: `0.137.0`

Commit `46cdb741f Fix Hermione 0.137 release-fast build` сделал два вида
ремонта.

Первый ремонт: terminal title config.

- В `codex-rs/core/src/config/mod.rs` восстановлено поле:

  ```rust
  pub tui_terminal_title_label: Option<String>,
  ```

- В config loading добавлено:

  ```rust
  tui_terminal_title_label: cfg
      .tui
      .as_ref()
      .and_then(|t| t.terminal_title_label.clone()),
  ```

Это закрывало merge-regression: TOML type уже знал
`tui.terminal_title_label`, но effective runtime `Config` потерял это значение.

Второй ремонт: TUI history line type.

- Upstream изменил `HistoryCellDisplayItem::Line` так, что он несёт
  `HyperlinkLine`, а не plain `Line`.
- `UserHistoryCell` должен создавать display items через
  `HistoryCellDisplayItem::from(Line::from(...))`.
- В `HistoryCellDisplayItem` нужна вспомогательная функция:

  ```rust
  pub(crate) fn line(self) -> Option<Line<'static>>
  ```

  Он возвращает `Some(line.into())` для line item и `None` для `LocalImage`.

- `insert_history_lines_with_wrap_policy` должен использовать `line_to_static`
  перед `plain_hyperlink_lines`.

Commit `671afe3ee Update 0.137 lockfile formatting` дополнительно содержит:

- large `Cargo.lock` formatting/version normalization;
- small rustfmt-style layout changes in `resize_reflow.rs` and
  `app_backtrack.rs`;
- visible semantic intent: сохранить build после migration, а не добавить
  новую feature.

## Migration check: `0.140.0`

Во время переноса на `rust-v0.140.0` upstream `[profile.release]` уже содержит:

```toml
[profile.release]
lto = "thin"
debug = "line-tables-only"
split-debuginfo = "off"
strip = false
codegen-units = 4
```

Hermione fork всё равно должен сохранять отдельный профиль:

```toml
[profile.release-fast]
inherits = "release"
lto = "thin"
codegen-units = 32
debug = "none"
strip = "symbols"
```

Это не откат upstream release profile, а отдельный compile-check на `f-ms-dev` и
путь сборки для установки. В `0.140.0` migration должны сохраняться code anchors
`[profile.release-fast]`, `codegen-units = 32`, `debug = "none"`,
`strip = "symbols"` и owned target `build-fast-release`; проверка сборки
принадлежит skill-owned `fork build-fast`.

## Migration check: `0.142.5`

Во время one-card переноса на `rust-v0.142.5` в текущем checkout подтверждены
кодовые якоря `[profile.release-fast]`, `codegen-units = 32`, `debug = "none"`,
`strip = "symbols"`, owned target `build-fast-release`, effective
`Config.tui_terminal_title_label` и конвертации TUI history.

Единственная правка по карточке: комментарий в `codex-rs/Cargo.toml` больше не
использует устаревшую формулировку `remote build` и говорит о
`release-fast build`. Сборка, тесты, генераторы, форматирование и markdownlint в
этом one-card проходе не запускались; проверку артефакта должен выполнить общий
родительский проход через `fork build-fast`.

## Migration check: `0.143.0`

Во время one-card переноса на `rust-v0.143.0` в текущем checkout подтверждены
кодовые якоря `[profile.release-fast]`, `codegen-units = 32`, `debug = "none"`,
`strip = "symbols"` и owned target `build-fast-release`.

Связанные migration-repair якоря тоже сохранены: effective
`Config.tui_terminal_title_label` заполняется из `cfg.tui.terminal_title_label`,
`HistoryCellDisplayItem::Line` несёт `HyperlinkLine`,
`HistoryCellDisplayItem::from(Line<'static>)` создаёт `HyperlinkLine::new(...)`,
а `insert_history_lines_with_wrap_policy` переводит plain `Line` в static через
`line_to_static` перед `plain_hyperlink_lines(...)`.

Кодовые правки для этой карточки не потребовались. Сборка, тесты, генераторы,
форматирование и markdownlint в этом one-card проходе не запускались; проверку
stripped-артефакта должен выполнить общий родительский проход через
`fork build-fast`.

## Migration check: `0.144.1`

Во время one-card переноса на `rust-v0.144.1` в текущем checkout подтверждены
кодовые якоря `[profile.release-fast]`, `codegen-units = 32`, `debug = "none"`,
`strip = "symbols"` и owned target `build-fast-release`.

Связанные migration-repair якоря тоже сохранены: effective
`Config.tui_terminal_title_label` заполняется из `cfg.tui.terminal_title_label`,
`HistoryCellDisplayItem::Line` несёт `HyperlinkLine`,
`HistoryCellDisplayItem::from(Line<'static>)` создаёт `HyperlinkLine::new(...)`,
а `insert_history_lines_with_wrap_policy` переводит plain `Line` в static через
`line_to_static` перед `plain_hyperlink_lines(...)`.

В `codex-rs/tui/src/history_cell/messages.rs` разрешён конфликт между
upstream-очисткой пользовательского текста и fork-поддержкой элементов
`LocalImage`: `raw_lines` вызывает `sanitize_user_text(...)`, а rich-режим
продолжает возвращать `HistoryCellDisplayItem`, включая `LocalImage` для
истории терминала.
Сборка, тесты, генераторы, форматирование и markdownlint в этом one-card проходе
не запускались; проверку stripped-артефакта должен выполнить общий родительский
проход через `fork build-fast`.

## Migration check: `0.144.4`

Во время проверки одной карточки при переносе на `rust-v0.144.4` в текущей
рабочей копии подтверждены кодовые якоря `[profile.release-fast]`,
`codegen-units = 32`, `debug = "none"`, `strip = "symbols"` и внутренний target
`build-fast-release`.

Связанные якоря ремонта миграции также сохранены: итоговый
`Config.tui_terminal_title_label` заполняется из
`cfg.tui.terminal_title_label`, `HistoryCellDisplayItem::Line` несёт
`HyperlinkLine`, `HistoryCellDisplayItem::from(Line<'static>)` создаёт
`HyperlinkLine::new(...)`, а `insert_history_lines_with_wrap_policy` переводит
исходные `Line` в `Line<'static>` через `line_to_static` перед вызовом
`plain_hyperlink_lines(...)`. Режимы отображения истории пользователя сохраняют
очистку текста через `sanitize_user_text(...)`, а `HistoryRenderMode::Rich`
продолжает возвращать `HistoryCellDisplayItem`, включая `LocalImage`.

Кодовые правки для этой карточки не потребовались. Сборка, тесты, генераторы,
форматирование и markdownlint в этом проходе по одной карточке не запускались;
проверку stripped-артефакта должен выполнить общий родительский проход через
`fork build-fast`.

## Migration check: `0.144.5`

Во время проверки одной карточки при переносе на `rust-v0.144.5` в текущей
рабочей копии подтверждены кодовые якоря `[profile.release-fast]`,
`inherits = "release"`, `lto = "thin"`, `codegen-units = 32`,
`debug = "none"`, `strip = "symbols"` и внутренний target
`build-fast-release`.

Skill-owned workflow сохраняет разделение владельцев: `fork build-fast`
вызывает внутренний target, проверяет исполняемый файл
`codex-rs/target/release-fast/codex`, его файловые метаданные и вывод
`--version`;
`fork install` использует этот артефакт по умолчанию и повторяет проверки
метаданных и версии для исходного, временного и установленного исполняемого
файла до и после атомарной замены `codex-hermione`.

В наблюдаемом состоянии разрешённые изменения версий задают версию workspace
`0.144.5` и версию пакета `codex-cli` `0.144.5+hermione`. Эти изменения
принадлежат общей миграции и в проходе этой карточки не редактировались.
Связанные якоря ремонта миграции также сохранены: итоговый
`Config.tui_terminal_title_label` заполняется из
`cfg.tui.terminal_title_label`, TUI history сохраняет `HyperlinkLine`,
преобразование исходных `Line` в `Line<'static>` и элементы `LocalImage`.

Кодовые правки для этой карточки не потребовались. Сборка, установка, тесты,
генераторы, форматирование и markdownlint в этом проходе по одной карточке не
запускались; stripped-артефакт и метаданные версии должен подтвердить общий
родительский проход через `fork build-fast`, а установку при необходимости —
через `fork install`.

## Проверки

### Смысловое покрытие

| Контракт | Обязательность | Где покрывается |
| --- | --- | --- |
| `[profile.release-fast]` наследует `release` и задаёт `lto = "thin"`, `codegen-units = 32`, `debug = "none"`, `strip = "symbols"` | `required` | `codex-rs/Cargo.toml`; skill-owned gate сборки `fork build-fast` |
| Корневой `justfile` сохраняет owned target `build-fast-release` для сборки `codex-cli` с Cargo profile `release-fast` | `required` | `justfile`; skill-owned gate сборки `fork build-fast` |
| Артефакт установки остаётся `codex-rs/target/release-fast/codex` и должен быть stripped без ручного `strip` | `required` | `fork build-fast`; install workflow для `codex-hermione` |
| Source-of-truth build path остаётся `f-ms-dev:/home/slader/Projects/codex` | `required` | общий fork workflow и gate сборки `fork build-fast` |
| Migration repair для `0.137.0` сохраняет effective config и TUI history conversions после upstream API changes | `conditional` | gate сборки `fork build-fast`; targeted TUI/config tests только по отдельному разрешению пользователя |

### Владелец исполняемой карты

`not-applicable`: у этой карточки нет блока `fork-tests.v1`, потому что
release-fast profile contract покрывается gate сборки, а не card-level test argv.
Не придумывать автоматизированные card-level tests для профиля без отдельной
реализации.

Skill-owned owner для проверки доработки: `fork build-fast`.

### Дополнительные gates

| Gate | Когда нужен | Что подтверждает |
| --- | --- | --- |
| `fork build-fast` | общий проверочный проход, если пользователь разрешил compile-check | release-fast build, stripped binary и version evidence для `codex-hermione` |
| `fork cards validate` | общий проверочный проход после правки карточек | strict-форма `## Проверки`, отсутствие runbook command leakage и machine-readable исключение для отсутствующего `fork-tests.v1` |

### Исторические результаты

| Проверка или источник | Результат | Примечание |
| --- | --- | --- |
| Commit `6be4eae58` | добавлен Cargo profile `release-fast` с `codegen-units = 16` | исходная fork-доработка |
| Commit `f2797c6e8` | `release-fast` поднят до `codegen-units = 32` | текущий fork contract сохраняет это значение |
| Commit `46cdb741f` | восстановлен release-fast build после migration `0.137.0` | terminal title config и TUI history line type repair |
| Commit `671afe3ee` | lockfile formatting/version normalization и small rustfmt-style layout changes | semantic intent: сохранить build после migration, а не добавить новую feature |
| Исторический внутренний argv для just target | `cargo build -p codex-cli --profile release-fast` | это деталь реализации target `build-fast-release`, не нормативный runbook карточки |
| Историческая проверочная команда commit `697bad938` | `just build-fast-release` | была зафиксирована как выполненная verification-команда; текущий turn её не запускал |
| Историческая форма проверки артефакта | `file codex-rs/target/release-fast/codex`; `codex-rs/target/release-fast/codex --version` | `file` должен показывать stripped binary, а `--version` должен возвращать ожидаемую Hermione version metadata |
| Migration `0.140.0` | локально подтверждены anchors `[profile.release-fast]`, `codegen-units = 32`, `debug = "none"`, `strip = "symbols"` и target `build-fast-release`; сборка была зафиксирована на `f-ms-dev` | эта карточка не утверждает, что build запускался в текущем turn |
| Migration `0.142.5` | локально подтверждены якоря профиля, target `build-fast-release`, восстановление effective config и конвертации TUI history; комментарий `remote build` заменён на формулировку про `release-fast build` | сборка, тесты, генераторы, форматирование и markdownlint не запускались в текущем one-card проходе |
| Migration `0.143.0` | локально подтверждены якоря профиля, target `build-fast-release`, восстановление effective config и конвертации TUI history; кодовые правки не потребовались | сборка, тесты, генераторы, форматирование и markdownlint не запускались в текущем one-card проходе |
| Migration `0.144.1` | локально подтверждены якоря профиля, target `build-fast-release`, восстановление effective config и конвертации TUI history; конфликт в `messages.rs` разрешён с сохранением upstream-очистки и fork-поддержки элементов `LocalImage` | сборка, тесты, генераторы, форматирование и markdownlint не запускались в текущем one-card проходе |
| Migration `0.144.4` | локально подтверждены якоря профиля, target `build-fast-release`, итоговый `Config.tui_terminal_title_label` и преобразования истории TUI; кодовые правки не потребовались | сборка, тесты, генераторы, форматирование и markdownlint не запускались в текущем проходе по одной карточке |
| Migration `0.144.5` | локально подтверждены якоря профиля, target `build-fast-release`, владение workflow сборки и установки, версии workspace `0.144.5` и `codex-cli` `0.144.5+hermione`, итоговый `Config.tui_terminal_title_label` и преобразования истории TUI; кодовые правки не потребовались | сборка, установка, тесты, генераторы, форматирование и markdownlint не запускались в текущем проходе по одной карточке |

### Известные падения и пропуски

- У commits `6be4eae58`, `f2797c6e8`, `46cdb741f`, `671afe3ee` не было
  отдельных тестовых additions, закрепляющих именно profile.
- Checkpoint перед карточкой был пропущен по явному разрешению пользователя от
  2026-06-08.
- Текущая one-card правка не запускает сборку, тесты, генераторы, форматирование
  или markdownlint; это обязанность родительского проверочного прохода.
- Targeted TUI/config tests для migration repair выполняются только если
  пользователь отдельно разрешил такие проверки.

## Ограничения

- Не заменять upstream `release` profile: он нужен для canonical
  release-артефакта.
- Не использовать canonical release build path для обычной Hermione compile-check:
  он проверяет upstream release profile, а не быстрый fork build path.
- Не полагаться на ручной `strip` после сборки: он легко теряется при переносе
  и не должен быть частью обычного install workflow.
- Не строить workflow вокруг локальной сборки с последующим переносом на
  `f-ms-dev`; использовать source-of-truth checkout на `f-ms-dev`.
- Не считать успешный merge достаточным: `release-fast` compile-check нужен
  после migration.

## Риски

- Если `release-fast` не наследует `release`, build может отличаться слишком
  сильно от shipped optimized behavior.
- Если `codegen-units` снова станет `1`, profile потеряет смысл.
- Если `debug` и `strip` снова будут только наследоваться из upstream
  `release`, артефакт установки может стать unstripped и вырасти до
  непрактичного размера.
- При upstream migration compile errors могут проявляться в unrelated-looking
  Hermione patches: terminal title, TUI images, history cells. Карточка
  фиксирует это как обязательную проверочную развилку.

## Проверка покрытия

| Пункт | Статус | Где отражено |
| --- | --- | --- |
| Добавить `[profile.release-fast]` | перенесено | "Итоговый контракт", "Порядок повторения при переносе" |
| Использовать thin LTO и `codegen-units = 32` | перенесено | "Итоговый контракт" |
| Зафиксировать stripped-артефакт, готовый к установке | перенесено | "Обзор", "Зачем это нужно", "Итоговый контракт", "Проверки" |
| Сохранить owned target `build-fast-release` и владельца `fork build-fast` | перенесено | "Итоговый контракт", "Порядок повторения при переносе", "Проверки" |
| Сохранить `f-ms-dev` как source-of-truth для Rust/Cargo/`just` workflow | перенесено | "Итоговый контракт", "Проверки", "Ограничения" |
| Зафиксировать `0.137.0` migration repair | перенесено | "Migration repair: `0.137.0`" |
| Проверить перенос profile на `0.140.0` | перенесено; сборка на `f-ms-dev` должна подтвердить stripped-артефакт | "Migration check: `0.140.0`", "Проверки" |
| Проверить перенос profile на `0.142.5` | перенесено; `fork build-fast` должен подтвердить stripped-артефакт | "Migration check: `0.142.5`", "Проверки" |
| Проверить перенос profile на `0.143.0` | перенесено; `fork build-fast` должен подтвердить stripped-артефакт | "Migration check: `0.143.0`", "Проверки" |
| Проверить перенос profile на `0.144.1` | перенесено; `fork build-fast` должен подтвердить stripped-артефакт | "Migration check: `0.144.1`", "Проверки" |
| Проверить перенос profile на `0.144.4` | перенесено; `fork build-fast` должен подтвердить stripped-артефакт | "Migration check: `0.144.4`", "Проверки" |
| Проверить перенос profile на `0.144.5` | перенесено; `fork build-fast` должен подтвердить stripped-артефакт и метаданные версии, `fork install` — установленный исполняемый файл при необходимости | "Migration check: `0.144.5`", "Проверки" |
