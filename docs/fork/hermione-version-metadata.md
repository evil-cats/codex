---
id: fork-hermione-version-metadata
status: active
created: 2026-06-08
updated: 2026-09-04
---

# Метаданные версии Hermione

## Обзор

Эта карточка фиксирует fork-доработку, которая маркирует сборки CLI и TUI как
Hermione через метаданные сборки Cargo: `0.153.2+hermione`.

Карточка также фиксирует изменения проверки обновлений: сравнение версий
upstream должно игнорировать метаданные сборки после `+`, чтобы `0.153.2+hermione`
сравнивался как `0.153.2`.

## Зачем это нужно

Fork Hermione должен отличаться от исполняемого файла upstream при диагностике,
в выводе `--version` и локальном сравнении сборок. При этом
логика проверки обновлений должна учитывать, что базовая SemVer-версия остаётся
совместимой с upstream:
`0.153.2+hermione` не должен ломать сравнение с `rust-v0.154.0` или
`rust-v0.153.2`.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/Cargo.toml` | Держит `workspace.package.version = "0.153.2"` без метаданных Hermione |
| `codex-rs/cli/Cargo.toml` | Явно задаёт `version = "0.153.2+hermione"` для `codex-cli` |
| `codex-rs/tui/Cargo.toml` | Явно задаёт `version = "0.153.2+hermione"` для `codex-tui` |
| `codex-rs/Cargo.lock` | Фиксирует `codex-cli` и `codex-tui` как `0.153.2+hermione`, а 147 текущих source-less пакетов, наследующих workspace-версию, — как `0.153.2` |
| `codex-rs/cli/src/main.rs` | Закрепляет имя `codex` и текущую Cargo-версию пакета в выводе `--version` |
| `codex-rs/cli/src/doctor/updates.rs` | Игнорирует метаданные сборки при сравнении версий в `doctor` CLI |
| `codex-rs/tui/src/update_versions.rs` | Игнорирует метаданные сборки при сравнении версий в TUI |
| `codex-rs/tui/src/version.rs` | Использует `env!("CARGO_PKG_VERSION")` вне тестов и стабильное тестовое значение |

## Итоговый контракт

1. `codex-cli` больше не должен наследовать `version.workspace = true`.
2. `codex-tui` больше не должен наследовать `version.workspace = true`.
3. В обоих манифестах должна быть явно задана версия текущего выпуска upstream с
   метаданными Hermione:

   ```toml
   version = "0.153.2+hermione"
   ```

4. При следующем выпуске upstream нужно менять базовую часть:
   - `0.153.2+hermione` -> `X.Y.Z+hermione`, если базовый tag равен `rust-vX.Y.Z`.
5. `workspace.package.version` должен оставаться обычной базовой версией upstream
   без суффикса `+hermione`, сейчас `0.153.2`.
6. `Cargo.lock` должен отражать явно заданные версии `0.153.2+hermione` для
   `codex-cli` и `codex-tui`. Остальные 147 текущих локальных package-блока без
   `source` наследуют `workspace.package.version` и должны иметь базовую версию
   `0.153.2`; stale-значений `0.0.0` среди них быть не должно. Если source-less
   crate действительно объявляет собственную версию в package-манифесте, при
   нормализации нужно сохранить именно её.
7. Сравнение версий при проверке обновлений должно отрезать суффикс после `+`
   перед разбором тройки SemVer.
8. `0.133.0+hermione` должен парситься как `(0, 133, 0)`.
9. `is_newer("0.134.0", "0.133.0+hermione")` должен быть `Some(true)`.
10. `is_newer("0.133.0", "0.133.0+hermione")` должен быть `Some(false)`.
11. Предварительные версии вроде `0.11.0-beta.1` по-прежнему не должны
    интерпретироваться как обычная тройка SemVer.
12. `codex-rs/tui/src/version.rs` должен использовать версию пакета во время
    компиляции, но в тестах давать стабильное `CODEX_CLI_VERSION = "0.0.0"`.
13. Первая строка `codex --version` должна показывать
    `codex 0.153.2+hermione`: имя пользовательской команды остаётся `codex`, а
    версия берётся из метаданных пакета `codex-cli`. Имя Cargo-пакета
    `codex-cli` не должно попадать в эту строку. Вторая строка имеет формат
    `revision <value>` и принадлежит контракту происхождения runtime-комплекта.

## Архитектурное решение

Метка fork закреплена только за выпускаемыми пакетами CLI и TUI, а версия
workspace остаётся совместимой с upstream. Отображение сохраняет `+hermione`, но
разбор при проверке обновлений удаляет метаданные только на границе сравнения
версий. Так диагностика отличает исполняемый файл Hermione, не добавляя
`+hermione` к crates, наследующим workspace-версию, и не меняя SemVer-порядок
выпусков upstream.

Макрос `Parser` из Clap по умолчанию формирует внутреннее имя команды из
`CARGO_PKG_NAME`, а `DisplayVersion` выбирает `display_name` либо это внутреннее имя.
Поэтому корневая команда должна сохранять оба атрибута: `bin_name = "codex"`
закрепляет имя в справке, а `display_name = "codex"` — в первой строке вывода
версии. `multitool_command` добавляет отдельную вторую строку
`revision <value>`; значение SemVer при этом не меняется, а корневой `--help`
ревизию не показывает.

## Порядок повторения при переносе

### 1. Обновить манифесты

В `codex-rs/cli/Cargo.toml`:

```toml
[package]
name = "codex-cli"
version = "0.153.2+hermione"
```

В `codex-rs/tui/Cargo.toml`:

```toml
[package]
name = "codex-tui"
version = "0.153.2+hermione"
```

Не оставлять `version.workspace = true` для этих двух crates.

### 2. Обновить lockfile

После изменения манифестов обновить `codex-rs/Cargo.lock` через fork workflow.
В lockfile должны быть:

```toml
name = "codex-cli"
version = "0.153.2+hermione"
```

и:

```toml
name = "codex-tui"
version = "0.153.2+hermione"
```

Текущий `Cargo.lock` должен различать два вида package-блоков workspace без поля
`source`:

- `codex-cli` и `codex-tui` — два пакета с явно заданной версией
  `0.153.2+hermione`;
- остальные 147 пакетов, наследующие `workspace.package.version`, — с базовой
  версией `0.153.2`.

Принадлежность к этим категориям определяется package-манифестом, а не старым
значением в lockfile. Если в будущем source-less crate будет действительно
объявлять собственную версию, нормализация должна сохранить это явное значение.

Внешние пакеты с полем `source`, их `checksum` и списки `dependencies` такая
нормализация не изменяет.

### 3. Исправить разбор версии при проверке обновлений CLI

В `codex-rs/cli/src/doctor/updates.rs` в `parse_version` сначала вызвать `trim()`
для значения, затем отрезать метаданные сборки:

```rust
let version = value.trim();
let base_version = version.split_once('+').map_or(version, |(base, _)| base);
let mut parts = base_version.split('.');
```

Остальная логика разбора major/minor/patch остаётся прежней.

### 4. Исправить разбор версии при проверке обновлений TUI

В `codex-rs/tui/src/update_versions.rs` так же выделить базовую версию:

```rust
let version = v.trim();
let base_version = version.split_once('+').map_or(version, |(base, _)| base);
let mut iter = base_version.split('.');
```

### 5. Зафиксировать версию TUI во время компиляции

В `codex-rs/tui/src/version.rs`:

```rust
/// The current Codex CLI version as embedded at compile time.
#[cfg(not(test))]
pub const CODEX_CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
pub const CODEX_CLI_VERSION: &str = "0.0.0";
```

Это нужно, чтобы тесты не зависели от текущей версии пакета.

### 6. Закрепить имя в выводе версии CLI

В Clap-атрибутах `MultitoolCli` сохранить оба имени:

```rust
bin_name = "codex",
display_name = "codex",
```

`bin_name` не заменяет `display_name`: вывод `DisplayVersion` использует
`display_name` или внутреннее имя команды, которое макрос `Parser` из Clap по
умолчанию берёт из `CARGO_PKG_NAME`.

## Проверки

Исполняемая карта регрессионных тестов уровня карточки:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "Имя команды, метаданные CLI и сравнение обновлений с +hermione",
      "argv": ["just", "test", "-p", "codex-cli"]
    },
    {
      "purpose": "Версия TUI во время компиляции и сравнение обновлений с +hermione",
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

## Риски и ограничения

### Ограничения

- Метаданные сборки `+hermione` не должны попадать в разбор tag upstream.
  Актуальные tags upstream имеют вид `rust-v0.153.2`;
  `extract_version_from_latest_tag` по-прежнему отрезает `rust-v`.
- Не менять версию пакета workspace глобально ради Hermione. Маркировка нужна
  именно для выпускаемых crates CLI и TUI.
- При следующем выпуске upstream нельзя забыть обновить оба манифеста и
  lockfile одновременно.

### Риски

- Если обновить только `codex-cli`, TUI может показывать версию upstream или
  тесты и snapshots начнут расходиться.
- Если оставить только `bin_name`, Clap покажет в `--version` имя Cargo-пакета
  `codex-cli`, хотя строка использования и справка продолжат показывать `codex`.
- Если проверка обновлений не отрезает метаданные сборки, сборка Hermione может
  оказаться неразбираемой и не получать корректный статус обновления.
- Если принять отличающееся значение lockfile за собственную версию без сверки
  package-манифеста, наследующий workspace-версию crate может сохранить stale
  значение вместо текущей базовой версии.
- Разница в lockfile может быть большой после миграции upstream. Нужно отделять
  смысловые изменения метаданных версии Hermione от форматирования Cargo и
  нормализации версий.
