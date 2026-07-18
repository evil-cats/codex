---
id: fork-multi-agent-v1-spawn-agent-guidance
status: active
created: 2026-07-05
updated: 2026-07-18
source_scope: discussion-2026-07-05..discussion-2026-07-10
---

# MultiAgent V1: tool-owned `spawn_agent` guidance

## Обзор

Эта карточка фиксирует fork-доработку Hermione для V1 `spawn_agent` tool
description. Текущий контракт больше не опирается на профильную
`session-policy`: правила делегирования подагентов были убраны из policy как
неудачная модель, поэтому V1 tool description должен быть самодостаточным и не
ссылаться на отсутствующий policy-owner.

Цель доработки: сохранить полезную возможность запускать sub-agents для
конкретных bounded subtasks, но не возвращать старый запрет "только если
пользователь явно попросил sub-agents" и не поощрять схему, где родитель после
делегирования параллельно выполняет ту же содержательную работу локально.

| Поле | Значение |
| --- | --- |
| Статус | `active`; V1 tool-owned guidance согласован, policy-split удален как legacy |
| Целевой tool | V1 `spawn_agent` |
| Owner-файл prompt | `codex-rs/core/src/tools/handlers/multi_agents_spec.rs` |
| Тесты | `codex-rs/core/src/tools/handlers/multi_agents_spec_tests.rs` |
| Видимая для модели поверхность | описание tool, раскрываемое напрямую или через `tool_search` |
| Главный контракт | V1 `spawn_agent` description сам содержит критерии useful delegation и механику уже выбранного concrete bounded subtask |

## Зачем это нужно

Старый V1 usage hint содержал запрет:

```text
Do not spawn sub-agents unless the user explicitly asks for sub-agents, delegation, or parallel agent work.
Requests for depth, thoroughness, research, investigation, or detailed codebase analysis do not count as permission to spawn.
{agent_role_usage_hint}
```

Этот запрет слишком сильно сужал полезность MultiAgent V1 в профиле Hermione и
конфликтовал с проверенным fork workflow, где parent-agent разбивает задачу на
карточки или bounded subtasks, запускает sub-agent, ждёт короткий результат и
анализирует его.

Следующая модель со split между `session-policy` и tool description тоже
оказалась legacy: делегирование подагентов из policy убрано, потому что эта
модель не работала достаточно предсказуемо. Поэтому теперь владелец правил V1
usage hint - сам `spawn_agent` tool description, а карточка не требует правок
профильных policy-файлов.

Дополнительная проблема старых формулировок: они учили родителя делать
"meaningful non-overlapping work" после запуска sub-agent. На практике это часто
превращалось в дублирование исследования или реализации, тратило токены и
засоряло контекст. Новый текст должен учить другой модели:
`choose useful delegation -> spawn bounded task -> wait -> close -> integrate`.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/core/src/tools/handlers/multi_agents_spec.rs` | Владеет V1 и V2 `spawn_agent` tool descriptions; здесь V1 default usage hint должен оставаться tool-owned и самодостаточным |
| `codex-rs/core/src/tools/handlers/multi_agents_spec_tests.rs` | Закрепляет model-visible текст V1 `spawn_agent`, отсутствие старого запрета, отсутствие legacy policy-ссылки и поведение `usage_hint_text` override |
| `docs/fork/multi-agent-v1-spawn-agent-guidance.md` | Handoff-карточка с текущим tool-owned контрактом, обоснованием, проверками и условиями переноса |

Намеренно не менять в этой доработке:

- `${HOME}/.codex/policies/session-policy.md` и другие профильные policy-файлы;
- runtime API V1 `spawn_agent`;
- V2 inter-agent протокол;
- config keys вроде `usage_hint_text`, `root_agent_usage_hint_text` и
  `subagent_usage_hint_text`;
- `usage_hint_enabled`, потому что в текущем checkout это deprecated
  compatibility field, значение которого runtime игнорирует.

## Итоговый контракт

1. V1 `spawn_agent` description сам содержит компактный tool-owned usage hint:
   когда делегирование полезно, когда его не применять и как оформить bounded
   subtask.
2. V1 `spawn_agent` больше не ссылается на `session delegation policy` или
   профильный policy-owner.
3. Старый запрет на spawn без явного запроса пользователя не возвращается.
4. `{agent_role_usage_hint}` не возвращается как authorization guard.
5. `usage_hint_text` override продолжает заменять дефолтный V1 usage hint.
6. V2 `spawn_agent` description, runtime API, config keys и
   `usage_hint_enabled` не меняются в этой доработке.
7. Родитель после delegation ждёт всех подагентов текущего раунда, закрывает их и
   не дублирует delegated work локально, пока подагенты работают.

## Текст для V1 `spawn_agent` tool description

```text
This spawn_agent tool creates a sub-agent for an already selected concrete,
bounded subtask that is useful to delegate. Consider delegation for non-trivial
work that can be split into independent research, implementation, or
verification tasks whose results will materially affect your next steps.
Do not spawn agents for trivial, vague, tightly coupled work, or just to create
parallel activity.

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
then continue substantive parent work. Do not duplicate delegated work locally
while agents are running, and do not repeatedly wait without using returned
information.
```

## Архитектурное решение

### Почему policy-split удалён

Предыдущая версия карточки считала, что критерии "когда рассматривать
delegation" и "когда не делегировать" должны жить в `session-policy`, потому что
tool description виден модели только после раскрытия tool metadata. Это решение
отменено: делегирование агентов из policy убрано, потому что такая модель не
работала нормально и оставляла несогласованность между policy, tool metadata и
реальным workflow.

Текущий контракт проще: V1 `spawn_agent` tool description является владельцем
своего usage hint. Он не пытается быть общим rulebook-ом делегирования, но
содержит минимальные критерии, без которых tool metadata подталкивает модель к
устаревшему или вредному поведению.

### Что остается в tool description

Tool description оставляет:

- criteria for useful delegation: non-trivial, independent research,
  implementation или verification tasks;
- negative criteria: trivial, vague, tightly coupled work и parallel activity
  ради самой параллельности;
- inherited model и правила `model` override;
- task message shape;
- read-only/coding output contract;
- disjoint write scopes;
- agent roles;
- wait/close/integrate hint;
- запрет дублировать delegated work локально, пока подагенты работают.

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
   [Текст для V1 `spawn_agent` tool description](#текст-для-v1-spawn_agent-tool-description)
   без редакторской переработки.
4. Сохранить базовый `tool_description`, `available_models_description`,
   inherited model guidance и логику `usage_hint_text` override.
5. Обновить тесты в
   `codex-rs/core/src/tools/handlers/multi_agents_spec_tests.rs`, чтобы они
   проверяли tool-owned V1 guidance, отсутствие legacy policy-ссылки, отсутствие
   старого explicit-user-request запрета и отсутствие sidecar-work workflow.
6. Если upstream успел изменить V2 description, использовать его только как
   дополнительный контекст; tool-owned V1 guidance выше остается owner-текстом
   этой карточки, пока пользователь не примет новое решение.

## Проверки

### Смысловое покрытие

| Контракт | Обязательность | Где покрывается |
| --- | --- | --- |
| V1 `spawn_agent` содержит самодостаточный tool-owned guidance для useful concrete bounded subtask | `required` | `spawn_agent_tool_v1_uses_tool_owned_delegation_guidance` |
| V1 `spawn_agent` больше не ссылается на `session delegation policy` | `required` | `spawn_agent_tool_v1_uses_tool_owned_delegation_guidance` |
| V1 `spawn_agent` содержит критерии useful delegation для non-trivial independent research/implementation/verification tasks | `required` | `spawn_agent_tool_v1_uses_tool_owned_delegation_guidance` |
| V1 `spawn_agent` запрещает trivial, vague, tightly coupled work и parallel activity ради самой параллельности | `required` | `spawn_agent_tool_v1_uses_tool_owned_delegation_guidance` |
| V1 `spawn_agent` больше не содержит старый запрет "Do not spawn sub-agents unless the user explicitly asks..." | `required` | `spawn_agent_tool_v1_uses_tool_owned_delegation_guidance` |
| V1 `spawn_agent` больше не содержит "Requests for depth, thoroughness..." | `required` | `spawn_agent_tool_v1_uses_tool_owned_delegation_guidance` |
| `{agent_role_usage_hint}` не возвращается как отдельный authorization guard | `required` | `spawn_agent_tool_v1_uses_tool_owned_delegation_guidance` |
| V1 `spawn_agent` не возвращает `### Delegation workflow` и sidecar-work guidance | `required` | `spawn_agent_tool_v1_uses_tool_owned_delegation_guidance` |
| V1 `spawn_agent` сохраняет task message shape, read-only findings, coding worker subtasks, direct fork workspace edits, disjoint write scopes, agent roles и wait/close hint | `required` | `spawn_agent_tool_v1_uses_tool_owned_delegation_guidance` |
| V1 сохраняет legacy `fork_context`, а V2 отдельно использует `fork_turns`, канонические вложенные имена задач и разрешает подагентам запускать собственных подагентов | `required` | `spawn_agent_tool_v1_keeps_legacy_fork_context_field`, `spawn_agent_tool_v2_requires_task_name_and_lists_visible_models` |
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
      "argv": ["just", "test", "-p", "codex-core", "spawn_agent_tool_"]
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
- Build gate нужен в общем проверочном проходе миграции перед установкой или
  релизным handoff.

### Исторические результаты

При переносе на `rust-v0.143.0` подагент разрешил конфликт в V1
`spawn_agent` description, сохранив короткий tool-specific guidance и не вернув
upstream-блок с explicit-request guard и sidecar-work workflow. Проверки,
форматирование, генераторы и сборка в том card-pass не запускались; общий
проверочный проход оставался за parent-agent.

При переносе на `rust-v0.144.1` обнаружено, что предыдущая версия карточки
ошибочно требовала восстановить policy-split. По решению пользователя
`session-policy` не правится; legacy policy text удален из карточки, а V1
`spawn_agent` prompt и тест синхронизированы с tool-owned guidance.

При переносе на `rust-v0.144.5` V1 `spawn_agent` description, schema и тесты
уже соответствовали tool-owned контракту карточки, поэтому кодовая правка не
потребовалась. Поверхность V2, профильные policy-файлы, код и тесты не менялись.
Команды уровня карточки и проекта в one-card проходе не запускались; они
остаются для общего проверочного прохода родительского агента.

При переносе на `rust-v0.144.6` изменения в upstream между
`rust-v0.144.5..rust-v0.144.6` не затронули owner-файлы этой карточки.
После слияния V1 description сохранил согласованный tool-owned guidance, а
schema V1 по-прежнему использует legacy-поле `fork_context`: `true` наследует
текущую историю thread, `false` или отсутствие поля передаёт только начальный
prompt. Граница с V2 не размыта: только V2 использует обязательный `task_name`,
поле `fork_turns` со значениями `none`, `all` или положительным числом последних
ходов, канонические вложенные имена задач и явный model-visible текст о
возможности подагента запускать собственных подагентов. Тесты усилены точными
проверками model-visible описаний `fork_context` и `fork_turns`, а также полной
V2-инструкции о вложенности. Фильтр тестов карточки расширен с
`spawn_agent_tool_v1` до `spawn_agent_tool_`, чтобы общий проход проверял эту
V1/V2 границу. Runtime-код
не менялся; команды уровня карточки и проекта в one-card проходе не запускались
и остаются для общего проверочного прохода родительского агента.

| Проверка | Результат | Существенное подтверждение |
| --- | --- | --- |
| `fork format --fix` | `ok` | Форматирование прошло через skill-owned wrapper |
| `fork format --check` | `ok` | Финальная проверка форматирования прошла без изменений |
| `fork cards validate` | `ok` | Проверено 19 карточек, ошибок формы нет |
| `fork tests --mode list --card docs/fork/multi-agent-v1-spawn-agent-guidance.md` | `ok` | Исполняемая карта печатает одну проверку `multi agent v1 spawn agent guidance` |
| `fork tests --mode list` | `ok` | Общая исполняемая карта содержит строку `fork-multi-agent-v1-spawn-agent-guidance` |
| `fork tests --mode cards --card docs/fork/multi-agent-v1-spawn-agent-guidance.md` | `ok` | Проверка уровня карточки прошла после prompt-правки |
| `fork build-fast` | `ok` | После prompt-правки `release-fast` сборка прошла; wrapper проверил metadata и версию бинарника |
| `fork install` | `ok` | После prompt-правки установлен `${HOME}/.local/bin/codex-hermione`; wrapper проверил source, temporary и installed binary |
| `${HOME}/.local/bin/codex-hermione --version` | `ok` | Установленный бинарник ответил `codex-cli 0.142.5+hermione`; неблокирующий warning про PATH aliases не повлиял на результат |
| `markdownlint-cli2 --config docs/.markdownlint-cli2.yaml docs/fork/multi-agent-v1-spawn-agent-guidance.md` | `ok` | Локальная Markdown-конфигурация проекта не нашла ошибок |
| `git diff --check` | `ok` | Whitespace-проверка diff прошла |

### Известные падения и пропуски

- Поведенческий smoke test с реальным V1 `tool_search -> spawn_agent` после
  установки нового binary ещё не выполнялся.
- Первый повтор card-level test после старой split-правки упал только из-за
  line-wrap-sensitive assert в тесте для строки про agent roles; assert был
  исправлен, повторный запуск прошёл успешно.
- Исторические проверки profile policy больше не относятся к этой карточке,
  потому что policy-owner удален из текущего контракта.

## Runtime, сборка и установка

По явной просьбе пользователя после ранней prompt/spec-правки выполнялись
skill-owned сборка и установка:

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
- Tool-owned guidance намеренно разрешительнее старого V1 запрета. Риск
  смягчается тем, что prompt требует concrete bounded subtasks, запрещает
  trivial/vague/tightly coupled delegation, запрещает parallel activity ради
  самой параллельности и требует ждать/закрывать текущий delegation round перед
  продолжением содержательной работы.
- Если будущий upstream снова изменит V1/V2 multi-agent prompts, эту карточку
  нужно сверять с актуальным `multi_agents_spec.rs`, но не переписывать
  согласованный prompt без отдельного решения пользователя.
- Конфиговый `usage_hint_text` остаётся резервной настройкой и по-прежнему
  заменяет дефолтный V1 prompt целиком.

## Проверка покрытия

| Согласованный пункт | Статус | Где покрыто |
| --- | --- | --- |
| Убрать старую фразу `Do not spawn sub-agents unless...` | `перенесено в карточку` | `Итоговый контракт`, `Проверки` |
| Убрать фразу `Requests for depth, thoroughness...` | `перенесено в карточку` | `Итоговый контракт`, `Проверки` |
| Убрать legacy-ссылку на `session delegation policy` | `перенесено в карточку` | `Итоговый контракт`, `Проверки` |
| Не править `${HOME}/.codex/policies/session-policy.md` | `перенесено в карточку` | `Карта файлов`, `Архитектурное решение` |
| Не возвращать `{agent_role_usage_hint}` | `перенесено в карточку` | `Итоговый контракт`, `Архитектурное решение`, `Проверки` |
| Использовать из V2 хорошую идею concrete/bounded subtasks | `перенесено в карточку` | `Архитектурное решение`, `Текст для V1 spawn_agent tool description` |
| Не переносить V2 "alongside useful local work" | `перенесено в карточку` | `Архитектурное решение`, `Итоговый контракт` |
| Parent после delegation ждёт всех подагентов текущего раунда, закрывает их и не делает delegated work сам | `перенесено в карточку` | `Текст для V1 spawn_agent tool description`, `Итоговый контракт` |
| Parent анализирует и интегрирует ответы до продолжения содержательной работы | `перенесено в карточку` | `Текст для V1 spawn_agent tool description`, `Итоговый контракт` |
| Кодовая реализация и тесты выполнены | `перенесено в карточку` | `Карта файлов`, `Проверки` |
| Проверка уровня карточки принадлежит `fork tests` | `перенесено в карточку` | `Владелец исполняемой карты`, блок `fork-tests.v1` |
