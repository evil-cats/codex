---
id: fork-resume-profile-startup-latency
status: active
created: 2026-08-25
updated: 2026-09-06
---

# Ускорение запуска больших сессий через `resume`

## Обзор

Карточка определяет требования к быстрому и стабильному начальному
восстановлению больших сессий через `resume`. Она объединяет два связанных
участка:

- один `InlineVisualizationContext` на hydration и один snapshot разрешённых
  `entries` permission profile на составную проверку либо построение writable
  roots;
- согласованный row budget постраничной истории и один итоговый `reflow` после
  автоматической догрузки scrollback.

Эти гарантии не ослабляют защиту кэшей viewers, не уменьшают объём доступной
истории, не меняют свойства выбранного permission profile и не удаляют общий
терминальный scrollback между последовательными сессиями.

## Зачем это нужно

При построении inline visualization links TUI обязана доказать, что sandboxed
session не может записать viewer cache или его родительский каталог. Для этого
`InlineVisualizationContext::from_config` получает writable roots и проверяет
несколько candidate paths через действующую `FileSystemSandboxPolicy`.

Restricted policy может содержать несколько writable roots и более узкие
read-only carveouts. Без snapshot составные access checks повторно разрешают и
нормализуют весь набор `entries`, а постраничная hydration истории может заново
создавать тот же visualization context. Стоимость такого пути растёт
сверхлинейно и делает `resume` больших сессий заметно медленнее при permission
profile с расширенной filesystem policy.

При восстановлении длинной paginated-сессии начальная hydration и последующее
воспроизведение истории независимо оценивают заполненность row budget. Если они
используют разную ширину или разные единицы подсчёта, hydration может завершиться
как достаточная, а TUI сразу после неё обнаружит недозаполненный scrollback и
запросит ещё одну страницу.

Каждая автоматически полученная страница сейчас может немедленно назначить
полную очистку терминала и повторное воспроизведение transcript, ещё до решения о
следующей странице. Сам `reflow` необходим, потому что терминал не позволяет
вставить старые строки в начало уже напечатанного scrollback. Промежуточный
`reflow` после каждой страницы не нужен: он увеличивает время запуска и делает
последнее сообщение заметно мигающим.

Повторный `resume` в том же TUI может случайно скрыть дефект, потому что общий
`transcript_cells` уже содержит строки ранее показанных сессий. Решение о
догрузке истории текущего thread не должно зависеть от такого накопленного
состояния: холодный и горячий пути обязаны одинаково определять достаточность
истории и различаться только допустимым кэшированием внутренних данных.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/protocol/src/permissions.rs` | Владеет временным snapshot `restricted` policy, пакетной проверкой путей, построением writable roots без повторного разрешения `entries` и test-only счётчиком полных проходов |
| `codex-rs/tui/src/inline_visualization.rs` | Проверяет кэши viewers и их родительские каталоги одним пакетным запросом, сохраняя fail-closed поведение |
| `codex-rs/tui/src/app_server_session/history.rs` | Вычисляет visualization context один раз на начальную hydration и ограничивает постраничную загрузку истории согласованным row budget |
| `codex-rs/tui/src/thread_transcript.rs` | Передаёт готовый context в преобразование transcript и сохраняет обычный entrypoint для остальных callers |
| `codex-rs/tui/src/app/resize_reflow.rs` | Владеет `InitialHistoryReplayBuffer`, очисткой терминала и итоговым `reflow` transcript |
| `codex-rs/tui/src/app/history_row_budget.rs` | Считает физические строки, применяет row cap и отделяет строки текущего resumed thread от общего transcript |
| `codex-rs/tui/src/app/history_pagination.rs` | Догружает старые страницы, различает автоматическое заполнение scrollback и явную загрузку из transcript overlay |
| `codex-rs/tui/src/app/resize_reflow_tests.rs` | Проверяет row budget, финальный transcript и пользовательски видимый результат единственного `reflow` |
| `codex-rs/tui/src/app/tests/session_lifecycle_requests.rs` | Проверяет единственное построение visualization context, многостраничную догрузку и одинаковый холодный и горячий `resume` |
| `docs/fork/resume-profile-startup-latency.md` | Владеющий документ передачи доработки |

## Итоговый контракт

### Восстановление истории

- `resume` полностью восстанавливает предусмотренную row budget часть истории и
  не подменяет сессию новым thread.
- В пределах одного `hydrate_initial_thread_history` значение
  `InlineVisualizationContext::from_config(config, thread_id)` вычисляется не
  более одного раза и передаётся всем загруженным item pages.
- Результат `None` также переиспользуется до завершения текущей hydration, чтобы
  отсутствие безопасного context не запускало повторную policy-проверку.
- Context не сохраняется в `App`, `ChatWidget`, static state или другом
  долгоживущем кэше. Следующая hydration заново учитывает `thread_id`, `cwd`,
  `codex_home` и filesystem policy.
- Callers вне initial hydration сохраняют обычный transcript entrypoint, который
  самостоятельно строит context для своего единичного преобразования.

### Пагинация и отрисовка TUI

- Начальная hydration, `InitialHistoryReplayBuffer` и итоговый `reflow`
  используют одну семантику row budget с фактической шириной истории и
  действующим режимом отображения.
- Решение о достаточности загруженной истории принимается по строкам текущего
  resumed thread. Строки других сессий, уже находящиеся в общем
  `transcript_cells`, не могут подавлять или инициировать его автоматическую
  догрузку.
- Автоматическая догрузка может получить несколько страниц старой истории, но
  не показывает их как последовательность промежуточных полных перерисовок.
- Успешное начальное восстановление выполняет не более одного полного очищения и
  повторного воспроизведения терминальной истории после достижения row budget,
  исчерпания cursor либо действующего защитного ограничения на сканирование.
- Ошибка или отмена автоматической догрузки сохраняет уже собранную корректную
  часть истории и завершает её отображение не более чем одним итоговым `reflow`;
  TUI не остаётся с пустым или частично очищенным экраном.
- Явная загрузка старой истории пользователем из transcript overlay сохраняет
  немедленное отображение страницы. Обычный `reflow` после изменения ширины
  терминала также не откладывается до будущей pagination.
- Общий терминальный scrollback между последовательными сессиями сохраняется.
  Эта история может участвовать в итоговом воспроизведении, но не в row budget
  текущего resumed thread.

### Snapshot filesystem policy

- Restricted policy разрешает entries относительно одного `cwd` один раз на
  составную операцию. Snapshot не переживает вызов и не используется после
  изменения policy или `cwd`.
- Публичные одиночные `resolve_access_with_cwd`, `can_read_path_with_cwd` и
  `can_write_path_with_cwd` используют upstream `PathUri` и
  `FileSystemSandboxPolicyContext` и сохраняют поведение 0.152.0.
- Snapshot сохраняет исходный `resolved_entry_precedence`: наиболее специфичный
  path имеет приоритет, а access mode сохраняет прежний tie-breaking.
- `get_writable_roots_with_cwd` использует один snapshot для writable entries,
  non-write carveouts и protected metadata.
- Effective writable roots группируются после одной нормализации каждого raw
  path. Raw aliases сохраняются рядом с effective root, чтобы symlink carveouts
  продолжали маскировать пользовательский path, а не только resolved target.
- `get_writable_roots_with_cwd_preserving_mutable_paths` использует тот же
  snapshot, но применяет `PreserveMutableComponents`: trusted top-level aliases
  нормализуются, а более глубокие mutable components остаются в исходном виде.
- Non-write paths и `cwd` обрабатываются выбранной
  `WritableRootPathResolution` до root loop. Построение каждого `WritableRoot`
  не повторяет filesystem probes для всего набора.
- Защита `.git`, `.agents` и `.codex`, explicit write overrides, missing-path
  behavior и полный disk access сохраняют прежнюю семантику.

### Batch-проверка viewer caches

- `can_write_any_path_with_cwd` возвращает `true`, если policy может записать
  хотя бы один candidate path, и `false` для пустого набора.
- Restricted policy проверяет весь набор одним локальным snapshot и применяет к
  каждому path защиту workspace metadata. Для unrestricted и external sandbox
  любой непустой набор считается writable.
- TUI одним batch-запросом проверяет оба viewer caches и их parents. Отдельная
  проверка пересечения с materialized `WritableRoot` остаётся обязательной.
- Если хотя бы один viewer cache, его parent или пересекающийся root доступен для
  записи sandboxed session, `InlineVisualizationContext::from_config` возвращает
  `None` и visualization link не создаётся.
- Full-disk-write policy по-прежнему отключает inline visualization context до
  materialization viewer document.

## Архитектурное решение

`ResolvedRestrictedFileSystemPolicy` является приватным временным представлением
`FileSystemSandboxPolicy` для составных операций с путями. Он хранит resolved
`entries`, `cwd`, признак full disk write и один раз подготовленный порядок
индексов по precedence. Пакетные проверки и построение writable roots после этого
не перечитывают исходную policy и не повторяют разрешение путей в filesystem.

Upstream-модель 0.153.0 на `PathUri` и `FileSystemSandboxPolicyContext` остаётся
источником семантики одиночных access checks, metadata denial и deny-read globs.
Fork-snapshot не заменяет этот API и не вмешивается в unrelated permission
semantics.

Оба entrypoint построения writable roots используют тот же snapshot на всём
пути. Writable aliases группируются по root, полученному через выбранный
`WritableRootPathResolution`, а соответствующие non-write entries вычисляются
один раз независимо от числа roots. Protected metadata helpers принимают
resolved entries либо snapshot напрямую.

Batch-метод принадлежит `FileSystemSandboxPolicy`, потому что только policy
может сохранить одинаковую семантику для restricted, unrestricted и external
sandbox. Он не является долгоживущим кэшем и не раскрывает внутренний resolved
тип за границу crate.

`hydrate_initial_thread_history` создаёт context до page loop и передаёт
`Option<&InlineVisualizationContext>` в
`thread_items_to_transcript_cells_with_context`. Transcript cells получают
собственный clone context, как и до доработки. Обычный
`thread_items_to_transcript_cells` остаётся совместимым wrapper для callers,
которые не управляют временем жизни context.

Начальное заполнение scrollback является одной операцией представления истории.
Её состояние отдельно хранит объём строк текущего resumed thread, следующий
cursor и причину загрузки страницы. Автоматическая догрузка при начальном
восстановлении и явная загрузка из transcript overlay не кодируются неочевидным
позиционным флагом: режим должен быть выражен отдельным типом или разными
entrypoint.

Подсчёт строк выполняется общим путём отображения с той же эффективной шириной,
режимом `Rich`/`Raw`, переносами, разделителями, продолжениями потока и высотой
локальных изображений, которые будут применены при повторном воспроизведении в
терминале. Независимое чтение ширины терминала и сравнение физической высоты с
числом логических `HistoryCellDisplayItem` не могут быть двумя сторонами одного
row budget.

Обработчик автоматически загруженной страницы старой истории сначала объединяет
страницу, проверяет row budget и при необходимости запрашивает следующую. Полный
`schedule_immediate_resize_reflow` назначается только при завершении всей
операции. Путь transcript overlay сохраняет существующую немедленную
перерисовку, потому что там каждая страница является отдельным явным действием
пользователя.

## Порядок повторения при переносе

1. Проверить, не реализует ли новый upstream гарантии карточки; если
   реализует, использовать upstream-механизм вместо восстановления fork-diff.
2. Сохранить upstream `PathUri`-реализацию одиночных access checks, metadata
   denial и deny-read globs.
3. Для составных операций с путями перенести временный snapshot `restricted`
   policy, `resolved_entry_precedence` и обе стратегии
   `WritableRootPathResolution`.
4. Сохранить raw symlink aliases, protected metadata, explicit write overrides,
   missing-path behavior и unrestricted/external policy semantics.
5. Проверять кэши viewers и их родительские каталоги одним пакетным запросом, не
   раскрывая resolved policy type за границу `codex-protocol`.
6. Вычислять `InlineVisualizationContext` до page loop и передавать
   `Option<&InlineVisualizationContext>` в общий transcript conversion path.
7. Перенести card-level regression tests для одного policy-resolution pass и
   одного context build на многостраничную hydration.
8. Согласовать подсчёт row budget в initial hydration,
   `InitialHistoryReplayBuffer` и `reflow` терминальной истории с фактической
   шириной и режимом отображения.
9. Отделить строки текущего resumed thread от ранее накопленного общего
   `transcript_cells` при решении об автоматической догрузке.
10. Догружать автоматические страницы старой истории до row budget, конца cursor
    или защитного ограничения и только затем назначать один итоговый `reflow`.
11. Сохранить немедленное отображение явно запрошенной страницы из transcript
    overlay и обычный resize-reflow вне начального восстановления.
12. Перенести регрессионные и snapshot-проверки холодного и горячего `resume`,
    многостраничной догрузки и единственной итоговой перерисовки.

## Проверки

Данные card-level запусков для `fork tests`:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "filesystem policy snapshot строится один раз для effective и preserve-mutable roots и сохраняет precedence, symlink carveouts и protected metadata",
      "argv": ["just", "test", "-p", "codex-protocol"]
    },
    {
      "purpose": "initial hydration один раз вычисляет visualization context, а холодный и горячий resume пакетно догружают историю и выполняют один итоговый reflow",
      "argv": ["just", "test", "-p", "codex-tui"]
    }
  ]
}
```

`writable_roots_resolve_policy_entries_once_for_many_nested_checks` вызывает оба
entrypoint построения writable roots для наборов из одной и 32 пар `writable
root + read-only carveout`. Test-only thread-local счётчик требует ровно один
полный проход `resolved_entries_with_cwd` для каждой комбинации entrypoint и
размера policy и сравнивает все материализованные carveouts.

`initial_history_hydration_builds_visualization_context_once_across_pages` и
`initial_history_hydration_caches_missing_visualization_context_across_pages`
проверяют один вызов `InlineVisualizationContext::from_config` для доступного и
отсутствующего context при нескольких item pages.

Регрессия TUI подаёт одну и ту же paginated-историю в новый `App` и в `App` с
ранее накопленными строками transcript. Оба сценария обязаны одинаково решить,
нужны ли дополнительные страницы текущего thread, не назначать промежуточный
полный `reflow` и завершиться единственной перерисовкой. Отдельная проверка
сохраняет немедленную явную загрузку из transcript overlay, а snapshot фиксирует
окончательный вид истории. Проверка не опирается на реальное время выполнения.

## Риски и ограничения

- Snapshot отражает filesystem и policy только на время текущего вызова. Его
  нельзя сохранять между составными операциями или изменениями `cwd`.
- Нормализация может наблюдать изменение symlink между независимыми вызовами;
  внутри одного вызова намеренно используется единый согласованный результат.
- Batch API устраняет повторную обработку только внутри переданного набора.
  Несвязанные одиночные access checks по-прежнему самостоятельно разрешают
  policy entries.
- Snapshot устраняет повторные path resolution и filesystem probes, но lookup по
  precedence и группировка большого числа уникальных roots всё ещё могут
  выполнять сверхлинейное число сравнений в памяти.
- Новые viewer cache paths должны добавляться в существующий batch. Возврат к
  отдельным `can_write_path_with_cwd` внутри цикла способен восстановить
  сверхлинейную задержку.
- Пакетная автоматическая pagination не должна обходить действующий row/item
  budget: устранение мигания не разрешает безгранично загружать длинную сессию.
- Row budget должен учитывать переносы, пустые разделители, продолжения потока и
  высоту локальных изображений. Подсчёт только элементов или только текстовых
  строк снова создаст расхождение между hydration и фактическим воспроизведением
  в терминале.
- Общий transcript между сессиями остаётся частью пользовательского scrollback,
  но не является доказательством полноты истории текущего thread.
- Ошибка промежуточной страницы не должна приводить ни к повторным очисткам, ни
  к потере уже загруженной корректной части истории.
