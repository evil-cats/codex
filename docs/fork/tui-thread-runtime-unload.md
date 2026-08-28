---
id: fork-tui-thread-runtime-unload
status: active
created: 2026-07-09
updated: 2026-08-26
---

# Выгрузка live-runtime при переключении TUI thread

## Обзор

Эта карточка фиксирует fork-доработку Hermione для явной выгрузки live-runtime
при TUI-переходах между threads. Пользовательская цель: `/resume`, `/clear`,
новая сессия, `/fork` и явное закрытие side conversation не должны размножать
MCP stdio-процессы, если старый thread больше не является активным runtime в
TUI. При этом сохраненная сессия, rollout, metadata, архивность и возможность
будущего resume не должны удаляться или изменяться как побочный эффект.

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

Цепочка выгрузки runtime:

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
| `codex-rs/tui/src/app/session_lifecycle.rs` | `/resume`, `/clear`, новая сессия и cleanup устаревшего startup-thread переведены на правильный lifecycle; строго `#[cfg(test)]` hook воспроизводит ошибку attach после создания replacement runtime |
| `codex-rs/tui/src/app/event_dispatch.rs` | `/fork` и shutdown-first exit переведены на runtime unload там, где live-runtime больше не нужен |
| `codex-rs/tui/src/app/safety_buffering.rs` | Safety-buffering retry сначала прикрепляет forked thread, затем выгружает прежние tracked runtimes |
| `codex-rs/tui/src/app/side.rs` | Явный и post-switch cleanup side conversation ожидают interrupt plus runtime unload и только затем удаляют локальное UI state |
| `codex-rs/core/src/agent/control/legacy.rs` | Не менять без новой причины; `close_agent` уже является настоящим shutdown path |
| `codex-rs/app-server/tests/suite/v2/thread_unload.rs` | Добавлено регрессионное покрытие unload без удаления persisted session |
| `codex-rs/tui/src/app/tests.rs` | Добавлено TUI-регрессионное покрытие для `/resume`, `/clear`, новой сессии, `/fork`, prompt backtrack, attach failure, shutdown-first и выгрузки runtime при side close |
| `codex-rs/tui/src/app/tests/safety_buffering.rs` | Проверяет успешный safety-buffering retry и сохранение старого runtime с черновиком при ранней ошибке fork |
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
| Prompt backtrack с переходом на forked thread | Старые tracked runtimes выгружаются только после успешного attach ветки для редактирования prompt |
| Safety-buffering retry | Старые tracked runtimes выгружаются только после успешного attach retry thread |
| Shutdown-first exit | TUI просит app-server закрыть current runtime, а не только отписаться от событий |

Если новый thread не стартовал, resume/fork не удался или TUI не смог
прикрепиться к новому thread, старый runtime не выгружается раньше времени. В
реализации `/resume`, `/clear`, новая сессия, `/fork`, prompt backtrack и
safety-buffering retry сначала получают и прикрепляют новый runtime, а затем
выгружают старые tracked runtimes, исключая новый thread id.

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
Post-switch cleanup также не должен возвращаться к fire-and-forget
`thread/unsubscribe`: он ожидает тот же unload path перед удалением локального
state, даже если пользователь уже переключился на parent thread.

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
   Отдельно проверить появившиеся в upstream переходы prompt backtrack и
   safety-buffering retry: новый thread должен быть прикреплен до выгрузки старых
   runtime.
7. Не менять автоматическое `/agent` view switching без отдельного продуктового
   решения.
8. Добавить app-server и TUI regression tests.
9. Обновить schema/TS artifacts через skill-owned generators, если добавлен
   новый app-server API.
10. Синхронизировать `fork-tests.v1` с фактическими test names после реализации.

## Проверки

Исполняемая карта card-level regression tests:

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
      "purpose": "tui new session выгружает предыдущий primary runtime",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-tui",
        "new_session_requests_unload_for_previous_conversation"
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
      "purpose": "tui prompt backtrack подключает replacement до выгрузки source runtime",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-tui",
        "prompt_backtrack_unloads_source_runtime_after_replacement_attach"
      ]
    },
    {
      "purpose": "tui сохраняет прежний runtime при ранней ошибке resume",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-tui",
        "failed_resume_keeps_previous_runtime_loaded_and_active"
      ]
    },
    {
      "purpose": "tui сохраняет source runtime при ошибке attach уже созданного fork runtime",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-tui",
        "fork_attach_failure_keeps_source_runtime_loaded_and_active"
      ]
    },
    {
      "purpose": "tui safety retry выгружает source только после attach replacement runtime",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-tui",
        "safety_retry_unloads_source_runtime_after_replacement_attach"
      ]
    },
    {
      "purpose": "tui safety retry сохраняет source runtime и черновик при ошибке fork",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-tui",
        "safety_retry_branch_failure_keeps_source_runtime_and_unsent_draft"
      ]
    },
    {
      "purpose": "tui shutdown-first выгружает current runtime без legacy Op::Shutdown",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-tui",
        "shutdown_first_exit_unloads_current_runtime_without_submitting_op"
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

Новые сценарии проверяют полный список загруженных app-server threads и активное
состояние TUI. Test-only attach hook срабатывает после создания replacement
runtime, но до замены `ChatWidget`: при этой ошибке source runtime и прежнее
состояние TUI сохраняются. Код этих тестов принят статической вычиткой, но пока
не компилировался, не форматировался и не запускался; исполняемая проверка карты
отложена до общего тестового прохода по карточкам.

Дополнительно обязателен `fork generators`, поскольку доработка добавляет `thread/unload` в app-server API и generated TypeScript schema.

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
- После ошибки attach уже созданный replacement runtime сейчас остаётся loaded,
  хотя source runtime сохраняется. Регрессионный тест явно наблюдает это состояние
  и выгружает replacement при очистке fixture; автоматический cleanup требует
  отдельной production-доработки.

## Открытые вопросы

- Нужно ли показывать пользователю warning, если unload старого runtime
  завершился timeout, но TUI уже перешел на новый thread?
- Достаточно ли аудита исходного кода для MCP process termination, или нужен
  отдельный runtime integration test с fixture MCP-процессом и подсчетом live
  launches?
