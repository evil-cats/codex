---
id: fork-release-fast-build-profile
status: active
created: 2026-06-08
updated: 2026-08-13
---

# Release-fast build profile

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая добавляет быстрый
optimized build path: Cargo profile `release-fast` и skill-owned gate
`fork build-fast`. Профиль сохраняет параллельность финальных стадий и сразу
создаёт stripped artifact, не меняя canonical upstream `release` profile.

## Зачем это нужно

Upstream `release` является профилем для packaging и symbolication:

- `debug = "line-tables-only"`;
- `split-debuginfo = "off"`;
- `strip = false`.

Hermione нужен отдельный быстрый optimized profile с большей параллельностью
финальных стадий и готовым stripped-артефактом. Если `release-fast` наследует
upstream-настройки `debug` и `strip` без переопределения, binary становится
unstripped и может вырасти до непрактичного размера.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/Cargo.toml` | Добавляет `[profile.release-fast]` |
| `justfile` | Добавляет `build-fast-release` |

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

## Архитектурное решение

`release-fast` остаётся отдельным Cargo profile, наследующим upstream `release`,
а `build-fast-release` служит внутренней целью за skill-owned gate
`fork build-fast`. Это отделяет upstream packaging profile от локального
optimized artifact и даёт fork workflow одну стабильную границу сборки.

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

### 3. Проверить build contract

Skill-owned `fork build-fast` должен создать
`codex-rs/target/release-fast/codex`. Артефакт должен быть stripped без
дополнительного ручного `strip`.

## Проверки

`not-applicable`: у build profile нет отдельного card-level test; обязательный skill-owned gate `fork build-fast` подтверждает optimized stripped artifact и version metadata.

## Риски и ограничения

### Ограничения

- Не заменять upstream `release` profile: он нужен для canonical
  release-артефакта.
- Не использовать canonical release build path для обычной Hermione compile-check:
  он проверяет upstream release profile, а не быстрый fork build path.
- Не полагаться на ручной `strip` после сборки: он легко теряется при переносе
  и не должен быть частью build contract.

### Риски

- Если `release-fast` не наследует `release`, build может отличаться слишком
  сильно от shipped optimized behavior.
- Если `codegen-units` снова станет `1`, profile потеряет смысл.
- Если `debug` и `strip` снова будут только наследоваться из upstream
  `release`, артефакт может стать unstripped и вырасти до
  непрактичного размера.
