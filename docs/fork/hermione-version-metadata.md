---
id: fork-hermione-version-metadata
status: active
created: 2026-06-08
updated: 2026-06-16
source_scope: rust-v0.140.0..hermione-0.140.0
---

# Hermione version metadata

## Обзор

Эта карточка фиксирует fork-доработку, которая маркирует CLI и TUI builds как
Hermione через Cargo build metadata: `0.140.0+hermione`.

Карточка также фиксирует update-check изменения: сравнение upstream versions
должно игнорировать build metadata после `+`, чтобы `0.140.0+hermione`
сравнивался как `0.140.0`.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Основные commits | `33e804a51`, `67067a90f`, перенос через merge до `671afe3ee`, перенос на `0.140.0` |
| Текущая версия ветки | `0.140.0+hermione` |
| Затронутые crates | `codex-cli`, `codex-tui` |
| Checkpoint перед карточкой | Пропущен по явному разрешению пользователя от 2026-06-08 |

## Зачем это нужно

Hermione fork должен быть отличим от upstream binary при диагностике,
`--version`, install workflow и локальном сравнении сборок. При этом update
logic должен продолжать понимать, что base semver остается upstream-compatible:
`0.140.0+hermione` не должен ломать сравнение с `rust-v0.141.0` или
`rust-v0.140.0`.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/cli/Cargo.toml` | Задаёт explicit `version = "0.140.0+hermione"` для `codex-cli` |
| `codex-rs/tui/Cargo.toml` | Задаёт explicit `version = "0.140.0+hermione"` для `codex-tui` |
| `codex-rs/Cargo.lock` | Фиксирует `codex-cli` и `codex-tui` как `0.140.0+hermione` |
| `codex-rs/cli/src/doctor/updates.rs` | Игнорирует build metadata при CLI doctor update comparison |
| `codex-rs/tui/src/update_versions.rs` | Игнорирует build metadata при TUI update comparison |
| `codex-rs/tui/src/version.rs` | Использует `env!("CARGO_PKG_VERSION")` вне tests и stable test value |

## Итоговый контракт

1. `codex-cli` больше не должен наследовать `version.workspace = true`.
2. `codex-tui` больше не должен наследовать `version.workspace = true`.
3. В обоих manifests должна стоять explicit version текущего upstream release с
   Hermione metadata:

   ```toml
   version = "0.140.0+hermione"
   ```

4. При следующем upstream release нужно менять base part:
   - `0.140.0+hermione` -> `0.141.0+hermione`, если base tag `rust-v0.141.0`.
5. `Cargo.lock` должен отражать explicit versions для `codex-cli` и
   `codex-tui`.
6. Version comparison для update checks должен отрезать suffix после `+` перед
   parsing semver triplet.
7. `0.133.0+hermione` должен парситься как `(0, 133, 0)`.
8. `is_newer("0.134.0", "0.133.0+hermione")` должен быть `Some(true)`.
9. `is_newer("0.133.0", "0.133.0+hermione")` должен быть `Some(false)`.
10. Pre-release values вроде `0.11.0-beta.1` по-прежнему не должны
    интерпретироваться как обычный semver triplet.
11. `codex-rs/tui/src/version.rs` должен использовать package version at
    compile time, но в tests давать стабильное `CODEX_CLI_VERSION = "0.0.0"`.

## Пошаговое воспроизведение

### 1. Обновить manifests

В `codex-rs/cli/Cargo.toml`:

```toml
[package]
name = "codex-cli"
version = "0.140.0+hermione"
```

В `codex-rs/tui/Cargo.toml`:

```toml
[package]
name = "codex-tui"
version = "0.140.0+hermione"
```

Не оставлять `version.workspace = true` для этих двух crates.

### 2. Обновить lockfile

После изменения manifests обновить `codex-rs/Cargo.lock` на `f-ms-dev`, если
пользователь разрешил Rust/Cargo command. В lockfile должны быть:

```toml
name = "codex-cli"
version = "0.140.0+hermione"
```

и:

```toml
name = "codex-tui"
version = "0.140.0+hermione"
```

Во время `0.137.0` migration commit `671afe3ee` также нормализовал многие
workspace packages в lockfile с `0.0.0` на `0.137.0`; это lockfile formatting
noise, который не следует путать с Hermione metadata.

Во время переноса на `0.140.0` workspace packages в `Cargo.lock` должны быть
синхронизированы с upstream workspace version `0.140.0`, а `codex-cli` и
`codex-tui` должны остаться единственными shipped crates с suffix
`+hermione`.

### 3. Исправить CLI update parsing

В `codex-rs/cli/src/doctor/updates.rs` в `parse_version` сначала trim value,
затем отрезать build metadata:

```rust
let version = value.trim();
let base_version = version.split_once('+').map_or(version, |(base, _)| base);
let mut parts = base_version.split('.');
```

Остальная логика parsing major/minor/patch остается прежней.

### 4. Исправить TUI update parsing

В `codex-rs/tui/src/update_versions.rs` сделать тот же base version extraction:

```rust
let version = v.trim();
let base_version = version.split_once('+').map_or(version, |(base, _)| base);
let mut iter = base_version.split('.');
```

### 5. Зафиксировать TUI compile-time version

В `codex-rs/tui/src/version.rs`:

```rust
/// The current Codex CLI version as embedded at compile time.
#[cfg(not(test))]
pub const CODEX_CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
pub const CODEX_CLI_VERSION: &str = "0.0.0";
```

Это нужно, чтобы tests не зависели от текущего package version.

## Регрессионное покрытие

Покрытие, которое должно быть в diff:

- `codex-rs/cli/src/doctor/updates.rs`:
  - `is_newer_ignores_build_metadata`;
  - проверяет `parse_version("0.133.0+hermione") == Some((0, 133, 0))`;
  - проверяет newer/equal comparison.
- `codex-rs/tui/src/update_versions.rs`:
  - `build_metadata_is_ignored_for_version_comparison`;
  - проверяет тот же parsing и comparison.
- Existing tests для plain semver, prerelease и whitespace должны остаться.

## Проверки

Для повторения доработки:

1. На `f-ms-dev:/home/slader/Projects/codex`, если пользователь разрешил:
   - целевые тесты для CLI/TUI update version modules;
   - `just build-fast-release`;
   - проверить `codex --version` установленного binary, если задача включает
     install.
2. Локально без Rust/Cargo:
   - `rg -n "0\\.140\\.0\\+hermione|split_once\\('\\+'\\)|CARGO_PKG_VERSION" codex-rs`;
   - `git diff --check`.

## Ограничения

- Build metadata `+hermione` не должен попадать в upstream tag parsing.
  Upstream latest tags имеют вид `rust-v0.140.0`; `extract_version_from_latest_tag`
  по-прежнему отрезает `rust-v`.
- Не менять workspace package version глобально ради Hermione. Marking нужен
  именно для shipped CLI/TUI crates.
- При следующем upstream release нельзя забыть обновить оба manifests и
  lockfile одновременно.

## Риски

- Если обновить только `codex-cli`, TUI может показывать upstream version или
  tests/snapshots начнут расходиться.
- Если update-check не отрезает build metadata, Hermione build может выглядеть
  unparsable и не получать нормальный update status.
- Lockfile diff может быть большим после upstream migration. Нужно отделять
  semantic Hermione version metadata от Cargo formatting/version normalization.

## Сводка покрытия

| Пункт | Статус | Где отражено |
| --- | --- | --- |
| Explicit `0.140.0+hermione` для CLI | перенесено | "Итоговый контракт", "Пошаговое воспроизведение" |
| Explicit `0.140.0+hermione` для TUI | перенесено | "Итоговый контракт", "Пошаговое воспроизведение" |
| Ignore build metadata in update comparison | перенесено | "Итоговый контракт", "Регрессионное покрытие" |
| Stable TUI test version | перенесено | "Пошаговое воспроизведение" |
| Lockfile migration nuance | перенесено | "Пошаговое воспроизведение", "Риски" |
