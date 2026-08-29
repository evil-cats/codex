---
id: fork-resume-profile-startup-latency
status: active
created: 2026-08-25
updated: 2026-08-28
---

# Ускорение `resume` с расширенной filesystem policy

## Обзор

Карточка владеет fork-доработкой, которая устраняет повторное разрешение и
нормализацию filesystem policy в горячем пути начального восстановления истории
TUI. Доработка переиспользует один `InlineVisualizationContext` в пределах одной
hydration и один snapshot разрешённых policy entries в пределах одной составной
проверки.

Исправление не ослабляет защиту inline visualization viewers, не уменьшает
бюджет восстанавливаемой истории и не отключает свойства выбранного permission
profile.

## Зачем это нужно

При построении inline visualization links TUI обязана доказать, что sandboxed
session не может записать viewer cache или его родительский каталог. Для этого
`InlineVisualizationContext::from_config` получает writable roots и проверяет
несколько candidate paths через действующую `FileSystemSandboxPolicy`.

Restricted policy может содержать несколько writable roots и более узкие
read-only carveouts. Прежний путь повторно разрешал и нормализовал весь набор
entries из вложенных access checks, а постраничное восстановление истории могло
заново создавать тот же visualization context. Стоимость поэтому росла
сверхлинейно и делала `resume` больших сессий заметно медленнее при profile с
расширенной filesystem policy.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/protocol/src/permissions.rs` | Владеет локальным snapshot restricted policy, batch-проверкой candidate paths, построением effective и preserve-mutable writable roots без повторного разрешения entries и test-only счётчиком полных проходов |
| `codex-rs/tui/src/inline_visualization.rs` | Проверяет viewer caches и их parents одним batch-запросом, сохраняя fail-closed поведение |
| `codex-rs/tui/src/app_server_session/history.rs` | Вычисляет visualization context один раз на initial hydration до цикла item pages |
| `codex-rs/tui/src/thread_transcript.rs` | Даёт transcript conversion готовый context и сохраняет обычный entrypoint для остальных callers |
| `codex-rs/tui/src/app/tests/session_lifecycle_requests.rs` | Проверяет единственное построение доступного и отсутствующего visualization context при многостраничной initial hydration |
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
- Access сохраняет исходный `resolved_entry_precedence`: наиболее специфичный
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
- Публичные одиночные `resolve_access_with_cwd`, `can_read_path_with_cwd` и
  `can_write_path_with_cwd` сохраняют прежний результат.

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

`ResolvedRestrictedFileSystemPolicy` является приватным ephemeral view над
`FileSystemSandboxPolicy`. Он хранит resolved entries, `cwd`, признак full disk
write и один раз подготовленный порядок entry indices по precedence. Access
checks после этого не перечитывают исходную policy и не повторяют path URI
resolution.

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

1. Проверить, как новый upstream строит inline visualization context при initial
   history hydration и как `FileSystemSandboxPolicy` формирует writable roots.
2. Если upstream уже переиспользует context на одну hydration и выполняет
   составные access checks через один resolved policy view, использовать его
   реализацию вместо восстановления fork-diff.
3. Сохранить upstream-защиту viewer documents от sandbox writes и paginated
   history hydration; ускорение не должно удалять ни одну из этих гарантий.
4. При необходимости перенести ephemeral restricted-policy snapshot, порядок
   precedence и построение writable roots из предварительно нормализованных
   групп. Сохранить raw symlink aliases и обе стратегии
   `WritableRootPathResolution`.
5. Перенести batch-проверку viewer caches и их parents без раскрытия resolved
   policy type за границу `codex-protocol`.
6. Вычислять `InlineVisualizationContext` до page loop и передавать его по ссылке
   в общий transcript conversion path. Не добавлять кэш с временем жизни дольше
   одной hydration.
7. Проверить protected metadata, explicit overrides, symlink carveouts,
   unrestricted/external policies и неизменность transcript output через
   card-level tests.
8. Сравнить baseline и migrated optimized binaries на одной большой сессии,
   отдельно с profile и без него. Сборка, toolchain, features, row budget и
   порядок повторений должны совпадать; абсолютный порог, зависящий от host, в
   контракт не входит.

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

Тест `writable_roots_resolve_policy_entries_once_for_many_nested_checks`
вызывает настоящие `get_writable_roots_with_cwd` и
`get_writable_roots_with_cwd_preserving_mutable_paths` для наборов из одной и 32
пар `writable root + read-only carveout`. Строго test-only thread-local счётчик
инкрементируется внутри `resolved_entries_with_cwd` и требует ровно один полный
проход для каждой комбинации entrypoint и размера policy. Дополнительно тест
сравнивает все материализованные carveouts. При возврате вложенных одиночных
access checks число полных проходов снова вырастет вместе с набором policy
entries.

Измерение производительности выполняется отдельно от card-level tests и требует
оптимизированных binaries. Baseline и migrated binary собираются одним
toolchain и с одинаковыми features; на обоих поочерёдно повторяется `resume`
одного и того же большого rollout с одинаковым row budget сначала без profile,
затем с `--profile lora`. Для каждого варианта фиксируются hardware, ОС, тип
сборки, toolchain, команда, размер rollout, число повторений и распределение
wall time. Сравнивается не только абсолютное время, но и добавочная стоимость
profile относительно запуска без него. Одиночный smoke подтверждает лишь
работоспособность сценария и не доказывает ускорение.

## Риски и ограничения

- Snapshot отражает filesystem и policy только на время текущего вызова. Его
  нельзя сохранять между hydration operations или изменениями `cwd`.
- Нормализация может наблюдать изменение symlink между независимыми вызовами;
  внутри одного вызова намеренно используется единый согласованный результат.
- Batch API устраняет повторную обработку только внутри переданного набора.
  Несвязанные одиночные access checks по-прежнему создают собственный snapshot.
- Snapshot устраняет повторные path resolution и filesystem probes, но lookup по
  precedence и группировка большого числа уникальных roots всё ещё могут
  выполнять сверхлинейное число сравнений в памяти. Отдельное измерение должно
  показать, существенна ли эта остаточная стоимость для реального profile.
- Автоматические тесты проверяют семантику policy и transcript, но не задают
  переносимый wall-time threshold. После миграции нужно отдельное сравнимое
  измерение baseline и migrated optimized binaries на большой сессии.
- Новые viewer cache paths должны добавляться в существующий batch. Возврат к
  отдельным `can_write_path_with_cwd` внутри цикла способен восстановить
  сверхлинейную задержку.
