# Checks and gates

Статус: `перенесено`.

Этот файл владеет skill-owned проверками, генераторами, тестами, сборкой,
логами и правилами обновления исполняемых карт.

Skill-owned scripts являются активным интерфейсом fork workflow. Retired legacy
scripts удалены после switch и не являются runtime dependency skill-owned
commands.

## Skill commands

Интерфейс:

```bash
.codex/skills/fork/scripts/fork preflight --version X.Y.Z
.codex/skills/fork/scripts/fork format --check
.codex/skills/fork/scripts/fork generators
.codex/skills/fork/scripts/fork tests --mode list
.codex/skills/fork/scripts/fork tests --mode cards --version X.Y.Z
.codex/skills/fork/scripts/fork tests --mode full --version X.Y.Z
.codex/skills/fork/scripts/fork build-fast --version X.Y.Z
.codex/skills/fork/scripts/fork cards list
.codex/skills/fork/scripts/fork cards validate
.codex/skills/fork/scripts/fork render-subagent-prompt
.codex/skills/fork/scripts/fork check-source-coverage
```

`--version` можно опустить, если команда однозначно выводит версию из текущей
ветки `hermione-X.Y.Z` или единственной `docs/fork/migration-X.Y.Z.md`.

## Skill-owned scripts и логи

Skill-owned CLI `.codex/skills/fork/scripts/fork` является каноническим способом
выполнять проверки, генераторы, форматирование, тесты и быструю сборку в ходе
миграции fork.

Команды выполняются в текущем локальном checkout.

Каждая команда, которая запускает внешние проверки или сборку, должна писать
полный вывод в подкаталог `target/fork-migration/*-logs/` или новый
skill-owned совместимый log directory. При ошибке на экран выводи только
короткий хвост лога, чтобы не раздувать контекст. Этот хвост является
подсказкой, а не полным разбором. Если команда печатает `RESULT: failed`,
открой указанный `LOG`, найди настоящую причину, исправь ее и повтори ту же
skill-owned команду.

Не обходи skill-owned команду ручным запуском ее внутренних команд, если
пользователь явно не попросил разбирать внутренний шаг.

Глобальная проверка отсутствия маркеров конфликтов является финальной проверкой
после обработки карточек и завершающей очистки. Она не является обязательной
ранней проверкой сразу после `git merge --no-commit`, потому что на этапе
карточек незавершенные конфликты допустимы.

## Проверочный интерфейс карточек

Для fork-карточек `docs/fork/*.md` skill-owned commands являются основным
проверочным интерфейсом, но карточка не является runbook запуска этих
команд. Раздел `Проверки` в карточке должен описывать смысловое покрытие,
skill-owned command или script-владельца исполняемой карты и уже полученные
результаты, если они есть.

Если карточке нужна новая целевая проверка, сначала добавь ее в `fork tests`
или другой подходящий skill-owned command. Запуск проверок выполняй не из
карточки, а в общем проверочном проходе после подтверждения карточек и
подготовки нужного diff по порядку из раздела `Начальный порядок после прохода
по карточкам`.

Прямые `just`/`cargo` команды не должны быть в карточке инструкцией к запуску.
Их можно упоминать только как исторический результат уже выполненной проверки,
точное имя test target или внутреннюю деталь skill-owned command, если без
этого нельзя восстановить проверочное покрытие. Перед запуском проверки
сначала определи, какой skill-owned command владеет этой проверкой. Если
такого command нет, обнови skill-owned command или зафиксируй blocker; не
обходи workflow прямым запуском внутренней команды.

Новые source/task-owned файлы не должны оставаться только `??` в `git status`.
Обычные untracked-файлы видны в `git status`, но не входят в `git diff HEAD`.
Для новых source/task-owned файлов текущей задачи используй `git add -N <path>`
или, при подготовке staged diff/commit, `git add <path>`.

Не добавляй build artifacts, logs, cache, temporary output, `target/` и
unrelated untracked files. Если untracked файл не является source/task-owned
файлом текущей задачи, не добавляй его в индекс ради выполнения этого правила.
Если `fork preflight` поднял source-like candidate, явно классифицируй его как
task-owned source или unrelated перед финализацией.

`fork cards validate` является form/owner-artifact gate. Он проверяет
структурные группы секций активных fork-карточек и совместим с историческими
русскими alias вроде `Пошаговое воспроизведение` и `Сводка покрытия`. Эта
проверка не доказывает, что карточка семантически соответствует текущему коду:
такое соответствие подтверждает review, подагент одной карточки или отдельный
semantic audit.

## Начальный порядок после прохода по карточкам

После прохода по карточкам:

1. Запусти skill-owned проверки в текущем локальном source-of-truth checkout:
   `fork preflight`, `fork format --check` и, если затронуты сгенерированные
   schema/API surfaces, `fork generators`.
2. Если локальные генераторы создали новые файлы, добавь их в индекс через
   `git add <paths>` или хотя бы отметь через `git add -N <paths>`, иначе
   `git diff HEAD` и последующий review могут не увидеть их содержимое.
3. Запусти целевые тесты и сборку в этом же checkout: минимум
   `fork tests --mode cards` и `fork build-fast`. Полный тестовый проход
   запускай через `fork tests --mode full`, когда нужен полный регрессионный
   gate.
4. Зафиксируй в migration-карточке реальные результаты, известные падения,
   пропущенные проверки, metadata сборки и статус установки бинарника.
5. Перед commit проверь, что каждая активная fork-доработка имеет статус
   `перенесено`, `не применимо`, `reverted`, `open question` или
   `требует исправления`. Статус `проверяется` является незавершенным и
   блокирует финальные gates.

`fork tests --mode full` не запускается по инерции: нужен явный запрос
пользователя или понятная необходимость полного регрессионного gate.

## Исполняемые карты при изменении fork-карточек

`fork tests` является единственным владельцем исполняемой карты card-level
проверок. Карточки `docs/fork/*.md` описывают смысловое покрытие, владельца
исполняемой карты, исторические результаты и известные падения, но не являются
runbook запуска команд.

Если в `docs/fork/` добавилась карточка, удалилась карточка, изменилась
активность карточки или в карточке изменился раздел `Проверки`, список
обязательного покрытия, crate/test target, имя теста, требование к
snapshot/schema/generator, условие пропуска проверки либо ожидаемый результат,
обнови skill-owned исполняемые карты, которые зависят от состава карточек.

Всегда проверяй `fork tests`. Этот command содержит исполняемую карту проверок
конкретных карточек в card-тестах. Обнови `CARD_TESTS` и проверь
`fork tests --mode list`, если карточка:

- добавлена как активная fork-доработка;
- удалена или больше не участвует в текущей миграции;
- переведена из `reverted`, `не применимо` или другого неактивного состояния в
  активную проверяемую доработку;
- меняет раздел `Проверки`, обязательное покрытие, crate, test target или имя
  теста;
- добавляет видимое пользователю UI, TUI или text-output поведение, для
  которого нужно snapshot coverage;
- меняет app-server, protocol, config/schema, tool spec, prompt или
  model-visible context и требует отдельного regression test.

Нельзя финализировать работу только на основании прямых запусков `just ...` или
`cargo ...`, если active карточка должна иметь card-level regression coverage:
прямой запуск является evidence, а исполняемая карта принадлежит `fork tests`.
`fork cards validate` является hard gate соответствия карточек исполняемым
картам и должен падать, если active карточка ссылается на проверки, которых нет
в `CARD_TESTS`, либо если карточка хранит канонический command runbook вместо
ссылки на skill-owned owner.

Проверяй `fork preflight`, только если изменилась форма migration-карты или
общий контракт предварительной проверки. Например:

- добавлен новый статус строки таблицы миграции;
- изменен формат строк `docs/fork/migration-X.Y.Z.md`;
- переименована migration-карта;
- изменены правила для `pending`, `проверяется`, `open question`,
  `требует исправления`, `перенесено`, `не применимо` или `reverted`;
- добавлен новый общий invariant, который должен проверяться до тестов и
  сборки.

Проверяй `fork generators`, если новая или измененная карточка вводит новую
категорию сгенерированных артефактов. Если карточка затрагивает только уже
покрытые config/app-server schema surfaces, текущие команды генератора обычно
менять не нужно:

- `just write-config-schema`;
- `just write-app-server-schema`;
- `just write-app-server-schema --experimental`.

Остальные skill-owned commands считаются инфраструктурными и обычно не
меняются из-за новой fork-карточки:

- `fork format`;
- `fork build-fast`.

Их меняют только при изменении самой цепочки миграции, формата логов, build
artifact или правил запуска сборки.

## Bootstrap status

Реализованы skill-owned команды:

- `check-source-coverage`;
- `render-subagent-prompt`;
- `cards list`;
- `cards validate`;
- `format --check`;
- `format --fix`;
- `preflight --skill-only`;
- `preflight --version X.Y.Z`;
- `generators`;
- `tests --mode list`;
- `tests --mode cards --version X.Y.Z`;
- `tests --mode full --version X.Y.Z`;
- `build-fast --version X.Y.Z`.

Heavy gates (`generators`, `tests --mode cards`, `tests --mode full`,
`build-fast`) запускай только когда они нужны текущему этапу. Режим
`tests --mode list` печатает исполняемую карту проверок без запуска тестов.
