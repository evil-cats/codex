---
id: fork-exec-command-output-spill-files
status: active
created: 2026-06-21
updated: 2026-08-13
---

# Spill-файлы для длинного output `exec_command`

## Обзор

Эта карточка фиксирует fork-доработку Hermione для unified
`exec_command`: если обычная команда завершилась до `yield_time_ms`, вернула
финальный `exit_code`, и захваченный output больше настроенного inline-лимита,
Codex должен сохранить захваченный output в файл, а модели передать короткий
excerpt и путь к файлу.

## Зачем это нужно

Сейчас модель видит только inline-представление результата tool-а. Если
`exec_command` output не помещается в текущий лимит, Codex передает модели
усеченный текст с marker-ом:

```text
…<removed_count> tokens truncated…
```

Другого места, откуда модель могла бы добрать retained output, нет. Поэтому
после усечения модель вынуждена работать только с тем фрагментом, который уже
попал в conversation history.

Нужная доработка уменьшает model-visible context для больших команд и
одновременно сохраняет возможность точечного follow-up:

```text
короткий inline excerpt -> модель быстро понимает форму результата
файл с сохраненным output -> модель явно читает детали только если они нужны
```

Это сохраняет bounded context, не переписывает историю, не раздувает prompt
cache и дает более удобный путь восстановления деталей для больших build/test/search logs.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/config/src/config_toml.rs` | Добавить `ToolsToml.exec` и новый TOML struct для `[tools.exec]` |
| `codex-rs/core/src/config/mod.rs` | Провести effective config value в `Config` и выставить default `1000` |
| `codex-rs/core/config.schema.json` | Сгенерированный schema artifact для нового key `[tools.exec].inline_output_max_tokens` |
| `codex-rs/core/src/tools/handlers/unified_exec/exec_command.rs` | Передать обычный exec request в process manager и оставить `SandboxDenied` вне цепочки spill |
| `codex-rs/core/src/unified_exec/mod.rs` | Добавить request/result поля или helper types для exec output spill |
| `codex-rs/core/src/unified_exec/process_manager.rs` | В immediate-finished branch записать `raw_output` в файл при превышении лимита |
| `codex-rs/core/src/tools/context.rs` | Отрендерить spill metadata, `Output saved to` и `Output excerpt` для `ExecCommandToolOutput` |
| `codex-rs/utils/path-utils/src/lib.rs` | При необходимости добавить atomic bytes write helper |
| `codex-rs/core/src/tools/context_tests.rs` | Проверить model-visible formatting для spill и non-spill cases |
| `codex-rs/core/src/unified_exec/*tests.rs` | Проверить immediate-finished branch, отсутствие spill для running process и запись файла |
| `codex-rs/core/src/config/config_tests.rs` | Проверить parsing/default/schema-facing config behavior |
| `docs/fork/exec-command-output-spill-files.md` | Owner handoff этой fork-доработки |

Намеренно не меняется в MVP:

| Зона | Почему не меняется |
| --- | --- |
| `write_stdin` | Polling long-running process имеет отдельный жизненный цикл |
| Interactive PTY/SSH | Сессия может жить долго и требует отдельного решения по partial chunks |
| MCP tool outputs | Другая tool surface и другой shape результата |
| Code mode result | В MVP меняется model-facing `FunctionCallOutput` для normal unified exec |
| Legacy shell | Целевая доработка относится к unified `exec_command` |
| `SandboxDenied` | Отказ sandbox-политики не проходит через цепочку записи output-файлов |
| Streaming capture buffer | MVP работает после текущего retained/captured output cap |
| Workspace files | Output artifacts не должны загрязнять пользовательский проект |
| Существующий `tool_output_token_limit` | Это общий config для context manager/model info, не exec-specific spill настройка |

## Итоговый контракт

### Короткий output

Если захваченный command output укладывается в effective
`inline_output_max_tokens`, файл не создается, а model-visible формат остается
как сейчас:

```text
Chunk ID: <chunk_id>
Wall time: <seconds> seconds
Process exited with code <exit_code>
Original token count: <approx_count>
Output:
HELLO WORLD!
```

### Длинный output

Если захваченный command output больше effective `inline_output_max_tokens`,
Codex сохраняет retained output в файл и передает модели excerpt:

```text
Chunk ID: <chunk_id>
Wall time: <seconds> seconds
Process exited with code <exit_code>
Original token count: <approx_count>
Output exceeded inline limit of <limit> tokens.
Output saved to: <output-log-file>
Output excerpt:
Total output lines: <line_count>

<начало реального вывода>
…<removed_count> tokens truncated…
<конец реального вывода>
```

`<limit>` должен быть effective inline limit, которым реально резали command
output excerpt.

### Отказ sandbox-политики

Если обычный запуск был классифицирован как `UnifiedExecError::SandboxDenied`,
этот результат не считается spill-case:

- output не сохраняется в `exec_outputs`;
- `ExecCommandToolOutput.output_spill` остается `None`;
- model-visible ответ использует обычную секцию `Output:`;
- `Output exceeded inline limit`, `Output saved to` и `Output excerpt:` не
  появляются.

Причина: `SandboxDenied` является отказом политики исполнения. Его не нужно
проводить через цепочку записи output-файлов, даже если в ошибке есть
retained stdout/stderr.

### Config

Добавить новый exec-specific config key:

```toml
[tools.exec]
inline_output_max_tokens = 1000
```

Контракт:

- значение относится только к command output excerpt;
- metadata wrapper не входит в этот лимит;
- если значение не задано, используется default `1000`;
- значение должно быть положительным целым числом;
- если model tool call задает `max_output_tokens` меньше config value, excerpt
  должен уважать меньший лимит;
- excerpt limit не должен превышать
  `turn.model_info.truncation_policy.into().token_budget()`;
- существующий top-level `tool_output_token_limit` не переиспользуется и
  остается общим механизмом ограничения tool/function outputs в context manager.

Концептуальная формула:

```text
effective_inline_limit =
  min(
    config.tools.exec.inline_output_max_tokens.unwrap_or(1000),
    request.max_output_tokens.unwrap_or(usize::MAX),
    turn.model_info.truncation_policy.into().token_budget()
  )
```

`request.max_output_tokens` может уменьшить effective limit, но не увеличить
config/default cap.

### Хранение

Путь хранения:

```text
<codex_home>/exec_outputs/<thread_id>/<call_id>-<chunk_id>.log
```

Требования:

- путь должен быть absolute;
- директория создается Codex-ом;
- `<thread_id>`, `<call_id>` и `<chunk_id>` проходят sanitization до
  безопасных path components;
- текст команды не попадает в имя файла;
- содержимое файла равно `ExecCommandToolOutput.raw_output` для этого tool
  response;
- если upstream-коллектор вывода отбросил середину из-за capture cap,
  `raw_output` и spill-файл содержат одинаковые head/tail bytes с каноническим
  marker-ом `... <bytes> bytes omitted ...`; spill не восстанавливает уже
  отброшенные коллектором bytes;
- запись файла выполняется один раз до render model-visible response;
- `response_text()` не выполняет filesystem side effects;
- файл является Codex-owned runtime artifact, а не пользовательским файлом в
  workspace.

### Ошибка записи файла

Если команда завершилась успешно или с финальным `exit_code`, но запись
output-файла не удалась, tool call не должен становиться failed только из-за
ошибки spill artifact.

Model-visible формат должен оставаться bounded:

```text
Output exceeded inline limit of <limit> tokens.
Failed to save output: <short error>
Output excerpt:
...
```

Ошибка также должна попасть в tracing/logging как warning.

## Архитектурное решение

### Почему работать после текущего capture cap

Unified exec уже удерживает bounded output. MVP принимает эту границу как часть
существующего runtime-контракта и не пытается писать raw stdout/stderr stream
параллельно с process reader.

Плюсы:

- маленький blast radius;
- не меняется streaming reader;
- нет нового unbounded disk write path;
- совпадает с тем, что модель в любом случае не получала больше текущего cap;
- можно быстро дать модели путь к retained output без глубокого refactor.

Минус:

- файл не является полным stdout/stderr процесса, если process напечатал больше
  existing capture cap.

Этот минус не показывается модели в обычном случае, потому что текущий cap уже
является существующей границей captured output. В документации и карточке
нельзя называть файл `full output`.

### Почему `Output excerpt:`

Сейчас `Output:` является единственным представлением результата для модели.
Даже если он truncated, другого источника нет, поэтому текущий заголовок
логичен.

После spill появляются две сущности:

```text
Output saved to: <path>
Output excerpt:
<inline fragment>
```

`Output excerpt:` нужен только в spill-case, чтобы не смешивать сохраненный
artifact и inline фрагмент.

### Почему новый config key

`tool_output_token_limit` уже существует и влияет на общий механизм tool output
truncation/model info. Новая настройка должна управлять другим контрактом:
когда сохранять `exec_command` output в файл и сколько command stdout/stderr
показывать inline.

Поэтому новый key должен быть exec-specific:

```toml
[tools.exec]
inline_output_max_tokens = 1000
```

Изменение имени или области этого key является отдельным изменением контракта.
Настройка не заменяет `tool_output_token_limit`.

### Общий поток обработки

Для unified `exec_command` текущий поток такой:

```text
exec_command(...)
  -> process завершается до yield_time_ms или остается running
  -> process_manager собирает retained bytes из OutputBuffer
  -> строит ExecCommandToolOutput
  -> ExecCommandToolOutput::response_text()
  -> FunctionCallOutput(call_id=..., output=<response_text>)
  -> conversation history
  -> следующий model request
```

`ExecCommandToolOutput::response_text()` сейчас добавляет metadata wrapper:

```text
Chunk ID: <chunk_id>
Wall time: <seconds> seconds
Process exited with code <exit_code>
Original token count: <approx_count>
Output:
<possibly truncated command output>
```

Если command output больше текущего model output limit, секция `Output:`
получает результат `formatted_truncate_text(...)`:

```text
Output:
Total output lines: <line_count>

<начало реального вывода>
…<removed_count> tokens truncated…
<конец реального вывода>
```

Отдельного поля с количеством токенов после truncation нет.
`Original token count` означает approximate token count до inline truncation
для захваченного output. Факт truncation модель видит по marker-у внутри
`Output:`.

## Порядок повторения при переносе

1. Добавить config model для `[tools.exec].inline_output_max_tokens`:
   - TOML parsing;
   - default `1000`;
   - effective `Config` field;
   - schema update.
2. Добавить helper для path:

   ```text
   codex_home/exec_outputs/<thread_id>/<call_id>-<chunk_id>.log
   ```

3. Добавить safe filename sanitization:
   - ASCII alphanumeric, `-`, `_`;
   - fallback для пустого значения;
   - не использовать command text.
4. Добавить bytes write helper:
   - создать parent directory;
   - писать bytes atomically, если это удобно;
   - не lossy-convert'ить output ради записи файла.
5. В immediate-finished `exec_command` branch:
   - получить `raw_output`;
   - посчитать approximate token count;
   - если count больше effective inline limit, записать файл;
   - сохранить path/error в `ExecCommandToolOutput`;
   - в `UnifiedExecError::SandboxDenied` не вызывать spill helper и оставлять
     `output_spill: None`.
6. Изменить `ExecCommandToolOutput::response_text()`:
   - non-spill формат оставить текущим;
   - spill формат добавить через `Output exceeded...`, `Output saved to...`,
     `Output excerpt:`.
7. Убедиться, что `response_text()` не пишет файлы и остается pure render.
8. Обновить tests и schema.

## Проверки

Исполняемая карта card-level regression tests:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "разбор и применение лимита inline excerpt для exec output",
      "argv": ["just", "test", "-p", "codex-core", "inline_output_max_tokens"]
    },
    {
      "purpose": "сохранение большого immediate-finished output и ссылка на файл для модели",
      "argv": ["just", "test", "-p", "codex-core", "output_spill"]
    },
    {
      "purpose": "формат excerpt и metadata при успешном сохранении output",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "exec_command_tool_output_formats_spilled_response"
      ]
    },
    {
      "purpose": "warning и excerpt без падения tool call при ошибке сохранения",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "exec_command_tool_output_formats_spill_save_failure"
      ]
    },
    {
      "purpose": "сквозной spill большого вывода через exec_command",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "exec_command_spills_large_completed_output_to_file"
      ]
    },
    {
      "purpose": "отсутствие преждевременного spill в long-running polling ветке",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "unified_exec_timeouts"
      ]
    }
  ]
}
```

Дополнительно обязателен `fork generators`, поскольку доработка добавляет
`[tools.exec].inline_output_max_tokens` в config schema.

## Риски и ограничения

- Политику усечения нужно брать из
  `turn.model_info.truncation_policy.into()`; не восстанавливать старое поле
  `turn.truncation_policy`.
- В ветке `UnifiedExecError::SandboxDenied` сохраняется `output_spill: None`;
  retained output передается через заранее собранный `raw_output`, без повторного
  `output_text.into_bytes()`.
- Saved output file может содержать secrets, если команда вывела secrets.
  Это уже риск terminal output, но file artifact делает его дольше живущим.
  Файл должен создаваться в Codex-owned directory с безопасными permissions.
- Путь к файлу попадает в conversation history. Поэтому обычный `/tmp` хуже,
  чем `codex_home`: после resume путь из history должен иметь шанс остаться
  читаемым.
- Файл не должен называться `full output`; MVP сохраняет retained captured
  output после существующего capture buffer.
- Если future implementation захочет сохранять весь stdout/stderr stream до
  `1 MiB` cap или сверх него, это отдельный design change.
- `inline_output_max_tokens` использует approximate token counting, как текущий
  truncation stack. Не обещать точную model tokenizer семантику.
- `Output saved to` дает модели возможность читать файл через subsequent shell
  tool call. Если sandbox/permissions запретят чтение `codex_home`, нужно
  будет отдельно расширить readable roots или выбрать другой Codex-owned path.
- Если `response_text()` вызовут несколько раз, он не должен повторно создавать
  или перезаписывать файл.
