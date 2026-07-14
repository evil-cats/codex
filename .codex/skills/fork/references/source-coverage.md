# Source coverage for `fork` skill

Этот файл является handoff, планом переноса и проверяемой матрицей покрытия.
Он фиксирует, что правила перенесены в skill-owned workflow, и остается
проверяемой матрицей покрытия, чтобы не потерять смысл при дальнейших правках.

## Текущий статус

Статус: `active-skill-owner`.

Skill `fork` является активным владельцем fork workflow. Switch выполнен после:

1. structural coverage gate;
2. independent semantic audit без блокирующих P0/P1 findings;
3. явного подтверждения пользователя.

После switch legacy owner artifacts удалены отдельным cleanup-этапом: активный
workflow больше не зависит от `FORK.md`,
`docs/migration-one-card-for-agent.md` или `scripts/fork-migration/*`.

Structural coverage по-прежнему не доказывает семантическую полноту переноса:
он остается structural gate для будущих изменений этого skill.

## Согласованный контракт

<!-- markdownlint-disable MD013 -->

| Пункт | Статус | Target |
| --- | --- | --- |
| Skill `fork` должен стать полным владельцем правил fork workflow | перенесено | `SKILL.md`, этот файл |
| Skill должен владеть собственными scripts, а не вызывать legacy scripts | перенесено | `SKILL.md`, `references/checks-and-gates.md`, `scripts/fork` |
| Switch на skill меняет routing в `AGENTS.md`; post-switch cleanup удаляет legacy `FORK.md` и `docs/migration-one-card-for-agent.md` после переноса правил | перенесено | `AGENTS.md`, этот файл |
| Post-switch cleanup удаляет `scripts/fork-migration/*` после переноса поведения в skill-owned scripts | перенесено | этот файл |
| До switch skill проходил review; после switch skill является активным владельцем workflow | перенесено | `SKILL.md`, этот файл |
| В новом workflow не закреплять host-specific зависимость | перенесено | `references/local-development.md` |
| Source of truth формулировать как текущий локальный checkout | перенесено | `references/local-development.md` |
| Новые source/task-owned файлы должны попадать в Git index минимум через `git add -N`; подагент может точечно добавить созданный им новый файл или полностью разрешённый конфликт своей карточки; build artifacts и unrelated untracked не добавляются | перенесено с нормализацией | `references/fork-rules.md`, `references/local-development.md`, `references/checks-and-gates.md`, `references/subagent-one-card.md`, `references/parent-migration.md`, `assets/templates/parent-subagent-prompt.md`, `scripts/fork_cli.py` |
| Структурную полноту переноса проверять coverage artifact и script | перенесено | этот файл, `scripts/fork check-source-coverage` |
| Семантическую полноту переноса проверять отдельным independent audit | перенесено | `SKILL.md`, этот файл |

## Target artifacts

| Target | Назначение | Статус |
| --- | --- | --- |
| `SKILL.md` | Короткий entrypoint и маршрутизатор | bootstrap |
| `references/fork-rules.md` | Что является fork-доработкой и когда нужна карточка | перенесено |
| `references/fork-card-contract.md` | Контракт и готовность `docs/fork/*.md`, включая strict раздел `Проверки` | перенесено |
| `references/parent-migration.md` | Parent-agent migration workflow | перенесено |
| `references/subagent-one-card.md` | Правила подагента одной карточки | перенесено |
| `references/local-development.md` | Host-agnostic local checkout workflow | перенесено |
| `references/checks-and-gates.md` | Skill-owned gates, исполняемые карты и scripts | перенесено |
| `assets/templates/fork-card.md` | Шаблон fork-карточки с разделением смыслового покрытия, владельца исполняемой карты и evidence | перенесено |
| `scripts/migration_map.py` | Строгий формат `fork-migration.v1`, вычисляемый прогресс и атомарные обновления JSON-карты | перенесено с нормализацией |
| `scripts/migration_cli.py` | Лёгкие команды `fork migration` для создания, чтения, точечного обновления, валидации и завершения JSON-карты | перенесено с нормализацией |
| `assets/templates/parent-subagent-prompt.md` | Шаблон prompt для подагента | перенесено |
| `scripts/fork` | Skill-owned CLI entrypoint | перенесено |
| `scripts/fork_cli.py` | Маршрутизация готовности JSON-карты в `preflight`, тесты и сборку; строгая validation active cards и `fork-tests.v1` | перенесено |

## Status values

- `pending` - перенос еще не выполнен.
- `draft` - есть начальный перенос, нужна сверка с источником.
- `перенесено` - смысл перенесен без известных потерь.
- `перенесено с нормализацией` - смысл перенесен, но формулировка изменена
  по согласованной причине.
- `требует проверки` - нужна ручная сверка или независимый аудит.
- `не переносится` - осознанно не входит в skill, причина указана.

## `FORK.md` coverage

| Source section | Target | Статус | Примечание |
| --- | --- | --- | --- |
| `# Правила обслуживания fork` | `SKILL.md`, все references | перенесено с нормализацией | Owner model перенесен в skill; legacy остается эталоном до switch |
| `## Что считается fork-доработкой` | `references/fork-rules.md` | перенесено | Определение и исключения для служебных изменений перенесены |
| `## Карточки docs/fork/` | `references/fork-card-contract.md` | перенесено | Назначение, подробность и запрет transcript-summary перенесены |
| `### Критерии готовности fork-карточки` | `references/fork-card-contract.md` | перенесено с нормализацией | `wrapper или скрипт` нормализовано как skill-owned владелец исполняемой карты; раздел `Проверки` теперь strict |
| `### Проверка покрытия` | `references/fork-card-contract.md` | перенесено | Статусы и запрет молчаливой потери перенесены |
| `## Проверка перед коммитом` | `references/fork-rules.md` | перенесено | Предкоммитная проверка и card/staging правила перенесены |
| `## Апгрейд на новую версию Codex` | `references/parent-migration.md` | перенесено с нормализацией | `f-ms-dev` заменен на local checkout, subagent source заменен на skill reference |
| `### Возобновление миграции после прерывания` | `references/parent-migration.md` | перенесено с нормализацией | `f-ms-dev` заменен на local checkout |
| `### Шаблон prompt для подагента одной карточки` | `assets/templates/parent-subagent-prompt.md` | перенесено с нормализацией | Legacy subagent source заменен на skill reference, parent-only запрет добавлен |
| `После прохода по карточкам` | `references/checks-and-gates.md` | перенесено с нормализацией | Legacy `local-*` команды заменены на skill-owned `fork ...` команды |
| `### Wrapper-скрипты миграции и логи` | `references/checks-and-gates.md`, `scripts/fork` | перенесено с нормализацией | Legacy wrappers заменены на skill-owned commands; parity rows остаются ниже |
| `### Обновление скриптов при изменении fork-карточек` | `references/checks-and-gates.md`, `scripts/fork_cli.py` | перенесено с нормализацией | Legacy script names заменены на skill-owned исполняемые карты; `fork cards validate` проверяет связь active cards с блоками `fork-tests.v1` |
| `## Source-of-truth checkout на f-ms-dev` | `references/local-development.md` | перенесено с нормализацией | Host-specific часть заменена на текущий локальный checkout |

## `docs/migration-one-card-for-agent.md` coverage

| Source section | Target | Статус | Примечание |
| --- | --- | --- | --- |
| `# Инструкция для подагента: одна fork-карточка` | `references/subagent-one-card.md` | перенесено с нормализацией | Родитель назван parent-only references, `FORK.md` запрет сохранен |
| `## Главное правило` | `references/subagent-one-card.md` | перенесено | Смысл и handoff-обязанность перенесены |
| `## Что читать` | `references/subagent-one-card.md` | перенесено | Список разрешенных источников перенесен |
| `## Что запрещено` | `references/subagent-one-card.md` | перенесено с нормализацией | Добавлен запрет читать parent-only references; полный запрет `git add` заменён точечным исключением для новых файлов и полностью разрешённых конфликтов выбранной карточки; `commit`, `push` и широкие Git-операции по-прежнему запрещены |
| `## Рабочий порядок` | `references/subagent-one-card.md` | перенесено | Шаги перенесены |
| `## Формат отчета` | `references/subagent-one-card.md` | перенесено с нормализацией | Parent-only граница добавлена в старое подтверждение, лимит 12 пунктов сохранен |

## Legacy scripts coverage

Новые skill-owned scripts не должны вызывать legacy files как runtime
dependency. Local scripts используются как эталон поведения во время переноса.
Remote patch/mirror workflow retired и не переносится в новый skill-owned
workflow.

| Legacy script | Skill command | Статус | Что должно совпасть |
| --- | --- | --- | --- |
| `scripts/fork-migration/local-preflight.sh` | `fork preflight` | перенесено с нормализацией | whitespace, conflict markers, markdownlint, migration checks, logs |
| `scripts/fork-migration/local-format.sh` | `fork format` | перенесено с нормализацией | `check/apply` перенесены как `--check/--fix`, log semantics сохранены |
| `scripts/fork-migration/local-generators.sh` | `fork generators` | перенесено | generated schema/API surfaces |
| `scripts/fork-migration/local-tests.sh` | `fork tests` | перенесено с нормализацией | `list`, `cards`, `full`, строгая карта карточечных тестов |
| `scripts/fork-migration/local-build-fast.sh` | `fork build-fast` | перенесено с нормализацией | fast build, binary check, version check, logs |
| `scripts/fork-migration/common.sh` | `scripts/fork` internals | перенесено с нормализацией | shared logging, PATH setup and repo checks |
| `scripts/fork-migration/remote-prepare-host.sh` | не переносится | не переносится | Remote mirror reset/clean retired; replacement is current local source-of-truth checkout |
| `scripts/fork-migration/remote-apply-patch.sh` | не переносится | не переносится | Remote patch transfer/checksum flow retired; replacement is direct work in the current checkout |
| `scripts/fork-migration/remote-tests.sh` | не переносится | не переносится | Remote test wrapper retired; replacement is `fork tests` in the current checkout |
| `scripts/fork-migration/remote-build-fast.sh` | не переносится | не переносится | Remote build wrapper retired; replacement is `fork build-fast` in the current checkout |

<!-- markdownlint-enable MD013 -->

## Structural coverage gate

`fork check-source-coverage --strict` является structural gate. Он должен
падать, пока в этой матрице есть:

- `pending`;
- `draft`;
- `требует проверки`;
- явные незакрытые placeholder markers;
- target-файлы, которые не существуют;
- файлы в `scripts/fork-migration/`, если каталог существует, без строки в script parity table;
- retired legacy scripts без явной строки `не переносится`;
- script parity rows без допустимого статуса;
- rows со статусом `не переносится` без причины.

Structural gate не доказывает смысловой перенос и не заменяет `fork cards
validate`. Для будущих существенных изменений skill-owned workflow после
structural green нужен отдельный semantic audit. После cleanup legacy owner
artifacts удалены, а старые имена scripts остаются только в script parity table
как доказательство переноса.
