---
id: fork-internal-docs-workflow
status: active
created: 2026-06-08
updated: 2026-06-19
source_scope: rust-v0.141.0..hermione-0.141.0
---

# Internal fork docs workflow

## Обзор

Эта карточка фиксирует fork-доработку, которая разрешила внутреннюю
документацию Hermione fork внутри `docs/`, не превращая ее в пользовательскую
документацию upstream Codex.

В текущей миграции на `rust-v0.141.0` обязательные каталоги
`docs/architecture`, `docs/plans`, `docs/follow-ups` и `docs/backlog` не
восстанавливаются. Они остаются историческим контекстом раннего workflow, но не
являются требованием к текущему checkout. Текущие handoff-документы fork
живут в `docs/fork/`; общие служебные документы могут оставаться прямо в
`docs/`, если они описывают fork-specific решения, проверки, заметки
сопровождения, учет работы или handoff.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Основные коммиты документации | `f93bce631`, `0e058cb2c`, `6acebea1f`, `24e05533e`, `43b294f2e`, `314419581`, `4bb9ded75` |
| Поздние docs-коммиты по TUI images | `a041a3820`, `23726d4c6`, `bc35d2615`, `697bad938` |
| Область `docs/` | Без обязательных подкаталогов |
| Текущий handoff-каталог | `docs/fork/` |
| Локальная lint-конфигурация | `docs/.markdownlint-cli2.yaml` |
| Визуальный fixture | `docs/table-rendering-long-links-test.md` |
| Checkpoint перед карточкой | Пропущен по явному разрешению пользователя от 2026-06-08 |

## Зачем это нужно

Правило upstream в `AGENTS.md` запрещало широкую продуктовую и пользовательскую
документацию в `docs/`, потому что официальная документация Codex живет
отдельно. Hermione fork при этом нуждается во внутренней инженерной
документации: решениях, проверках, handoff, заметках сопровождения и учете
работы для fork-доработок.

Доработка уточняет правило: не добавлять широкую документацию продукта
upstream, но разрешить fork-specific внутренние документы в `docs/` без
обязательной привязки к старым каталогам.

## Изменение правила в `AGENTS.md`

Было по смыслу:

```text
Do not add general product or user-facing documentation to the docs folder.
The official Codex documentation lives elsewhere.
```

Стало:

```text
Do not add broad upstream product or user-facing documentation to the `docs/`
folder. The official Codex documentation lives elsewhere. This fork may keep
fork-specific internal development documentation in `docs/` when it records
implementation decisions, verification, maintenance notes, work tracking, or
handoff material. The exception for app-server API documentation is covered by
the app-server guidance below.
```

Контракт:

- широкую документацию продукта upstream для пользователей по-прежнему не добавлять;
- fork-specific internal docs разрешены внутри `docs/`;
- старые каталоги `docs/architecture`, `docs/plans`, `docs/follow-ups` и
  `docs/backlog` не являются обязательными для текущей миграции;
- новые подкаталоги внутри `docs/` можно заводить только при реальной
  необходимости и с понятным владельцем;
- app-server docs exception остаётся отдельным правилом.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `AGENTS.md` | Уточняет границу между upstream product docs и внутренними fork-docs |
| `docs/fork/*.md` | Живые handoff-карточки fork-доработок и migration-карты |
| `docs/migration-one-card-for-agent.md` | Инструкция для подагента, который проверяет одну fork-карточку |
| `docs/.markdownlint-cli2.yaml` | Локальная markdownlint-конфигурация для внутренних документов |
| `docs/table-rendering-long-links-test.md` | Временный визуальный fixture для длинных Markdown-ссылок в таблицах |

## Цепочка коммитов

| Коммит | Смысл |
| --- | --- |
| `f93bce631 Add long table rendering test doc` | Создал визуальный fixture для длинных строк таблиц и множества локальных ссылок |
| `0e058cb2c Add fork documentation workflow docs` | Создал исторические docs-каталоги, markdownlint config, architecture README/card, follow-ups и начальные stages плана |
| `6acebea1f Use Russian labels in fork docs` | Перевел видимые labels, headings и table labels на русский |
| `24e05533e Use English status enums in fork docs` | Сохранил status enum values как английские machine-readable tokens |
| `43b294f2e Link code map paths in architecture docs` | Перевел code map paths в reference-style links на реальные repo files |
| `314419581 Translate feature card prose` | Перевел prose feature-карточки |
| `4bb9ded75 Polish internal docs language` | Финальный проход по русскому техническому языку |
| `a041a3820 Wire view_image into local image history` | Обновил architecture/plan, перенес `FU-2026-002` в архив, добавил stages 003 и 004 |
| `23726d4c6 Preserve TUI image previews during replay` | Перенес `FU-2026-001` и plan в архив, добавил stage 005 и summary |
| `bc35d2615 Anchor Kitty history images in scrollback` | Обновил architecture и добавил `FU-2026-006` |
| `697bad938 Add configurable TUI image preview sizes` | Обновил architecture, перенес `FU-2026-006` в архив, зафиксировал `preview_size` и проверку config/schema |

## Контракт внутренних документов

Текущий контракт не закрепляет обязательные каталоги вроде `docs/architecture`,
`docs/plans`, `docs/follow-ups` или `docs/backlog`. Если такие каталоги
появятся снова, они должны появиться из текущей задачи и с понятным владельцем,
а не потому, что историческая версия этой карточки когда-то называла их
обязательными.

Правила:

- явно отделять внутренние документы fork от пользовательской документации
  upstream Codex;
- хранить handoff по кодовым fork-доработкам в `docs/fork/`;
- для новой кодовой fork-доработки обновлять или создавать owner-карточку в
  `docs/fork/` по правилам `FORK.md`;
- для текущей migration использовать `docs/fork/migration-0.141.0.md` как
  таблицу покрытия, а `docs/migration-one-card-for-agent.md` как инструкцию для
  подагента одной карточки;
- не восстанавливать старые каталоги `docs/architecture`, `docs/plans`,
  `docs/follow-ups` и `docs/backlog` механически;
- при появлении нового документационного каталога сразу определить его назначение,
  индекс, правила обновления и проверки.

### `docs/table-rendering-long-links-test.md`

Назначение: временный визуальный fixture для GitHub и терминального вывода.

Файл намеренно содержит длинные строки Markdown-таблицы с множеством локальных
ссылок. Он поддерживает исторические follow-ups `FU-2026-003` и `FU-2026-004`.

## Правила языка и enum

1. Видимые заголовки и labels написаны по-русски:
   - `Кратко`;
   - `Статус`;
   - `Суть`;
   - `Почему важно`;
   - `Когда вернуться`;
   - `Следующий шаг`;
   - `Связи`.
2. Status/workflow enum values остаются английскими и code-formatted:
   - `implemented`;
   - `archived`;
   - `completed`;
   - `accepted`;
   - `done`;
   - `decided`;
   - `deferred`.
3. Обычная проза написана по-русски.
4. Английский остается только для защищенных технических токенов: путей,
   идентификаторов кода, команд, config keys, enum/status values, link labels,
   API/protocol names.
5. В плотных таблицах предпочтительны reference-style links.
6. Таблицы нельзя сокращать до потери смысла. Локальный markdownlint разрешает
   длинные строки и отключает проверку длины для таблиц, но таблицы все равно
   должны оставаться обзорными.

## Контракт markdownlint

`docs/.markdownlint-cli2.yaml`:

```yaml
globs:
  - "architecture/**/*.md"
  - "plans/**/*.md"
  - "follow-ups/**/*.md"
  - "table-rendering-long-links-test.md"

config:
  MD013:
    line_length: 200
    tables: false
```

Важное поведение, найденное при инвентаризации:

- `markdownlint-cli2 --config docs/.markdownlint-cli2.yaml` сам по себе не
  проверил ни одного файла в одном read-only запуске.
- явные globs проверили 18 Markdown-файлов и нашли существующие lint issues в
  committed docs.
- Для `docs/fork/*.md` работает прямой вызов по пути:

  ```bash
  markdownlint-cli2 --config docs/.markdownlint-cli2.yaml docs/fork/*.md
  ```

## Исторические lint-заметки

Инвентаризация в режиме чтения для исторических docs-каталогов нашла
существующие lint issues в committed fork docs. В текущей миграции на
`rust-v0.141.0` эти пути могут отсутствовать; не восстанавливай их только ради
этой карточки. Заметки ниже - контекст для будущей очистки, если эти каталоги
снова появятся или будут проверяться по старой истории:

- `docs/architecture/features/tui-history-image-previews.md:13`:
  - `MD038/no-space-in-code`;
  - `MD056/table-column-count`;
  - причина: pipe-like enum list внутри ячейки Markdown-таблицы.
- `docs/follow-ups/archive/2026/FU-2026-006-tui-history-image-preview-size.md:33`:
  - `MD056/table-column-count`;
  - тот же класс lint issue.
- `docs/follow-ups/README.md`:
  - unused reference definitions на строках, найденных markdownlint во время
    inventory.
- `docs/plans/README.md`:
  - unused reference definition, найденная markdownlint во время inventory.

Вероятное направление исправления:

- переписать enum lists для `preview_size` в ячейках таблицы как текст через
  запятую;
- удалить или использовать unused reference definitions;
- повторно запустить markdownlint с explicit globs.

Эта карточка не исправляет эти файлы: текущая миграция сохраняет только границу
fork-docs и текущий handoff workflow.

## Пошаговое воспроизведение

### 1. Обновить `AGENTS.md`

Сохранить запрет на upstream product docs, но добавить исключение для
fork-specific внутренних документов в `docs/`.

### 2. Добавить локальную markdownlint-конфигурацию

Создать `docs/.markdownlint-cli2.yaml` с ограниченными globs и настройками
`MD013`.

### 3. Синхронизировать fork handoff-карточки

Для кодовой fork-доработки:

- создать или обновить owner-карточку в `docs/fork/`;
- держать карточку синхронизированной с runtime-поведением, тестами,
  config/schema, prompts и известным статусом проверки;
- для release migrations обновлять соответствующую строку и секцию в
  `docs/fork/migration-X.Y.Z.md`.

### 4. Создавать новые каталоги только при необходимости

Если будущей задаче нужен отдельный каталог для architecture, plans, follow-ups
или backlog, создай его как часть этой задачи и зафиксируй:

- назначение;
- индекс или навигационную запись;
- владельца и правила обновления;
- проверки или lint-маршрут;
- связь с `docs/fork/`.

Не восстанавливай старые каталоги механически во время этой миграции.

### 5. Синхронизировать индексы

После каждого перемещения, архивирования или обновления синхронизируй индекс
или migration-таблицу, которые указывают на измененный документ.

### 6. Сохранять правила языка

Выполнить редакторский проход:

- видимые заголовки и проза написаны по-русски;
- английский остается только для защищенных токенов;
- status enums остаются английскими machine-readable values.

## Проверки

Историческая read-only inventory запускала:

- `git status --short`;
- `git diff --name-status rust-v0.137.0..HEAD -- docs`;
- `git diff --stat rust-v0.137.0..HEAD -- docs`;
- `git diff --check rust-v0.137.0..HEAD -- docs`;
- `markdownlint-cli2 --config docs/.markdownlint-cli2.yaml`;
- explicit markdownlint globs для docs-каталогов.

Для будущего cleanup или воспроизведения:

```bash
git diff --check rust-v0.137.0..HEAD -- docs
markdownlint-cli2 --config docs/.markdownlint-cli2.yaml "docs/architecture/**/*.md" "docs/plans/**/*.md" "docs/follow-ups/**/*.md" "docs/table-rendering-long-links-test.md"
```

Для текущих карточек `docs/fork`:

```bash
markdownlint-cli2 --config docs/.markdownlint-cli2.yaml docs/fork/*.md
git diff --check
```

## Ограничения

- Не добавлять широкую документацию продукта upstream в `docs/`.
- Не считать карточки `docs/fork` историческим committed evidence для
  `rust-v0.137.0..HEAD`.
- Не запускать workflow checker для legacy-каталогов, пока проект намеренно не
  перенесет их в текущую workflow-модель.
- Не пересоздавать `docs/architecture`, `docs/plans`, `docs/follow-ups` или
  `docs/backlog` только потому, что они существовали в старой истории.
- Не утверждать, что markdownlint чист для committed docs, пока не прошли
  explicit globs.

## Риски

- Простой markdownlint-вызов с config может проверить ноль файлов; используй
  explicit globs или прямые пути.
- Таблицы с `|` внутри code spans могут ломать разбор таблиц в markdownlint.
  Предпочитай comma-separated enum lists в ячейках таблицы.
- Reference-style links могут стать unused после архивирования. После
  перемещений запускай lint и link review.
- Исторические docs могут расходиться с текущим checkout. Считай старые
  каталоги историческим контекстом, пока текущая задача не вернет их явно.

## Сводка покрытия

| Пункт | Статус | Где отражено |
| --- | --- | --- |
| Разрешить internal fork docs в `docs/` | перенесено | "Изменение правила в `AGENTS.md`" |
| Убрать обязательность старых каталогов | перенесено | "Контракт внутренних документов" |
| Зафиксировать текущий `docs/fork/` handoff | перенесено | "Контракт внутренних документов" |
| Зафиксировать markdownlint config | перенесено | "Контракт markdownlint" |
| Зафиксировать language/status enum rules | перенесено | "Правила языка и enum" |
| Зафиксировать historical lint notes | перенесено | "Исторические lint-заметки" |
| Зафиксировать repeatable structure | перенесено | "Пошаговое воспроизведение" |
