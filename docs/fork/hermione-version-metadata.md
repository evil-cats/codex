---
id: fork-hermione-version-metadata
status: active
created: 2026-06-08
updated: 2026-08-13
---

# Hermione version metadata

## Обзор

Эта карточка фиксирует fork-доработку, которая маркирует CLI и TUI builds как
Hermione через Cargo build metadata: `0.146.0+hermione`.

Карточка также фиксирует update-check изменения: сравнение upstream versions
должно игнорировать build metadata после `+`, чтобы `0.146.0+hermione`
сравнивался как `0.146.0`.

## Зачем это нужно

Hermione fork должен быть отличим от upstream binary при диагностике,
`--version`, install workflow и локальном сравнении сборок. При этом update
logic должен продолжать понимать, что base semver остается upstream-compatible:
`0.146.0+hermione` не должен ломать сравнение с `rust-v0.147.0` или
`rust-v0.146.0`.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/Cargo.toml` | Держит `workspace.package.version = "0.146.0"` без Hermione metadata |
| `codex-rs/cli/Cargo.toml` | Задаёт explicit `version = "0.146.0+hermione"` для `codex-cli` |
| `codex-rs/tui/Cargo.toml` | Задаёт explicit `version = "0.146.0+hermione"` для `codex-tui` |
| `codex-rs/Cargo.lock` | Фиксирует `codex-cli` и `codex-tui` как `0.146.0+hermione`, остальные package-блоки workspace без `source` как `0.146.0` |
| `codex-rs/cli/src/doctor/updates.rs` | Игнорирует build metadata при CLI doctor update comparison |
| `codex-rs/tui/src/update_versions.rs` | Игнорирует build metadata при TUI update comparison |
| `codex-rs/tui/src/version.rs` | Использует `env!("CARGO_PKG_VERSION")` вне tests и stable test value |

## Итоговый контракт

1. `codex-cli` больше не должен наследовать `version.workspace = true`.
2. `codex-tui` больше не должен наследовать `version.workspace = true`.
3. В обоих manifests должна стоять explicit version текущего upstream release с
   Hermione metadata:

   ```toml
   version = "0.146.0+hermione"
   ```

4. При следующем upstream release нужно менять base part:
   - `0.146.0+hermione` -> `X.Y.Z+hermione`, если base tag `rust-vX.Y.Z`.
5. `workspace.package.version` должен оставаться обычной upstream base version
   без суффикса `+hermione`, сейчас `0.146.0`.
6. `Cargo.lock` должен отражать явно заданные версии `0.146.0+hermione` для
   `codex-cli` и `codex-tui`; остальные package-блоки workspace без `source`,
   наследующие `workspace.package.version`, должны иметь обычную версию
   `0.146.0`.
7. Version comparison для update checks должен отрезать suffix после `+` перед
   parsing semver triplet.
8. `0.133.0+hermione` должен парситься как `(0, 133, 0)`.
9. `is_newer("0.134.0", "0.133.0+hermione")` должен быть `Some(true)`.
10. `is_newer("0.133.0", "0.133.0+hermione")` должен быть `Some(false)`.
11. Pre-release values вроде `0.11.0-beta.1` по-прежнему не должны
    интерпретироваться как обычный semver triplet.
12. `codex-rs/tui/src/version.rs` должен использовать package version at
    compile time, но в tests давать стабильное `CODEX_CLI_VERSION = "0.0.0"`.

## Архитектурное решение

Fork identity закреплена только за выпускаемыми пакетами CLI и TUI, а workspace
version остаётся upstream-совместимой. Отображение сохраняет `+hermione`, но
update parsing удаляет metadata только на границе сравнения версий. Так
диагностика отличает Hermione binary, не меняя версии остальных workspace crates
и semver-порядок upstream releases.

## Порядок повторения при переносе

### 1. Обновить manifests

В `codex-rs/cli/Cargo.toml`:

```toml
[package]
name = "codex-cli"
version = "0.146.0+hermione"
```

В `codex-rs/tui/Cargo.toml`:

```toml
[package]
name = "codex-tui"
version = "0.146.0+hermione"
```

Не оставлять `version.workspace = true` для этих двух crates.

### 2. Обновить lockfile

После изменения manifests обновить `codex-rs/Cargo.lock` через fork workflow.
В lockfile должны быть:

```toml
name = "codex-cli"
version = "0.146.0+hermione"
```

и:

```toml
name = "codex-tui"
version = "0.146.0+hermione"
```

`Cargo.lock` должен различать два вида package-блоков workspace без поля
`source`:

- `codex-cli` и `codex-tui` — два пакета с явно заданной версией
  `0.146.0+hermione`;
- остальные пакеты workspace, наследующие `workspace.package.version`, — с
  обычной base version без `+hermione`.

Внешние пакеты с полем `source`, их `checksum` и списки `dependencies` такая
нормализация не изменяет.

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

## Проверки

Исполняемая карта card-level regression tests:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "CLI package metadata, --version и update comparison с +hermione",
      "argv": ["just", "test", "-p", "codex-cli"]
    },
    {
      "purpose": "TUI compile-time version и update comparison с +hermione",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-tui",
        "--",
        "--skip",
        "ide_context::ipc::tests::fetch_ide_context_uses_unregistered_request_route"
      ]
    }
  ]
}
```

Дополнительно обязателен `fork build-fast`: он подтверждает package metadata
фактически собранного CLI binary.

## Риски и ограничения

### Ограничения

- Build metadata `+hermione` не должен попадать в upstream tag parsing.
  Upstream latest tags имеют вид `rust-v0.146.0`; `extract_version_from_latest_tag`
  по-прежнему отрезает `rust-v`.
- Не менять workspace package version глобально ради Hermione. Marking нужен
  именно для shipped CLI/TUI crates.
- При следующем upstream release нельзя забыть обновить оба manifests и
  lockfile одновременно.

### Риски

- Если обновить только `codex-cli`, TUI может показывать upstream version или
  tests/snapshots начнут расходиться.
- Если update-check не отрезает build metadata, Hermione build может выглядеть
  unparsable и не получать нормальный update status.
- Lockfile diff может быть большим после upstream migration. Нужно отделять
  semantic Hermione version metadata от Cargo formatting/version normalization.
