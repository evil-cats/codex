---
id: fork-resume-profile-startup-latency
status: active
created: 2026-08-25
updated: 2026-08-30
---

# Ускорение `resume` с расширенной filesystem policy

## Обзор

Карточка владеет двумя связанными гарантиями производительности начальной
hydration истории: один `InlineVisualizationContext` на hydration и один snapshot
разрешённых `entries` permission profile на составную проверку либо построение
writable roots. Эти гарантии не ослабляют защиту кэшей viewers, не уменьшают row
budget и не меняют свойства выбранного permission profile.

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

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/protocol/src/permissions.rs` | Владеет временным snapshot `restricted` policy, пакетной проверкой путей, построением writable roots без повторного разрешения `entries` и test-only счётчиком полных проходов |
| `codex-rs/tui/src/inline_visualization.rs` | Проверяет кэши viewers и их родительские каталоги одним пакетным запросом, сохраняя fail-closed поведение |
| `codex-rs/tui/src/app_server_session/history.rs` | Вычисляет visualization context один раз на начальную hydration до цикла страниц items |
| `codex-rs/tui/src/thread_transcript.rs` | Передаёт готовый context в преобразование transcript и сохраняет обычный entrypoint для остальных callers |
| `codex-rs/tui/src/app/tests/session_lifecycle_requests.rs` | Проверяет единственное построение доступного и отсутствующего visualization context при многостраничной начальной hydration |
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

### Snapshot filesystem policy

- Restricted policy разрешает entries относительно одного `cwd` один раз на
  составную операцию. Snapshot не переживает вызов и не используется после
  изменения policy или `cwd`.
- Публичные одиночные `resolve_access_with_cwd`, `can_read_path_with_cwd` и
  `can_write_path_with_cwd` используют upstream `PathUri` и
  `FileSystemSandboxPolicyContext` и сохраняют поведение 0.151.0.
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

Upstream-модель 0.151.0 на `PathUri` и `FileSystemSandboxPolicyContext` остаётся
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

## Порядок повторения при переносе

1. Проверить, не реализует ли новый upstream обе гарантии карточки; если
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
      "purpose": "многостраничная initial hydration один раз вычисляет доступный и отсутствующий visualization context",
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
