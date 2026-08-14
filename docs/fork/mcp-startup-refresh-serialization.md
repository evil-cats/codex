---
id: fork-mcp-startup-refresh-serialization
status: active
created: 2026-08-14
updated: 2026-08-14
---

# Сериализация обновления MCP во время запуска

## Обзор

Карточка владеет fork-доработкой, которая не позволяет автоматическому
обновлению MCP runtime заменить ещё запускающийся набор соединений. Запросы
обновления остаются объединёнными до завершения текущего startup, после чего
один refresh применяет последнее желаемое состояние.

Доработка относится только к конкуренции между первоначальным startup и
обычным coalesced refresh. Восстановлением уже работавшего транспорта после
`TransportClosed` или `BrokenPipe` владеет карточка
[`mcp-transport-recovery.md`](mcp-transport-recovery.md).

## Зачем это нужно

Инициализация MCP и обнаружение плагинов выполняются параллельно. Изменение
эффективного набора плагинов может пометить MCP runtime грязным, пока исходный
клиент ещё выполняет handshake. Немедленный refresh не может переиспользовать
такой клиент: текущая проверка reuse принимает только завершивший startup
клиент. Старый набор соединений освобождается, его cancellation token отменяет
первый запуск, а новый runtime запускает тот же сервер повторно.

В результате один автоматический refresh создаёт лишний процесс и гонку между
двумя startup. Первый запуск сообщает `Cancelled`, второй может успешно стать
`Ready`, однако интерфейс обоснованно показывает прерванный startup. Скрывать
это сообщение нельзя: нужно устранить ненужную отмену, сохранив видимость
настоящих отмен.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/codex-mcp/src/connection_manager.rs` | Предоставляет внутренний snapshot-scoped сигнал завершения startup всех соединений текущего `McpConnectionSet` |
| `codex-rs/codex-mcp/src/runtime.rs` | Позволяет проверить и дождаться завершения startup опубликованного runtime без использования protocol events |
| `codex-rs/codex-mcp/src/connection_manager_tests.rs` | Обновляет существующие test fixtures для snapshot-scoped сигнала завершения startup |
| `codex-rs/core/src/session/mcp.rs` | Не потребляет dirty-состояние и не публикует replacement, пока текущий MCP runtime ещё запускается |
| `codex-rs/core/src/session/mcp_prewarm.rs` | Дожидается завершения startup вне refresh gate и повторяет обработку того же coalesced запроса |
| `codex-rs/core/tests/suite/mcp_startup_refresh_serialization.rs` | Проверяет полный путь automatic invalidation, startup и отложенного refresh |
| `codex-rs/core/tests/suite/mod.rs` | Подключает интеграционные тесты доработки |
| `docs/fork/mcp-startup-refresh-serialization.md` | Владеющий документ передачи доработки |

Намеренно не меняются:

| Зона | Причина |
| --- | --- |
| TUI и `McpStartupComplete` | Настоящие `Cancelled` по-прежнему показываются; исправляется источник лишней отмены, а не её отображение |
| `codex-rs/rmcp-client` | Post-startup recovery транспорта остаётся отдельным контрактом |
| Rollout, thread state и контекст модели | Сериализация refresh не добавляет сохраняемые события, диагностику или model-visible данные |
| Процессы самого MCP-сервера | Архитектура stdio frontend и его собственного daemon не меняется |

## Итоговый контракт

### Граница завершения startup

Startup опубликованного `McpConnectionSet` считается завершённым, когда все
startup futures этого snapshot получили итог `Ready`, `Failed` или
`Cancelled`. Пустой набор и набор только из уже готовых переиспользованных
соединений завершены сразу.

Это не граница startup grace, после которой turn может продолжиться без
необязательного сервера. Поздний startup необязательного сервера остаётся
незавершённым до его результата и ограничен существующим server startup
timeout. Turn при этом продолжает использовать уже опубликованный runtime и
stable binding без ожидания полного startup.

Внутренний сигнал завершения не зависит от наличия `tx_event` и не использует
`EventMsg::McpStartupComplete` как управляющее сообщение. Protocol event только
отражает тот же итог для потребителей интерфейса.

### Отложенный automatic refresh

Если обычный dirty/coalesced refresh встречает ещё запускающийся текущий
runtime, он:

1. Не захватывает dirty-флаг как выполненную работу.
2. Не вызывает `McpRuntime::replace`.
3. Не освобождает текущий `McpConnectionSet` и не отменяет его startup.
4. Возвращает управление вызывающему пути, поэтому turn не ждёт поздние
   необязательные серверы.

Prewarm worker ожидает завершения startup отдельно от refresh semaphore. После
пробуждения он заново проверяет актуальный опубликованный runtime: если уже
появился другой незавершённый snapshot, ожидание продолжается для него.

Когда актуальный startup завершён, worker вычисляет последнее желаемое
состояние и потребляет все накопленные к этому моменту запросы одним обычным
refresh. Запрос, пришедший во время самой публикации, сохраняет существующий
контракт `McpRefresh` и вызывает ещё одну итерацию только для более нового
состояния.

### Переиспользование и допустимый новый запуск

После успешного startup неизменившееся соединение проходит существующую
проверку `McpServerConnection::reusable_client`; отложенный refresh не создаёт
для него второй процесс и не выдаёт `Cancelled`.

Новый запуск допустим после завершения ожидания, если исходный startup
завершился ошибкой или отменой, соединение уже закрыто либо последнее желаемое
состояние изменило identity сервера. Это обычная reconciliation, а не повтор
из-за гонки.

Явный hard refresh через `replace_fresh`, shutdown и другие операции, которые
по контракту требуют свежих соединений или отмены работы, не превращаются в
coalesced refresh и не скрывают свои настоящие `Cancelled`.

## Архитектурное решение

`McpConnectionSet` хранит snapshot-scoped completion primitive, связанный с
теми же shared startup futures, из которых собирается итог startup. Runtime
экспортирует core только минимальную возможность проверить pending-состояние и
асинхронно дождаться его завершения. Сигнал остаётся внутренним и не расширяет
protocol API.

Проверка выполняется в `Session::refresh_mcp_if_dirty` до `claim` dirty-флага.
Если startup ещё идёт, функция сохраняет pending-состояние, оставляет bounded
wake token для prewarm worker и выходит. Worker продолжает обслуживать dirty
состояние: ждёт startup, повторяет проверку и лишь затем запускает refresh.
Повторные wake token объединяются ограниченным каналом, поэтому между проверкой
и ожиданием нет потерянного wakeup, а несколько invalidation во время startup
не создают несколько публикаций.

Ожидание выполняется без удержания refresh semaphore. Иначе поздний startup,
смена snapshot или остановка worker могли бы оставить остальные refresh-пути
заблокированными. Shutdown по-прежнему отменяет ожидание через существующий
`mcp_prewarm_shutdown`.

Переиспользовать незавершённый `AsyncManagedClient` в новом
`McpConnectionSet` намеренно не требуется. Такой перенос усложнил бы владение
cancellation token, startup summary и publication gate. После короткой
сериализации уже существующий ready-reuse решает задачу без второго жизненного
цикла соединения.

## Порядок повторения при переносе

1. Проверить текущие границы `McpConnectionSet::new`,
   `McpServerConnection::reusable_client`, `McpRuntime::replace`,
   `Session::refresh_mcp_if_dirty` и prewarm worker.
2. Проверить, не сериализует ли новый upstream automatic refresh относительно
   startup с теми же гарантиями; не переносить дублирующий механизм.
3. Добавить к connection set внутренний snapshot-scoped сигнал завершения всех
   startup futures, включая немедленно завершённые empty/reused случаи.
4. Провести через `McpRuntime` минимальный pending/wait интерфейс без protocol,
   rollout или TUI событий.
5. До потребления dirty-флага откладывать обычный refresh, если актуальный
   runtime ещё запускается; не блокировать этим ожиданием turn.
6. В prewarm worker дождаться завершения snapshot вне refresh gate, повторно
   проверить актуальный runtime и применить последнее желаемое состояние.
7. Сохранить непосредственное поведение hard refresh, shutdown и identity
   change, а также существующий ready-reuse после успешного startup.
8. Перенести integration regression tests, проверяющие число запусков,
   отсутствие ложного `Cancelled`, coalescing и продолжение после неуспешного
   startup.
9. Убедиться, что перенос не добавил логирование, rollout-диагностику,
   model-visible context или часть post-startup transport recovery.

## Проверки

Исполняемая карта card-level regression tests:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "существующие startup и ready-reuse контракты MCP connection manager",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-mcp"
      ]
    },
    {
      "purpose": "automatic refresh ждёт исходный startup и переиспользует готовое соединение без отмены",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "mcp_refresh_waits_for_initial_startup_without_cancelling_it"
      ]
    },
    {
      "purpose": "несколько invalidation во время startup объединяются в один последующий refresh",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "mcp_refresh_requests_coalesce_while_startup_is_pending"
      ]
    },
    {
      "purpose": "неуспешный исходный startup не оставляет coalesced refresh заблокированным",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "mcp_refresh_continues_after_initial_startup_failure"
      ]
    }
  ]
}
```

## Риски и ограничения

- Automatic refresh может применить новое состояние только после завершения
  всех startup futures текущего snapshot. Медленный необязательный сервер
  откладывает refresh до своего существующего startup timeout, но не блокирует
  turn.
- После пробуждения нельзя публиковать состояние относительно устаревшего
  snapshot. Worker обязан повторно проверить текущий runtime и при необходимости
  дождаться уже нового startup.
- Dirty-флаг нельзя потреблять до решения выполнять refresh. Иначе отмена
  waiter или shutdown создадут потерянное обновление.
- Ожидание под refresh semaphore способно создать взаимную блокировку или
  неоправданный backpressure; оно намеренно находится вне gate.
- Настоящий hard refresh, shutdown, изменение identity, закрытый транспорт или
  неуспешный startup всё ещё могут создать новый процесс и событие `Cancelled`.
  Карточка устраняет только лишнюю отмену из-за automatic refresh.
- Доработка не является watchdog и не восстанавливает зависший или умерший
  после startup MCP transport. Этим поведением владеет отдельная карточка
  восстановления транспорта.
