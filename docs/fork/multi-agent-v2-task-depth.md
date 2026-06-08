---
id: fork-multi-agent-v2-task-depth
status: reverted
created: 2026-06-08
updated: 2026-06-08
source_scope: reverted-current-worktree
---

# MultiAgent V2: текущая задача, forked history и запрет nested spawn

## Обзор

Эта карточка сохранена как запись об откатанной доработке MultiAgent V2 в форке
Hermione. Она больше не является активным handoff к применению изменения:
пользователь решил вернуться на MultiAgent V1, потому что текущее развитие
MultiAgent V2 пока не подходит для fork-линии.

Кодовая часть, описанная ниже, откатана 2026-06-08 точечно по файлам из этой
карточки. Другие доработки, описанные в `docs/fork/`, не входят в этот откат и
не должны затрагиваться при обслуживании этой записи.

| Поле | Значение |
| --- | --- |
| Статус | `reverted` |
| Область | MultiAgent V2 runtime, prompt/tool surface, forked history и регрессионные тесты |
| Главная причина | При `fork_turns=N` дочерний агент мог воспринимать скопированную parent history как активную задачу, а не как фон |
| Решение после отката | Вернуться на MultiAgent V1; не применять эту V2-доработку без нового явного решения |
| Исторический контракт до отката | Текущая inter-agent задача должна быть явной `NEW_TASK`; скопированная history является справочным контекстом; V2 `spawn_agent` уважает `agent_max_depth` |
| Пользовательский config | Не менять; новые config keys не нужны |
| Проверка компиляции до отката | `just build-fast-release` на `f-ms-dev:/home/slader/Projects/codex` проходил для старой версии доработки |
| Проверки после отката | Сборка и тесты не запускались; для тестов и debug нужно отдельное согласие пользователя |
| Документ-владелец | Эта карточка в `docs/fork/` является исторической записью по откатанной fork-доработке; она не заменяет документацию продукта upstream |

## Состояние после отката

2026-06-08 кодовая часть откатана к состоянию `HEAD` для этих файлов:

- `codex-rs/protocol/src/protocol.rs`;
- `codex-rs/core/src/agent/control.rs`;
- `codex-rs/core/src/agent/control_tests.rs`;
- `codex-rs/core/src/config/mod.rs`;
- `codex-rs/core/src/context_manager/history.rs`;
- `codex-rs/core/src/context_manager/history_tests.rs`;
- `codex-rs/core/src/thread_rollout_truncation.rs`;
- `codex-rs/core/src/thread_rollout_truncation_tests.rs`;
- `codex-rs/core/src/tools/handlers/multi_agents_spec.rs`;
- `codex-rs/core/src/tools/handlers/multi_agents_spec_tests.rs`;
- `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`;
- `codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs`;
- `codex-rs/core/src/tools/spec_plan.rs`;
- `codex-rs/core/src/tools/spec_plan_tests.rs`.

При дальнейшей работе с fork-доработками не используй раздел
[Пошаговое воспроизведение доработки](#пошаговое-воспроизведение-доработки)
как инструкцию к действию. Он оставлен только для понимания того, что именно
было удалено из текущей линии.

## Исходная область

До отката карточка была основана на рабочем дереве
`/home/slader/Projects/evilcats/codex` после реализации доработки и на удалённой
проверке в `f-ms-dev:/home/slader/Projects/codex`. После решения вернуться на
MultiAgent V1 этот раздел оставлен как историческая карта файлов, которые были
затронуты и затем возвращены к `HEAD`.

Использованные источники в рабочем дереве:

- [protocol.rs](../../codex-rs/protocol/src/protocol.rs) -
  `InterAgentCommunication`, `Op::InterAgentCommunication`, модельный формат
  inter-agent сообщений и совместимость со старым JSON.
- [control.rs](../../codex-rs/core/src/agent/control.rs) - фильтрация элементов
  при forked history.
- [thread_rollout_truncation.rs](../../codex-rs/core/src/thread_rollout_truncation.rs) -
  границы `fork_turns=N`.
- [history.rs](../../codex-rs/core/src/context_manager/history.rs) - границы
  instruction turns в истории.
- [spawn.rs](../../codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs) -
  runtime-проверка глубины для V2 `spawn_agent`.
- [spec_plan.rs](../../codex-rs/core/src/tools/spec_plan.rs) - model-visible
  tool surface для MultiAgent V2.
- [multi_agents_spec.rs](../../codex-rs/core/src/tools/handlers/multi_agents_spec.rs) -
  описание V2 `spawn_agent` tool.
- [mod.rs](../../codex-rs/core/src/config/mod.rs) - кодовые default usage hints
  для MultiAgent V2.
- Регрессионные тесты:
  [protocol.rs](../../codex-rs/protocol/src/protocol.rs),
  [control_tests.rs](../../codex-rs/core/src/agent/control_tests.rs),
  [history_tests.rs](../../codex-rs/core/src/context_manager/history_tests.rs),
  [thread_rollout_truncation_tests.rs](../../codex-rs/core/src/thread_rollout_truncation_tests.rs),
  [multi_agents_spec_tests.rs](../../codex-rs/core/src/tools/handlers/multi_agents_spec_tests.rs),
  [multi_agents_tests.rs](../../codex-rs/core/src/tools/handlers/multi_agents_tests.rs),
  [spec_plan_tests.rs](../../codex-rs/core/src/tools/spec_plan_tests.rs).

Созданные checkpoints:

- `/home/slader/.cache/hermione/evilcats-codex/checkpoints/CHK-20260608-001-multiagent-v2-task-depth/hermione.md`
  - checkpoint перед кодовой доработкой.
- `/home/slader/.cache/hermione/evilcats-codex/checkpoints/CHK-20260608-002-fork-doc-multiagent-v2-task-depth/hermione.md`
  - checkpoint перед созданием этой карточки.
- `/home/slader/.cache/hermione/evilcats-codex/checkpoints/CHK-20260608-003-multi-agent-v2-task-depth-revert/hermione.md`
  - checkpoint перед откатом кодовой части.

Не использовалось как источник истины:

- сырой transcript чата;
- пользовательские profile/config-файлы;
- внешние сайты или upstream-документация;
- результаты тестов, потому что для их запуска требовалось отдельное согласие.

## Проблема

Пользователь заметил рассинхрон при работе с MultiAgent V2 и `fork_turns=N`:
дочерний агент получает часть контекста родителя, но иногда начинает исполнять
последнюю фразу из скопированного контекста так, будто это его текущий prompt.
В другой сессии это проявилось особенно явно: агент увидел в скопированной
history обсуждение про subagents и начал пытаться запускать subagents, хотя
его текущая задача была другой.

Разбор показал две связанные причины.

Во-первых, V2 `spawn_agent` передавал initial plain-text task через
`Op::InterAgentCommunication`. `InterAgentCommunication::to_response_input_item`
рендерил его как `assistant` message с `phase: Commentary` и JSON-текстом.
Одновременно `fork_turns=N` сохранял в child history настоящие `user` messages
из parent thread. Для модели скопированная `user` history могла выглядеть
более похожей на текущую задачу, чем assistant/commentary JSON envelope.

Во-вторых, V2 игнорировал существующий `[agents].max_depth`. В V1
`spawn_agent` уже проверял depth limit, а V2 имел тест, который прямо закреплял
ошибочное поведение: `multi_agent_v2_spawn_agent_ignores_configured_max_depth`.
При default `DEFAULT_AGENT_MAX_DEPTH = 1` ожидаемый контракт такой: root может
создать прямого child, но child на depth 1 не должен создавать nested child.

Дополнительный prompt-риск: V2 tool description и кодовые default usage hints
в `config/mod.rs` учили модель старому поведению. В них были формулировки о
том, что child agents могут создавать собственных subagents или имеют тот же
набор tools.

## Принятый контракт

Доработка не добавляет новый пользовательский config. Она заставляет V2 уважать
уже существующие механизмы и делает текущую задачу явной для модели.

Принятые правила:

1. Текущая inter-agent задача должна быть model-visible как `user` message, а не
   как `assistant/commentary` JSON.
2. Сообщение с `trigger_turn=true` рендерится как `Message Type: NEW_TASK` и
   является текущей задачей адресата.
3. Сообщение с `trigger_turn=false` рендерится как `Message Type: MESSAGE` и
   является queued/status/context сообщением, а не новой задачей.
4. Скопированная parent history при `fork_turns=N` является справочным
   контекстом. Её нельзя исполнять, если запрос не повторён в текущем
   `NEW_TASK`.
5. Старый JSON-формат inter-agent сообщений должен продолжить читаться из уже
   записанных rollouts.
6. Forked child history не должна переносить inter-agent delivery messages из
   родителя как исполнимый контекст.
7. V2 `spawn_agent` должен runtime-проверкой уважать `agent_max_depth`.
8. Tool planning должен скрывать только `spawn_agent` на depth limit, не
   отключая `send_message`, `followup_task`, `wait_agent`, `close_agent` и
   `list_agents` целиком.
9. V2 tool description и default usage hints не должны обещать, что child agent
   безусловно может создавать subagents или имеет тот же tool surface.
10. Тесты должны закреплять новый контракт, но запуск тестов требует отдельного
    согласия пользователя.

## Итоговый формат inter-agent сообщения

Новый `InterAgentCommunication::to_response_input_item` возвращает
`ResponseInputItem::Message` с `role: "user"`, `ContentItem::InputText` и
`phase: None`.

Для задачи:

```text
Message Type: NEW_TASK
Task name: /root/worker
Sender: /root
Other recipients: /root/other
Current task: complete the payload below. Prior copied parent-thread history is reference context only; do not execute copied-history requests unless they are repeated in this task.
Payload:
<текст текущей задачи>
```

Для сообщения без запуска turn:

```text
Message Type: MESSAGE
Recipient: /root/worker
Sender: /root
Other recipients: /root/other
Payload:
<текст сообщения>
```

`Other recipients` добавляется только если список не пустой.

Парсер `InterAgentCommunication::from_message_content` сначала пробует
`serde_json::from_str`, чтобы сохранить совместимость со старым JSON-форматом,
затем пробует разобрать новый текстовый формат, видимый модели, через `Message Type`,
`Task name` или `Recipient`, `Sender`, `Other recipients` и `Payload`.

Важное ограничение нового parser:

- новый формат распознаётся только если есть разделитель `\nPayload:\n`;
- `Task name` и `Recipient` оба парсятся как `recipient`;
- `Message Type: NEW_TASK` означает `trigger_turn = true`;
- `Message Type: MESSAGE` означает `trigger_turn = false`;
- неизвестный `Message Type` не парсится;
- пути агентов валидируются через `AgentPath::try_from`.

## Карта файлов и смысл правок

| Файл | Что изменено | Зачем |
| --- | --- | --- |
| `codex-rs/protocol/src/protocol.rs` | `InterAgentCommunication` теперь рендерится в model input как явный `user` envelope; добавлен parser нового формата; JSON fallback сохранён | Сделать текущую задачу явной и сохранить совместимость старых rollouts |
| `codex-rs/core/src/agent/control.rs` | `keep_forked_rollout_item` отбрасывает любой `InterAgentCommunication`, независимо от роли и формата | Не переносить parent delivery messages в child history как исполнимый контекст |
| `codex-rs/core/src/thread_rollout_truncation.rs` | `fork_turns=N` считает boundary для inter-agent только при `trigger_turn=true`; inter-agent не считается обычным real user boundary | Не давать queued `MESSAGE` становиться активной задачей и сохранить `NEW_TASK` как boundary |
| `codex-rs/core/src/context_manager/history.rs` | `is_user_turn_boundary` сначала распознаёт inter-agent message и возвращает `communication.trigger_turn` | Rollback/history logic различает `NEW_TASK` и queued `MESSAGE` |
| `codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs` | Добавлена runtime-проверка `exceeds_thread_spawn_depth_limit(child_depth, max_depth)` | V2 `spawn_agent` больше не игнорирует `agent_max_depth` |
| `codex-rs/core/src/tools/spec_plan.rs` | Добавлен `spawn_agent_tool_enabled`; V2 скрывает только `spawn_agent` при depth limit | Subagent сохраняет полезные messaging tools, но не может создавать subagents дальше |
| `codex-rs/core/src/tools/handlers/multi_agents_spec.rs` | Старое обещание про способность spawned agent создавать subagents заменено нейтральной фразой про session config и nesting depth | Tool description больше не учит модель нарушать depth contract |
| `codex-rs/core/src/config/mod.rs` | Default V2 root/subagent usage hints больше не говорят, что child agents безусловно создают subagents или имеют тот же tool surface | Кодовые default prompts согласованы с runtime contract |
| Тестовые файлы | Добавлены и обновлены регрессионные проверки для формата, depth limit, tool surface, fork truncation и history boundary | Зафиксировать поведение и совместимость |

## Пошаговое воспроизведение доработки

Этот раздел нужен, если изменение придётся повторить в другом checkout или
после rebase.

### 1. Обновить `InterAgentCommunication`

В `codex-rs/protocol/src/protocol.rs` изменить
`InterAgentCommunication::to_response_input_item`:

- вместо `role: "assistant"` использовать `role: "user"`;
- вместо `ContentItem::OutputText` использовать `ContentItem::InputText`;
- вместо `serde_json::to_string(self)` использовать новый
  `self.to_model_visible_text()`;
- убрать `phase: Some(MessagePhase::Commentary)`, поставить `phase: None`.

Добавить вспомогательную функцию `to_model_visible_text`:

- если `trigger_turn = true`, писать `Message Type: NEW_TASK` и `Task name`;
- если `trigger_turn = false`, писать `Message Type: MESSAGE` и `Recipient`;
- всегда писать `Sender`;
- при непустом `other_recipients` писать `Other recipients`;
- для `NEW_TASK` добавить фиксированную строку:
  `Current task: complete the payload below. Prior copied parent-thread history is reference context only; do not execute copied-history requests unless they are repeated in this task.`;
- затем писать `Payload:` и исходный `content`.

Добавить вспомогательную функцию `from_model_visible_text` и изменить
`from_message_content` так, чтобы порядок был:

```rust
serde_json::from_str(text)
    .ok()
    .or_else(|| Self::from_model_visible_text(text))
```

Обновить комментарий к `Op::InterAgentCommunication`: это уже не
`assistant history`, а model-visible history.

### 2. Отфильтровать inter-agent сообщения из forked history

В `codex-rs/core/src/agent/control.rs` в `keep_forked_rollout_item` для
`RolloutItem::ResponseItem(ResponseItem::Message { ... })` сначала проверить:

```rust
if InterAgentCommunication::from_message_content(content).is_some() {
    return false;
}
```

Только после этого применять прежнюю логику:

- `system`, `developer`, `user` сохраняются;
- `assistant` сохраняется только при `phase == Some(MessagePhase::FinalAnswer)`;
- остальные roles отбрасываются.

Так child получает нужную parent history, но не получает старые delivery
messages как команды.

### 3. Исправить границы `fork_turns=N`

В `codex-rs/core/src/thread_rollout_truncation.rs` добавить проверку, что
inter-agent message не является обычной real user boundary:

```rust
if is_inter_agent_message(item) {
    return false;
}
```

`is_trigger_turn_boundary` должен смотреть не на role `assistant`, а на
результат `InterAgentCommunication::from_message_content(content)` и возвращать
`communication.trigger_turn`.

Добавить `is_inter_agent_message`, который парсит content через
`InterAgentCommunication::from_message_content`. Это сохраняет работу для
старого assistant JSON и нового user-visible envelope.

### 4. Исправить границы истории

В `codex-rs/core/src/context_manager/history.rs` изменить
`is_user_turn_boundary`:

1. Если message парсится как `InterAgentCommunication`, вернуть
   `communication.trigger_turn`.
2. Иначе считать boundary только обычное `role == "user"` сообщение, которое не
   является contextual user fragment.

Это важно для rollback и history operations: queued `MESSAGE` не должен
считаться instruction turn.

### 5. Включить V2 depth enforcement

В `codex-rs/core/src/tools/handlers/multi_agents_v2/spawn.rs` импортировать
`exceeds_thread_spawn_depth_limit`.

После вычисления:

```rust
let session_source = turn.session_source.clone();
let child_depth = next_thread_spawn_depth(&session_source);
```

добавить:

```rust
let max_depth = turn.config.agent_max_depth;
if exceeds_thread_spawn_depth_limit(child_depth, max_depth) {
    return Err(FunctionCallError::RespondToModel(
        "Agent depth limit reached. Solve the task yourself.".to_string(),
    ));
}
```

Поведение должно совпадать с V1 по смыслу и тексту отказа.

### 6. Скрыть `spawn_agent` на depth limit, не отключая остальные tools

В `codex-rs/core/src/tools/spec_plan.rs` оставить `collab_tools_enabled` для V2
как `true`, чтобы не отключить все MultiAgent V2 tools.

Добавить отдельную вспомогательную функцию:

```rust
fn spawn_agent_tool_enabled(turn_context: &TurnContext) -> bool {
    !exceeds_thread_spawn_depth_limit(
        next_thread_spawn_depth(&turn_context.session_source),
        turn_context.config.agent_max_depth,
    )
}
```

В `add_collaboration_tools` для V2 оборачивать добавление
`SpawnAgentHandlerV2` в `if spawn_agent_tool_enabled(turn_context)`, но
оставить добавление `SendMessageHandlerV2`, `FollowupTaskHandlerV2`,
`WaitAgentHandlerV2`, `CloseAgentHandlerV2` и `ListAgentsHandlerV2`.

### 7. Исправить описание `spawn_agent`

В `codex-rs/core/src/tools/handlers/multi_agents_spec.rs` заменить строку:

```text
The spawned agent will have the same tools as you and the ability to spawn its own subagents.
```

на:

```text
The spawned agent's available tools are determined by the session configuration and the agent's nesting depth.
```

Это не косметическая правка. Tool description является prompt surface, и
старая формулировка прямо учила модель нарушать новый depth contract.

### 8. Исправить кодовые default usage hints

В `codex-rs/core/src/config/mod.rs` обновить
`DEFAULT_MULTI_AGENT_V2_ROOT_AGENT_USAGE_HINT_TEXT`:

- оставить, что root может создавать sub-agents;
- убрать утверждения, что эти sub-agents безусловно могут создавать своих
  sub-agents;
- убрать утверждение, что все agents имеют тот же набор tools;
- добавить, что создание sub-agent ограничено nesting-depth limit и child agents
  могут иметь более ограниченную tool surface.

Обновить `DEFAULT_MULTI_AGENT_V2_SUBAGENT_USAGE_HINT_TEXT`:

- убрать безусловное разрешение создавать subagents дальше;
- сказать, что доступные tools зависят от session configuration и nesting depth;
- если `spawn_agent` недоступен, subagent должен выполнить задачу сам, а если
  требуется дальнейшее деление, сообщить об этом в final answer;
- `send_message` описать как короткое сообщение о статусе или blocker;
- `followup_task`, `close_agent`, `list_agents` описывать как tools, которые
  используются только если доступны и прямо помогают назначенной задаче.

Это изменение не трогает пользовательский `hermione.config.toml`. Оно меняет
только default prompt strings в коде.

## Регрессионное покрытие в diff

Тесты были добавлены или обновлены, но не запускались без отдельного согласия
пользователя.

Покрытие, которое должно быть в diff:

- `codex-rs/protocol/src/protocol.rs`:
  - новый тест проверяет, что inter-agent task рендерится как `user`
    `NEW_TASK` envelope;
  - новый тест проверяет, что parser принимает и legacy JSON, и новый текстовый
    формат.
- `codex-rs/core/src/agent/control_tests.rs`:
  - вспомогательная проверка больше не ищет только assistant JSON;
  - проверки работают для любого формата inter-agent communication.
- `codex-rs/core/src/context_manager/history_tests.rs`:
  - новый visible `NEW_TASK` является turn boundary;
  - visible queued `MESSAGE` не является turn boundary;
  - legacy assistant JSON остаётся совместимым.
- `codex-rs/core/src/thread_rollout_truncation_tests.rs`:
  - `fork_turns=N` считает trigger-turn messages границами;
  - отдельный тест закрепляет legacy JSON trigger-turn boundary.
- `codex-rs/core/src/tools/handlers/multi_agents_spec_tests.rs`:
  - V2 `spawn_agent` description содержит новую нейтральную фразу;
  - старая фраза `spawn its own subagents` отсутствует.
- `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`:
  - старый тест `multi_agent_v2_spawn_agent_ignores_configured_max_depth`
    заменён на отказ при превышении depth limit;
  - добавлен allow-case, где `agent_max_depth = 2` разрешает nested spawn из
    child на depth 1.
- `codex-rs/core/src/tools/spec_plan_tests.rs`:
  - на depth limit V2 скрывает `spawn_agent`;
  - `send_message`, `followup_task`, `wait_agent`, `close_agent`, `list_agents`
    остаются model-visible.

## Проверки, выполненные после правки

Все Rust/Cargo/`just` операции выполнялись только на `f-ms-dev`, как просил
пользователь. Локально в checkout Rust/Cargo/`just` не запускались.

Выполнено:

1. Синхронизация изменённых файлов на `f-ms-dev:/home/slader/Projects/codex`
   через `rsync`.
2. `just fmt` на `f-ms-dev` из `/home/slader/Projects/codex/codex-rs`.
   - Rust formatter дошёл до `cargo fmt -- --config imports_granularity=Item`.
   - Общий recipe завершился с ошибкой из-за отсутствующего `uv` для Python
     SDK/scripts formatter.
   - Это не было ошибкой Rust formatting.
3. Обратная синхронизация отформатированных Rust-файлов с `f-ms-dev`.
   - Первая обратная команда `rsync -R` с абсолютными remote paths создала лишний
     untracked каталог `home/` в repo.
   - Каталог `home/` был подтверждён как артефакт этой команды и удалён.
   - Повторная обратная синхронизация выполнена с точкой relative-root
     `/./codex-rs/...`.
4. `git diff --check` локально завершился успешно.
5. Поиск старых prompt/tool-фраз ничего не нашёл:
   - `spawn their own sub-agents`;
   - `spawn their own subagents`;
   - `Child agents can also spawn`;
   - `The spawned agent will have the same tools as you`;
   - `assistant inter-agent`.
6. `cargo fmt -- --config imports_granularity=Item --check` на `f-ms-dev` из
   `/home/slader/Projects/codex/codex-rs` завершился с кодом `0`.
   - Вывод содержал повторяющиеся warnings stable `rustfmt`: `imports_granularity = Item`
     является nightly-only настройкой.
7. `just build-fast-release` на `f-ms-dev:/home/slader/Projects/codex` завершился
   успешно:

```text
Finished `release-fast` profile [optimized] target(s) in 8m 37s
```

Не выполнено:

- тесты не запускались;
- debug-команды не запускались;
- полный `just test` не запускался;
- целевые тесты не запускались;
- пользовательские config-файлы не проверялись повторно, потому что их не
  меняли.

## Ограничения и gates

Устойчивые ограничения для продолжения:

- Не добавлять новые config keys ради этой доработки.
- Не менять `hermione.config.toml` и другие profile config-файлы в рамках этой
  задачи.
- Не запускать тесты или debug-команды без отдельного согласия пользователя.
- Все Rust/Cargo/`just` операции для этого форка выполнять только на
  `f-ms-dev` в `/home/slader/Projects/codex`.
- `release-fast` разрешён как compile-check.
- Для синхронизации между локальным checkout и `f-ms-dev` можно использовать
  `rsync`; при обратной синхронизации с абсолютными remote paths использовать
  маркер relative-root `/./`, чтобы не создать вложенный `home/` внутри repo.

## Риски и важные развилки

### Изменение роли inter-agent сообщений

Новый формат меняет model-visible role с `assistant` на `user`. Это сделано
сознательно, чтобы текущая задача конкурировала с parent history как настоящая
задача, а не как assistant commentary JSON.

Риск: downstream code, который не использует
`InterAgentCommunication::from_message_content`, а напрямую ожидал assistant
JSON, может не распознать новый формат. В текущей доработке основные места
переведены на parser-based checks, но при review стоит искать прямые ожидания
assistant JSON.

### Совместимость старых rollouts

Совместимость обеспечивается тем, что parser сначала пробует JSON. Старые
rollouts с assistant JSON должны продолжить работать.

При будущей правке нельзя удалить JSON fallback без отдельной migration-модели.

### Смысл queued `MESSAGE`

`MESSAGE` с `trigger_turn=false` не должен считаться новой задачей и не должен
становиться turn boundary. Он может быть статусом, blocker update или queued
context. Это важно для `send_message`.

`followup_task` использует тот же inter-agent path, но с `trigger_turn=true`,
поэтому его payload должен быть `NEW_TASK`.

### Depth limit и tool surface

Не надо возвращать V2 в старую модель `collab_tools_enabled = false` для всего
набора tools. Это скрыло бы и messaging tools. Правильная граница сейчас:

- `spawn_agent` скрывается и runtime-отказывается при depth limit;
- остальные V2 tools остаются доступны, если они включены текущей сессией.

### Default hints и пользовательский config

Правка `codex-rs/core/src/config/mod.rs` меняет кодовые defaults. Если
пользователь переопределил `root_agent_usage_hint_text` или
`subagent_usage_hint_text` в своём config, его override продолжит иметь
приоритет.

Эта доработка не требует добавлять строки в пользовательский config.

## Как проверить, если доработку решат вернуть

Этот раздел не является текущим планом. Он нужен только на случай, если
пользователь заново решит вернуться к этой V2-доработке и разрешит
тесты/debug-команды.

Разумный порядок проверок для повторного применения:

1. На `f-ms-dev:/home/slader/Projects/codex` запустить целевые тесты для
   изменённой области, например отдельные тесты из:
   - `codex-rs/protocol/src/protocol.rs`;
   - `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`;
   - `codex-rs/core/src/tools/spec_plan_tests.rs`;
   - `codex-rs/core/src/thread_rollout_truncation_tests.rs`;
   - `codex-rs/core/src/context_manager/history_tests.rs`;
   - `codex-rs/core/src/agent/control_tests.rs`.
2. Если целевые тесты пройдут, обсудить с пользователем, запускать ли более
   широкий `just test`.
3. При необходимости сделать ручную быструю V2-проверку в отдельной сессии:
   - root создаёт child с `fork_turns=N`;
   - child получает `NEW_TASK`;
   - скопированная parent history не исполняется как задача;
   - child на default depth не видит или не может использовать `spawn_agent`;
   - `send_message` остаётся доступным для короткого статуса.

Точный набор команд для тестов не зафиксирован как выполненный результат,
потому что запуск тестов требовал отдельного согласия. После отката 2026-06-08
сборка и тесты для этой доработки не запускались.

## Историческое покрытие согласованного контекста

Эта таблица относится к исходному handoff до отката. Статус означает, что пункт
был сохранён в исторической записи, а не что кодовая доработка сейчас активна.

| Согласованный пункт | Статус | Где отражено |
| --- | --- | --- |
| В config ничего не добавлять | исторически зафиксировано | Разделы "Принятый контракт", "Ограничения и gates", "Default hints и пользовательский config" |
| Объяснить причину рассинхрона при `fork_turns=N` | исторически зафиксировано | Раздел "Проблема" |
| Сделать текущую задачу явной для модели | исторически зафиксировано | Разделы "Принятый контракт", "Итоговый формат inter-agent сообщения", "Пошаговое воспроизведение" |
| Считать copied parent history только справочным контекстом | исторически зафиксировано | Разделы "Принятый контракт", "Итоговый формат inter-agent сообщения", "Риски и важные развилки" |
| Сохранить совместимость старого JSON-формата | исторически зафиксировано | Разделы "Итоговый формат inter-agent сообщения", "Риски и важные развилки", "Регрессионное покрытие" |
| Включить V2 depth enforcement | исторически зафиксировано | Разделы "Принятый контракт", "Пошаговое воспроизведение", "Карта файлов" |
| Скрывать `spawn_agent`, но не ломать остальные V2 tools | исторически зафиксировано | Разделы "Пошаговое воспроизведение", "Depth limit и tool surface" |
| Убрать старые prompt/tool фразы про nested subagents | исторически зафиксировано | Разделы "Пошаговое воспроизведение", "Проверки" |
| Зафиксировать тесты как добавленные, но не выполненные | исторически зафиксировано | Разделы "Регрессионное покрытие", "Проверки", "Как проверить, если доработку решат вернуть" |
| Сохранить remote-only правило для Rust/Cargo/`just` | исторически зафиксировано | Разделы "Проверки", "Ограничения и gates" |
| Зафиксировать `release-fast` compile-check | исторически зафиксировано | Разделы "Обзор", "Проверки" |
| Указать rsync-ошибку и cleanup | исторически зафиксировано | Раздел "Проверки" |

## Текущий статус для следующего агента

Кодовая доработка откатана из рабочего дерева. Карточка оставлена, чтобы
следующий агент понимал причину отката, затронутые файлы, исторический diff и
не восстанавливал V2-поведение случайно.

Если пользователь заново решит вернуться к MultiAgent V2, ближайший осмысленный
шаг - сначала подтвердить новую область и только потом использовать исторические
разделы этой карточки как материал для повторной реализации. Без отдельного
согласия тесты и debug не запускать.
