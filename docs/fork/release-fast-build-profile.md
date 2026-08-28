---
id: fork-release-fast-build-profile
status: active
created: 2026-06-08
updated: 2026-08-24
---

# Release-fast build and runtime install

## Обзор

Эта карточка фиксирует быстрый optimized build path Hermione и установку его
runtime-комплекта. Cargo profile `release-fast` сохраняет параллельность
финальных стадий и symbols для профилирования, skill-owned `fork build-fast`
собирает и проверяет `codex` вместе с `codex-code-mode-host`, а `fork install`
выполняет `strip` staged copies и согласованно устанавливает оба файла локально
либо через `rsync` на явно перечисленные SSH-хосты, не меняя canonical upstream
`release` profile.

## Зачем это нужно

Upstream `release` является профилем для packaging и symbolication:

- `debug = "line-tables-only"`;
- `split-debuginfo = "off"`;
- `strip = false`.

Hermione нужен отдельный быстрый optimized profile с большей параллельностью
финальных стадий. Build artifacts должны сохранять symbols, чтобы `perf` мог
показывать Rust-функции без дорогой debug-сборки. Runtime-копии при этом должны
оставаться stripped, поэтому граница удаления symbols переносится из Cargo
profile в skill-owned install staging.

Code Mode исполняется отдельным `codex-code-mode-host`, который основной Codex
лениво запускает как sidecar. Обычная локальная сборка и установка только
`codex-hermione` оставляет Code Mode без запускаемого host, поэтому build и
install gates должны работать с этой парой как с одним runtime-комплектом.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/Cargo.toml` | Добавляет `[profile.release-fast]` |
| `justfile` | Сохраняет внутреннюю цель `build-fast-release`, собирающую оба runtime binaries |
| `.codex/skills/fork/scripts/fork_cli.py` | Проверяет оба build artifacts, выполняет `strip` staged copies и устанавливает их согласованной парой локально либо через `rsync`/SSH |
| `.codex/skills/fork/scripts/fork_cli_tests.py` | Проверяет пути, staging, remote hosts и общий `rsync` contract |
| `.codex/skills/fork/references/checks-and-gates.md` | Описывает публичный skill-owned build/install contract |
| `codex-rs/install-context/src/lib.rs` | Upstream integration point: ищет `codex-code-mode-host` среди ресурсов или рядом с текущим executable |
| `scripts/codex_package/targets.py` | Upstream source of truth для поддерживаемых native Cargo targets |
| `scripts/codex_package/v8.py` | Upstream resolver проверенных V8 archive и generated binding для Cargo build |

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
# Keep the optimized build artifact available for local profiling. The
# skill-owned install command strips staged copies before installing runtime
# binaries with rsync.
debug = "none"
strip = false
```

### Just target

Корневой `justfile` должен сохранять target `build-fast-release`. Этот target
остаётся деталью реализации для skill-owned `fork build-fast`: он одной Cargo
сборкой обрабатывает packages `codex-cli` и `codex-code-mode-host` с profile
`release-fast`, но не является нормативной workflow-командой карточки.
Перед его запуском `fork build-fast` определяет native target через rustc и
переиспользует upstream `scripts.codex_package.v8` для передачи Cargo
согласованной пары `RUSTY_V8_ARCHIVE` и `RUSTY_V8_SRC_BINDING_PATH`. Прямой
fallback к архивам `denoland/rusty_v8` не является частью release-fast
контракта.

Артефакты сборки:

```text
codex-rs/target/release-fast/codex
codex-rs/target/release-fast/codex-code-mode-host
```

Оба артефакта должны существовать, быть executable и сохранять symbols.
Проверка основного binary использует version probe, а host — help probe,
поскольку `codex-code-mode-host` не публикует отдельный version flag. Эти
build artifacts являются источником для профилирования и не устанавливаются
напрямую без skill-owned staging.

### Install contract

По умолчанию `fork install` переносит основной artifact в
`${HOME}/.local/bin/codex-hermione`, а Code Mode host — в тот же каталог под
каноническим именем `codex-code-mode-host`. Повторяемый `--host HOST` переключает
команду с локальной установки на установку только на явно перечисленные
SSH-хосты. Remote target по умолчанию равен `.local/bin/codex-hermione`
относительно login home; authentication, aliases и host keys остаются в SSH
configuration пользователя. Если основной source или target переопределён,
путь Code Mode host автоматически выводится как канонический сосед.

До изменения установленных файлов workflow копирует оба build artifacts во
временные файлы, выполняет над staged copies `strip` и проверяет их. Затем один
общий путь `rsync --delay-updates` доставляет оба staged файла в локальный каталог
либо на каждый выбранный SSH-хост. Remote-режим после обновления запускает probes
по SSH. `rsync` не публикует частично переданный файл под рабочим именем, но два
binary не образуют общую filesystem-транзакцию. При нескольких SSH-хостах каждый
host завершается отдельно; общей cross-host транзакции нет.

## Архитектурное решение

`release-fast` остаётся отдельным Cargo profile, наследующим upstream `release`,
а `build-fast-release` служит внутренней целью за skill-owned gate
`fork build-fast`. Это отделяет upstream packaging profile от локального
optimized runtime-комплекта и даёт fork workflow одну стабильную границу
сборки.

Build и install моделируют основной binary и sidecar явными artifact
contracts. Для каждого контракта определены каноническое имя и безопасный probe;
общий install алгоритм оставляет build sources нетронутыми, выполняет `strip`
только над staged copies и делегирует публикацию завершённых локальных и remote
передач `rsync --delay-updates`. Namespace или runtime-логика Code Mode для этого
не меняются.

V8 artifacts не копируются и не описываются fork workflow самостоятельно.
Skill-owned gate вызывает upstream resolver, поэтому checksum, release URL,
cache и соответствие generated binding остаются в одном source of truth с
canonical package builder.

## Порядок повторения при переносе

### 1. Добавить profile

В root workspace `codex-rs/Cargo.toml` рядом с `[profile.release]` добавить
`[profile.release-fast]`, наследующий `release`.

Не менять upstream `release`: он остаётся canonical profile для upstream
workflow упаковки. Hermione `release-fast` должен переопределять только
fork-specific настройки быстрой optimized-сборки и сохранять symbols до
skill-owned install staging.

### 2. Проверить owned target в `justfile`

В root `justfile` должен оставаться target `build-fast-release`. Он живёт рядом
с release/build targets, чтобы skill-owned `fork build-fast` имел стабильный
внутренний build target и не зависел от ручной команды в карточке.

При переносе нужно подтвердить, что target продолжает собирать и `codex-cli`, и
`codex-code-mode-host` одним profile `release-fast`.

### 3. Восстановить skill-owned artifact contract

В `fork_cli.py` сохранить два build artifacts: основной `codex` с version probe
и `codex-code-mode-host` с help probe. `fork build-fast` должен проверять
существование, executable metadata и probe обоих файлов.

До внутренней Cargo-сборки определить native rustc target и получить environment
overrides через upstream V8 resolver. Не копировать URL, checksum или правила
cache в fork skill: при изменении upstream packaging обновляется integration
point, а не создаётся второй V8 downloader.

Для `fork install` сохранить вывод Code Mode host source/target из основных
путей, `strip` staged copies и проверку обоих файлов. Локальная и remote
установки должны использовать общий `rsync --delay-updates` path. Повторяемый
`--host` использует SSH configuration пользователя, не добавляет hardcoded hosts
и не выполняет неявную локальную установку. При изменении этого порядка
синхронизировать unit tests и публичное описание в `checks-and-gates.md`.

### 4. Проверить build и install contracts

Skill-owned `fork build-fast` должен создать оба release-fast artifacts с
symbols. Проверка установки должна подтвердить, что build sources не изменены,
staged copies прошли `strip`, а основной binary и канонически названный Code Mode
host оказываются в одном целевом каталоге локально либо на каждом выбранном
SSH-хосте.

## Проверки

Исполняемая карта card-level regression tests:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "парный build/install contract для codex и Code Mode host",
      "argv": [
        "python3",
        ".codex/skills/fork/scripts/fork_cli_tests.py",
        "ReleaseFastWorkflowTests"
      ]
    }
  ]
}
```

Дополнительно обязателен skill-owned gate `fork build-fast`: он подтверждает
реальное появление и probes обоих optimized artifacts с symbols.

## Риски и ограничения

### Ограничения

- Не заменять upstream `release` profile: он нужен для canonical
  release-артефакта.
- Не использовать canonical release build path для обычной Hermione compile-check:
  он проверяет upstream release profile, а не быстрый fork build path.
- Не выполнять `strip` над build sources: удаление symbols принадлежит skill-owned
  install staging и не должно менять профилируемые artifacts.
- Не переименовывать установленный host: `InstallContext` ищет канонический
  `codex-code-mode-host` рядом с основным executable.
- Не запускать `rsync` до проверки обеих staged copies: основной binary и
  sidecar устанавливаются только как заранее проверенный комплект.
- Не возвращать release-fast Cargo build к неуправляемой загрузке
  `denoland/rusty_v8`: для Codex V8 profile требуются согласованные OpenAI
  archive и generated binding.

### Риски

- Если `release-fast` не наследует `release`, build может отличаться слишком
  сильно от shipped optimized behavior.
- Если `codegen-units` снова станет `1`, profile потеряет смысл.
- Если `fork install` перестанет выполнять `strip`, runtime-копии сохранят
  symbols и вырастут до непрактичного размера.
- `rsync --delay-updates` не превращает два binary в общую транзакцию;
  гарантия ограничена публикацией только полностью переданного отдельного файла.
- Установка на несколько SSH-хостов не является cross-host транзакцией: ошибка
  более позднего host не откатывает уже завершённые установки.
- Remote install требует совместимого OS/architecture target. Локальный staged
  probe не доказывает совместимость remote runtime, а post-install probe сообщает
  о ней уже после публикации файлов через `rsync`.
- Если upstream добавит host version flag или изменит способ discovery sidecar,
  probes, каноническое имя и install contract нужно пересмотреть вместе.
- Если native rustc target отсутствует в upstream `TARGET_SPECS` или V8 release
  pair ещё не опубликована, `fork build-fast` должен завершиться ошибкой до
  проверки binaries, а не переходить на непроверенный artifact.
