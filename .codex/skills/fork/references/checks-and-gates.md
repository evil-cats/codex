# Checks and gates

Статус: `перенесено`.

Этот файл владеет skill-owned проверками, генераторами, тестами, сборкой,
логами и правилами обновления исполняемых карт.

Skill-owned scripts являются активным интерфейсом fork workflow. Retired legacy
scripts удалены после switch и не являются runtime dependency skill-owned
commands.

## Skill-owned команды

Интерфейс:

```bash
.codex/skills/fork/scripts/fork preflight --version X.Y.Z
.codex/skills/fork/scripts/fork format --check
.codex/skills/fork/scripts/fork format --fix
.codex/skills/fork/scripts/fork fix
.codex/skills/fork/scripts/fork fix --package CRATE
.codex/skills/fork/scripts/fork generators
.codex/skills/fork/scripts/fork tests --mode list
.codex/skills/fork/scripts/fork tests --mode list --card CARD_ID_OR_PATH
.codex/skills/fork/scripts/fork tests --mode cards --version X.Y.Z
.codex/skills/fork/scripts/fork tests --mode cards --card CARD_ID_OR_PATH --version X.Y.Z
.codex/skills/fork/scripts/fork tests --mode full --version X.Y.Z
.codex/skills/fork/scripts/fork build-fast --version X.Y.Z
.codex/skills/fork/scripts/fork install
.codex/skills/fork/scripts/fork cards list
.codex/skills/fork/scripts/fork cards validate
.codex/skills/fork/scripts/fork migration init --version X.Y.Z
.codex/skills/fork/scripts/fork migration show --version X.Y.Z
.codex/skills/fork/scripts/fork migration next --version X.Y.Z
.codex/skills/fork/scripts/fork migration set-card-status --version X.Y.Z --card CARD --status STATUS
.codex/skills/fork/scripts/fork migration set-gate-status --version X.Y.Z --gate GATE --status STATUS
.codex/skills/fork/scripts/fork migration validate --version X.Y.Z
.codex/skills/fork/scripts/fork migration complete --version X.Y.Z
.codex/skills/fork/scripts/fork render-subagent-prompt
```

`--version` можно опустить, если команда однозначно выводит версию из текущей
ветки `hermione-X.Y.Z`. Наличие единственного файла в `docs/fork/migration/` или
исторического `docs/fork/migration-*.md` не выбирает версию и не является
решением пользователя о возобновлении миграции. Подкоманды `fork migration`
требуют явный `--version`.

`fork install` по умолчанию устанавливает
`codex-rs/target/release-fast/codex` в
`${HOME}/.local/bin/codex-hermione`. Если нужно явно переопределить источник
или цель, используй `--source PATH` и `--target PATH`; это остается
skill-owned установкой, а не ручным копированием бинарника.

`fork fix` запускает repo lint/fix recipe для всего workspace. Повторяемый
`--package CRATE` ограничивает запуск выбранными crates и передает каждую из них
отдельным аргументом `-p`.

`fork tests --mode list` и `fork tests --mode cards` принимают повторяемый
`--card`. Значение может быть `id` карточки, путь `docs/fork/*.md`, имя файла
или `id` без префикса `fork-`. Фильтр ограничивает результат выбранными
карточками. Режим `list` печатает строки из блоков `fork-tests.v1`, а для
карточки с обоснованным исключением — вид `manual-required`/`not-applicable` и
его причину. Режим `cards` запускает только `fork-tests.v1` и отклоняет
выбранную карточку без автоматизированных тестов. `--mode full` не принимает
`--card`, потому что полный проход не является card-level запуском.

## Модель владения командами

В fork-scope различай внешнюю workflow-команду и внутренний argv. Skill-owned
command является workflow-командой; `just`/`cargo` argv внутри него является
деталью реализации, подтверждением для аудита или отладочной подсказкой, но не
пользовательским runbook.

### Обертки над требованиями `AGENTS.md`

Эти команды выполняют обычные требования уровня репозитория из `AGENTS.md`, но
через fork-owned интерфейс, логи и предусловия:

| Требование | Команда | Примечание |
| --- | --- | --- |
| Форматирование после правок | `fork format --fix` | Запускает repo format recipe |
| Проверка форматирования | `fork format --check` | Финальный check без правок |
| Rust lint/fix | `fork fix [--package CRATE]` | Без `--package` обрабатывает workspace |
| Артефакты config/app-server schema | `fork generators` | Обновляет schema artifacts |
| Тесты карточки | `fork tests --mode cards --card CARD` | argv из блока `fork-tests.v1` |
| Полный регрессионный проход | `fork tests --mode full` | Полный набор тестов и pending snapshots |
| Быстрая release-сборка | `fork build-fast` | Fast build и проверка бинарника |
| Установка fork-бинарника | `fork install` | Атомарная установка release-fast бинарника |

Если `AGENTS.md` требует шаг, которого нет в этой таблице или другом
skill-owned command, это пробел workflow. Сначала обнови skill-owned command или
зафиксируй blocker; не выполняй внутренний `just`/`cargo` argv напрямую.

Просмотр и принятие snapshot также должны иметь skill-owned владельца. Если
текущий workflow покрывает только проверку pending snapshots, а для задачи нужен
просмотр или принятие snapshot, добавь skill-owned command или зафиксируй
blocker вместо прямого запуска `cargo insta ...` как fork gate.

### Fork-specific gates

Эти команды не являются заменой одного `just` recipe и не сводятся к общему
Rust workflow:

| Skill-owned command | Назначение |
| --- | --- |
| `fork cards list` | Навигация по fork-карточкам |
| `fork cards validate` | Current-state форма, `fork-tests.v1`, запрет legacy-секций |
| `fork migration *` | Генерация, чтение, точечное обновление и валидация JSON migration map |
| `fork tests --mode list` | Карта тестов и обоснованных исключений без запуска |
| `fork tests --mode cards` | Card-level запуск с предусловиями и логами |
| `fork preflight` | Составной gate для skill/files/cards/JSON map/untracked/conflicts/markdown |
| `fork render-subagent-prompt` | Генератор prompt для подагента одной карточки |
| `fork build-fast` | Fork build gate с проверкой бинарника и версии |
| `fork install` | Атомарная установка fork-бинарника в `${HOME}/.local/bin/codex-hermione` |

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

Локальные файлы логов из `target/fork-migration/*-logs/` являются отладочными
артефактами wrapper'а времени выполнения. Они не коммитятся и не должны
попадать в `docs/fork/*.md`.

Результаты конкретного запуска тестов, сборки, установки или migration gate
также не переносятся в owner-карточку. Закрытый gate status хранится в JSON
migration map, полный stdout/stderr и диагностические подробности — в
skill-owned логах, хронология изменений — в Git, итог текущего рабочего
прохода — в рабочем отчёте.

Обновляй карточку только тогда, когда запуск выявил новый постоянный контракт,
архитектурное ограничение, регрессионный сценарий, owner-файл, риск или условие
переноса. Дата запуска, версия и hash бинарника, путь установки либо уже
исправленное падение сами по себе не являются содержанием карточки.

Не обходи skill-owned команду ручным запуском ее внутренних argv, если
пользователь явно не попросил разбирать внутренний шаг. При падении skill-owned
команды открой указанный `LOG`, исправь причину и повтори ту же skill-owned
команду.

Глобальная проверка отсутствия маркеров конфликтов является финальной проверкой
после обработки карточек и завершающей очистки. Она не является обязательной
ранней проверкой сразу после `git merge --no-commit`, потому что на этапе
карточек незавершенные конфликты допустимы.

## Проверочный интерфейс карточек

Для fork-карточек `docs/fork/*.md` skill-owned commands являются основным
проверочным интерфейсом, но карточка не является runbook запуска этих команд.

Раздел `Проверки` содержит machine-readable блок `fork-tests.v1` либо
явное исключение `manual-required`/`not-applicable` с причиной. Поле
`purpose` описывает проверяемое поведение, а `argv` хранит внутренний вызов,
которым владеет `fork tests`. Не дублируй эти данные отдельной таблицей
смыслового покрытия.

Дополнительный skill-owned gate указывай только тогда, когда он действительно
нужен конкретной доработке. Не создавай обязательные подразделы или
`not-applicable`-заглушки для неприменимых gates.

Прямые `just`/`cargo` команды и workflow-вызовы запрещены в
человекочитаемом тексте карточки. Внутренний argv допустим только внутри
`fork-tests.v1`. Если требуемого skill-owned владельца нет, сначала обнови
workflow или зафиксируй blocker вне owner-карточки.

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
обязательные current-state разделы fork-карточек, отсутствие legacy-секций и
наличие корректного `fork-tests.v1` либо явного исключения. Эта проверка не
доказывает, что карточка семантически соответствует текущему коду: такое
соответствие подтверждает review, подагент одной карточки или отдельный
semantic audit.

## Начальный порядок после прохода по карточкам

После прохода по карточкам:

1. Запусти skill-owned проверки в текущем локальном source-of-truth checkout:
   `fork preflight`; если после правок нужно применить форматирование,
   `fork format --fix`, а для финальной проверки без изменений
   `fork format --check`; если для Rust-правок требуется lint/fix, используй
   `fork fix` или `fork fix --package CRATE`; если затронуты сгенерированные
   schema/API surfaces, `fork generators`.
2. Если локальные генераторы создали новые файлы, добавь их в индекс через
   `git add <paths>` или хотя бы отметь через `git add -N <paths>`, иначе
   `git diff HEAD` и последующий review могут не увидеть их содержимое.
3. Запусти целевые тесты и сборку в этом же checkout: минимум
   `fork tests --mode cards` и `fork build-fast`. Полный тестовый проход
   запускай через `fork tests --mode full`, когда нужен полный регрессионный
   gate.
4. Зафиксируй результат каждого общего gate только закрытым статусом
   `passed`, `failed` или `skipped` через
   `fork migration set-gate-status`. Подробный вывод остаётся в skill-owned
   логах и рабочем отчёте и не копируется в owner-карточку. Если результат
   изменил постоянный контракт, проверку, риск или условие переноса, обнови
   соответствующий нормативный раздел карточки.
5. Перед commit проверь, что каждая активная fork-доработка имеет статус
   `migrated`, `notApplicable` или `reverted`. Статусы `pending`, `inProgress`,
   `blocked` и `needsFix` являются незавершенными и блокируют финальные gates.
6. Когда все карточки финальны, а каждый gate получил `passed` или `skipped`,
   выполни `fork migration validate --version X.Y.Z`, затем
   `fork migration complete --version X.Y.Z`. Завершённая карта неизменна;
   исправляй ошибочный статус до `complete`, а не редактируй JSON после него.

`fork tests --mode full` не запускается по инерции: нужен явный запрос
пользователя или понятная необходимость полного регрессионного gate.

## Исполняемые карты при изменении fork-карточек

`fork tests` является единственным владельцем запуска card-level проверок.
Данные запусков живут в machine-readable блоках `fork-tests.v1` внутри
карточек `docs/fork/*.md`. Поле `purpose` является кратким описанием
регрессионного сценария; отдельные таблицы обязательности, история результатов
и списки прошлых падений в карточке не нужны.

Если в `docs/fork/` добавилась карточка, удалилась карточка, изменилась её
активность либо изменились регрессионный сценарий, `purpose`, crate/test
target, имя теста, условие пропуска или требуемый дополнительный gate, обнови
блок `fork-tests.v1` этой карточки или другой skill-owned command, который
владеет соответствующей проверкой.

Всегда проверяй `fork tests --mode list`, если карточка:

- добавлена как активная fork-доработка;
- удалена или больше не участвует в текущей миграции;
- переведена из `reverted`, `не применимо` или другого неактивного состояния
  в активную проверяемую доработку;
- меняет раздел `Проверки`, `purpose`, crate, test target, имя теста,
  условие пропуска либо ожидаемое поведение;
- добавляет видимое пользователю UI, TUI или text-output поведение, для
  которого нужно snapshot coverage;
- меняет app-server, protocol, config/schema, tool spec, prompt или
  model-visible context и требует отдельный regression test.

Нельзя финализировать работу только на основании прямых запусков `just ...`
или `cargo ...`: card-level исполняемая карта принадлежит `fork tests`.
`fork cards validate` должен отклонять active-карточку без корректного
`fork-tests.v1` или явного исключения, с прямым command runbook либо с
legacy-секциями.

Проверяй `fork preflight`, только если изменилась форма JSON migration map или
общий контракт предварительной проверки. Например:

- добавлен новый статус карточки или gate;
- изменены поля `docs/fork/migration/X.Y.Z.json`;
- переименована или перемещена JSON-карта;
- изменены правила для `pending`, `inProgress`, `blocked`, `needsFix`,
  `migrated`, `notApplicable` или `reverted`;
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
- `fork fix`;
- `fork build-fast`;
- `fork install`.

Их меняют только при изменении самой цепочки миграции, формата логов, артефакта
сборки или правил запуска сборки и установки.

## Bootstrap status

Реализованы skill-owned команды:

- `render-subagent-prompt`;
- `cards list`;
- `cards validate`;
- `migration init/show/next/set-card-status/set-gate-status/validate/complete`;
- `format --check`;
- `format --fix`;
- `fix [--package CRATE]`;
- `preflight --skill-only`;
- `preflight --version X.Y.Z`;
- `generators`;
- `tests --mode list`;
- `tests --mode cards --version X.Y.Z`;
- `tests --mode cards --card CARD_ID_OR_PATH --version X.Y.Z`;
- `tests --mode full --version X.Y.Z`;
- `build-fast --version X.Y.Z`;
- `install`.

Heavy or mutating gates (`fix`, `generators`, `tests --mode cards`,
`tests --mode full`, `build-fast`) запускай только когда они нужны текущему
этапу. `fork install` запускай только после явного решения установить собранный
бинарник. Режим `tests --mode list` печатает исполняемую карту проверок и
обоснованные исключения без запуска тестов.
