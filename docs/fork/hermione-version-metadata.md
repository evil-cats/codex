---
id: fork-hermione-version-metadata
status: active
created: 2026-06-08
updated: 2026-07-29
source_scope: rust-v0.146.0..hermione-0.146.0
---

# Hermione version metadata

## Обзор

Эта карточка фиксирует fork-доработку, которая маркирует CLI и TUI builds как
Hermione через Cargo build metadata: `0.146.0+hermione`.

Карточка также фиксирует update-check изменения: сравнение upstream versions
должно игнорировать build metadata после `+`, чтобы `0.146.0+hermione`
сравнивался как `0.146.0`.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Основные commits | `33e804a51`, `67067a90f`, перенос через merge до `671afe3ee`, перенос на `0.140.0`, перенос на `0.141.0`, перенос на `0.142.5`, перенос на `0.143.0`, перенос на `0.144.1`, перенос на `0.144.4`, перенос на `0.144.5`, перенос на `0.144.6`, перенос на `0.145.0`, перенос на `0.146.0` |
| Текущая версия ветки | `0.146.0+hermione` |
| Затронутые crates | `codex-cli`, `codex-tui` |
| Checkpoint перед карточкой | Пропущен по явному разрешению пользователя от 2026-06-08 |

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

## Пошаговое воспроизведение

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

После изменения manifests обновить `codex-rs/Cargo.lock` через общий fork
workflow, если пользователь разрешил Rust/Cargo command. В lockfile должны быть:

```toml
name = "codex-cli"
version = "0.146.0+hermione"
```

и:

```toml
name = "codex-tui"
version = "0.146.0+hermione"
```

Во время `0.137.0` migration commit `671afe3ee` также нормализовал многие
workspace packages в lockfile с `0.0.0` на `0.137.0`; это lockfile formatting
noise, который не следует путать с Hermione metadata.

Во время переноса на `0.146.0` `Cargo.lock` должен различать два вида
package-блоков workspace без поля `source`:

- `codex-cli` и `codex-tui` — два пакета с явно заданной версией
  `0.146.0+hermione`;
- остальные 130 пакетов workspace, наследующие `workspace.package.version`, — с
  обычной версией `0.146.0`.

`codex-exec-server-test-support` и `codex-git-attribution` — два пакета,
добавленные upstream; они также наследуют `workspace.package.version`. Их
оставшиеся после автоматического слияния lockfile-версии `0.0.0` нужно
нормализовать до `0.146.0`.

После нормализации package-блоков без `source` с версиями `0.145.0` и `0.0.0`
оставаться не должно. Внешние пакеты с полем `source`, их `checksum` и списки
`dependencies` эта нормализация не изменяет.

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

### Смысловое покрытие

Обязательное покрытие для diff:

- `codex-rs/cli/src/doctor/updates.rs`:
  - `is_newer_ignores_build_metadata`;
  - проверяет `parse_version("0.133.0+hermione") == Some((0, 133, 0))`;
  - проверяет сравнение newer/equal.
- `codex-rs/tui/src/update_versions.rs`:
  - `build_metadata_is_ignored_for_version_comparison`;
  - проверяет тот же parsing и comparison.
- Существующие tests для plain semver, prerelease и whitespace должны остаться.
- Доказательство version metadata должно подтверждать, что:
  - `codex-cli` и `codex-tui` имеют explicit
    `version = "0.146.0+hermione"`;
  - `Cargo.lock` фиксирует `codex-cli` и `codex-tui` как
    `0.146.0+hermione`;
  - `workspace.package.version` остается обычной upstream base version
    `0.146.0`, без суффикса `+hermione`;
  - остальные 130 package-блоков workspace без `source` имеют обычную версию
    `0.146.0`;
  - package-блоки без `source` с версиями `0.145.0` и `0.0.0` отсутствуют;
  - внешние пакеты с полем `source`, их `checksum` и списки `dependencies` не
    изменены;
  - TUI version constant берет package version at compile time, но в tests
    остается стабильным `CODEX_CLI_VERSION = "0.0.0"`.

### Владелец исполняемой карты

Проверки уровня карточки запускает skill-owned `fork tests`.
Внутренние argv живут в блоке `fork-tests.v1` ниже; карточка хранит их как
данные исполняемой карты, а не как нормативный runbook ручного запуска.

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "cli metadata",
      "argv": ["just", "test", "-p", "codex-cli"]
    },
    {
      "purpose": "tui metadata",
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

### Дополнительные gates

Для общего проверочного прохода родительского агента нужны:

- проверки CLI/TUI уровня карточки через владельца исполняемой карты `fork tests`;
- fast build/version evidence через skill-owned build gate;
- проверка `codex --version` установленного binary только если задача включает
  install workflow;
- текстовая сверка manifests, lockfile, `split_once('+')` и
  `CARGO_PKG_VERSION`, если Rust/Cargo запуск недоступен или отложен.

Точные внутренние argv для card-level проверок не повторяются здесь как runbook:
они принадлежат блоку `fork-tests.v1` выше.

### Исторические результаты

Старый формат карточки при переносе на `0.144.1` хранил следующий checklist как
исторический контекст.
Эти команды и маршруты оставлены только как historical evidence; актуальный
запуск проверок принадлежит skill-owned workflow.

1. На `f-ms-dev:/home/slader/Projects/codex`, если пользователь разрешил:
   - целевые тесты для CLI/TUI update version modules;
   - `just build-fast-release`;
   - проверить `codex --version` установленного binary, если задача включает
     install.
2. Локально без Rust/Cargo:
   - `rg -n "0\\.144\\.1\\+hermione|split_once\\('\\+'\\)|CARGO_PKG_VERSION" codex-rs`;
   - `git diff --check`.

В этой карточке не было зафиксировано свежих stdout/stderr, log path, timestamp
или pass/fail результата для этих исторических команд. Build/version evidence
зафиксировано как contract: shipped CLI/TUI crates должны показывать
`0.144.1+hermione`, а update comparison должен сравнивать base version без
build metadata.

В общем проверочном проходе migration `0.144.6` card-level проверки CLI и TUI
прошли через `fork tests`, а `fork build-fast` собрал
`codex-rs/target/release-fast/codex`. Отдельный запуск артефакта вернул
`codex-cli 0.144.6+hermione`; SHA-256 собранного файла:
`07dfa949a0d78f485186c06fc1dc7024beddf471522a7f41891c2097ba626880`.
Установка не выполнялась, поэтому это подтверждение относится к build artifact,
а не к `${HOME}/.local/bin/codex-hermione`.

### Известные падения и пропуски

- TUI argv уровня карточки сохраняет known skip:
  `ide_context::ipc::tests::fetch_ide_context_uses_unregistered_request_route`.
- Checkpoint перед карточкой пропущен по явному разрешению пользователя от
  2026-06-08.
- Install/version check требуется только если задача включает install workflow.
- Свежие сборка, тесты, генераторы, форматирование и `fix` в рамках этой
  subagent-карточки не запускались; их выполняет общий проверочный проход
  родительского агента.

## Ограничения

- Build metadata `+hermione` не должен попадать в upstream tag parsing.
  Upstream latest tags имеют вид `rust-v0.146.0`; `extract_version_from_latest_tag`
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

## Проверка покрытия

| Пункт | Статус | Где отражено |
| --- | --- | --- |
| Upstream `0.146.0` для workspace version | перенесено | "Итоговый контракт", "Карта файлов" |
| Explicit `0.146.0+hermione` для CLI | перенесено | "Итоговый контракт", "Пошаговое воспроизведение" |
| Explicit `0.146.0+hermione` для TUI | перенесено | "Итоговый контракт", "Пошаговое воспроизведение" |
| Ignore build metadata in update comparison | перенесено | "Итоговый контракт", "Проверки" |
| Stable TUI test version | перенесено | "Пошаговое воспроизведение" |
| Два пакета с явно заданным `+hermione` и 130 остальных package-блоков workspace без `source` | перенесено | "Пошаговое воспроизведение", "Проверки" |
| Card-level проверки принадлежат `fork tests` | перенесено | "Проверки" |
| Исторические команды не являются runbook | перенесено | "Проверки" |
