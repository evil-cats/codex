---
name: fork
description: >-
  Локальный project skill для обслуживания Codex Hermione fork: fork-доработки,
  карточки `docs/fork/`, миграция на новую upstream-версию, работа
  parent/subagent по одной карточке, prompt templates и skill-owned
  checks/build/test scripts. Используй, когда задача касается fork workflow,
  `docs/fork/*.md`, migration table, subagent one-card flow, fork gates, fast
  build, генераторов, retired legacy fork workflow или cleanup после переноса в
  skill-owned workflow.
---

# Fork

Этот skill является активным владельцем fork workflow для локальной разработки,
сборки, тестирования, миграции fork-доработок, карточек `docs/fork/*.md`,
parent/subagent flow и skill-owned scripts.

Switch выполнен после structural coverage gate, независимого semantic audit без
блокирующих P0/P1 findings и явного подтверждения пользователя.

## Навигация

- Общие правила fork-доработок: `references/fork-rules.md`.
- Контракт карточек `docs/fork/*.md`: `references/fork-card-contract.md`.
- Роль родительского агента при миграции: `references/parent-migration.md`.
- Роль подагента одной карточки: `references/subagent-one-card.md`.
- Локальный checkout без host-specific зависимости: `references/local-development.md`.
- Проверки, генераторы, тесты и сборка: `references/checks-and-gates.md`.
- Доказательство переноса из retired legacy sources: `references/source-coverage.md`.

## Skill-owned scripts

Основной entrypoint:

```bash
.codex/skills/fork/scripts/fork --help
```

Начальные команды:

- `fork check-source-coverage`
- `fork render-subagent-prompt`
- `fork cards list`
- `fork cards validate`
- `fork preflight --skill-only`

Skill-owned scripts не вызывают retired legacy scripts как runtime dependency.
Имена старых scripts сохраняются только в матрице покрытия переноса.

## Обязательный gate карточек и тестов

Если задача добавляет active fork-карточку, меняет `docs/fork/*.md`, раздел
`Проверки`, обязательное покрытие, crate/test target, tool spec, config/schema,
prompt или model-visible context, прочитай `references/checks-and-gates.md`.

Перед финалом такой задачи синхронизируй skill-owned исполняемые карты и
проверь `fork tests --mode list`. `fork cards validate` является строгим gate
связи карточек с исполняемыми картами, но не заменяет
`fork tests --mode list`.
