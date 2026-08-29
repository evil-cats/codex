---
id: fork-exec-command-output-spill-files
status: active
created: 2026-06-21
updated: 2026-08-28
---

# Spill-файлы для длинного exec output

## Обзор

Эта карточка фиксирует политику вывода Hermione для unified `exec_command` и
внешнего результата Code Mode. Прямой вызов модели при превышении настроенного
inline-лимита сохраняет захваченный output в файл и возвращает модели короткий
excerpt с путем. Вложенный вызов из Code Mode не создает spill-файл: он либо
возвращает JavaScript полный захваченный output, либо отклоняет `Promise`
вложенного инструмента, если результат нельзя передать целиком. После завершения
JavaScript-ячейки внешний `exec` или `wait` снова находится на границе перед
моделью: большой текстовый результат сохраняется до усечения, а в
`custom_tool_call_output` попадают первые целые строки в пределах budget и путь
к artifact.

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

Head/tail excerpt дополнительно склеивает начало и конец через marker. Для JSON
и других структурированных форматов это создает ложный единый документ и требует
от модели отдельно распознавать место разрыва. Target-формат вместо этого
возвращает только последовательный prefix из целых строк и явно описывает
оставшийся диапазон.

Для прямого вызова модели доработка уменьшает видимый модели контекст и
одновременно сохраняет возможность точечного follow-up:

```text
короткий inline excerpt -> модель быстро понимает форму результата
файл с сохраненным output -> модель явно читает детали только если они нужны
```

Это сохраняет bounded context, не переписывает историю, не раздувает prompt
cache и дает более удобный путь восстановления деталей для больших
build/test/search logs.

Вложенный `tools.exec_command()` внутри Code Mode имеет другого получателя:
результат сначала обрабатывает JavaScript, а не модель. Человекочитаемый excerpt
с `…N tokens truncated…` нельзя выдавать за успешный stdout: скрипт не способен
восстановить пропущенную середину и может получить неверный результат поиска,
подсчета, разбора или агрегации. Поэтому успешный результат Code Mode означает
только полный захваченный output; превышение лимита или ограничение
capture-буфера является ошибкой вложенного вызова инструмента.

После обработки результата JavaScript-ом получатель снова меняется. Полный
`RuntimeResponse` возвращается из `codex-code-mode-host` в Core, а затем Core
создает внешний `custom_tool_call_output` для модели. На этой границе передача
десятков килобайт текста в rollout уже не помогает программной оркестрации:
скрипт завершил работу, а большой результат непосредственно расходует контекст
модели. Поэтому внешний результат Code Mode использует тот же exec-specific
inline cap и spill-механику, что прямой `exec_command`.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/config/src/config_toml.rs` | Добавить `ToolsToml.exec` и новый TOML struct для `[tools.exec]` |
| `codex-rs/core/src/config/mod.rs` | Провести effective config value в `Config` и выставить default `1000` |
| `codex-rs/core/config.schema.json` | Сгенерированный schema artifact для нового key `[tools.exec].inline_output_max_tokens` |
| `codex-rs/core/src/tools/handlers/unified_exec/exec_command.rs` | Преобразовать `ToolCallSource` в явный тип получателя результата, передать его в process manager и оставить `SandboxDenied` вне цепочки spill |
| `codex-rs/core/src/tools/handlers/unified_exec_tests.rs` | Проверить через настоящий `ExecCommandHandler`, что `DirectPlaintextMessage` выбирает видимый модели spill и сохраняет точные байты spill-файла |
| `codex-rs/core/src/unified_exec/mod.rs` | Владеть типом получателя, request/result-полями и вспомогательными типами политики exec output, зависящей от источника |
| `codex-rs/core/src/unified_exec/output_spill.rs` | Владеть spill metadata, рассчитать действующий inline-лимит, выбрать line-prefix excerpt, построить безопасный путь и записать model-visible exec artifact для прямого exec или внешнего Code Mode результата |
| `codex-rs/core/src/unified_exec/process_manager.rs` | В immediate-finished branch выбрать spill для модели либо полный результат/ошибку Code Mode по типу получателя |
| `codex-rs/core/src/tools/context.rs` | Разделить ответ модели `response_text()` и журналирование `ToolOutput::log_output()`: первый ограничивает вывод или показывает сведения о spill-файле, второй получает весь сохранённый вывод и не дублирует отметку о пропущенных байтах; не превращать неполный результат Code Mode в успешный JSON |
| `codex-rs/utils/path-utils/src/lib.rs` | При необходимости добавить atomic bytes write helper |
| `codex-rs/core/src/tools/context_tests.rs` | Проверить `Lines`/`Full output`/`Output excerpt`, oversized first line и отсутствие suffix/marker в model-visible prefix |
| `codex-rs/core/src/tools/line_utils.rs` | Разбить текст на строки с сохранением завершающего `\n` для общего контракта `read_file` и spill excerpt |
| `codex-rs/core/src/tools/handlers/read_file.rs` | Использовать общий helper и сохранить прежнюю семантику первых целых строк и line endings |
| `codex-rs/core/src/unified_exec/*tests.rs` | Проверить immediate-finished branch, выбор политики по источнику, отсутствие spill для Code Mode/running process и запись файла для модели |
| `codex-rs/core/src/config/config_tests.rs` | Проверить parsing/default/schema-facing config behavior |
| `codex-rs/core/tests/suite/unified_exec.rs` | Сквозно проверить spill завершённого `exec_command` для модели, видимую модели ссылку на файл и отсутствие spill при большом `SandboxDenied` |
| `codex-rs/core/src/tools/code_mode/mod.rs` | В общем `handle_runtime_response()` вызвать spill до усечения и script status; после spill сохранить метаданные целиком, а внешний лимит расходовать только на line prefix и audio |
| `codex-rs/core/src/tools/code_mode/output_spill.rs` | Построить канонический текстовый projection, сохранить его и заменить text items единым model-visible spill body, не меняя media order |
| `codex-rs/core/src/tools/code_mode/output_spill_tests.rs` | Проверить порядок text projection и сохранение media items при замене текста |
| `codex-rs/core/src/tools/code_mode/execute_handler.rs` | Передать identity внешнего `exec` для безопасного имени artifact |
| `codex-rs/core/src/tools/code_mode/wait_handler.rs` | Передать identity внешнего `wait`; использовать тот же общий spill path, что initial response |
| `codex-rs/core/tests/suite/code_mode.rs` | Сквозно проверить машинный nested-контракт и model-visible spill внешних `exec`/`wait` результатов |
| `.codex/skills/fork/scripts/fork_cli.py` | Собрать отладочный `codex-code-mode-host` с каноническими V8-артефактами для тестов карточки |
| `.codex/skills/fork/scripts/fork_cli_tests.py` | Проверить argv, V8-окружение и отказ при отсутствии ожидаемого host-бинарника |
| `docs/fork/exec-command-output-spill-files.md` | Owner handoff этой fork-доработки |

Намеренно не меняется в MVP:

| Зона | Почему не меняется |
| --- | --- |
| `write_stdin` | Polling long-running process имеет отдельный жизненный цикл |
| Interactive PTY/SSH | Сессия может жить долго и требует отдельного решения по partial chunks |
| MCP tool outputs | Другая tool surface и другой shape результата |
| Legacy shell | Целевая доработка относится к unified `exec_command` |
| `SandboxDenied` | Отказ sandbox-политики не проходит через цепочку записи output-файлов |
| Streaming capture buffer | MVP работает после текущего retained/captured output cap |
| Media items внешнего Code Mode результата | Spill сохраняет текстовый projection; изображения, audio и encrypted content остаются под действующей media-политикой |
| Workspace files | Output artifacts не должны загрязнять пользовательский проект |
| Существующий `tool_output_token_limit` | Это общий config для context manager/model info, не exec-specific spill настройка |

## Итоговый контракт

### Источник вызова и получатель результата

`ToolCallSource` уже различает прямой вызов и вложенный вызов из Code Mode:

```text
Direct | DirectPlaintextMessage -> ModelVisible
CodeMode { .. }                -> CodeModeNested
```

Это различие должно сохраняться до выбора политики вывода. Нельзя определять
источник по `call_id`, имени команды или наличию Code Mode session. Внутренний
API не должен принимать неочевидный boolean; используй явный enum получателя
или сам `ToolCallSource`, если зависимость нижнего слоя остается приемлемой.

Для `ModelVisible` действует контракт spill/excerpt этой карточки. Для
`CodeModeNested` spill не создается, а успешный JSON-результат содержит только
полный захваченный output. Внешний `RuntimeResponse` Code Mode не является еще
одним значением `ToolCallSource`: после выполнения JavaScript это отдельная
model-visible граница, общая для initial `exec` и последующего `wait`.

### Короткий прямой exec output

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

### Длинный прямой exec output

Если command output для модели больше действующего
`inline_output_max_tokens`, Codex сохраняет retained output в файл и передает
модели только первые целые строки, помещающиеся в effective inline limit:

```text
Chunk ID: <chunk_id>
Wall time: <seconds> seconds
Process exited with code <exit_code>
Original token count: <approx_count>
Lines: total=530 returned=1-87 remaining=443 complete=no
Full output: <output-log-file>
Output excerpt:

<сырые строки 1-87>
```

Если первая строка не помещается в effective inline limit, body не возвращается:

```text
Chunk ID: <chunk_id>
Wall time: <seconds> seconds
Process exited with code <exit_code>
Original token count: <approx_count>
Lines: total=530 returned=none remaining=530 complete=no
Full output: <output-log-file>
Error: first line exceeds excerpt token limit
```

Общий контракт выбора excerpt:

- `total` — число строк в сохраненном текстовом output;
- `returned` — диапазон первых целых строк либо `none`;
- `remaining` — число строк после возвращенного prefix;
- `complete=no` всегда присутствует в spill-case;
- разбиение на строки совпадает с `read_file`:
  `split_lines_preserving_endings()` сохраняет завершающий `\n` каждой строки;
- выбирается максимальный prefix, approximate token count которого не превышает
  effective inline limit;
- строки и их окончания не изменяются и не получают line numbers;
- строки из конца не добавляются;
- `…N tokens truncated…`, `Warning: truncated output` и другие marker-ы
  model-visible усечения не вставляются.

### Ответ для модели и журнал телеметрии

`ExecCommandToolOutput` формирует два представления захваченного вывода.
`response_text()` строит ограниченный ответ для модели: короткий вывод проходит
через действующую политику усечения, а при записи spill-файла ответ содержит
сведения о строках, путь и последовательный префикс. `ToolOutput::log_output()`
не наследует ограничение ответа модели и передаёт в журнал общие сведения о
вызове вместе со всем доступным `raw_output`.

Если слой захвата сообщил `output_omitted_bytes`, `ToolOutput::log_output()`
обязан включить каноническую строку `format_output_omission_marker()` ровно один
раз: сохранить уже присутствующую строку либо добавить её перед `raw_output`.
Сведения о spill-файле не заменяют и не сокращают запись журнала. Общие поля
ответа формируются одним внутренним методом, но `Output:` используется только в
обычном ответе модели и заголовке журнала; в ответе модели со spill-файлом вместо
него выводится общий блок сведений о сохранённом результате.

### Вложенный вызов из Code Mode

`tools.exec_command()` внутри Code Mode возвращает `Promise`. Если захваченный
output целиком укладывается в действующий лимит результата Code Mode и
capture-буфер ничего не отбросил, `Promise` разрешается обычным
`ExecCommandToolOutput.code_mode_result()` с полным `output`.

Если приблизительное число токенов превышает действующий лимит результата Code
Mode либо `output_omitted_bytes` показывает потерю данных в capture-слое, вызов
должен:

- не создавать spill-файл;
- не возвращать начало, конец или человекочитаемый marker усечения как
  успешный `output`;
- завершиться через существующий канал `Err(String)`;
- отклонить JavaScript `Promise` с сообщением, содержащим вид ограничения,
  действующий лимит и доступный `original_token_count`;
- предложить скрипту сузить command output, а не продолжать вычисление по
  неполным данным.

Скрипт может обработать ошибку через `try/catch`. Если он этого не делает,
ячейка Code Mode завершается с `Script error`, и модель получает возможность
сформировать более узкую команду. Сейчас отклонение `Promise` может содержать
строку, поэтому надежная нормализация ошибки в JavaScript имеет вид
`error?.message ?? String(error)`.

Обычная ошибка команды, отмена и отказ sandbox сохраняют существующую семантику
ошибок. Новая ошибка превышения лимита результата не должна маскировать
`exit_code` или выдаваться за успешное выполнение с частичным stdout.

### Внешний результат Code Mode `exec` и `wait`

`CodeModeExecuteHandler` и `CodeModeWaitHandler` передают полученный от host
`RuntimeResponse` в общий `handle_runtime_response()`. Именно здесь Core еще
имеет полный результат ячейки и уже знает, что следующий получатель — модель.
Spill выполняется в этом общем path и поэтому одинаково действует для initial
`exec`, yielded cell и последующего `wait`.

Перед принятием решения Core строит канонический текстовый projection:

- берет все `FunctionCallOutputContentItem::InputText` в текущем порядке;
- объединяет их через один `\n`, как действующий
  `formatted_truncate_text_content_items_with_policy()`;
- для `RuntimeResponse::Result` включает добавленный `Script error`, если host
  вернул `error_text`;
- не включает `Script completed`, `Script yielded`, `Script terminated` и
  `Wall time`: этот status добавляется только после spill;
- не сериализует в `.log` изображения, audio или encrypted content.

Если projection не превышает effective inline limit, сохраняется действующий
порядок: `truncate_code_mode_result()` остается safety boundary, затем Core
добавляет script status и строит `FunctionToolOutput`.

Если projection превышает effective inline limit, Core до вызова
`truncate_code_mode_result()`:

1. сохраняет весь канонический текстовый projection в Codex-owned artifact;
2. заменяет текстовые items одним model-visible сообщением в общем spill-формате:
   line metadata, absolute path и максимальный prefix из целых строк;
3. сохраняет spill metadata целиком, не передавая готовое сообщение повторному
   head/tail truncation;
4. расходует остаток внешнего лимита после line prefix на audio, сохраняя
   действующую политику нетекстовых items;
5. только затем добавляет script status и возвращает внешний
   `custom_tool_call_output`.

Таким образом, внешний Code Mode output получает человекочитаемый excerpt и
путь именно потому, что его потребляет модель. Это не меняет nested
`tools.exec_command()`: его результат по-прежнему возвращается JavaScript целиком
или отклоняет `Promise`, без создания и последующего удаления временного файла.

Для имени artifact используются `thread_id`, внешний tool `call_id` и `cell_id`.
У initial `exec` и каждого `wait` собственный `call_id`, поэтому несколько
ответов одной yielded cell не конфликтуют между собой.

### Отказ sandbox-политики

Если обычный запуск был классифицирован как `UnifiedExecError::SandboxDenied`,
этот результат не считается spill-case:

- output не сохраняется в `exec_outputs`;
- `ExecCommandToolOutput.output_spill` остается `None`;
- model-visible ответ использует обычную секцию `Output:`;
- spill-specific `Lines: ... complete=no`, `Full output:` и `Output excerpt:` не
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

- значение относится к line-prefix excerpt любого exec-результата,
  непосредственно передаваемого модели: прямого `exec_command` и внешнего Code
  Mode `exec` или `wait`;
- значение не управляет вложенным машинным результатом
  `tools.exec_command()` внутри Code Mode;
- metadata wrapper не входит в этот лимит;
- если значение не задано, используется default `1000`;
- значение должно быть положительным целым числом;
- если текущая model-visible граница задает меньший request limit, line prefix
  должен учитывать меньший лимит: `max_output_tokens` для прямого exec или
  Code Mode `exec`, `max_tokens` для Code Mode `wait`;
- line-prefix limit не должен превышать
  `turn.model_info.truncation_policy.into().token_budget()`;
- существующий top-level `tool_output_token_limit` не переиспользуется и
  остается общим механизмом ограничения tool/function outputs в context manager.

Концептуальная общая формула:

```text
effective_model_visible_exec_inline_limit =
  min(
    config.tools.exec.inline_output_max_tokens.unwrap_or(1000),
    boundary_request_limit,
    turn.model_info.truncation_policy.into().token_budget()
  )
```

Для прямого `exec_command` `boundary_request_limit` равен
`request.max_output_tokens.unwrap_or(usize::MAX)`. Для внешнего Code Mode
результата он вычисляется существующим `resolve_max_tokens()` из
`exec.max_output_tokens` или `wait.max_tokens`; отсутствие значения дает
`DEFAULT_MAX_OUTPUT_TOKENS = 10_000`. Request limit может уменьшить effective
limit, но не увеличить config/default cap.

Для наблюдавшегося результата примерно в `4 600` токенов при config `1000` и
внешнем Code Mode default `10 000` effective limit равен `1000`: Core сохраняет
весь текстовый projection, а модели возвращает excerpt не более `1000` токенов
из первых целых строк плюс metadata и путь к файлу.

Для `CodeModeNested` `request.max_output_tokens` является границей успешной
доставки полного результата в JavaScript, а не размером excerpt. После
разрешения через существующий `resolve_max_tokens()` отсутствие значения дает
`DEFAULT_MAX_OUTPUT_TOKENS = 10_000`; превышение этой границы отклоняет
`Promise`. `[tools.exec].inline_output_max_tokens` не должен уменьшать этот
внутренний лимит: config key применяется позже, когда итог JavaScript-ячейки
становится внешним результатом для модели.

### Хранение

Путь хранения:

```text
<codex_home>/exec_outputs/<thread_id>/<call_id>-<artifact_id>.log
```

Для прямого `exec_command` `<artifact_id>` равен `chunk_id`. Для внешнего Code
Mode `exec` или `wait` он равен `cell_id`; уникальность последовательных ответов
одной cell обеспечивает внешний `call_id` каждого tool call.

Требования:

- путь должен быть absolute;
- директория создается Codex-ом;
- `<thread_id>`, `<call_id>` и `<artifact_id>` проходят sanitization до
  безопасных path components;
- текст команды не попадает в имя файла;
- для прямого `exec_command` содержимое файла равно
  `ExecCommandToolOutput.raw_output` этого tool response;
- для прямого `exec_command`, если upstream-коллектор вывода отбросил середину
  из-за capture cap,
  `raw_output` и spill-файл содержат одинаковые head/tail bytes с каноническим
  marker-ом `... <bytes> bytes omitted ...`; spill не восстанавливает уже
  отброшенные коллектором bytes;
- запись файла выполняется один раз до render model-visible response;
- `response_text()` не выполняет filesystem side effects;
- файл является Codex-owned runtime artifact, а не пользовательским файлом в
  workspace.

Для внешнего Code Mode результата содержимое `.log` равно полному каноническому
текстовому projection до `truncate_code_mode_result()` и до script status.
Границы отдельных text items представлены разделителем `\n`; это та же
последовательность, по которой действующий truncation считает budget. Media
items не попадают в текстовый artifact и продолжают передаваться отдельно.

Line metadata и excerpt рассчитываются по тому же сохраненному тексту. Prefix
выбирается до записи model-visible response; повторное форматирование не должно
менять сохраненный artifact.

Для `CodeModeNested` файл не создается ни при успехе, ни при превышении лимита.
Не создавай файл с последующим удалением: политика, зависящая от источника,
должна пропустить запись до файловой операции.

### Ошибка записи файла

Если команда завершилась с финальным `exit_code` либо Code Mode host вернул
внешний `RuntimeResponse`, но запись output-файла не удалась, tool call не должен
становиться failed только из-за ошибки spill artifact.

Видимый модели формат должен оставаться ограниченным:

```text
Lines: total=<total> returned=<range-or-none> remaining=<count> complete=no
Failed to save output: <short error>
Output excerpt:
<первые целые строки, если хотя бы одна помещается>
```

Если первая строка не помещается, `Output excerpt:` отсутствует, а последней
строкой становится `Error: first line exceeds excerpt token limit`.

Ошибка также должна попасть в tracing/logging как warning.

## Архитектурное решение

### Почему работать после текущего capture cap

Unified exec уже удерживает ограниченный output. Spill для модели принимает эту
границу как часть существующего runtime-контракта и не пытается писать raw
stdout/stderr stream параллельно с process reader.

Плюсы:

- маленький blast radius;
- не меняется streaming reader;
- нет нового unbounded disk write path;
- совпадает с тем, что модель в любом случае не получала больше текущего cap;
- можно быстро дать модели путь к retained output без глубокого refactor.

Минус:

- файл не является полным stdout/stderr процесса, если process напечатал больше
  existing capture cap.

Target-формат использует согласованный label `Full output:`. Пока streaming spill
до capture cap не реализован, для прямой команды свыше `1 MiB` этот label
указывает на artifact с явным omission marker, а не на байт-в-байт полный
stdout/stderr процесса. Это известное ограничение текущего MVP, а не скрытое
утверждение о реализации будущего streaming path.

Для вложенного Code Mode exec то же ограничение capture-буфера является
семантической границей: если `output_omitted_bytes` задан, JavaScript не получил
бы полный результат и не может безопасно продолжать вычисление. Поэтому nested
branch возвращает ошибку вместо успешного результата с capture omission marker.

Внешний Code Mode spill работает на другой границе: он сохраняет полный
текстовый projection уже полученного `RuntimeResponse`. Поэтому его нужно
выполнить до model-visible truncation в Core, но он не меняет capture или IPC
между nested tool, host и Core.

### Возможное продолжение: spill до capture cap

Текущая доработка намеренно сохраняет существующий RAM cap `1 MiB`:
`HeadTailBuffer` оставляет начало и конец, а spill прямого exec записывает уже
retained output. Поэтому для команды, превысившей этот cap, artifact не содержит
удаленную середину.

Отдельная будущая доработка может сохранить ограниченный `HeadTailBuffer` для
inline response, но писать spill из последовательного потока до этого буфера:

```text
stdout/stderr
  -> threshold buffer
  -> при превышении inline limit создать spill и записать накопленный prefix
  -> последующие chunks писать непосредственно в spill
  -> параллельно поддерживать HeadTailBuffer для bounded inline response
```

Такой writer не должен создавать файл для короткой команды. Для него также
нужен отдельный предел размера на диске: без него неограниченный output может
заполнить filesystem. При превышении дискового предела или ошибке записи нельзя
называть artifact `Full output`; поведение этого случая должно быть определено
в рамках той будущей доработки.

Этот streaming path не входит в текущий контракт и не требуется для реализации
остальных пунктов карточки.

### Почему `Output excerpt:`

Сейчас `Output:` является единственным представлением результата для модели.
Даже если он truncated, другого источника нет, поэтому текущий заголовок
логичен.

После spill появляются две сущности:

```text
Full output: <path>
Output excerpt:
<первые целые строки>
```

`Output excerpt:` нужен только в spill-case, чтобы не смешивать сохраненный
artifact и inline prefix. Он отсутствует, если первая строка превышает excerpt
token limit и body поэтому пуст.

### Почему новый config key

`tool_output_token_limit` уже существует и влияет на общий механизм tool output
truncation/model info. Новая настройка должна управлять другим контрактом:
когда сохранять model-visible exec output в файл и сколько текста показывать
inline. Это относится и к stdout/stderr прямой команды, и к итоговому тексту
Code Mode ячейки.

Поэтому новый key должен быть exec-specific:

```toml
[tools.exec]
inline_output_max_tokens = 1000
```

Изменение имени или области этого key является отдельным изменением контракта.
Настройка не заменяет `tool_output_token_limit`.

### Почему политика зависит от получателя

В цепочке есть три разные границы результата:

```text
ModelVisible direct exec -> последовательный line prefix и путь допустимы
CodeModeNested           -> JavaScript нужен полный результат либо ошибка
ModelVisible Code Mode   -> итог ячейки снова получает line prefix и путь
```

Process manager не должен угадывать интерфейс по данным результата. На входе
`ExecCommandHandler` уже есть `ToolCallSource`; handler должен один раз
преобразовать его в явную политику вывода и передать ниже. Для Code Mode это
также предотвращает ненужную запись spill-файла.

Внешнюю model-visible границу выбирает не process manager, а общий
`handle_runtime_response()`: к этому моменту JavaScript уже обработал nested
результаты, а Core готовит итоговый tool output для модели.

Нельзя переносить предназначенное для модели line-prefix форматирование в
машинный результат Code Mode. Line metadata и сообщение об oversized first line
не являются частью stdout.

### Почему Code Mode получает отклонение `Promise`

Code Mode runtime уже представляет каждый `tools.*()` как JavaScript
`Promise`: `Ok(JsonValue)` разрешает его, а `Err(String)` отклоняет. Новый случай
превышения лимита использует этот существующий канал и не добавляет новое
сообщение host-протокола.

Отклонение `Promise` лучше частичного успеха: обычный скрипт останавливается, а
скрипт с `try/catch` может сформировать более узкую команду. Частичный output не
должен сохраняться в поле `output`, потому что тогда потребитель обязан знать
человекочитаемый формат усечения и легко продолжит вычисление по неполным
данным.

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

Поток, зависящий от источника, добавляет развилку до файловой операции и
форматирования результата для модели:

```text
ToolCallSource
  -> ModelVisible
       -> output больше inline limit
       -> сохранить retained output
       -> response_text(): line metadata + prefix + path
  -> CodeModeNested
       -> capture не потерял данные и output <= лимит результата
          -> code_mode_result(): полный output
       -> capture потерял данные или output > лимит результата
          -> Err(String) -> rejected JavaScript Promise
```

`ExecCommandToolOutput::response_text()` сохраняет обычный metadata wrapper:

```text
Chunk ID: <chunk_id>
Wall time: <seconds> seconds
Process exited with code <exit_code>
Original token count: <approx_count>
Output:
<полный короткий command output>
```

В spill-case обычная секция `Output:` заменяется согласованным форматом:

```text
Lines: total=<total> returned=1-<end> remaining=<count> complete=no
Full output: <path>
Output excerpt:

<первые целые строки>
```

`Original token count` продолжает означать approximate token count всего
захваченного output. Факт неполного inline body задается структурно через
`returned`, `remaining` и `complete=no`; marker внутри output не используется.

Отдельный внешний поток Code Mode выглядит так:

```text
model вызывает Code Mode exec
  -> Core отправляет JavaScript в codex-code-mode-host
  -> JavaScript вызывает nested tools и получает их полные bounded результаты
  -> host возвращает RuntimeResponse в Core
  -> handle_runtime_response() строит полный текстовый projection
  -> projection больше effective model-visible inline limit
     -> сохранить projection в exec_outputs
     -> заменить text items на line metadata + prefix + absolute path
  -> truncate_code_mode_result() остается общей safety boundary
  -> добавить Script status / Wall time
  -> custom_tool_call_output
  -> следующий model request
```

Spill должен находиться после host IPC, иначе host не сможет программно
обработать полный результат nested tool, и до model-visible truncation, иначе в
artifact попадет уже усеченный итог ячейки. Общий `handle_runtime_response()`
гарантирует одинаковый контракт для initial `exec` и `wait`.

## Порядок повторения при переносе

1. Добавить config model для `[tools.exec].inline_output_max_tokens`:
   - TOML parsing;
   - default `1000`;
   - effective `Config` field;
   - schema update.
2. Добавить helper для path:

   ```text
   codex_home/exec_outputs/<thread_id>/<call_id>-<artifact_id>.log
   ```

3. Добавить safe filename sanitization:
   - ASCII alphanumeric, `-`, `_`;
   - fallback для пустого значения;
   - не использовать command text.
4. Добавить bytes write helper:
   - создать parent directory;
   - писать bytes atomically, если это удобно;
   - не lossy-convert'ить output ради записи файла.
5. В `ExecCommandHandler` преобразовать `ToolCallSource` в явный тип получателя
   результата:
   - `Direct` и `DirectPlaintextMessage` -> `ModelVisible`;
   - `CodeMode { .. }` -> `CodeModeNested`;
   - не использовать boolean или эвристику по `call_id`.
6. В immediate-finished `exec_command` branch:
   - получить `raw_output`;
   - посчитать approximate token count;
   - для `ModelVisible`, если count больше effective inline limit, записать
     файл и сохранить path/error в `ExecCommandToolOutput`;
   - для `CodeModeNested` не создавать spill-файл;
   - для `CodeModeNested` вернуть успех только при полном capture и числе
     токенов не выше действующего лимита результата;
   - при превышении лимита результата Code Mode или
     `output_omitted_bytes != None`
     вернуть ошибку, которая отклонит nested tool `Promise`;
   - в `UnifiedExecError::SandboxDenied` не вызывать spill helper и оставлять
     `output_spill: None`.
7. Добавить общий renderer spill body и использовать его из
   `ExecCommandToolOutput::response_text()`:
   - non-spill формат оставить текущим;
   - разбить сохраненный текст общим с `read_file` helper-ом с сохранением line
     endings;
   - выбрать максимальный prefix из целых строк в пределах effective inline
     limit;
   - отрендерить `Lines: total=... returned=... remaining=... complete=no`,
     `Full output:` и `Output excerpt:`;
   - если первая строка не помещается, вернуть `returned=none` и
     `Error: first line exceeds excerpt token limit` без excerpt body;
   - не добавлять suffix, line numbers или marker усечения.
   - сохранить отдельный `ToolOutput::log_output()` без ограничения ответа
     модели: общие поля вызова плюс весь доступный `raw_output`;
   - при `output_omitted_bytes` включить каноническую строку
     `format_output_omission_marker()` ровно один раз независимо от сведений о
     spill-файле.
8. Не использовать `ExecCommandToolOutput::truncated_output()` или
   `formatted_truncate_text()` для spill excerpt. Model-visible границы получают
   line prefix, а вложенный Code Mode при превышении лимита возвращает ошибку;
   line-prefix формат не должен пересекать машинную границу.
9. Убедиться, что `response_text()` и `code_mode_result()` не пишут файлы и
   остаются pure render.
10. Обобщить output spill helper так, чтобы model-visible Code Mode path мог
    использовать те же effective limit, безопасный путь, permissions и bounded
    save failure, не создавая второй файловый протокол.
11. Передать внешний `call_id` из `CodeModeExecuteHandler` и
    `CodeModeWaitHandler` в `handle_runtime_response()`; `cell_id` использовать
    как `<artifact_id>`.
12. В общем `handle_runtime_response()` после преобразования
    `RuntimeResponse`, sanitization media и добавления `Script error`, но до
    `truncate_code_mode_result()` и `prepend_script_status()`:
    - объединить ordered text items через `\n`;
    - рассчитать effective model-visible inline limit из config, request limit
      и model truncation policy;
    - при превышении записать весь text projection;
    - заменить text items на общий line-prefix spill-формат с absolute path либо
      короткой ошибкой сохранения;
    - оставить media items под действующей политикой.
13. Сохранить `truncate_code_mode_result()` после spill как safety boundary;
    его вход в spill-case уже должен быть bounded и содержать видимый путь.
14. Обновить tests и schema.

## Проверки

Исполняемая карта card-level regression tests:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "предусловие Code Mode тестов: отладочный host доступен TestCodexBuilder",
      "argv": [
        ".codex/skills/fork/scripts/fork",
        "build-code-mode-host"
      ]
    },
    {
      "purpose": "разбор и применение лимита line-prefix excerpt для exec output",
      "argv": ["just", "test", "-p", "codex-core", "inline_output_max_tokens"]
    },
    {
      "purpose": "политика output_spill: сохранение для модели, DirectPlaintextMessage и обход SandboxDenied",
      "argv": ["just", "test", "-p", "codex-core", "output_spill"]
    },
    {
      "purpose": "формат line metadata, Full output и prefix при успешном сохранении output",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "exec_command_tool_output_formats_spilled_response"
      ]
    },
    {
      "purpose": "ответ response_text() ограничен, а ToolOutput::log_output() содержит весь сохранённый вывод",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "exec_command_tool_output_formats_truncated_response"
      ]
    },
    {
      "purpose": "ToolOutput::log_output() включает format_output_omission_marker() ровно один раз",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "exec_command_tool_output_preserves_omission_metadata_when_truncated"
      ]
    },
    {
      "purpose": "spill excerpt содержит только максимальный prefix из целых строк и не подмешивает suffix",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "output_spill_excerpt_returns_complete_prefix_lines"
      ]
    },
    {
      "purpose": "первая строка сверх лимита дает returned=none и явную ошибку без excerpt body",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "output_spill_excerpt_reports_oversized_first_line"
      ]
    },
    {
      "purpose": "line metadata и prefix без падения tool call при ошибке сохранения",
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
      "purpose": "вложенный Code Mode получает полный output в пределах лимита результата без человекочитаемого marker-а усечения",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "code_mode_exec_command_returns_complete_output_within_limit"
      ]
    },
    {
      "purpose": "вложенный Code Mode exec_command не создает spill-файл для модели",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "code_mode_exec_command_does_not_create_output_spill"
      ]
    },
    {
      "purpose": "превышение лимита результата Code Mode отклоняет Promise без частичного output",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "code_mode_exec_command_rejects_output_above_limit"
      ]
    },
    {
      "purpose": "потеря данных на capture layer отклоняет Code Mode Promise",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "code_mode_exec_command_rejects_incomplete_captured_output"
      ]
    },
    {
      "purpose": "внешний Code Mode exec учитывает request-level limit, сохраняет полный текст и возвращает целые spill metadata с line prefix и путем",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "code_mode_exec_spills_large_outer_text_result"
      ]
    },
    {
      "purpose": "внешний Code Mode spill сохраняет metadata и применяет остаток лимита к audio",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "spilled_output_preserves_metadata_and_omits_over_budget_audio"
      ]
    },
    {
      "purpose": "Code Mode wait применяет тот же spill-контракт к следующему ответу yielded cell",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "code_mode_wait_spills_large_outer_text_result"
      ]
    },
    {
      "purpose": "ошибка записи внешнего Code Mode artifact оставляет line prefix и не превращает tool call в ошибку",
      "argv": [
        "just",
        "test",
        "-p",
        "codex-core",
        "code_mode_outer_spill_save_failure_returns_line_prefix"
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

Предусловие под управлением fork-skill собирает отладочный вариант отдельного
`codex-code-mode-host` с каноническими артефактами V8. `TestCodexBuilder`
разрешает этот файл через `cargo_bin`. Без него узкий Cargo-прогон `codex-core`
откатывается к прямым инструментам и не исполняет JavaScript; такой ответ не
является успешной проверкой Code Mode. Bazel получает тот же бинарник через
`extra_binaries`.

Фильтр `output_spill` включает два дополнительных сценария:

- `direct_plaintext_message_selects_model_visible_output_spill` проводит
  `DirectPlaintextMessage` через настоящие `ExecCommandHandler` и
  `UnifiedExecProcessManager`, проверяет метаданные построчного префикса, путь и
  точные байты spill-файла;
- `sandbox_denied_large_output_bypasses_output_spill` создаёт реальный отказ
  записи в локальном sandbox только для чтения, проверяет обычную секцию
  `Output:` и отсутствие метаданных spill, spill-файла и каталога
  `exec_outputs`.

Новые тесты приняты статической вычиткой. Их компиляция, форматирование и запуск
отложены до общего прохода по карточкам.

Дополнительно обязателен `fork generators`, поскольку доработка добавляет
`[tools.exec].inline_output_max_tokens` в config schema.

## Риски и ограничения

- Политику усечения нужно брать из
  `turn.model_info.truncation_policy.into()`; не восстанавливать старое поле
  `turn.truncation_policy`.
- В ветке `UnifiedExecError::SandboxDenied` сохраняется `output_spill: None`;
  retained output передается через заранее собранный `raw_output`, без повторного
  `output_text.into_bytes()`.
- `ToolCallSource::CodeMode` нельзя терять через `..` до выбора политики вывода:
  иначе process manager создаст ненужный spill artifact для модели.
- Для вложенного Code Mode exec отсутствие spill не разрешает неограниченный
  успешный результат: полный `raw_output` возвращается только в пределах
  действующего nested-лимита и при `output_omitted_bytes == None`.
- Случаи превышения nested-лимита результата и capture-лимита не должны
  разрешать `Promise` с каким-либо частичным output. Они используют канал
  ошибки, чтобы JavaScript не принял неполные данные за stdout.
- Saved output file может содержать secrets, если команда вывела secrets.
  Это уже риск terminal output, но file artifact делает его дольше живущим.
  Файл должен создаваться в Codex-owned directory с безопасными permissions.
- Путь к файлу попадает в conversation history. Поэтому обычный `/tmp` хуже,
  чем `codex_home`: после resume путь из history должен иметь шанс остаться
  читаемым.
- `Full output:` является согласованным model-visible label spill-файла. Для
  прямой команды свыше текущего capture cap `1 MiB` artifact пока содержит
  retained head/marker/tail, поэтому label остается известным несовершенством до
  отдельной streaming-доработки. Эта карточка не должна выдавать ту доработку за
  уже реализованную.
- Внешний Code Mode artifact сохраняет полный текстовый projection, полученный
  Core от host, но не media items и не данные, которые host не передал Core.
- Запись последовательного stdout/stderr stream до `HeadTailBuffer` является
  отдельной design change из отложенного раздела этой карточки.
- `inline_output_max_tokens`, nested-лимит результата и внешний Code Mode limit
  используют приблизительный подсчет токенов, как текущий стек усечения. Не
  обещать точную семантику model tokenizer.
- `Full output` дает модели возможность читать файл диапазонами через
  `read_file`. Если sandbox/permissions запретят чтение `codex_home`, нужно будет
  отдельно расширить readable roots или выбрать другой Codex-owned path.
- Если `response_text()` вызовут несколько раз, он не должен повторно создавать
  или перезаписывать файл.
- `handle_runtime_response()` также не должен повторно писать artifact: файловая
  операция выполняется один раз до построения `FunctionToolOutput`.
- Spill-ветка получает retained bytes из уже собранного `collected` и не зависит
  от прямого импорта `OutputBuffer` в `process_manager.rs`.
- Внешние Code Mode `exec` и `wait` применяют
  `[tools.exec].inline_output_max_tokens` только после завершения обработки
  JavaScript-ом. Этот cap не должен протечь назад и ограничить промежуточные
  nested tool results молчаливым truncation.
- Spill внешнего Code Mode результата должен предшествовать
  `truncate_code_mode_result()`: иначе `.log` сохранит уже усеченный текст.
- Script status и `Wall time` не входят в artifact и не расходуют inline excerpt
  budget; они добавляются к уже bounded model-visible результату.
- Prefix структурированного документа, включая JSON, может быть синтаксически
  незавершенным. Это допустимо: `complete=no` сообщает о продолжении, а отсутствие
  suffix и marker не создает ложную склейку двух несмежных частей.
- Однострочный output больше excerpt limit возвращает `returned=none`; нельзя
  резать такую строку посередине ради непустого preview.
