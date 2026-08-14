---
id: fork-release-fast-build-profile
status: active
created: 2026-06-08
updated: 2026-08-14
---

# Release-fast build and local install

## Обзор

Эта карточка фиксирует быстрый optimized build path Hermione и локальную
установку его runtime-комплекта. Cargo profile `release-fast` сохраняет
параллельность финальных стадий и создаёт stripped artifacts, skill-owned
`fork build-fast` собирает и проверяет `codex` вместе с
`codex-code-mode-host`, а `fork install` устанавливает оба файла рядом, не
меняя canonical upstream `release` profile.

## Зачем это нужно

Upstream `release` является профилем для packaging и symbolication:

- `debug = "line-tables-only"`;
- `split-debuginfo = "off"`;
- `strip = false`.

Hermione нужен отдельный быстрый optimized profile с большей параллельностью
финальных стадий и готовыми stripped-артефактами. Если `release-fast` наследует
upstream-настройки `debug` и `strip` без переопределения, binary становится
unstripped и может вырасти до непрактичного размера.

Code Mode исполняется отдельным `codex-code-mode-host`, который основной Codex
лениво запускает как sidecar. Обычная локальная сборка и установка только
`codex-hermione` оставляет Code Mode без запускаемого host, поэтому build и
install gates должны работать с этой парой как с одним runtime-комплектом.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/Cargo.toml` | Добавляет `[profile.release-fast]` |
| `justfile` | Сохраняет внутреннюю цель `build-fast-release`, собирающую оба runtime binaries |
| `.codex/skills/fork/scripts/fork_cli.py` | Проверяет оба build artifacts и устанавливает их согласованной парой |
| `.codex/skills/fork/scripts/fork_cli_tests.py` | Проверяет вывод путей, обязательность host и порядок замены при install |
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
# This profile is used for Hermione's local install artifact. Upstream release
# keeps line tables for symbolication before packaging; release-fast should be
# ready to install directly after the release-fast build.
debug = "none"
strip = "symbols"
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

Оба артефакта должны существовать, быть executable и stripped. Проверка
основного binary использует version probe, а host — help probe, поскольку
`codex-code-mode-host` не публикует отдельный version flag. Проверочная команда
`file` не должна показывать `with debug_info, not stripped` для результатов
skill-owned build gate.

### Install contract

По умолчанию `fork install` переносит основной artifact в
`${HOME}/.local/bin/codex-hermione`, а host — в тот же каталог под каноническим
именем `codex-code-mode-host`. Если основной source или target переопределён,
путь host автоматически выводится как канонический сосед соответствующего
пути; отдельное имя host не является частью публичного интерфейса.

До изменения установленных файлов workflow копирует и проверяет оба временных
артефакта. Затем он атомарно заменяет host и последним основной binary. Такой
порядок не делает пару одной filesystem-транзакцией, но не допускает успешной
установки нового `codex-hermione` со старым или отсутствующим sidecar.

## Архитектурное решение

`release-fast` остаётся отдельным Cargo profile, наследующим upstream `release`,
а `build-fast-release` служит внутренней целью за skill-owned gate
`fork build-fast`. Это отделяет upstream packaging profile от локального
optimized runtime-комплекта и даёт fork workflow одну стабильную границу
сборки.

Build и install моделируют основной binary и sidecar явными artifact
contracts. Для каждого контракта определены каноническое имя и безопасный probe;
общий install алгоритм сначала проверяет все sources и temporaries, а только
потом начинает замену. Namespace или runtime-логика Code Mode для этого не
меняются: доработка обеспечивает наличие уже существующего upstream host рядом
с установленным fork binary.

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
fork-specific настройки быстрого stripped-артефакта.

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

Для `fork install` сохранить вывод host source/target из основных путей,
предварительное staging и проверку обоих файлов, а также замену host перед
основным binary. При изменении этого порядка синхронизировать unit tests и
публичное описание в `checks-and-gates.md`.

### 4. Проверить build и install contracts

Skill-owned `fork build-fast` должен создать оба release-fast artifacts. Оба
должны быть stripped без дополнительного ручного `strip`. Проверка установки
должна подтвердить, что основной binary и канонически названный host оказываются
в одном целевом каталоге.

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
реальное появление и probes обоих optimized stripped artifacts.

## Риски и ограничения

### Ограничения

- Не заменять upstream `release` profile: он нужен для canonical
  release-артефакта.
- Не использовать canonical release build path для обычной Hermione compile-check:
  он проверяет upstream release profile, а не быстрый fork build path.
- Не полагаться на ручной `strip` после сборки: он легко теряется при переносе
  и не должен быть частью build contract.
- Не переименовывать установленный host: `InstallContext` ищет канонический
  `codex-code-mode-host` рядом с основным executable.
- Не устанавливать основной binary до проверки временного host: новый Codex не
  должен успешно устанавливаться без согласованного sidecar.
- Не возвращать release-fast Cargo build к неуправляемой загрузке
  `denoland/rusty_v8`: для Codex V8 profile требуются согласованные OpenAI
  archive и generated binding.

### Риски

- Если `release-fast` не наследует `release`, build может отличаться слишком
  сильно от shipped optimized behavior.
- Если `codegen-units` снова станет `1`, profile потеряет смысл.
- Если `debug` и `strip` снова будут только наследоваться из upstream
  `release`, артефакт может стать unstripped и вырасти до
  непрактичного размера.
- Замена двух файлов не является общей filesystem-транзакцией. Host заменяется
  первым, поэтому сбой между `replace` оставляет новый host со старым основным
  binary, но не обратную и более опасную комбинацию.
- Если upstream добавит host version flag или изменит способ discovery sidecar,
  probes, каноническое имя и install contract нужно пересмотреть вместе.
- Если native rustc target отсутствует в upstream `TARGET_SPECS` или V8 release
  pair ещё не опубликована, `fork build-fast` должен завершиться ошибкой до
  проверки binaries, а не переходить на непроверенный artifact.
