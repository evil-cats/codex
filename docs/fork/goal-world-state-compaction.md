---
id: fork-goal-world-state-compaction
status: active
created: 2026-07-29
updated: 2026-07-29
source_scope: hermione-0.145.0..HEAD
---

# Active goal в `WorldState` после compaction

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая сохраняет точный
`objective` активной thread goal в model-visible context после local, remote и
`TokenBudget` compaction.

Доработка была реализована и проверена на `hermione-0.145.0`. При переносе на
`hermione-0.146.0` обнаружены и исправлены два проявления регрессии для thread,
в котором goal никогда не была активна: состояние `Unknown` после compaction и
нормализованный `Known(inactive)` при обычном tool follow-up ошибочно
рендерились как переход из active goal в inactive.

До доработки goal продолжает существовать в `state_db.thread_goals`, но
model-visible goal fragment является обычным contextual user fragment. Local и
remote compaction не сохраняют такой fragment дословно, а `TokenBudget`
полностью заменяет историю начальным контекстом с текущим `WorldState`. В
результате продолжение того же turn может потерять точную цель до следующего
перехода thread в `idle`.

| Поле | Значение |
| --- | --- |
| Статус карточки | `active`: доработка поддерживается в fork |
| Статус реализации | `implemented` |
| Проверочный статус | `verified` на `0.145.0`; целевые тесты `0.146.0` ожидают готовности общей migration map |
| Пользовательская цель | Восстанавливать точную active goal перед первым sampling после compaction |
| Источник истины | `state_db.thread_goals` |
| Model-visible механизм | Extension-owned секция `WorldState` |
| Внешние API и config | Без изменений |
| Текущая база | `0.146.0`, ветка `hermione-0.146.0` |
| Бинарник `release-fast` | Для `0.145.0` собран, метаданные и версия проверены |
| Локальная установка | Бинарник `0.145.0` в `/home/slader/.local/bin/codex-hermione`, проверен |
| Установка на `f-ms-dev` | Бинарник `0.145.0` в `/home/slader/.local/bin/codex-hermione`, проверен |

## Зачем это нужно

`GoalExtension::continue_if_idle()` повторно загружает persisted goal только
после завершения всего активного turn. Mid-turn compaction не переводит thread в
`idle`: `run_turn()` продолжает выполнять sampling и tool calls в том же turn.

До этой доработки сохранение objective зависело от compaction output:

- local compaction мог перенести objective только через сгенерированный summary;
- remote compaction мог сохранить его только внутри возвращённого compaction
  item;
- `TokenBudget` reset не имел ни summary, ни отдельного goal reinjection.

Такое поведение не обеспечивает точное продолжение пользовательской цели.
Compaction summary не является authoritative goal storage и может сократить,
исказить либо полностью пропустить objective.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/ext/goal/src/extension.rs` | Регистрирует `GoalExtension` как `ContextContributor` |
| `codex-rs/ext/goal/src/extension/world_state.rs` | Загружает active goal и формирует extension-owned `WorldState` section |
| `codex-rs/ext/goal/src/steering.rs` | Рендерит устойчивый active-goal context и короткие turn steering prompts |
| `codex-rs/ext/goal/templates/goals/active_context.md` | Хранит objective и постоянные goal-инварианты |
| `codex-rs/ext/goal/templates/goals/continuation.md` | Запускает автоматический continuation turn без дублирования objective |
| `codex-rs/ext/goal/templates/goals/objective_updated.md` | Ссылается на обновлённый objective из `WorldState` |
| `codex-rs/app-server/tests/suite/v2/compaction.rs` | Проверяет model-visible request после обычного tool follow-up, local и `TokenBudget` mid-turn compaction |

Намеренно не меняются:

- schema, app-server protocol и публичные goal RPC;
- persisted формат `thread_goals`;
- budget accounting и правила переходов goal status;
- `codex-rs/prompts/src/goals.rs` и
  `codex-rs/prompts/templates/goals/*.md`: эти одноимённые legacy prompt helpers
  не имеют production-callers в текущем checkout и не являются runtime-владельцем
  `GoalExtension`.

## Итоговый контракт

### Authoritative active-goal context

Если goal feature доступен для thread и persisted goal имеет status `Active`,
перед sampling модель получает internal user fragment с устойчивыми markers:

```text
<thread_goal_context>
...
<objective>
точный XML-escaped objective
</objective>
...
</thread_goal_context>
```

Fragment содержит:

- точный `objective`;
- текущее значение `token_budget` либо значение отсутствующего бюджета;
- правила сохранения полного scope;
- требования работать от текущих доказательств;
- completion audit;
- blocked audit;
- разрешённые вызовы `update_goal`.

Objective остаётся user-provided data и не повышается до developer- или
system-инструкции. Для него сохраняется существующее ограничение
`MAX_THREAD_GOAL_OBJECTIVE_CHARS`.

### Diff и cache contract

Extension-owned section использует стабильный section ID и snapshot,
содержащий отрендеренный active-goal body:

- неизменившаяся goal не добавляет новый fragment перед каждым sampling;
- изменение objective или `token_budget` добавляет полный актуальный fragment;
- переход из `Active` в другой status добавляет clearing fragment, запрещающий
  продолжать прежнюю goal как активную;
- отсутствие goal при `PreviousWorldStateSection::Absent` или
  `PreviousWorldStateSection::Unknown` ничего не добавляет;
- отсутствие goal при `Known(inactive)` ничего не добавляет;
- clearing fragment создаётся только для доказанного перехода
  `Known(active) -> inactive`; `Unknown` не является доказательством прежней
  active goal;
- нормализация snapshot из `{"state":"inactive","body":null}` в
  `{"state":"inactive"}` не превращает повторный model step в переход из active
  goal: решение о clearing fragment опирается на доказанное значение `state`, а
  не на полное равенство inactive snapshot.

Динамические `tokens_used`, `time_used_seconds` и `remaining_tokens` не входят в
snapshot. Их изменение не должно инвалидировать prompt cache на каждом model
step.

### Compaction contract

`WorldState` является частью initial context, которую compaction runtime
разворачивает независимо от summary:

- local mid-turn compaction получает точный active-goal fragment через
  `build_compaction_initial_context`;
- remote compaction получает тот же fragment через общий initial-context path;
- `TokenBudget` reset получает fragment через `start_new_context_window`;
- если goal создана или изменена в model step, который сам вызвал compaction,
  следующий step заново строит `WorldState` перед sampling и добавляет свежий
  fragment;
- thread, в котором goal никогда не создавалась, не получает
  `<thread_goal_context>` после local или `TokenBudget` compaction.

Первый post-compaction model request обязан содержать objective дословно после
XML escaping, даже если compaction output намеренно не содержит goal.

### Steering prompts

Постоянные goal-инварианты и objective принадлежат
`templates/goals/active_context.md`.

`templates/goals/continuation.md` остаётся turn trigger и содержит только:

- указание продолжить goal из `<thread_goal_context>`;
- актуальные `tokens_used`, `token_budget` и `remaining_tokens`.

`templates/goals/objective_updated.md` сообщает о замене objective и ссылается
на актуальный `<thread_goal_context>`, не дублируя сам objective.

`templates/goals/budget_limit.md` сохраняет objective внутри собственного
fragment: при status `BudgetLimited` goal уже не является active section, а
модель всё ещё должна получить точную цель для корректного завершения текущего
turn.

## Архитектурное решение

### Почему `WorldState`

`WorldState` уже владеет точным model-visible состоянием, которое:

- строится перед каждым sampling;
- записывает только diff относительно предыдущего snapshot;
- сохраняет baseline в rollout;
- полностью восстанавливается после замены context window;
- одинаково обслуживает local, remote и `TokenBudget` compaction.

Goal реализуется как `ContextContributor` внутри extension crate. `codex-core`
не получает зависимости или условной ветки, знающей о goal feature.

### Отклонённые альтернативы

#### Goal-specific вызов после `run_auto_compact`

Отклонён: связывает общий compaction runtime с конкретной extension, требует
отдельной обработки нескольких compaction implementations и не решает
восстановление после resume либо других context replacements.

#### Новый post-compaction extension hook

Отклонён: добавляет новый lifecycle API для поведения, уже покрываемого
`ContextContributor` и extension-owned `WorldState`.

#### Указание compactor обязательно включать goal в summary

Отклонено: summary остаётся вероятностным пересказом, а не точным persisted
state. Такой prompt не гарантирует дословный objective и не покрывает
`TokenBudget`.

#### Повторять полный goal fragment перед каждым sampling

Отклонено: создаёт лишние model-visible items, расходует context и нарушает
prompt caching. Section snapshot должен подавлять неизменившийся fragment.

## Порядок повторения при переносе

1. Найти runtime-владельца goal extension и способ регистрации extension
   contributors в новом upstream.
2. Подтвердить, что compaction по-прежнему восстанавливает extension-owned
   `WorldState` через общий initial-context path.
3. Добавить стабильную секцию active goal рядом с runtime goal extension, а не
   в общий `codex-core`.
4. Сохранить role `user`, XML escaping и ограничение размера objective.
5. Разделить устойчивый active-goal context и динамический continuation
   steering, не допуская двойного objective.
6. Проверить local summary compaction и context-window reset, при которых
   compaction output не содержит objective.
7. Если upstream изменил владельца goal prompts, перенести шаблоны к актуальному
   production-owner и удалить либо синхронизировать ставшие активными legacy
   копии.

## Проверки

### Смысловое покрытие

Обязательное regression coverage:

- `active_goal_world_state_survives_mid_turn_local_compaction`:
  - первый model response вызывает `create_goal`;
  - тот же response превышает auto-compaction threshold;
  - local compaction summary намеренно не содержит objective;
  - первый post-compaction request содержит `<thread_goal_context>`;
  - objective присутствует в XML-escaped форме ровно один раз;
  - goal завершается через `update_goal`, чтобы test не запускал бесконечные
    continuation turns.
- `active_goal_world_state_survives_mid_turn_token_budget_compaction`:
  - goal создаётся в model step, который вызывает `TokenBudget` reset;
  - новый context window не содержит старую conversation history;
  - первый request нового окна содержит точный active-goal context;
  - objective не зависит от summary, поскольку summary в этом режиме нет.
- `never_active_goal_world_state_stays_absent_after_ordinary_tool_follow_up`:
  - goal feature включён, но goal никогда не создаётся;
  - первый model response вызывает `update_plan` без запуска compaction;
  - второй sampling остаётся в исходном context window;
  - нормализованный `Known(inactive)` не создаёт `<thread_goal_context>` или
    маркер очистки.
- `never_active_goal_world_state_stays_absent_after_mid_turn_local_compaction`:
  - goal feature включён, но `create_goal` не вызывается;
  - `update_plan` удерживает turn до следующего sampling;
  - local compaction создаёт summary без goal;
  - первый post-compaction request не содержит `<thread_goal_context>` или
    маркер очистки.
- `never_active_goal_world_state_stays_absent_after_mid_turn_token_budget_compaction`:
  - goal feature включён, но goal никогда не создаётся;
  - `TokenBudget` полностью заменяет context window;
  - первый request нового окна не содержит `<thread_goal_context>` или маркер
    очистки.
- существующие tests `codex-goal-extension` продолжают подтверждать tool, status,
  budget и accounting contracts.

### Владелец исполняемой карты

Проверки уровня карточки запускает skill-owned command `fork tests`. Внутренний
argv хранится как данные `fork-tests.v1`, а не как пользовательский runbook.

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "goal extension backend",
      "argv": ["just", "test", "-p", "codex-goal-extension"]
    },
    {
      "purpose": "active and never-active goal world state compaction",
      "argv": ["just", "test", "-p", "codex-app-server", "active_goal_world_state"]
    }
  ]
}
```

### Дополнительные gates

- `fork format` владеет обязательным форматированием Rust-кода.
- `fork cards validate` проверяет форму карточки и связь с исполняемой картой.
- Режим списка `fork tests` должен обнаружить
  `fork-goal-world-state-compaction`.
- `fork build-fast` проверяет release-fast сборку после целевых tests.
- Schema/API generators не требуются: config, app-server protocol и schema не
  меняются.
- Полный workspace test suite не является автоматическим требованием этой
  локальной доработки; его запуск требует отдельного решения пользователя.

### Исторические результаты

2026-07-29:

#### Перенос на `0.146.0`

- при переносе на `0.146.0` воспроизведена регрессия: для thread без когда-либо
  созданной goal `active_goal_world_state_section(None)` получал
  `PreviousWorldStateSection::Unknown` после compaction и ошибочно рендерил
  `NO_ACTIVE_GOAL_BODY`, хотя прежняя active goal не была доказана;
- исправление различает доказанный `Known(active)` и состояния `Absent`,
  `Unknown`, `Known(inactive)` и `Known(unavailable)`; маркер очистки остаётся
  только для перехода из доказанной active goal;
- прямой test без compaction проверяет второй sampling после `update_plan`:
  нормализация inactive snapshot из `{"state":"inactive","body":null}` в
  `{"state":"inactive"}` не должна создавать goal fragment;
- отдельные интеграционные регрессионные тесты проверяют local summary
  compaction и `TokenBudget` reset без когда-либо созданной goal;
- первый запуск
  `fork tests --mode cards --card fork-goal-world-state-compaction --version
  0.146.0` остановлен до компиляции общей проверкой `migration map ready`, поскольку
  соседние карточки ещё имеют незавершённые статусы;
- `fork format --fix` и финальный `fork format --check` дошли до formatter
  репозитория и остановились на чужих неразрешённых конфликтах слияния вне
  файлов карточки;
- `fork cards validate` — успешно, проверено 25 карточек, ошибок нет;
- `fork tests --mode list --card fork-goal-world-state-compaction` — успешно,
  обнаружены обе команды `fork-tests.v1`, включая новые never-active tests по
  фильтру `active_goal_world_state`;
- при переносе на `0.146.0` upstream заменил тестовую конфигурацию app-server
  `write_mock_responses_config_toml` на `MockResponsesConfig` и добавил
  `build_initialized_with_timeout`; оба регрессионных теста карточки переведены
  на новый API без изменения проверяемого контракта compaction, целевые тесты
  оставлены общему проверочному проходу родительского агента;

#### Исходная реализация и проверка на `0.145.0`

- первый запуск тестов карточки остановился при компиляции
  `codex-goal-extension`:
  фактический builder API регистрирует `ContextContributor` методом
  `prompt_contributor`, а не `context_contributor`; вызов исправлен без изменения
  архитектуры;
- `fork format --fix` — успешно;
- `fork cards validate` — успешно, проверено 25 карточек, ошибок нет;
- `fork tests --mode list --card fork-goal-world-state-compaction` — успешно,
  обнаружены обе команды `fork-tests.v1`;
- `fork tests --mode cards --card fork-goal-world-state-compaction --version
  0.145.0` — успешно:
  - `just test -p codex-goal-extension`;
  - `just test -p codex-app-server active_goal_world_state`;
  - локальный тест дополнительно подтверждает clearing fragment после
    `update_goal complete`;
- `fork build-fast --version 0.145.0` — успешно: бинарник профиля `release-fast`
  собран, его метаданные и версия проверены;
- после добавления состояния `unavailable` и устранения вложенного маркера оба
  теста карточки и `fork build-fast` повторно прошли на окончательной версии;
- финальные `fork format --check` и `git diff --check` — успешно;
- `fork install` атомарно установил бинарник профиля `release-fast` в
  `/home/slader/.local/bin/codex-hermione`;
- тот же бинарник скопирован на
  `f-ms-dev:/home/slader/.local/bin/.codex-hermione.new`, проверен до замены и
  атомарно переименован в
  `f-ms-dev:/home/slader/.local/bin/codex-hermione`;
- обе установки подтверждены одинаковыми параметрами:
  `codex-cli 0.145.0+hermione`, `372635784` байта, права `755`, SHA-256
  `bf21d75d30abc8d61f29167177bfafaa4456276872cd198fc66543abb170f040`.

### Известные падения и пропуски

- На незавершённой migration map `0.146.0` тесты уровня карточки блокируются до
  компиляции общей проверкой `migration map ready`, пока соседние карточки имеют
  незавершённые статусы. Родительский агент должен повторить запуск после
  завершения прохода по карточкам.
- Общая проверка formatter блокируется чужими неразрешёнными конфликтами
  слияния. Родительский агент должен повторить форматирование после их
  устранения.
- Remote compaction не получает отдельный goal-specific test: общий
  initial-context path совпадает с уже используемым extension-owned
  `WorldState`, а отдельные local и `TokenBudget` tests различают summary и
  reset semantics. Если upstream разделит эти пути, remote scenario станет
  обязательным отдельным test.

## Runtime, сборка и установка

Доработка не меняет формат release-бинарника и не требует schema generation.

Бинарник профиля `release-fast` установлен и проверен:

- локально: `/home/slader/.local/bin/codex-hermione`;
- на `f-ms-dev`: `/home/slader/.local/bin/codex-hermione`.

На `f-ms-dev` целевой файл заменён только после проверки временного бинарника по
версии и SHA-256. После атомарного переименования удалённая цель повторно
проверена.

## Риски и ограничения

- `contribute_world_state` читает persisted goal перед sampling. Ошибка чтения
  не подменяет неизвестное состояние clearing fragment: она логируется и
  создаёт состояние `unavailable`, которое запрещает считать старый
  `<thread_goal_context>` актуальным. После восстановления чтения section
  переходит к свежей active goal либо к inactive snapshot без маркера очистки:
  `Known(unavailable)` не доказывает, что goal прежде была active.
- Только status `Active` создаёт active-goal section. `BudgetLimited`,
  `UsageLimited`, `Paused`, `Blocked` и `Complete` не должны продолжаться как
  активная goal.
- `budget_limit.md` остаётся отдельным status-transition steering contract.
- **P0 model-context review:** fragment может превысить 1k tokens из-за
  разрешённого objective длиной до 4000 символов и постоянных goal-инвариантов.
  Ручная проверка подтвердила, что доработка переносит уже существовавшие
  objective и инварианты из continuation prompt, не увеличивая их максимальный
  model-visible payload; размер остаётся ограничен существующим objective cap и
  статическим шаблоном.
- Objective ограничен `MAX_THREAD_GOAL_OBJECTIVE_CHARS = 4000`; новый fragment
  не добавляет unbounded data и сохраняет прежнюю верхнюю границу goal
  continuation item: существующий objective cap плюс статический шаблон.
- Стабильный section ID и markers становятся rollout/model-context contract и
  не должны переименовываться без migration-разбора retained history.
- Legacy goal helpers в `codex-prompts` не изменяются, пока у них нет
  production-callers. Появление такого caller требует синхронизации или
  удаления дублирования.

## Проверка покрытия

| Согласованный пункт | Статус |
| --- | --- |
| Persisted goal остаётся источником истины | перенесено в карточку |
| Objective восстанавливается после compaction | перенесено в карточку |
| Local, remote и `TokenBudget` используют общий механизм | перенесено в карточку |
| Goal не связывается напрямую с `codex-core` compaction | перенесено в карточку |
| Неизменившийся objective не повторяется перед каждым sampling | перенесено в карточку |
| Continuation prompt не дублирует objective | перенесено в карточку |
| Goal, созданная в compaction-triggering step, видна следующему sampling | перенесено в карточку |
| Неактивный status очищает прежний active-goal context | перенесено в карточку |
| Нормализованный `Known(inactive)` не создаёт маркер очистки при обычном tool follow-up | перенесено в карточку и покрыто прямым integration test |
| Никогда не активная goal не создаёт маркер очистки после compaction | перенесено в карточку и покрыто local/`TokenBudget` tests |
| API, config и schema остаются неизменными | перенесено в карточку |
| Проверки local summary и `TokenBudget` reset | перенесено в карточку |
| Remote-specific regression test | не применимо: общий initial-context path; условие пересмотра зафиксировано |
| Runtime install и внешний host | не применимо: не входят в текущую задачу |
