---
id: fork-goal-world-state-compaction
status: active
created: 2026-07-29
updated: 2026-08-28
---

# Thread goal в `WorldState` после compaction и terminal transition

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая:

- сохраняет точный `objective` активной thread goal в model-visible context
  после local, remote и `TokenBudget` compaction;
- позволяет модели немедленно отменить существующую goal через
  `update_goal(status="cancel")`;
- доставляет clearing fragment после `complete`, `blocked` или `cancel` только
  в ближайший sampling и не превращает его в постоянное напоминание истории.

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
| `codex-rs/ext/goal/src/spec.rs` | Объявляет `cancel` в model-visible schema и отделяет его от blocked audit |
| `codex-rs/ext/goal/src/tool.rs` | Немедленно учитывает финальный progress и удаляет отменённую goal |
| `codex-rs/ext/goal/src/analytics.rs` | Сохраняет turn attribution для tool-side clear |
| `codex-rs/ext/goal/src/events.rs` | Передаёт host уведомление об очистке goal |
| `codex-rs/ext/goal/src/runtime.rs` | Отличает внешний clear без turn attribution от model tool action |
| `codex-rs/ext/goal/src/steering.rs` | Рендерит устойчивый active-goal context и короткие turn steering prompts |
| `codex-rs/ext/goal/templates/goals/active_context.md` | Хранит objective и постоянные goal-инварианты |
| `codex-rs/ext/goal/templates/goals/continuation.md` | Запускает автоматический continuation turn без дублирования objective |
| `codex-rs/ext/goal/templates/goals/objective_updated.md` | Ссылается на обновлённый objective из `WorldState` |
| `codex-rs/ext/goal/templates/goals/budget_limit.md` | Разрешает завершить либо отменить уже ограниченную по бюджету goal |
| `codex-rs/ext/extension-api/src/contributors/world_state.rs` | Помечает diff fragment как предназначенный только для ближайшего sampling |
| `codex-rs/ext/extension-api/src/contributors.rs` | Даёт extension способ распознать собственный model-context fragment при замене истории |
| `codex-rs/ext/extension-api/src/capabilities/events.rs` | Даёт extension host-owned канал уведомления об очищенной goal |
| `codex-rs/core/src/compact.rs` | Не переносит extension context в local summary как пользовательское сообщение |
| `codex-rs/core/src/compact_tests.rs` | Проверяет отбор реальных user messages с сохранением metadata без extension prompt state |
| `codex-rs/core/src/compact_remote.rs` | Удаляет extension context из remote replacement перед актуальным initial context |
| `codex-rs/core/src/compact_remote_v2.rs` | Удаляет группы extension-owned source items из Remote V2 history перед новым initial context |
| `codex-rs/core/src/compact_remote_history.rs` | Сохраняет source item вместе с attached notice при фильтрации remote replacement |
| `codex-rs/core/src/compact_token_budget.rs` | Устанавливает свежий WorldState как новое окно без summary request |
| `codex-rs/core/src/context/world_state/mod.rs` | Разделяет сохраняемые и одноразовые WorldState fragments |
| `codex-rs/core/src/context_manager/history.rs` | Продвигает snapshot, не записывая одноразовый fragment в history |
| `codex-rs/core/src/context_manager/history_tests.rs` | Проверяет baseline deduplication и сверку snapshot с retained history |
| `codex-rs/core/src/prompt_debug.rs` | Показывает одноразовый fragment в том же prompt, который получает sampling |
| `codex-rs/core/src/session/mod.rs` | Возвращает одноразовые WorldState items вызывающему sampling path |
| `codex-rs/core/src/session/rollout_reconstruction.rs` | Не восстанавливает extension prompt state как user message в legacy replacement path |
| `codex-rs/core/src/session/turn.rs` | Добавляет одноразовые items только в ближайший Responses request |
| `codex-rs/core/src/session/world_state.rs` | Строит актуальный extension-owned WorldState для sampling и replacement history |
| `codex-rs/app-server/src/extensions.rs` | Преобразует tool-side clear в `thread/goal/cleared` |
| `codex-rs/ext/goal/tests/goal_extension_backend.rs` | Проверяет немедленное удаление goal и clear event |
| `codex-rs/app-server/tests/suite/v2/compaction.rs` | Проверяет active, never-active и terminal goal context до и после compaction |

Намеренно не меняются:

- app-server schema/protocol и публичные goal RPC;
- persisted формат `thread_goals`;
- persisted enum `ThreadGoalStatus`: `cancel` является действием удаления, а не
  новым status;
- budget accounting и правила переходов остальных goal status;
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
- clearing fragment не записывается в conversation history: он добавляется
  только в ближайший sampling request, после чего исчезает;
- goal extension сохраняет для fragment актуальные upstream-метаданные
  `ContentItemKind("active_goal.instructions")`; одноразовая доставка меняет
  только способ сохранения, а не классификацию содержимого;
- snapshot при этом сразу продвигается в `inactive`, поэтому следующий model
  step не создаёт тот же fragment повторно;
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

### Cancel contract

`update_goal(status="cancel")`:

- доступен только при существующей goal;
- немедленно учитывает накопленный progress текущего turn;
- удаляет goal из `state_db.thread_goals`, очищает accounting state и возвращает
  `goal: null`;
- отправляет host уведомление `thread/goal/cleared`;
- не создаёт persisted status `Cancelled`;
- не применяет трёхходовый blocked audit: этот audit относится только к
  `status="blocked"`;
- после удаления позволяет обычному `create_goal` создать новую goal.

`complete` и `blocked` остаются persisted terminal statuses. Все три исхода —
`complete`, `blocked` и `cancel` — переводят active-goal WorldState section в
`inactive`.

### Compaction contract

`WorldState` является частью initial context, которую compaction runtime
разворачивает независимо от summary:

- local mid-turn compaction получает точный active-goal fragment через
  `build_compaction_initial_context`;
- remote compaction получает тот же fragment через общий initial-context path;
- Remote V2 сохраняет retained history с metadata, но перед установкой удаляет
  группу extension-owned source item вместе с attached notice и затем добавляет
  только свежий full `WorldState`;
- `TokenBudget` reset получает fragment через `start_new_context_window`;
- если goal создана или изменена в model step, который сам вызвал compaction,
  следующий step заново строит `WorldState` перед sampling и добавляет свежий
  fragment;
- thread, в котором goal никогда не создавалась, не получает
  `<thread_goal_context>` после local или `TokenBudget` compaction.

Если tool call меняет goal в том же sampling step, который достигает порога
compaction, runtime повторно строит `WorldState` перед заменой history. Full
replacement получает уже новое состояние, а одноразовый terminal fragment
остаётся отложенным до следующего обычного sampling. Поэтому создание goal не
дублирует active context, а terminal transition не оставляет старый active
fragment внутри нового окна.

Первый post-compaction model request обязан содержать objective дословно после
XML escaping, даже если compaction output намеренно не содержит goal.

Если goal стала terminal или была отменена до compaction, replacement history
не должна восстанавливать ни прежний active-goal fragment, ни clearing fragment.
Для этого extension распознаёт собственные model-context markers, а local и
remote compaction исключают такие fragments из conversation messages до
добавления актуального full `WorldState`. Общий runtime при этом не знает о
goal feature или её XML tags.

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

### Почему clearing fragment одноразовый

Persisted WorldState snapshot отвечает за сравнение состояния, но обычный
contextual fragment, записанный в conversation history, повторно отправляется в
каждом следующем Responses request. Поэтому одного подавления повторного render
недостаточно: clearing fragment всё равно становится постоянным напоминанием.

WorldState diff разделяет:

- сохраняемые fragments, которые входят в conversation history и rollout;
- fragments ближайшего sampling, которые добавляются только в создаваемый
  request.

Снимок сохраняется в обоих случаях. Это сохраняет инкрементальную историю,
не требует её переписывать и гарантирует одноразовую доставку terminal
transition.

### Почему cancel не является новым status

Новый persisted status `Cancelled` расширил бы state/protocol/app-server schema,
хотя требуемая семантика — отсутствие текущей goal. Удаление через существующий
goal store даёт прямой контракт `get_goal -> null`, не оставляет отменённую цель
кандидатом на continuation и позволяет создать новую goal без специального
status transition.

### Почему compaction фильтрует extension context

Active-goal diff хранится как contextual user message, чтобы оставаться частью
инкрементальной model history до следующей замены окна. Без явного
extension-owned matcher local и remote compaction принимали такой standalone
fragment за реальную пользовательскую реплику и переносили его в replacement
history даже после terminal transition.

`ContextContributor::matches_model_context_fragment` оставляет владение markers
внутри extension. Local, Remote V1 и legacy rollout-reconstruction применяют
признак при сборке replacement user messages. Remote V2 отдельно владеет
retained history с metadata, поэтому применяет тот же признак к группе source
item до добавления актуального `WorldState` обычным initial-context path;
attached notice не отделяется от удалённого source item.

## Порядок повторения при переносе

1. Найти runtime-владельца goal extension и способ регистрации extension
   contributors в новом upstream.
2. Подтвердить, что compaction по-прежнему восстанавливает extension-owned
   `WorldState` через общий initial-context path.
3. Если remote history группирует source item с attached notices, фильтровать
   extension context на source group до `flat_map`, сохраняя notices только у
   оставшихся source items. Для Remote V2 повторять фильтрацию после его
   собственного retained-history builder и до вставки нового initial context.
4. Добавить стабильную секцию active goal рядом с runtime goal extension, а не
   в общий `codex-core`.
5. Сохранить role `user`, XML escaping и ограничение размера objective.
6. Разделить устойчивый active-goal context и динамический continuation
   steering, не допуская двойного objective.
7. Проверить local summary compaction и context-window reset, при которых
   compaction output не содержит objective.
8. Проверить terminal transition в model step, который сам запускает
   compaction: replacement не содержит старую active goal, а следующий sampling
   получает ровно один clearing fragment.
9. Если upstream изменил владельца goal prompts, перенести шаблоны к актуальному
   production-owner и удалить либо синхронизировать ставшие активными legacy
   копии.

## Проверки

Исполняемая карта card-level regression tests:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "local compaction исключает extension prompt state без потери metadata реальных user messages",
      "argv": ["just", "test", "-p", "codex-core", "collect_annotated_user_messages_excluding_preserves_retained_metadata"]
    },
    {
      "purpose": "goal lifecycle, persistence, unavailable read failure и world-state contributor extension",
      "argv": ["just", "test", "-p", "codex-goal-extension"]
    },
    {
      "purpose": "goal world state до, во время и после всех compaction transitions",
      "argv": ["just", "test", "-p", "codex-app-server", "active_goal_world_state"]
    }
  ]
}
```

Тест `goal_world_state_reports_unavailable_when_persisted_goal_read_fails`
создаёт active goal через установленный tool, получает active snapshot, затем
закрывает настоящий `StateRuntime`. Ошибка чтения persisted goal из закрытого
pool должна дать snapshot `{"state":"unavailable"}` и сохраняемый
model-visible fragment, а не одноразовый clearing fragment. Тест принят
статической вычиткой; его компиляция, форматирование и запуск отложены до общего
прохода по карточкам.

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
- Post-sampling rebuild перед compaction обязан использовать тот же
  request-scoped `StepContext`: повторный захват окружения в середине шага может
  рассинхронизировать advertised tools, context и фактический tool execution.
- Legacy goal helpers в `codex-prompts` не изменяются, пока у них нет
  production-callers. Появление такого caller требует синхронизации или
  удаления дублирования.
