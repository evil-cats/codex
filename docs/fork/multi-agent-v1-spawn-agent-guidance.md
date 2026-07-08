---
id: fork-multi-agent-v1-spawn-agent-guidance
status: active
created: 2026-07-05
updated: 2026-07-08
source_scope: discussion-2026-07-05
---

# MultiAgent V1: session-policy split for `spawn_agent` guidance

## Обзор

Эта карточка фиксирует согласованный split для fork-доработки Hermione:
root-visible `session-policy` получает правила выбора и workflow делегирования,
а V1 `spawn_agent` tool description остается короткой инструкцией по механике
конкретного tool.

Цель доработки: вернуть родительскому агенту практическую возможность запускать
sub-agents без старого запрета "только если пользователь явно попросил", решить
bootstrap-проблему видимости правил до tool discovery и одновременно убрать
неэффективную схему, где родитель после делегирования продолжает делать ту же
работу локально и расходует в несколько раз больше токенов и контекста.

| Поле | Значение |
| --- | --- |
| Статус | `active`; split согласован, code/policy правки вносятся в текущем проходе |
| Root-visible owner | `${HOME}/.codex/policies/session-policy.md` |
| Целевой tool | V1 `spawn_agent` |
| Owner-файл prompt | `codex-rs/core/src/tools/handlers/multi_agents_spec.rs` |
| Тесты | `codex-rs/core/src/tools/handlers/multi_agents_spec_tests.rs` |
| Видимая для модели поверхность | `session-policy` в developer prompt; описание tool, раскрываемое напрямую или через `tool_search` |
| Главный контракт | `session-policy` решает, когда рассматривать delegation; `spawn_agent` description объясняет, как оформить уже выбранный concrete bounded subtask |

## Зачем это нужно

Текущий V1 usage hint содержит запрет:

```text
Do not spawn sub-agents unless the user explicitly asks for sub-agents, delegation, or parallel agent work.
Requests for depth, thoroughness, research, investigation, or detailed codebase analysis do not count as permission to spawn.
{agent_role_usage_hint}
```

Этот запрет слишком сильно сужает полезность MultiAgent V1 в профиле Hermione и
конфликтует с проверенным fork workflow, где parent-agent разбивает задачу на
карточки или bounded subtasks, запускает sub-agent, ждёт короткий результат и
анализирует его.

Дополнительная проблема текущего блока: он учит родителя делать
"meaningful non-overlapping work" после запуска sub-agent. На практике это часто
превращается в дублирование исследования или реализации, тратит токены и
засоряет контекст. Новый split должен учить другой модели работы:
`consider delegation -> decompose -> spawn batch -> wait -> integrate`.

Главная bootstrap-проблема: правила из `spawn_agent` tool description не видны
модели до обращения к tool metadata. Поэтому критерии "когда рассматривать
delegation" и "когда не делегировать" должны жить в `session-policy`, а не в
описании `spawn_agent`.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `${HOME}/.codex/policies/session-policy.md` | Владеет root-visible правилами выбора, декомпозиции, ожидания и интеграции подагентов |
| `codex-rs/core/src/tools/handlers/multi_agents_spec.rs` | Владеет V1 и V2 `spawn_agent` tool descriptions; здесь V1 usage hint должен остаться tool-specific |
| `codex-rs/core/src/tools/handlers/multi_agents_spec_tests.rs` | Закрепляет короткий model-visible текст V1 `spawn_agent`, отсутствие старого запрета и поведение `usage_hint_text` override |
| `docs/fork/multi-agent-v1-spawn-agent-guidance.md` | Handoff-карточка с согласованным split, обоснованием, проверками и условиями переноса |

Намеренно не менять в этой доработке:

- runtime API V1 `spawn_agent`;
- V2 inter-agent протокол;
- config keys вроде `usage_hint_text`, `root_agent_usage_hint_text` и
  `subagent_usage_hint_text`;
- `usage_hint_enabled`, потому что в текущем checkout это deprecated
  compatibility field, значение которого runtime игнорирует.

## Итоговый контракт

Правка разделяет прежний длинный V1 usage hint на два уровня:

1. `${HOME}/.codex/policies/session-policy.md` получает root-visible правила
   выбора, декомпозиции, ожидания и интеграции подагентов.
2. `spawn_agent_tool_description(...)` в
   `codex-rs/core/src/tools/handlers/multi_agents_spec.rs` получает короткий V1
   usage hint только про механику уже выбранного `spawn_agent` вызова.
3. Старый запрет на spawn без явного запроса пользователя не возвращается.
4. `{agent_role_usage_hint}` не возвращается как authorization guard.
5. `usage_hint_text` override продолжает заменять дефолтный V1 usage hint.
6. V2 `spawn_agent` description, runtime API, config keys и
   `usage_hint_enabled` не меняются в этой доработке.

## Согласованный split

### Текст для `session-policy`

```markdown
## Делегирование задач подагентам

### Назначение делегирования
- Используй подагентов для ограниченных частей сложной работы, когда делегирование
  улучшает качество, скорость, покрытие проверки или дисциплину контекста.
- Родительский агент остаётся ответственным за решение, что делегировать,
  ожидание нужных результатов, проверку decisive evidence и интеграцию итогового
  ответа.
- Делегирование не требует отдельной явной просьбы пользователя, если текущая
  сессия предоставляет инструменты подагентов и декомпозиция реально продвигает
  задачу пользователя.

### Когда рассматривать делегирование
- Рассматривай делегирование для нетривиальных задач, которые можно разложить на
  независимые, ограниченные и проверяемые подзадачи.
- Хорошие кандидаты: независимые исследования разных частей кодовой базы,
  disjoint implementation slices, verification passes, миграционные или
  workflow-единицы, review passes, сравнение альтернатив или сбор evidence из
  разных областей.
- Подзадачи должны быть concrete bounded subtasks, результаты которых родитель
  сможет объединить в общий ответ или общий diff.
- Делегирование должно materially advance основную задачу, а не просто создавать
  параллельную активность.
- Если результат подагента не нужен для дальнейших решений родителя, такого
  подагента не запускай.

### Когда не делегировать
- Не делегируй тривиальную, одношаговую, неясную, tightly coupled работу или
  задачи, требующие непрерывного локального judgement.
- Не запускай подагентов только потому, что пользователь попросил глубину,
  тщательность, исследование, расследование или подробный анализ.
- Не делегируй, если разделение работы будет стоить дороже, чем локальное
  выполнение.
- Не создавай пересекающиеся назначения или нескольких подагентов на одну и ту
  же работу.

### Перед запуском
- Сформируй короткий план декомпозиции.
- Определи каждую подзадачу, почему ей должен владеть подагент и какой результат
  нужен родителю.
- По возможности запускай все полезные независимые подзадачи в одном раунде
  делегирования.
- Давай каждому подагенту concrete, self-contained task: objective, scope,
  relevant files или modules, constraints, expected output и режим edit/report.
- Для coding subtasks держи write scopes disjoint.

### После запуска подагентов
- Дождись выполнения всех подзадач, которые были переданы подагентам в текущем
  раунде делегирования.
- Жди результаты, потому что ответы подагентов могут существенно изменить
  дальнейший план работы родителя.
- Не выполняй дальнейшую содержательную работу родителем, пока все подагенты
  текущего раунда не завершили задачи и не закрыты.
- Не выполняй delegated investigation, implementation или verification локально,
  пока назначенные им подагенты работают.
- Во время ожидания занимайся только координацией и unblockers, если это
  необходимо для завершения уже запущенных подзадач.

### После завершения подагентов
- Сначала дождись final answers от всех подагентов текущего раунда.
- Закрой подагентов текущего раунда, когда их результаты получены и они больше
  не нужны.
- Затем сравни final answers с исходным планом декомпозиции.
- Проверяй только decisive evidence, changed files, контракты и риски, нужные для
  доверенной интеграции результата.
- Не повторяй полный delegated audit, кроме случаев, когда результат неполный,
  противоречивый или high-risk.
- Разрешай конфликты между ответами подагентов явно.
- Если нужно, попроси focused follow-up, запусти более узкую replacement subtask
  или заверши остаток работы локально.
- Продолжай содержательную работу родителем только после того, как все
  подагенты текущего раунда завершены и закрыты, а их результаты
  проанализированы и интегрированы.
- Сообщай пользователю, что было делегировано, что вернулось, что родитель
  проверил и какие риски или ограничения остались.
```

### Текст для V1 `spawn_agent` tool description

```text
This spawn_agent tool creates a sub-agent for an already selected concrete,
bounded subtask. The session delegation policy owns when to consider delegation,
when not to delegate, how to avoid duplicate parent/sub-agent work, and how to
integrate results.

Spawned agents inherit your current model by default. Do not set the `model`
field unless the task clearly needs a different model, a configured agent role
requires it, or a higher-priority instruction explicitly asks for it.

Give each sub-agent a self-contained task message: objective, scope, relevant
files or modules, constraints, expected output, and whether it should edit files
or only report findings.

Avoid overlapping assignments. If multiple sub-agents edit code, give them
disjoint write scopes.

For read-only tasks, ask for concise findings with file paths, line references,
evidence, and open questions.

For coding tasks, prefer concrete code-change worker subtasks when the write
scope is clear and bounded. Instruct the sub-agent to edit files directly in its
forked workspace and list the file paths it changed in the final answer.

If agent roles are available, use them to choose the best owner for an already
selected subtask.

After spawning agents for delegated work, wait for all agents in the current
delegation round to complete, close them when they are no longer needed, and only
then continue substantive parent work. Do not repeatedly wait without using
returned information.
```

## Архитектурное решение

### Почему split нужен

`spawn_agent` tool description виден модели только после раскрытия tool metadata
или обращения к tool. Поэтому он не должен владеть решением "рассматривать ли
delegation вообще". Это решение должно жить в `session-policy`, который попадает
в root-visible developer prompt.

### Что остается в tool description

Tool description оставляет только механику конкретного вызова: inherited model,
`model` override, task message shape, read-only/coding output contract, disjoint
write scopes, agent roles и минимальную связку с `wait_agent`.

Если результат подагента не нужен для дальнейшей работы родителя, запускать
такого подагента не следует. Если результат нужен, родитель ждёт завершения
текущего раунда подагентов до продолжения содержательной работы, потому что
результаты могут изменить дальнейший план.

### Что не возвращаем из старого V1/V2

Не возвращать старый explicit-user-request запрет:

```text
Do not spawn sub-agents unless the user explicitly asks for sub-agents, delegation, or parallel agent work.
Requests for depth, thoroughness, research, investigation, or detailed codebase analysis do not count as permission to spawn.
```

Не переносить из V2 хвост:

```text
that can run independently alongside useful local work
```

Причина: такая формулировка снова подталкивает parent-agent к параллельному
локальному выполнению похожей содержательной работы. Для Hermione fork целевое
поведение другое: если работа передана sub-agent, родитель ждёт результат и
анализирует ответ, а не повторяет ту же задачу.

## Порядок повторения при переносе

При переносе этой fork-доработки на новый upstream checkout:

1. Найти `spawn_agent_tool_description(...)` в
   `codex-rs/core/src/tools/handlers/multi_agents_spec.rs`.
2. Найти ветку дефолтного V1 usage hint, которая добавляется, когда
   `usage_hint_text` не задан, через вызов `create_spawn_agent_tool_v1(...)`.
3. Заменить дефолтный usage hint на блок из раздела
   [Согласованный split](#согласованный-split) без редакторской переработки.
4. Сохранить базовый `tool_description`, `available_models_description`,
   inherited model guidance и логику `usage_hint_text` override.
5. Обновить тесты в
   `codex-rs/core/src/tools/handlers/multi_agents_spec_tests.rs`, чтобы они
   проверяли короткий V1 tool-specific guidance, отсутствие старого
   explicit-user-request запрета и отсутствие перенесённого в `session-policy`
   workflow-блока в tool description.
6. Если upstream успел изменить V2 description, использовать его только как
   дополнительный контекст; согласованный split выше остается owner-текстом этой
   карточки, пока пользователь не примет новое решение.

## Проверки

### Смысловое покрытие

| Контракт | Обязательность | Где покрывается |
| --- | --- | --- |
| `session-policy` содержит root-visible правила: когда рассматривать delegation, когда не делегировать, как декомпозировать, ждать всех подагентов, закрывать их, анализировать и интегрировать результаты | `required` | профильная проверка `check-hermione-profile.py`; редакторская проверка policy |
| V1 `spawn_agent` содержит короткий tool-specific guidance для already selected concrete bounded subtask | `required` | `spawn_agent_tool_v1_uses_session_policy_scoped_guidance` |
| V1 `spawn_agent` больше не содержит старый запрет "Do not spawn sub-agents unless the user explicitly asks..." | `required` | `spawn_agent_tool_v1_uses_session_policy_scoped_guidance` |
| V1 `spawn_agent` больше не содержит "Requests for depth, thoroughness..." | `required` | `spawn_agent_tool_v1_uses_session_policy_scoped_guidance` |
| `{agent_role_usage_hint}` не возвращается как отдельный authorization guard | `required` | `spawn_agent_tool_v1_uses_session_policy_scoped_guidance` |
| V1 `spawn_agent` больше не дублирует `session-policy` workflow вроде `### Delegation workflow` | `required` | `spawn_agent_tool_v1_uses_session_policy_scoped_guidance` |
| V1 `spawn_agent` сохраняет task message shape, read-only findings, coding worker subtasks, direct fork workspace edits, disjoint write scopes, agent roles и wait/close hint | `required` | `spawn_agent_tool_v1_uses_session_policy_scoped_guidance` |
| `usage_hint_text` override продолжает заменять дефолтный usage hint | `required` | `spawn_agent_tool_v1_usage_hint_text_replaces_default_guidance` |

### Владелец исполняемой карты

Проверки уровня карточки запускает skill-owned command `fork tests`.
Внутренние argv и назначение точечной проверки живут только в блоке
`fork-tests.v1` ниже; они являются данными для `fork tests`, а не пользовательским
runbook прямого запуска.

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "multi agent v1 spawn agent guidance",
      "argv": ["just", "test", "-p", "codex-core", "spawn_agent_tool_v1"]
    }
  ]
}
```

### Дополнительные gates

- Форму карточки проверяет skill-owned валидатор карточек.
- Исполняемая карта проверки уровня карточки хранится в блоке `fork-tests.v1` и
  принадлежит владельцу `fork tests`.
- После кодовой правки применяются обычные skill-owned format gates для
  Rust-файлов.
- Build gate нужен только в общем проверочном проходе перед установкой или
  релизным handoff. В текущем проходе он выполнен по явной просьбе установить
  новый fork-бинарник.

### Исторические результаты

На момент создания карточки код, тесты и сборка не запускались. В текущем
проходе после кодовой правки выполнено:

При переносе на `rust-v0.143.0` подагент разрешил конфликт в V1
`spawn_agent` description, сохранив согласованный короткий tool-specific
guidance и не вернув upstream-блок с explicit-request guard и sidecar-work
workflow. Проверки, форматирование, генераторы и сборка в этом card-pass не
запускались; общий проверочный проход остаётся за parent-agent.

| Проверка | Результат | Существенное подтверждение |
| --- | --- | --- |
| `fork format --fix` | `ok` | Форматирование прошло через skill-owned wrapper |
| `fork format --check` | `ok` | Финальная проверка форматирования прошла без изменений |
| `fork cards validate` | `ok` | Проверено 19 карточек, ошибок формы нет |
| `fork tests --mode list --card docs/fork/multi-agent-v1-spawn-agent-guidance.md` | `ok` | Исполняемая карта печатает одну проверку `multi agent v1 spawn agent guidance` |
| `fork tests --mode list` | `ok` | Общая исполняемая карта содержит новую строку `fork-multi-agent-v1-spawn-agent-guidance` |
| `fork tests --mode cards --card docs/fork/multi-agent-v1-spawn-agent-guidance.md` | `ok` | Проверка уровня карточки прошла после split; внутренний target `spawn_agent_tool_v1` зеленый |
| `python3 -B ${HOME}/.codex/scripts/check-hermione-profile.py` | `ok` | `session-policy.md` виден в `role: developer` |
| `markdownlint-cli2 --config ${HOME}/.codex/.markdownlint-cli2.yaml ${HOME}/.codex/policies/session-policy.md` | `ok` | Профильная policy проходит fallback Markdown-конфигурацию |
| `fork build-fast` | `ok` | После split-правки `release-fast` сборка прошла; wrapper проверил metadata и версию бинарника |
| `fork install` | `ok` | После split-правки установлен `${HOME}/.local/bin/codex-hermione`; wrapper проверил source, temporary и installed binary |
| `${HOME}/.local/bin/codex-hermione --version` | `ok` | Установленный бинарник ответил `codex-cli 0.142.5+hermione`; неблокирующий warning про PATH aliases не повлиял на результат |
| `markdownlint-cli2 --config docs/.markdownlint-cli2.yaml docs/fork/multi-agent-v1-spawn-agent-guidance.md` | `ok` | Локальная Markdown-конфигурация проекта не нашла ошибок |
| `git diff --check` | `ok` | Whitespace-проверка diff прошла |

### Известные падения и пропуски

- Поведенческий smoke test с реальным V1 `tool_search -> spawn_agent` после
  установки нового binary ещё не выполнялся.
- Первый повтор card-level test после split упал только из-за line-wrap-sensitive
  assert в тесте для строки про agent roles; assert исправлен, повторный запуск
  прошёл успешно.

## Runtime, сборка и установка

По явной просьбе пользователя после prompt/spec-правки, а затем повторно после
финального split-а выполнены skill-owned сборка и установка:

- `release-fast` бинарник собран через `fork build-fast`; wrapper подтвердил
  metadata и версию.
- Бинарник установлен через `fork install` в `${HOME}/.local/bin/codex-hermione`.
- Установочный wrapper подтвердил metadata и версию source, temporary и
  installed binary.
- Прямая проверка `${HOME}/.local/bin/codex-hermione --version` вернула
  `codex-cli 0.142.5+hermione`. Команда напечатала неблокирующий warning про
  PATH aliases на read-only filesystem и завершилась успешно.
- Отдельный интерактивный smoke test поведения `tool_search -> spawn_agent` на
  новом установленном binary не выполнялся.

## Риски и ограничения

- Tool description является model-visible context. Даже небольшая правка меняет
  поведение агента и должна проверяться как prompt/tool spec change, а не как
  обычная документационная строка.
- Новый split намеренно разрешительнее старого V1 запрета. Риск смягчается тем,
  что `session-policy` запрещает trivial/vague delegation, требует bounded
  subtasks, заставляет родителя дождаться всех подагентов текущего раунда и
  продолжать содержательную работу только после анализа и интеграции результатов.
- Если будущий upstream снова изменит V1/V2 multi-agent prompts, эту карточку
  нужно сверять с актуальным `multi_agents_spec.rs`, но не переписывать
  согласованный prompt без отдельного решения пользователя.
- Конфиговый `usage_hint_text` остаётся резервной настройкой и по-прежнему заменяет
  дефолтный V1 prompt целиком.

## Проверка покрытия

| Согласованный пункт | Статус | Где покрыто |
| --- | --- | --- |
| Убрать старую фразу `Do not spawn sub-agents unless...` | `перенесено в карточку` | `Итоговый контракт`, `Проверки` |
| Убрать фразу `Requests for depth, thoroughness...` | `перенесено в карточку` | `Итоговый контракт`, `Проверки` |
| Не возвращать `{agent_role_usage_hint}` | `перенесено в карточку` | `Итоговый контракт`, `Архитектурное решение`, `Проверки` |
| Использовать из V2 хорошую идею concrete/bounded subtasks | `перенесено в карточку` | `Архитектурное решение`, `Согласованный split` |
| Не переносить V2 "alongside useful local work" | `перенесено в карточку` | `Архитектурное решение`, `Итоговый контракт` |
| Split-текст для `session-policy` и V1 tool description переносится без переработки | `перенесено в карточку` | `Согласованный split` |
| Parent после delegation ждёт всех подагентов текущего раунда, закрывает их и не делает delegated work сам | `перенесено в карточку` | `Согласованный split`, `Итоговый контракт` |
| Parent анализирует, проверяет decisive evidence и интегрирует ответы до продолжения содержательной работы | `перенесено в карточку` | `Согласованный split`, `Итоговый контракт` |
| Если результат подагента не нужен для дальнейших решений родителя, такого подагента не запускать | `перенесено в карточку` | `Согласованный split`, `Архитектурное решение` |
| Проверенный fork skill workflow со схемой parent/subagent/wait является обоснованием | `перенесено в карточку` | `Зачем это нужно`, `Архитектурное решение` |
| Кодовая реализация и тесты выполнены | `перенесено в карточку` | `Карта файлов`, `Проверки` |
| Проверка уровня карточки принадлежит `fork tests` | `перенесено в карточку` | `Владелец исполняемой карты`, блок `fork-tests.v1` |
