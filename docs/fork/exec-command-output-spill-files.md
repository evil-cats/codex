---
id: fork-exec-command-output-spill-files
status: active
created: 2026-06-21
updated: 2026-06-21
source_scope: discussion-2026-06-21
---

# Spill-файлы для длинного output `exec_command`

## Обзор

Эта карточка фиксирует fork-доработку Hermione для unified
`exec_command`: если обычная команда завершилась до `yield_time_ms`, вернула
финальный `exit_code`, и захваченный output больше настроенного inline-лимита,
Codex должен сохранить захваченный output в файл, а модели передать короткий
excerpt и путь к файлу.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Область MVP | Только immediate-finished unified `exec_command` |
| Не входит в MVP | `write_stdin`, polling long-running process, interactive PTY/SSH, отказ sandbox-политики, MCP tools, code mode, legacy shell |
| Новый config key | `[tools.exec].inline_output_max_tokens` |
| Встроенное значение | `1000` approximate tokens |
| Что лимитирует key | Только excerpt command output, не metadata wrapper |
| Место файла | `<codex_home>/exec_outputs/<thread_id>/<call_id>-<chunk_id>.log` |
| Содержимое файла | `ExecCommandToolOutput.raw_output` как retained captured bytes |
| Старый общий лимит | `tool_output_token_limit` не переиспользуется |
| Усечение excerpt | Текущий `formatted_truncate_text(..., TruncationPolicy::Tokens(limit))` |

Главный контракт:

- короткий output остается в текущем формате и не создает файл;
- длинный захваченный output сохраняется в Codex-owned artifact file;
- модель получает `Output exceeded inline limit of <limit> tokens.`,
  `Output saved to: <path>` и `Output excerpt:`;
- `SandboxDenied` не проходит через цепочку spill и остается обычным
  ограниченным `Output:`;
- в model-visible тексте не используется слово `full`;
- обычный output не получает объяснение про внутренний `1 MiB` capture cap.

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

## Согласованные решения

| Пункт | Решение | Причина |
| --- | --- | --- |
| Область MVP | Только immediate-finished `exec_command` | Это исходная ветка поведения: команда завершилась до `yield_time_ms`, есть финальный `exit_code`, нет live `process_id` |
| Ветка long-running process | Не менять в MVP | `write_stdin`, polling и interactive sessions имеют отдельный жизненный цикл и требуют отдельного дизайна |
| `SandboxDenied` | Не проводить через цепочку spill | Если sandbox запретил действие, этот результат не должен создавать Codex-owned output artifact |
| Граница захвата | Работать после существующего unified exec capture buffer | `1 MiB` retained/captured cap уже существует и считается нормальной границей MVP |
| "Full output" | Не писать `full` в model-visible тексте | Файл содержит retained/captured output, а не обязательно весь stdout/stderr процесса до capture cap |
| `1 MiB` cap в тексте | Не объяснять модели в обычном случае | Модель никогда не получала больше этого cap; постоянная оговорка будет шумом |
| Условие spill | `approx_token_count(raw_output) > inline_output_max_tokens` | Настройка относится к command output excerpt |
| Config key | Добавить новый `[tools.exec].inline_output_max_tokens` | Существующий `tool_output_token_limit` является общим механизмом truncation и не должен смешиваться с exec spill behavior |
| Встроенное значение | `1000` approximate tokens | Дает маленький model-visible excerpt по умолчанию |
| Бюджет metadata | Не вычитать из `inline_output_max_tokens` | Лимит относится к stdout/stderr excerpt, metadata wrapper идет отдельно |
| Усечение excerpt | Переиспользовать текущий truncator | Текущий код уже сохраняет начало и конец и вставляет marker в середину |
| `Output excerpt:` | Использовать только в spill-case | При наличии saved file inline часть становится excerpt относительно файла |
| Формулировка пути | `Output saved to: <path>` | Коротко, без обещания `full` и без внутренних подробностей capture buffer |
| Расположение файла | `codex_home/exec_outputs/<thread_id>/...` | Не загрязняет workspace, лучше переживает resume, чем обычный `/tmp` |
| Имя файла | Только sanitized ids | Команду нельзя класть в filename: она может быть длинной, нестабильной или содержать секреты |
| Побочные эффекты | Не писать файл внутри `response_text()` | `response_text()` может вызываться для preview/model response; запись файла должна быть одноразовой до render |
| Ошибка записи файла | Не превращать успешную команду в failed tool call | Команда уже завершилась; model-facing response должен дать warning и excerpt |

## Текущее поведение

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
- excerpt limit не должен превышать `turn.truncation_policy.token_budget()`;
- существующий top-level `tool_output_token_limit` не переиспользуется и
  остается общим механизмом ограничения tool/function outputs в context manager.

Концептуальная формула:

```text
effective_inline_limit =
  min(
    config.tools.exec.inline_output_max_tokens.unwrap_or(1000),
    request.max_output_tokens.unwrap_or(usize::MAX),
    turn.truncation_policy.token_budget()
  )
```

Если итоговая реализация решит иначе трактовать `request.max_output_tokens`,
карточку нужно обновить вместе с кодом. Исходная договоренность: model call
может попросить меньше, но не больше config/default cap.

### Хранение

Предлагаемый путь:

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

## Карта файлов

| Файл | Ожидаемая роль |
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

Если при реализации структура config делает `[tools.exec]` неудобной, нужно
обсудить новое имя и обновить эту карточку. Исходная договоренность:
настройка новая и не заменяет `tool_output_token_limit`.

## Порядок реализации

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

Эта карточка является handoff-артефактом реализации, а не runbook-ом запуска
тестов. В ней фиксируется проверочное покрытие, которое нужно сохранить при
переносе доработки. Исполняемая карта конкретных проверок живет в skill-owned
`fork tests`.

Минимальное смысловое покрытие для реализации:

| Проверка | Ожидаемый результат |
| --- | --- |
| Разбор config для `[tools.exec].inline_output_max_tokens = 1234` | Effective config содержит `1234` |
| Встроенное значение config | При отсутствии key используется `1000` |
| `config.schema.json` | `codex-rs/core/config.schema.json` содержит новый key |
| Завершенная команда ниже лимита | Файл не создается; model-visible format остается `Output:` |
| Завершенная команда выше лимита | Файл создается; model-visible text содержит `Output exceeded...`, `Output saved to...`, `Output excerpt:` |
| Содержимое файла | Файл содержит exact `raw_output` bytes |
| Содержимое excerpt | Excerpt создается текущим middle-truncation marker-ом |
| Running process после initial yield | Spill не происходит в MVP |
| Отказ sandbox-политики | Spill не происходит; model-visible format остается `Output:` |
| Ошибка сохранения | Tool response не становится failed, есть bounded warning и excerpt |
| `max_output_tokens` меньше config | Excerpt уважает меньший effective limit |

Для этой fork-доработки в исполняемой карте должно быть представлено такое
покрытие:

Исполняемая карта `fork tests`:

Данные ниже являются текущим блоком `fork-tests.v1`, который читает
`fork tests`.

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "inline token limit",
      "argv": ["just", "test", "-p", "codex-core", "inline_output_max_tokens"]
    },
    {
      "purpose": "spill output",
      "argv": ["just", "test", "-p", "codex-core", "output_spill"]
    },
    {
      "purpose": "spill formatting",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "exec_command_tool_output_formats_spill"
      ]
    },
    {
      "purpose": "large output spill",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "exec_command_spills_large_completed_output_to_file"
      ]
    },
    {
      "purpose": "glob deny policy",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "unified_exec_enforces_glob_deny_read_policy"
      ]
    },
    {
      "purpose": "timeout poll",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "unified_exec_timeout_and_followup_poll"
      ]
    }
  ]
}
```

| Область покрытия | Что должно проверяться |
| --- | --- |
| Config parsing/default | `[tools.exec].inline_output_max_tokens`, положительное значение и default `1000` |
| Spill helper | Effective limit, отсутствие файла ниже лимита, sanitized path и exact bytes |
| Model-visible format | `Output:` без spill и `Output excerpt:` при saved/failure spill |
| Immediate-finished branch | Завершенная команда выше лимита создает output-файл и bounded excerpt |
| Отказ sandbox-политики | `SandboxDenied` не создает output-файл и остается обычным `Output:` |
| Long-running/polling branch | Initial running process и follow-up polling не получают spill |

Запуск проверок не задается этой карточкой. Общий порядок запуска принадлежит
project skill `fork`; конкретные внутренние команды принадлежат skill-owned
wrappers.

## Риски и ограничения

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

## Проверка покрытия

| Согласованный пункт | Статус |
| --- | --- |
| Только immediate-finished `exec_command` входит в MVP | перенесено в карточку |
| `write_stdin`, polling, interactive PTY/SSH, MCP, code mode и legacy shell не входят в MVP | перенесено в карточку |
| `SandboxDenied` не проходит через цепочку spill и не пишет output-файл | перенесено в карточку |
| Существующий `1 MiB` capture cap принимается как граница MVP | перенесено в карточку |
| MVP работает после capture buffer и сохраняет `ExecCommandToolOutput.raw_output` | перенесено в карточку |
| В model-visible тексте не используется `full` | перенесено в карточку |
| В обычном случае модели не объясняется `1 MiB` cap | перенесено в карточку |
| Short output сохраняет текущий формат `Output:` | перенесено в карточку |
| Long output получает saved file и inline excerpt | перенесено в карточку |
| Spill text содержит `Output exceeded inline limit`, `Output saved to`, `Output excerpt` | перенесено в карточку |
| `Output excerpt:` используется из-за наличия saved file | перенесено в карточку |
| `inline_output_max_tokens` относится только к command output excerpt | перенесено в карточку |
| Excerpt использует текущий middle truncation utility | перенесено в карточку |
| Marker остается текущим, если MVP переиспользует truncator | перенесено в карточку |
| `Original token count` остается count до inline truncation | перенесено в карточку |
| Поле token count после truncation не добавляется | перенесено в карточку |
| Добавляется новый config key, а не reuse `tool_output_token_limit` | перенесено в карточку |
| Предложенное имя key: `[tools.exec].inline_output_max_tokens` | перенесено в карточку |
| Effective limit учитывает config, меньший `max_output_tokens` и `turn.truncation_policy` | перенесено в карточку |
| Output files являются Codex runtime artifacts, не workspace files | перенесено в карточку |
| Предложенный путь: `<codex_home>/exec_outputs/<thread_id>/<call_id>-<chunk_id>.log` | перенесено в карточку |
| Имя файла строится только из sanitized ids | перенесено в карточку |
| Файл пишется до `to_response_item`; `response_text()` без side effects | перенесено в карточку |
| Ошибка записи файла не делает команду failed | перенесено в карточку |
| Config schema нужно обновлять через fork generator wrapper | перенесено в карточку как требование к schema; конкретная команда принадлежит wrapper-у |
| Нужны точечные тесты для порога, пути, содержимого файла, `SandboxDenied` и веток вне MVP | смысловое покрытие перенесено в карточку; конкретные команды живут в skill-owned `fork tests` |
| Новая активная fork-карточка должна быть добавлена в исполняемую карту tests | перенесено в карточку и покрывается правкой skill-owned `fork tests` |
| Запуск проверок выполняется не из карточки, а после подтверждения карточек по общему порядку fork | намеренно не перенесено как инструкция карточки; источник правила - project skill `fork` |
| Прямые внутренние команды не должны быть основным проверочным входом в карточке | Сырой блок команд удален из карточки; карточка описывает проверочное покрытие, а не внутренние команды wrapper-а |
| Код, schema и тесты реализованы в working tree | перенесено в карточку |
| Migration-карта должна получить строку этой активной карточки | open question |
