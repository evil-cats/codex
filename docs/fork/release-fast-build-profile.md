---
id: fork-release-fast-build-profile
status: active
created: 2026-06-08
updated: 2026-06-16
source_scope: rust-v0.140.0..hermione-0.140.0
---

# Release-fast build profile

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая добавляет быстрый
optimized build path: Cargo profile `release-fast` и `just build-fast-release`.

Эта доработка нужна для remote сборок fork binary, когда нужен быстрый
optimized compile-check с более высокой параллельностью финальных стадий. В
`0.140.0` upstream `release` уже использует thin LTO, но остается canonical
release profile для workflow, где binary остается пригодным для symbolication
до упаковки. Hermione сохраняет отдельный named profile с
`codegen-units = 32`, без debug symbols и с явным strip, потому что этот
профиль используется для установки `codex-hermione` после удаленной сборки.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Основные commits | `6be4eae58`, `f2797c6e8` |
| Migration repair commits | `46cdb741f`, `671afe3ee` |
| Cargo profile | `[profile.release-fast]` |
| Just target | `just build-fast-release` |
| Remote build path в текущем workflow | `f-ms-dev:/home/slader/Projects/codex` |
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
лишнее: `just build-fast-release` используется как быстрый путь удаленной
сборки, а его результат напрямую копируется в локальный `codex-hermione`.

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
# ready to install directly after the remote build.
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

Root `justfile` должен содержать:

```just
build-fast-release:
    cargo build -p codex-cli --profile release-fast
```

Артефакт сборки:

```text
codex-rs/target/release-fast/codex
```

Этот артефакт должен быть stripped. Проверочная команда `file` не должна
показывать `with debug_info, not stripped` для результата
`just build-fast-release`.

### Remote-only workflow

В текущих договорённостях с пользователем Rust/Cargo/`just` для этого
репозитория запускаются только на:

```text
f-ms-dev:/home/slader/Projects/codex
```

`release-fast` разрешён пользователем как compile-check и путь сборки для
установки.
Тесты и debug-команды требуют отдельного согласия, если они не были явно
оговорены в текущей задаче.

## Пошаговое воспроизведение

### 1. Добавить profile

В root workspace `codex-rs/Cargo.toml` рядом с `[profile.release]` добавить
`[profile.release-fast]`, наследующий `release`.

Не менять upstream `release`: он остаётся canonical profile для upstream
workflow упаковки. Hermione `release-fast` должен переопределять только
fork-specific настройки артефакта установки.

### 2. Добавить just target

В root `justfile` добавить:

```just
build-fast-release:
    cargo build -p codex-cli --profile release-fast
```

Target должен жить рядом с release/build commands, чтобы команда была видна в
обычном build workflow.

### 3. Проверить build на remote

Если пользователь разрешил compile-check, синхронизировать changes на
`f-ms-dev:/home/slader/Projects/codex` и запускать:

```bash
just build-fast-release
```

Ожидаемый результат: успешная optimized сборка в
`target/release-fast/codex`. Артефакт должен быть stripped без дополнительного
ручного `strip`.

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

Это не откат upstream release profile, а отдельный remote compile-check и путь
сборки для установки. В `0.140.0` migration локально подтверждены code anchors
`[profile.release-fast]`, `codegen-units = 32`, `debug = "none"`,
`strip = "symbols"` и `just build-fast-release`; remote build на `f-ms-dev`
выполняется через `just build-fast-release`.

## Регрессионное покрытие

У commits `6be4eae58`, `f2797c6e8`, `46cdb741f`, `671afe3ee` не было
отдельных тестовых additions, закрепляющих именно profile. Поэтому проверка
этой доработки в основном build-oriented:

- `just build-fast-release`;
- проверка артефакта:

  ```text
  file codex-rs/target/release-fast/codex
  codex-rs/target/release-fast/codex --version
  ```

  `file` должен показывать stripped binary, а `--version` должен возвращать
  ожидаемую Hermione version metadata.

- для migration repair: targeted TUI/config tests, если пользователь разрешил.

Исторически commit `697bad938` позже указывал `just build-fast-release` как
выполненную verification-команду, но эта карточка не утверждает, что этот build
запускался в текущем turn. Для переноса на `0.140.0` текущий запуск выполнен на
`f-ms-dev`.

## Проверки

Для повторения:

1. На `f-ms-dev:/home/slader/Projects/codex`:
   - `just build-fast-release`;
   - `file codex-rs/target/release-fast/codex`;
   - при install: проверить `codex --version` или целевой binary path.
2. Локально без Rust/Cargo:
   - `rg -n "release-fast|build-fast-release|codegen-units = 32|debug = \"none\"|strip = \"symbols\"" codex-rs/Cargo.toml justfile`;
   - `git diff --check`.

## Ограничения

- Не заменять upstream `release` profile: он нужен для canonical
  release-артефакта.
- Не запускать `cargo build --release` для обычной Hermione compile-check:
  это проверяет canonical release path, а не быстрый fork build path.
- Не полагаться на ручной `strip` после сборки: он легко теряется при переносе
  и не должен быть частью обычного install workflow.
- Не запускать Rust/Cargo/`just` локально в текущем workflow; использовать
  `f-ms-dev`, если пользователь разрешил.
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

## Сводка покрытия

| Пункт | Статус | Где отражено |
| --- | --- | --- |
| Добавить `[profile.release-fast]` | перенесено | "Итоговый контракт", "Пошаговое воспроизведение" |
| Использовать thin LTO и `codegen-units = 32` | перенесено | "Итоговый контракт" |
| Зафиксировать stripped-артефакт, готовый к установке | перенесено | "Обзор", "Зачем это нужно", "Итоговый контракт", "Проверки" |
| Добавить `just build-fast-release` | перенесено | "Итоговый контракт", "Пошаговое воспроизведение" |
| Сохранить remote-only Rust/Cargo/`just` workflow | перенесено | "Итоговый контракт", "Проверки", "Ограничения" |
| Зафиксировать `0.137.0` migration repair | перенесено | "Migration repair: `0.137.0`" |
| Проверить перенос profile на `0.140.0` | перенесено; сборка на `f-ms-dev` должна подтвердить stripped-артефакт | "Migration check: `0.140.0`", "Проверки" |
