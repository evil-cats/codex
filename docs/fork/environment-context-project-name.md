---
id: fork-environment-context-project-name
status: active
created: 2026-06-08
updated: 2026-08-27
---

# Environment context: `project_name`

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая добавляет видимое модели
имя проекта в `<environment_context>` через тег `<project_name>`.

## Зачем это нужно

TUI уже умеет показывать `project-name` в поверхностях статуса, но
`status_line` и terminal title являются только интерфейсными поверхностями. Они
помогают человеку отличать окна и сессии, однако не попадают в модельный
контекст.

Для Hermione нужна та же смысловая метка внутри `<environment_context>`, чтобы
модель видела имя текущего проекта как отдельный ограниченный фрагмент, а не
выводила его косвенно из `cwd` или длинного списка `workspace_roots`.

Важно различать две поверхности:

- `status_line`/terminal title - интерфейсная подсказка для пользователя;
- `<environment_context>` - видимый модели контекст, который отправляется
  модели.

Эта доработка добавляет только вторую поверхность и не меняет отрисовку TUI.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/core/src/context/world_state/environment.rs` | Добавляет поле `project_name`, вычисление из workspace roots, рендеринг, snapshot и поведение при `diff`/replay в текущей upstream-модели `EnvironmentsState` |
| `codex-rs/core/src/context/environment_context.rs` | Сохраняет общие helper-типы `FileSystemContext`, `NetworkContext` и XML escaping, которые использует `world_state::environment` |
| `codex-rs/core/src/context/world_state/environment_render_tests.rs` | Проверяет XML escaping, выбор рабочего корня, восстановление старого `TurnContextItem` через резервный `cwd` и diff при смене проекта |
| `docs/fork/environment-context-project-name.md` | Описывает fork-доработку, контракт и порядок повторения |

Файлы, которые намеренно не меняются:

| Файл или зона | Почему не меняется |
| --- | --- |
| `codex-rs/protocol` и `TurnContextItem` schema | `project_name` выводится из уже сохраненных `workspace_roots`; новое поле protocol не требуется |
| Код TUI-поверхностей статуса | `environment_context` живет в `codex-core` и должен работать вне TUI |
| Config schema | Новый config key не добавляется |
| App-server protocol | Внешняя поверхность API не меняется |

## Итоговый контракт

1. `EnvironmentsState` получает поле:

   ```rust
   project_name: Option<String>,
   ```

2. `RenderedEnvironments` переносит это поле в рендеримый фрагмент, чтобы полный
   render и diff-render использовали один порядок вывода, не вытесняя upstream-
   флаг `include_primary` для multi-environment представления.
3. `EnvironmentsSnapshot` переносит это поле в сохраненный базовый снимок
   world-state, чтобы значение участвовало в текущей snapshot-модели diff.
4. В отрендеренном `<environment_context>` при наличии значения появляется строка:

   ```xml
   <project_name>codex</project_name>
   ```

5. Тег называется `project_name`, а не `project-name`, потому что соседние
   структурированные поля в `environment_context` уже используют snake_case:
   `current_date`, `workspace_roots`.
6. Для живого `TurnContext` значение берется из первого workspace root основного
   окружения в `TurnEnvironmentSnapshot`. Для обычного запуска в этом checkout
   это дает `codex`; для тестового root `/repo` дает `repo`.
7. `EnvironmentsState::from_turn_context_with_environments(...)` вычисляет
   основное окружение и `workspace_roots` один раз через
   `environments.primary()` и использует один и тот же срез roots для
   `project_name` и `FileSystemContext`.
8. Для текущего diff по world-state значение сохраняется в `EnvironmentsSnapshot`.
   Для совместимого восстановления из `TurnContextItem` отдельное protocol-поле
   не добавляется: значение реконструируется из `workspace_roots`; если старый
   rollout не содержит `workspace_roots`, `workspace_roots_from_turn_context_item(...)`
   использует `cwd` как резервный источник.
9. Если `workspace_roots` пустой, `project_name` отсутствует.
10. Если у пути нет имени последнего компонента, резервное значение - строковое
    представление пути через `inferred_native_path_string()`.
11. Если имя содержит символы, требующие XML-escaping, рендеринг использует
     `push_optional_element(...)` и `push_xml_escaped_text(...)`, поэтому
     `repo & docs` превращается в
     `<project_name>repo &amp; docs</project_name>`.
12. В выводе порядок такой:
     - `cwd`/`shell` или выбранные environments;
     - `project_name`;
     - `current_date`;
     - `timezone`;
     - `network`;
     - `filesystem`;
     - `subagents`.
13. `WorldStateSection::snapshot(...)` и `WorldStateSection::render_diff(...)`
    считают смену `project_name` значимым изменением значений контекста
    world-state.
14. Если project name изменился, diff содержит новое значение.
15. Если project name не изменился, но другое значение turn context изменилось,
     тело diff остается самодостаточным и содержит текущее вычисленное значение
     `project_name`, как для `network` и `filesystem`.
16. Доработка не должна вызывать код status line TUI из `codex-core`.

## Архитектурное решение

### Почему источник - основное `TurnEnvironment`

`status_line` визуально показывает project name через TUI-поверхности статуса, но
эта логика находится выше `codex-core`. Привязывать `environment_context` к TUI
нельзя: core используется не только в terminal UI, а видимый модели context
должен собираться в одном месте независимо от интерфейса.

Живые рабочие корни принадлежат объектам `TurnEnvironment`, выбранным для хода.
`TurnEnvironmentSnapshot::primary()` даёт основное готовое окружение, а его
`TurnEnvironment::workspace_roots()` уже используется рядом для
`FileSystemContext`, поэтому тот же срез корней подходит как источник имени
проекта.

### Почему берется первый workspace root

`environment_context` уже отображает все workspace roots в `filesystem`, но
короткое имя проекта должно быть одной меткой. Первый effective root является
основным рабочим корнем текущей сессии и соответствует тому, что пользователь
ожидает от `project-name` в обычном checkout с одним root.

Для multi-root окружения это осознанное упрощение: полный список roots остается
в `filesystem`, а `project_name` дает короткую опорную метку.

### Почему нет нового protocol field

`TurnContextItem` уже содержит `workspace_roots`. Добавлять
`project_name: Option<String>` означало бы расширять сохраненную форму protocol
ради значения, которое можно стабильно вывести из уже сохраненных данных.

Это также сохраняет совместимость со старыми rollouts: при replay достаточно
взять `workspace_roots`, а если их нет, существующая логика резервного перехода
на `cwd` сохраняет работоспособность.

## Порядок повторения при переносе

### 1. Расширить `EnvironmentsState`

В `codex-rs/core/src/context/world_state/environment.rs` добавить поле рядом с
другими turn-context values:

```rust
project_name: Option<String>,
```

То же поле добавить в `RenderedEnvironments` и `EnvironmentsSnapshot`, чтобы
полный render, diff-render и сохраненный базовый снимок world-state использовали
один набор значений. Сохранить upstream-поле `RenderedEnvironments::include_primary`:
оно независимо управляет атрибутом `primary` при рендеринге нескольких окружений.

### 2. Вычислить имя проекта из workspace roots

Добавить helper в `world_state/environment.rs`:

```rust
fn project_name_from_workspace_roots(workspace_roots: &[PathUri]) -> Option<String> {
    workspace_roots.first().map(|root| {
        root.basename()
            .unwrap_or_else(|| root.inferred_native_path_string())
    })
}
```

Семантика helper:

- `[]` -> `None`;
- `["/home/slader/Projects/evilcats/codex"]` -> `Some("codex")`;
- путь без имени последнего компонента -> полный путь строкой.

### 3. Обновить создание из живого `TurnContext`

В `EnvironmentsState::from_turn_context_with_environments(...)` получить roots
основного готового окружения:

```rust
let workspace_roots = environments
    .primary()
    .map(TurnEnvironment::workspace_roots)
    .unwrap_or_default();
```

Затем выставить:

```rust
project_name: project_name_from_workspace_roots(workspace_roots),
```

Тот же `workspace_roots` передать в `FileSystemContext::from_permission_profile`.
Это важно: `project_name` и `filesystem` должны строиться из одного снимка roots
основного окружения.

### 4. Обновить восстановление из `TurnContextItem`

В `EnvironmentsState::from_turn_context_item(...)` восстановить roots и один раз
преобразовать прежние `AbsolutePathBuf` в текущий тип `PathUri`:

```rust
let workspace_roots = workspace_roots_from_turn_context_item(turn_context_item)
    .iter()
    .map(PathUri::from_abs_path)
    .collect::<Vec<_>>();
```

Использовать этот же вектор для:

```rust
project_name: project_name_from_workspace_roots(&workspace_roots),
```

и для восстановления filesystem:

```rust
filesystem: Some(FileSystemContext::from_permission_profile(
    &turn_context_item.permission_profile(),
    &workspace_roots,
)),
```

Так `project_name` и `filesystem` остаются согласованными при replay старых и
новых rollouts, несмотря на переход живого environment-контекста на `PathUri`.

### 5. Обновить snapshot и diff

В `WorldStateSection::snapshot(...)` добавить `project_name` в
`EnvironmentsSnapshot`:

```rust
project_name: self.project_name.clone(),
```

В `WorldStateSection::render_diff(...)` добавить `project_name` в сравнение
значений контекста world-state через текущую upstream-модель
`PreviousSectionState`:

```rust
let turn_context_values_changed = current.project_name != previous.project_name
    || current.current_date != previous.current_date
    || current.timezone != previous.timezone
    || current.network != previous.network
    || current.filesystem != previous.filesystem;
```

Передать `self.project_name.clone()` в `RenderedEnvironments`.

Так diff ведет себя как остальные вычисляемые части:

- если имя проекта изменилось, модель получает новое значение;
- если имя проекта не изменилось, но изменилось другое значение turn context,
  тело обновления контекста остается самодостаточным и содержит текущее
  вычисленное значение.

### 6. Обновить рендеринг

В `RenderedEnvironments::body(...)` после блока environments и до `current_date`
добавить:

```rust
push_optional_element(&mut rendered, "project_name", self.project_name.as_deref());
```

Не использовать ручной `format!("<project_name>{project_name}</project_name>")`,
потому что значение должно проходить XML-escaping так же, как другие текстовые
элементы.

### 7. Сохранить текущую границу world-state

Рендеринг `<environment_context>`, видимого модели, находится в
`world_state/environment.rs`, а `environment_context.rs` остаётся владельцем
общих `FileSystemContext`, `NetworkContext` и XML helper-функций.

При разрешении конфликта нельзя возвращать старую структуру `EnvironmentContext`
как основной путь рендера и нельзя возвращать старую сигнатуру
`render_diff(&self, Option<&Self>)`: текущий upstream-контракт использует
`WorldStateSection::snapshot(...)`, `EnvironmentsSnapshot` и
`PreviousSectionState`.

## Проверки

Исполняемая карта card-level regression tests:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "project name в полном environment context, replay, snapshot и diff",
      "argv": ["just", "test", "-p", "codex-core", "environment_context"]
    }
  ]
}
```

Фильтр `environment_context` включает сценарий
`turn_context_item_without_workspace_roots_uses_cwd_for_environment_context`.
Он сравнивает полный видимый модели результат рендеринга старого
`TurnContextItem` без `workspace_roots`: последний компонент резервного `cwd`
становится `project_name`, а сам путь — единственным рабочим корнем файлового
контекста.

Новый тест принят статической вычиткой. Его компиляция, форматирование и запуск
отложены до общего прохода по карточкам.

Тестовые экземпляры `TurnContextItem` в
`codex-rs/core/src/context/world_state/environment_render_tests.rs` должны явно
задавать `active_permission_profile: None`. Так сценарии `project_name` не
включают именованный профиль разрешений и проверяют только принадлежащий
карточке контракт.

## Риски и ограничения

### Ограничения и граничные случаи

- `project_name` является подсказкой, а не источником прав доступа. Реальные
  writable/read-only roots по-прежнему задаются в `filesystem`.
- Multi-root сессия получает имя первого effective root. Остальные roots
  остаются видимыми в `<filesystem><workspace_roots>...`.
- Значение не должно читаться из terminal title, status line или TUI preview
  cache.
- Значение не должно добавляться в config, потому что оно вычисляется из
  текущего workspace.
- При старых rollouts без `workspace_roots` резервный источник - `cwd`, как и
  раньше для восстановления filesystem.
- XML escaping обязателен. Нельзя рендерить значение через сырую интерполяцию.
- Не менять написание тега на `<project-name>`: это нарушит стиль соседних
  структурированных полей.

### Риски

- Если будущий рефакторинг изменит выбор основного `TurnEnvironment` или
  `TurnEnvironment::workspace_roots()`, нужно проверить, что `project_name` и
  `filesystem.workspace_roots` по-прежнему строятся из одного снимка roots.
- Если будет добавлен явный `project_name` в protocol, нужно решить приоритет:
  сохраненное явное значение или значение, вычисленное из roots. В текущей
  версии такого поля нет.
- Если появится multi-root UI с отдельным display name, нельзя автоматически
  переносить его в `environment_context` без проверки владения на уровне core.
- Если `WorldStateSection::render_diff(...)` начнет исключать неизменившиеся
  значения turn context, нужно отдельно проверить, должен ли `project_name`
  оставаться в теле обновления.
