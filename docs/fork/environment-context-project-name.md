---
id: fork-environment-context-project-name
status: active
created: 2026-06-08
updated: 2026-07-18
source_scope: 67319964b1090368a256b2cb50bc4d4ea44f3630..working-tree
---

# Environment context: `project_name`

## Обзор

Эта карточка фиксирует fork-доработку Hermione, которая добавляет видимое модели
имя проекта в `<environment_context>` через тег `<project_name>`.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Пользовательская цель | Показывать в `environment_context` тот же смысловой `project-name`, который уже можно отображать в `status_line` |
| Видимый модели тег | `<project_name>...</project_name>` |
| Источник значения | первый путь из `Config::effective_workspace_roots()` |
| Формат значения | имя последнего компонента первого workspace root, с резервом в виде полного пути |
| Основной файл | `codex-rs/core/src/context/world_state/environment.rs` |
| Вспомогательный файл | `codex-rs/core/src/context/environment_context.rs` |
| Тестовый файл | `codex-rs/core/src/context/world_state/environment_render_tests.rs` |
| Локальный режим | локально только разработка, поиск и `diff`; Rust-сборка и тесты выполняются на `f-ms-dev` |
| Удаленный checkout | `slader@f-ms-dev:/home/slader/Projects/codex` |

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
| `codex-rs/core/src/context/world_state/environment_render_tests.rs` | Проверяет XML escaping, выбор workspace root и diff при смене проекта |
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
   render и diff-render использовали один порядок вывода.
3. `EnvironmentsSnapshot` переносит это поле в сохраненный базовый снимок
   world-state, чтобы `rust-v0.143.0` сравнивал значение через новую
   snapshot-модель.
4. В отрендеренном `<environment_context>` при наличии значения появляется строка:

   ```xml
   <project_name>codex</project_name>
   ```

5. Тег называется `project_name`, а не `project-name`, потому что соседние
   структурированные поля в `environment_context` уже используют snake_case:
   `current_date`, `workspace_roots`.
6. Для живого `TurnContext` значение берется из первого элемента
   `Config::effective_workspace_roots()`. Для обычного запуска в этом checkout
   это дает `codex`; для тестового root `/repo` дает `repo`.
7. `EnvironmentsState::from_turn_context_with_environments(...)` вычисляет
   `workspace_roots` один раз и использует один и тот же снимок roots для
   `project_name` и `FileSystemContext`.
8. Для текущего diff по world-state значение сохраняется в `EnvironmentsSnapshot`.
   Для совместимого восстановления из `TurnContextItem` отдельное protocol-поле
   не добавляется: значение реконструируется из `workspace_roots`; если старый
   rollout не содержит `workspace_roots`, `workspace_roots_from_turn_context_item(...)`
   использует `cwd` как резервный источник.
9. Если `workspace_roots` пустой, `project_name` отсутствует.
10. Если у пути нет имени последнего компонента, резервное значение - полный путь
   через `to_string_lossy()`.
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

### Почему источник - `effective_workspace_roots`

`status_line` визуально показывает project name через TUI-поверхности статуса, но
эта логика находится выше `codex-core`. Привязывать `environment_context` к TUI
нельзя: core используется не только в terminal UI, а видимый модели context
должен собираться в одном месте независимо от интерфейса.

`Config::effective_workspace_roots()` уже является источником уровня core для
рабочих корней. Он используется рядом для `FileSystemContext`, поэтому тот же
вектор корней подходит как источник имени проекта.

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

## Пошаговое воспроизведение

### 1. Расширить `EnvironmentsState`

В `codex-rs/core/src/context/world_state/environment.rs` добавить поле рядом с
другими turn-context values:

```rust
project_name: Option<String>,
```

То же поле добавить в `RenderedEnvironments` и `EnvironmentsSnapshot`, чтобы
полный render, diff-render и сохраненный базовый снимок world-state использовали
один набор значений.

### 2. Вычислить имя проекта из workspace roots

Добавить helper в `world_state/environment.rs`:

```rust
fn project_name_from_workspace_roots(workspace_roots: &[AbsolutePathBuf]) -> Option<String> {
    workspace_roots.first().map(|root| {
        root.as_path()
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.to_string_lossy().into_owned())
    })
}
```

Семантика helper:

- `[]` -> `None`;
- `["/home/slader/Projects/evilcats/codex"]` -> `Some("codex")`;
- путь без имени последнего компонента -> полный путь строкой.

### 3. Обновить создание из живого `TurnContext`

В `EnvironmentsState::from_turn_context_with_environments(...)` сначала сохранить
`effective_workspace_roots` в локальную переменную:

```rust
let workspace_roots = turn_context.config.effective_workspace_roots();
```

Затем выставить:

```rust
project_name: project_name_from_workspace_roots(&workspace_roots),
```

Тот же `workspace_roots` передать в `FileSystemContext::from_permission_profile`.
Это важно: `project_name` и `filesystem` должны строиться из одного снимка roots,
а не вызывать `effective_workspace_roots()` повторно.

### 4. Обновить восстановление из `TurnContextItem`

В `EnvironmentsState::from_turn_context_item(...)` вычислить roots один раз:

```rust
let workspace_roots = workspace_roots_from_turn_context_item(turn_context_item);
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
новых rollouts.

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

### 7. Разрешить перенос после upstream-реорганизации

В `rust-v0.143.0` рендеринг `<environment_context>`, видимого модели, находится в
`world_state/environment.rs`, а `environment_context.rs` остается владельцем
общих `FileSystemContext`, `NetworkContext` и XML helper-функций.

При разрешении конфликта нельзя возвращать старую структуру `EnvironmentContext`
как основной путь рендера и нельзя возвращать старую сигнатуру
`render_diff(&self, Option<&Self>)`: текущий upstream-контракт использует
`WorldStateSection::snapshot(...)`, `EnvironmentsSnapshot` и
`PreviousSectionState`.

## Проверки

### Смысловое покрытие

Обязательное покрытие этой карточки:

- `serialize_environment_context_with_project_name`:
  - создает `EnvironmentsState` через тестовый helper `environment_state(...)`;
  - вручную ставит `context.project_name = Some("repo & docs".to_string())`;
  - ожидает строку `<project_name>repo &amp; docs</project_name>`;
  - проверяет порядок: после `shell`, перед `current_date`.
- `turn_context_item_project_name_uses_workspace_root_name`:
  - задает `cwd = /repo/nested`;
  - задает `workspace_roots = Some(vec![/repo])`;
  - создает context через `EnvironmentsState::from_turn_context_item(...)`;
  - проверяет, что отрендеренный context содержит
    `<project_name>repo</project_name>`;
  - тем самым подтверждает, что используется workspace root, а не имя последнего
    компонента `cwd`.
- `diff_environment_context_includes_changed_project_name`:
  - создает `before` через `EnvironmentsState::from_turn_context_item(...)` с
    root `/old-repo`;
  - создает `after` из `before.clone()` и меняет `project_name` на
    `Some("new-repo")`;
  - создает предыдущий snapshot через `WorldStateSection::snapshot(&before)`;
  - вызывает `WorldStateSection::render_diff(...)` с
    `PreviousSectionState::Known(&previous)`;
  - проверяет наличие `<project_name>new-repo</project_name>`;
  - проверяет отсутствие `<project_name>old-repo</project_name>`.

Существующие тесты для восстановления `filesystem` остаются важными, потому что
новая логика переиспользует тот же `workspace_roots_from_turn_context_item`.

### Владелец исполняемой карты

Проверки уровня карточки запускает skill-owned команда `fork tests`.
Карточка не является нормативным runbook запуска `just` или `cargo`: внутренние
argv хранятся только как данные для `fork tests` в блоке `fork-tests.v1` ниже.

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "environment context",
      "argv": ["just", "test", "-p", "codex-core", "environment_context"]
    }
  ]
}
```

### Дополнительные gates

- Доработка меняет видимый модели контекст в `codex-core`, поэтому
  регрессионное покрытие уровня карточки принадлежит `fork tests` и блоку
  `fork-tests.v1`.
- Схема `Config`, протокол app-server и внешние API не меняются; отдельные
  проверки генераторов schema/API для этой карточки не требуются.
- TUI-поверхности не меняются; проверка snapshots для `codex-tui` не требуется.
- `fork format`, широкий `fork tests` и `fork build-fast` относятся к общему
  проверочному проходу родительского агента, а не к runbook этой карточки.
- Локальный checkout в историческом описании использовался для разработки,
  чтения и проверки diff; сборка и Rust-тесты в старом workflow выполнялись на
  `f-ms-dev`. Текущие запуски должны проходить через skill-owned workflow, если
  родительский агент решит их выполнять.

### Исторические результаты

Историческая карточка фиксировала следующие команды для повторения. Они
сохранены как подтверждение старого workflow и не являются текущим нормативным
runbook запуска. Пути в этом блоке относятся к первоначальной раскладке до
upstream-переноса рендеринга environment context в `world_state/environment.rs`:

```bash
git diff --check -- \
  codex-rs/core/src/context/environment_context.rs \
  codex-rs/core/src/context/environment_context_tests.rs

rg -n "project_name|effective_workspace_roots|workspace_roots_from_turn_context_item" \
  codex-rs/core/src/context/environment_context.rs \
  codex-rs/core/src/context/environment_context_tests.rs
```

Исторический блок для `f-ms-dev` фиксировал Rust-форматирование, сборку и
тесты на удаленном checkout:

```bash
ssh slader@f-ms-dev 'git -C /home/slader/Projects/codex diff --check -- codex-rs/core/src/context/environment_context.rs codex-rs/core/src/context/environment_context_tests.rs'

ssh slader@f-ms-dev 'cd /home/slader/Projects/codex/codex-rs && just fmt'

ssh slader@f-ms-dev 'cd /home/slader/Projects/codex/codex-rs && just test -p codex-core environment_context'
```

Старый блок дополнительно фиксировал условное действие для `cargo-nextest` на
`f-ms-dev`:

```bash
cargo install --locked cargo-nextest
```

Для полного теста crate старая карточка фиксировала remote test environment:

```bash
ssh slader@f-ms-dev 'cd /home/slader/Projects/codex && scripts/test-remote-env.sh'
```

Старая инструкция также указывала: если полный тест crate нужен для
дополнительной уверенности, использовать remote test environment, затем выполнить
`just test -p codex-core` с переменными из вывода `scripts/test-remote-env.sh`.

Проверки, выполненные для первоначального переноса этой доработки:

| Проверка | Где | Результат |
| --- | --- | --- |
| `git diff --check` для двух измененных core-файлов | локально | passed |
| `git diff --check` для двух измененных core-файлов | `f-ms-dev` | passed |
| SHA256 измененных файлов local vs remote | локально и `f-ms-dev` | совпали |
| `just fmt` | локально и `f-ms-dev` | часть Rust formatter прошла, общая команда завершалась ошибкой из-за отсутствующего `uv` |
| `just test -p codex-core environment_context` | `f-ms-dev` | passed, 22 tests passed |
| `bench-smoke` после узкого теста | `f-ms-dev` | passed |
| `just test -p codex-core` без remote env | `f-ms-dev` | failed из-за не связанной с доработкой тестовой инфраструктуры |
| `just test -p codex-core` с remote env | `f-ms-dev` | failed из-за не связанной с доработкой тестовой инфраструктуры |

SHA256 контрольные суммы измененных файлов:

| Файл | SHA256 |
| --- | --- |
| `codex-rs/core/src/context/environment_context.rs` | `2d0e9c481b5f6caef44254cf743c41b0b2e17a88f964d72e292501a07beb556c` |
| `codex-rs/core/src/context/environment_context_tests.rs` | `25b1120b741406cc08f0f40925049ec22efa11432f78bd4fede5d318dbe6a6bc` |

Причины падения полных тестов crate на `f-ms-dev` были инфраструктурными и не
относились к этой доработке:

- `test_stdio_server` binary не найден;
- `bwrap: loopback: Failed RTM_NEWADDR: Operation not permitted`;
- request-permissions/network-denial remote exec failures.

После запуска remote env очистка проверена командой:

```bash
docker ps --format "{{.Names}}" | grep codex-remote-test-env || true
```

Команда не вернула контейнеров `codex-remote-test-env`.

### Известные падения и пропуски

- Локальная Rust-сборка и локальные Rust-тесты не выполнялись: исторический
  локальный checkout использовался только для разработки, чтения и проверки
  diff.
- Форматирование repo recipe в историческом запуске проходило часть Rust
  formatter, но общий шаг завершался ошибкой из-за отсутствующего `uv`.
- Полный crate-level проход на `f-ms-dev` с remote env и без него падал по
  инфраструктурным причинам, не связанным с этой доработкой:
  - `test_stdio_server` binary не найден;
  - `bwrap: loopback: Failed RTM_NEWADDR: Operation not permitted`;
  - request-permissions/network-denial remote exec failures.
- Установка `cargo-nextest` была зафиксирована в старой карточке только как
  условие для узкого теста при отсутствии инструмента на `f-ms-dev`; это не
  является отдельным текущим gate карточки.

## Ограничения и граничные случаи

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

## Риски

- Если будущий рефакторинг изменит `Config::effective_workspace_roots()`, нужно
  проверить, что `project_name` и `filesystem.workspace_roots` по-прежнему
  строятся из одного снимка roots.
- Если будет добавлен явный `project_name` в protocol, нужно решить приоритет:
  сохраненное явное значение или значение, вычисленное из roots. В текущей
  версии такого поля нет.
- Если появится multi-root UI с отдельным display name, нельзя автоматически
  переносить его в `environment_context` без проверки владения на уровне core.
- Если `WorldStateSection::render_diff(...)` начнет исключать неизменившиеся
  значения turn context, нужно отдельно проверить, должен ли `project_name`
  оставаться в теле обновления.

## Аудит миграции `rust-v0.144.6`

Проверка после слияния `rust-v0.144.6` не потребовала изменений Rust-кода:

| Область | Результат |
| --- | --- |
| Вычисление project name | сохранено: берется имя последнего компонента первого `effective_workspace_roots`, причем `project_name` и filesystem используют один снимок roots |
| Видимый модели контракт | сохранено: `project_name` участвует в полном render, snapshot и diff и выводится как `<project_name>` перед `current_date` |
| Экранирование и резервное значение | сохранено: значение проходит через `push_xml_escaped_text`; для пути без последнего компонента используется полный путь через `to_string_lossy()` |
| Регрессионное покрытие | сохранены `serialize_environment_context_with_project_name`, `turn_context_item_project_name_uses_workspace_root_name` и `diff_environment_context_includes_changed_project_name` |

Тесты, сборка, генераторы и форматирование в one-card проходе не запускались.
Узкий тест из `fork-tests.v1` и общие gates выполняет родительский проверочный
проход.

## Проверка покрытия

| Требование | Статус | Где покрыто |
| --- | --- | --- |
| Добавить видимый модели project name | перенесено | `EnvironmentsState.project_name` и `RenderedEnvironments::body(...)` |
| Использовать тот же смысл, что `status_line` `project-name` | перенесено | имя последнего компонента первого effective workspace root |
| Не связывать core с TUI | перенесено | источник `Config::effective_workspace_roots()` |
| Не расширять protocol без нужды | перенесено | восстановление из `TurnContextItem.workspace_roots` |
| Сохранять XML escaping | перенесено | `push_optional_element`, `push_xml_escaped_text` и test с `repo & docs` |
| Проверить workspace root вместо `cwd` | перенесено | `turn_context_item_project_name_uses_workspace_root_name` |
| Проверить diff при смене проекта | перенесено | `EnvironmentsSnapshot` и `diff_environment_context_includes_changed_project_name` |
| Собирать и тестировать не локально, а на `f-ms-dev` | перенесено | `## Проверки`, исторические результаты и известные пропуски |
