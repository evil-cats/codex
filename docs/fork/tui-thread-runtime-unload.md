---
id: fork-tui-thread-runtime-unload
status: active
created: 2026-07-09
updated: 2026-07-09
source_scope: discussion-2026-07-09-tui-mcp-runtime-leak
---

# Выгрузка live-runtime при переключении TUI thread

## Обзор

Эта карточка фиксирует fork-доработку Hermione для явной выгрузки live-runtime
при TUI-переходах между threads. Пользовательская цель: `/resume`, `/clear`,
новая сессия, `/fork` и явное закрытие side conversation не должны размножать
MCP stdio-процессы, если старый thread больше не является активным runtime в
TUI. При этом сохраненная сессия, rollout, metadata, архивность и возможность
будущего resume не должны удаляться или изменяться как побочный эффект.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Пользовательская цель | При переключении TUI закрывать старый live-runtime и его MCP, не удаляя persisted session |
| Основной симптом | Одна активная TUI-сессия может оставить несколько живых MCP-процессов после `/resume` или `/clear` |
| Предполагаемая причина | TUI-путь вызывает `thread/unsubscribe`, хотя ему нужна выгрузка или shutdown runtime |
| Реализованный API app-server | `thread/unload` для выгрузки live-runtime без delete/archive |
| Связанная карточка | `docs/fork/mcp-rollout-diagnostics.md` |
| Не входит в границы задачи | Удаление истории, семантика archive/delete, автоматическое закрытие работающих subagents при обычной `/agent` навигации |

Связь с `docs/fork/mcp-rollout-diagnostics.md` ограничена lifecycle-уровнем:
недавняя MCP recovery-доработка усиливает shutdown/recovery path, но текущий
TUI-баг находится выше: старый thread может вообще не попасть в явный shutdown.

## Зачем это нужно

Текущее поведение выглядит странно для пользователя:

```text
/resume или /clear
  -> новый thread поднимает новый MCP runtime
  -> старый thread только отписывается от TUI events
  -> старый live-runtime остается loaded в app-server
  -> старый MCP stdio-процесс может жить до idle unload или shutdown app-server
```

Для пользователя `/resume` на другой thread и `/clear` означают смену активного
рабочего контекста, а не удержание старого runtime в памяти. История должна
оставаться доступной, но live handles старого thread должны освобождаться сразу.

Ожидаемая цепочка после исправления:

```text
TUI переключается на другой primary thread
  -> persisted session старого thread остается на диске
  -> app-server выгружает старый live-runtime
  -> CodexThread::shutdown_and_wait()
  -> McpConnectionManager::shutdown()
  -> RmcpClient::shutdown()
  -> stdio MCP process terminate
  -> новый active thread работает с единственным своим MCP runtime
```

Важно различать:

- `thread/unsubscribe` - публичная операция "клиент отписался от событий"; она
  не должна внезапно стать destructive runtime close для всех клиентов app-server.
- `thread/unload` - новая явная операция "выгрузить live-runtime из app-server
  memory, но сохранить persisted session".
- `close_agent` core tool - уже идет через `AgentControl::close_agent` и
  `shutdown_agent_tree`, поэтому не является главным источником этой утечки.

## Карта файлов

| Файл | Роль в реализации |
| --- | --- |
| `codex-rs/app-server-protocol/src/protocol/common.rs` | Добавлен v2 JSON-RPC метод `thread/unload` с thread serialization scope |
| `codex-rs/app-server-protocol/src/protocol/v2/thread.rs` | Добавлены `ThreadUnloadParams`, `ThreadUnloadResponse` и `ThreadUnloadStatus` |
| `codex-rs/app-server-protocol/schema/` | Сгенерированы JSON Schema и TypeScript artifacts для `thread/unload` |
| `codex-rs/app-server/README.md` | Документирован публичный non-destructive lifecycle метод `thread/unload` |
| `codex-rs/app-server/src/request_processors/thread_processor.rs` | Реализован обработчик unload через существующий teardown/shutdown path без archive/delete |
| `codex-rs/app-server/src/request_processors/thread_lifecycle.rs` | Существующий `wait_for_thread_shutdown` переиспользован из `thread_processor.rs`; прямых правок не потребовалось |
| `codex-rs/tui/src/app_server_session.rs` | Добавлена TUI-обертка для `thread/unload` |
| `codex-rs/tui/src/app/thread_routing.rs` | Разделены helpers для event subscription cleanup и runtime unload вместо обманчивого `shutdown_current_thread` |
| `codex-rs/tui/src/app/session_lifecycle.rs` | `/resume`, `/clear`, новая сессия и cleanup устаревшего startup-thread переведены на правильный lifecycle |
| `codex-rs/tui/src/app/event_dispatch.rs` | `/fork` и shutdown-first exit переведены на runtime unload там, где live-runtime больше не нужен |
| `codex-rs/tui/src/app/side.rs` | Side conversation close переведен на runtime unload, а не только interrupt plus unsubscribe |
| `codex-rs/core/src/agent/control/legacy.rs` | Не менять без новой причины; `close_agent` уже является настоящим shutdown path |
| `codex-rs/app-server/tests/suite/v2/thread_unload.rs` | Добавлено регрессионное покрытие unload без удаления persisted session |
| `codex-rs/tui/src/app/tests.rs` | Добавлено TUI-регрессионное покрытие для `/resume`, `/clear`, `/fork` и выгрузки runtime при side close |
| `docs/fork/tui-thread-runtime-unload.md` | Владеющий handoff-артефакт этой fork-доработки |

Намеренно не менять в рамках этой карточки:

| Зона | Почему не менять |
| --- | --- |
| Семантика `thread/unsubscribe` | Существующие app-server tests закрепляют, что unsubscribe сохраняет loaded thread до idle unload |
| `thread/archive` и `thread/delete` | Эти операции меняют persisted state; пользовательская цель требует не удалять и не архивировать историю |
| Core tool `close_agent` | Аудит исходного кода показывает, что он вызывает `AgentControl::close_agent` и `shutdown_agent_tree` |
| Обычная `/agent` навигация между live subagents | Простое переключение view не должно автоматически убивать работающий agent |
| Логика MCP recovery/retry | Проблема находится в TUI/app-server lifecycle до попадания в MCP shutdown |

## Итоговый контракт

### Новый runtime unload

Добавить или переиспользовать app-server v2 метод с семантикой:

```text
thread/unload({ threadId })
  -> если thread loaded: отправить shutdown, дождаться завершения в bounded path,
     снять pending requests/subscriptions/listener state, удалить из loaded map
  -> если thread already not loaded: вернуть status: "notLoaded"
  -> не вызывать archive/delete
  -> не менять rollout/history/thread metadata как пользовательское действие
```

Рабочее имя `thread/unload` выбрано потому, что оно точнее, чем `thread/close`:
закрывается live runtime в памяти, а не сохраненная сессия.

Итоговая форма ответа:

```text
ThreadUnloadResponse {
  status: "unloaded" | "notLoaded"
}
```

`status: "unloaded"` возвращается только после успешного
`CodexThread::shutdown_and_wait()` и удаления thread из loaded map. Ошибки
submit shutdown и timeout возвращаются как request error, чтобы TUI не удалял
локальное state после неуспешной server-side выгрузки.

### TUI primary transitions

TUI должен использовать runtime unload, когда старый primary thread больше не
должен оставаться live:

| Сценарий | Ожидаемое поведение |
| --- | --- |
| `/resume` на другой thread | Старый primary runtime выгружается, новый resumed runtime становится active |
| `/clear` | Старый primary runtime выгружается, новый clean thread стартует; старая история остается resumable |
| Новая сессия / `NewSession` | Старый primary runtime выгружается перед или во время перехода на новый thread |
| `/fork` с переходом на forked thread | Старый primary runtime выгружается после успешного attach forked runtime |
| Shutdown-first exit | TUI просит app-server закрыть current runtime, а не только отписаться от событий |

Если новый thread не стартовал, resume/fork не удался или TUI не смог
прикрепиться к новому thread, старый runtime не выгружается раньше времени. В
реализации `/resume`, `/clear`, новая сессия и `/fork` сначала получают и
прикрепляют новый runtime, а затем выгружают старые tracked runtimes, исключая
новый thread id.

### TUI side conversation

TUI side conversation close/discard должен выгружать side runtime:

```text
discard_side_thread
  -> interrupt active/startup work, если он нужен
  -> thread/unload side thread runtime
  -> удалить локальное side UI state только после успешной server-side выгрузки
```

Если unload завершился ошибкой, side conversation остается видимой или
восстанавливается в UI, как текущий код уже делает при ошибке cleanup.

### Что остается только unsubscribe

`thread/unsubscribe` остается публичной операцией отписки. Она подходит для
клиентов, которые закрывают subscription, но хотят оставить thread loaded для
быстрого reattach или idle unload policy. TUI может использовать plain
unsubscribe только там, где runtime действительно должен продолжить жить.

### Agent navigation

`/agent` selection является переключением просмотра между known threads. Она не
должна автоматически unload'ить предыдущий live agent, если этот agent может
продолжать работу. Отдельное UI-действие "close/discard side conversation" уже
является закрытием runtime и должно идти через unload.

## Архитектурное решение

### Почему не менять `thread/unsubscribe`

Изменение `thread/unsubscribe` на immediate shutdown сломало бы app-server
контракт для внешних клиентов и существующие tests. Сейчас unsubscribe сохраняет
thread loaded до idle unload, что может быть полезно для reattach. Баг не в
самом unsubscribe, а в том, что TUI использует его в местах, где нужна выгрузка
runtime.

### Почему не delete/archive

Пользовательская цель прямо запрещает удалять сохраненную сессию. `/resume`,
`/clear` и side close должны освобождать ресурсы runtime, но не менять
историю, rollout, archived state или thread metadata. После unload прежний
thread должен снова открываться через resume.

### Почему `close_agent` не главный источник

Core tool `close_agent` вызывает `AgentControl::close_agent`, который помечает
spawn edge closed для non-ephemeral agents и затем вызывает shutdown дерева
agents. Для live agents это отправляет `Op::Shutdown`, ждет termination и
удаляет thread из manager. Поэтому утечка MCP из обсуждаемого симптома скорее
идет из TUI paths, где "закрытие" реализовано как `thread/unsubscribe`.

### Почему `/agent` navigation исключена из автоматической выгрузки

Переключение просмотра между main thread и subagent не равно закрытию subagent.
Если agent продолжает работу или ожидает approval, автоматическая выгрузка при
каждом переключении view нарушит multi-agent workflow. Закрывать runtime нужно
только для явных lifecycle transitions и close/discard actions.

## Порядок повторения при переносе

1. Проверить upstream app-server v2 API: если уже есть runtime unload/close
   метод с нужной non-destructive семантикой, использовать его вместо нового
   `thread/unload`.
2. Добавить protocol types и mapping метода для `thread/unload`, если такого API
   нет.
3. Реализовать app-server handler через существующий `wait_for_thread_shutdown` и
   teardown helpers, не вызывая archive/delete store mutations.
4. Добавить TUI-обертку в `AppServerSession`.
5. Разделить TUI helpers: plain unsubscribe для event subscription cleanup и
   runtime unload для переходов active primary thread.
6. Перевести `/resume`, `/clear`, новую сессию, `/fork`, shutdown-first exit и
   TUI side close на runtime unload, сохраняя UX восстановления после ошибки.
7. Не менять автоматическое `/agent` view switching без отдельного продуктового
   решения.
8. Добавить app-server и TUI regression tests.
9. Обновить schema/TS artifacts через skill-owned generators, если добавлен
   новый app-server API.
10. Синхронизировать `fork-tests.v1` с фактическими test names после реализации.

## Проверки

### Смысловое покрытие

| Контракт | Обязательность | Где должно покрываться |
| --- | --- | --- |
| `thread/unload` закрывает loaded runtime и вызывает thread shutdown | `required` | app-server v2 integration test |
| `thread/unload` не удаляет и не архивирует persisted session | `required` | app-server resume-after-unload test |
| `/resume` на другой thread выгружает предыдущий primary runtime | `required` | TUI app lifecycle test |
| `/clear` выгружает предыдущий primary runtime перед clean thread | `required` | TUI app lifecycle test |
| `/fork` после перехода на forked thread выгружает old primary runtime | `required` | TUI app lifecycle test |
| TUI side close/discard выгружает side runtime | `required` | TUI side conversation test |
| Plain `thread/unsubscribe` сохраняет прежний контракт | `required` | существующие app-server unsubscribe tests |
| Core `close_agent` не регрессирует | `required` | существующие agent control tests |
| MCP process старого runtime завершается через shutdown chain | `required` | runtime-аудит исходного кода или integration test с stdio MCP fixture |

### Владелец исполняемой карты

Проверки уровня карточки запускает skill-owned command `fork tests`. Карточка
хранит машиночитаемый блок `fork-tests.v1`; внутренние `argv` ниже являются
данными для `fork tests`, а не пользовательским runbook прямого запуска.

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "app-server runtime unload preserves persisted session",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-app-server",
        "thread_unload_shuts_down_loaded_runtime_without_deleting_session"
      ]
    },
    {
      "purpose": "tui resume выгружает предыдущий primary runtime",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-tui",
        "resume_target_session_unloads_previous_thread_runtime"
      ]
    },
    {
      "purpose": "tui clear выгружает предыдущий primary runtime",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-tui",
        "clear_ui_unloads_previous_thread_runtime"
      ]
    },
    {
      "purpose": "tui fork выгружает предыдущий primary runtime",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-tui",
        "fork_current_session_unloads_previous_thread_runtime"
      ]
    },
    {
      "purpose": "tui side close выгружает side runtime",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-tui",
        "discard_side_thread_unloads_side_runtime"
      ]
    }
  ]
}
```

### Дополнительные gates

| Gate | Когда нужен | Статус |
| --- | --- | --- |
| Проверка формы fork-карточек | После добавления этой карточки | `done` |
| Печать исполняемой карты card-level проверок | После добавления `fork-tests.v1` | `done` |
| Форматирование Rust-кода | После реализации Rust-правок | `done` |
| Генераторы schema/TS artifacts | Если добавлен `thread/unload` app-server API | `done` |
| Card-level tests | После реализации тестовых targets из `fork-tests.v1` | `done` |
| Быстрая fork-сборка | После реализации lifecycle path | `done` |
| Markdown lint карточки | После обновления документации карточки | `done` |
| Diff hygiene | Перед handoff | `done` |

### Исторические результаты

| Проверка или источник | Результат | Примечание |
| --- | --- | --- |
| Аудит исходного кода `resume_target_session` | `found` | Новый resumed thread создается через `app_server.resume_thread`, затем старый current thread проходит через `shutdown_current_thread` |
| Аудит исходного кода `start_fresh_session_with_summary_hint` | `found` | `/clear` и новая сессия используют тот же helper для старого current thread |
| Аудит исходного кода `shutdown_current_thread` | `found` | Helper делает `thread_unsubscribe` и abort listener, но не app-server runtime shutdown |
| Аудит исходного кода `thread/unsubscribe` | `found` | App-server unsubscribe снимает subscription и не вызывает `CodexThread::shutdown_and_wait` |
| Аудит исходного кода MCP shutdown path | `found` | `McpConnectionManager::shutdown` и `RmcpClient::shutdown` уже умеют terminate stdio process при явном shutdown |
| Аудит исходного кода `discard_side_thread` | `found` | TUI side close делает interrupt plus `thread_unsubscribe`, затем удаляет local UI state |
| Аудит исходного кода `close_agent` tool | `found` | Core `close_agent` идет через `AgentControl::close_agent` и `shutdown_agent_tree` |
| Реализация `thread/unload` | `done` | Добавлен v2 method, handler, README и schema/TS artifacts |
| Реализация TUI runtime unload | `done` | `/resume`, `/clear`, новая сессия, `/fork`, shutdown-first exit и side discard переведены на `thread/unload` |
| `.codex/skills/fork/scripts/fork generators` | `ok` | Config schema и app-server schema artifacts синхронизированы |
| `.codex/skills/fork/scripts/fork format --fix` | `ok` | Rust/doc formatting wrapper применен после правок |
| `.codex/skills/fork/scripts/fork format --check` | `ok` | Форматирование проверено после реализации |
| `.codex/skills/fork/scripts/fork tests --mode list --card docs/fork/tui-thread-runtime-unload.md` | `ok` | Исполняемая карта содержит пять card-level targets |
| `.codex/skills/fork/scripts/fork tests --mode cards --card docs/fork/tui-thread-runtime-unload.md` | `ok` | App-server unload test и четыре TUI lifecycle regression tests прошли |
| `.codex/skills/fork/scripts/fork cards validate` | `ok` | `cards_checked: 23`, `card_errors: 0` |
| `markdownlint-cli2 --config docs/.markdownlint-cli2.yaml docs/fork/tui-thread-runtime-unload.md` | `ok` | `0 error(s)` |
| `git diff --check` | `ok` | Whitespace/conflict-marker issues не найдены |
| `.codex/skills/fork/scripts/fork build-fast` | `ok` | Release-fast binary собран и прошел binary metadata/version checks |
| `.codex/skills/fork/scripts/fork install` | `ok` | Fork binary установлен в `${HOME}/.local/bin/codex-hermione` |

### Известные падения и пропуски

- Runtime smoke с реальным stdio MCP процессом для `/resume`, `/clear` и
  `/fork` еще не выполнялся; покрытие идет через shutdown chain и loaded-list
  regression tests.

## Runtime, сборка и установка

Реализация выполнена в локальном checkout. Быстрая сборка прошла и собрала
`codex-rs/target/release-fast/codex`. Новый fork-бинарник установлен через
skill-owned `fork install` в `${HOME}/.local/bin/codex-hermione`.

## Риски и ограничения

- Нельзя выгружать старый primary runtime до того, как новый thread успешно
  создан или восстановлен, иначе ошибка `/resume` может оставить пользователя
  без активного runtime.
- `thread/unload` должен быть bounded и не должен зависнуть навсегда на
  shutdown: существующий app-server path уже использует timeout для thread
  shutdown.
- Side conversation close должен сохранить текущий UX восстановления после
  ошибки: если server-side close завершился ошибкой, локальное UI state нельзя
  молча удалить.
- `/agent` view switching нельзя смешивать с lifecycle close. Для работающих
  subagents автоматический unload при навигации был бы behavioral regression.
- Если MCP server игнорирует terminate, нижний `RmcpClient::shutdown` и
  `StdioServerProcessHandle::terminate` должны оставаться владельцами
  platform-specific process cleanup.

## Проверка покрытия

| Смысловой пункт | Статус | Где покрыто |
| --- | --- | --- |
| Persisted session нельзя delete/archive при переключении TUI | `перенесено в карточку` | "Обзор", "Итоговый контракт" |
| `/resume` должен закрывать старый live-runtime и MCP | `перенесено в карточку` | "TUI primary transitions", `fork-tests.v1` |
| `/clear` и новая сессия имеют тот же lifecycle bug | `перенесено в карточку` | "Зачем это нужно", `fork-tests.v1` |
| `/fork` после перехода на forked thread входит в scope | `перенесено в карточку` | "TUI primary transitions" |
| TUI side close/discard тоже должен unload runtime | `перенесено в карточку` | "TUI side conversation", `fork-tests.v1` |
| `thread/unsubscribe` не менять как публичный контракт | `перенесено в карточку` | "Почему не менять `thread/unsubscribe`" |
| Core `close_agent` уже является настоящим shutdown path | `перенесено в карточку` | "Почему `close_agent` не главный источник" |
| `/agent` navigation не должна автоматически убивать работающих agents | `перенесено в карточку` | "Agent navigation" |
| MCP cleanup должен происходить через существующую shutdown chain | `перенесено в карточку` | "Зачем это нужно", "Карта файлов" |
| Нужны app-server/TUI regression tests | `перенесено в карточку` | "Проверки", `fork-tests.v1` |

## Открытые вопросы

- Нужно ли показывать пользователю warning, если unload старого runtime
  завершился timeout, но TUI уже перешел на новый thread?
- Достаточно ли аудита исходного кода для MCP process termination, или нужен
  отдельный runtime integration test с fixture MCP-процессом и подсчетом live
  launches?
