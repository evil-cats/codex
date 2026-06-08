---
id: fork-internal-docs-workflow
status: active
created: 2026-06-08
updated: 2026-06-08
source_scope: rust-v0.137.0..HEAD
---

# Internal fork docs workflow

## Обзор

Эта карточка фиксирует fork-доработку, которая разрешила и оформила внутреннюю
документацию Hermione fork внутри `docs/`: architecture cards, plans,
follow-ups, archive summaries, local markdownlint config и fixture для длинных
Markdown table links.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Основные docs commits | `f93bce631`, `0e058cb2c`, `6acebea1f`, `24e05533e`, `43b294f2e`, `314419581`, `4bb9ded75` |
| Later docs updates from TUI images | `a041a3820`, `23726d4c6`, `bc35d2615`, `697bad938` |
| Docs roots | `docs/architecture`, `docs/plans`, `docs/follow-ups` |
| Local lint config | `docs/.markdownlint-cli2.yaml` |
| Fixture | `docs/table-rendering-long-links-test.md` |
| Checkpoint перед карточкой | Пропущен по явному разрешению пользователя от 2026-06-08 |

## Зачем это нужно

Upstream rule in `AGENTS.md` запрещал broad product/user docs in `docs/`,
потому что официальная документация живёт отдельно. Hermione fork при этом
нуждается во внутренней инженерной документации: решения, планы, проверки,
handoff, maintenance notes и follow-ups для fork-specific patches.

Доработка уточняет правило: не добавлять широкую документацию продукта upstream, но
разрешить fork-internal docs под controlled roots.

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
internal development documentation under `docs/architecture/`, `docs/plans/`,
`docs/follow-ups/`, and `docs/backlog/` when it records fork-specific
implementation decisions, verification, maintenance notes, or work tracking.
The exception for app-server API documentation is covered by the app-server
guidance below.
```

Контракт:

- широкую документацию продукта upstream для пользователей по-прежнему не добавлять;
- fork-specific internal docs разрешены;
- owner roots: `docs/architecture`, `docs/plans`, `docs/follow-ups`,
  `docs/backlog`;
- app-server docs exception остаётся отдельным правилом.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `AGENTS.md` | Уточняет boundary для internal fork docs |
| `docs/.markdownlint-cli2.yaml` | Local markdownlint config for internal docs |
| `docs/architecture/README.md` | Index for fork architecture docs |
| `docs/architecture/features/tui-history-image-previews.md` | Architecture owner for TUI image previews feature |
| `docs/plans/README.md` | Index for active and archived plans |
| `docs/plans/archive/2026/PLAN-TUI-ASSISTANT-IMAGES-001/plan.md` | Archived plan owner |
| `docs/plans/archive/2026/PLAN-TUI-ASSISTANT-IMAGES-001/stages/*.md` | Completed implementation stages |
| `docs/plans/archive/2026/PLAN-TUI-ASSISTANT-IMAGES-001/summary.md` | Plan closure summary |
| `docs/follow-ups/README.md` | Index for active and archived follow-ups |
| `docs/follow-ups/FU-2026-003-tui-local-link-label-rendering.md` | Active follow-up for link labels |
| `docs/follow-ups/FU-2026-004-tui-adaptive-table-row-separators.md` | Active follow-up for long table rendering |
| `docs/follow-ups/FU-2026-005-autonomous-spark-subagents-policy.md` | Active follow-up for Spark subagent policy |
| `docs/follow-ups/archive/2026/FU-2026-001-tui-history-image-reflow-reemit.md` | Archived implemented replay/reflow follow-up |
| `docs/follow-ups/archive/2026/FU-2026-002-tui-assistant-tool-image-source.md` | Archived implemented controlled source follow-up |
| `docs/follow-ups/archive/2026/FU-2026-006-tui-history-image-preview-size.md` | Archived implemented preview size follow-up |
| `docs/table-rendering-long-links-test.md` | Temporary visual fixture for long Markdown table links |

## Commit chain

| Commit | Смысл |
| --- | --- |
| `f93bce631 Add long table rendering test doc` | Создал visual fixture for long table rows and many local links |
| `0e058cb2c Add fork documentation workflow docs` | Создал docs roots, markdownlint config, architecture README/card, follow-ups, initial plan stages |
| `6acebea1f Use Russian labels in fork docs` | Перевёл visible labels/headings/table labels на русский |
| `24e05533e Use English status enums in fork docs` | Сохранил status enum values как English machine-readable tokens |
| `43b294f2e Link code map paths in architecture docs` | Перевёл code map paths в reference-style links на реальные repo files |
| `314419581 Translate feature card prose` | Перевёл feature card prose |
| `4bb9ded75 Polish internal docs language` | Финальный проход по русскому техническому языку |
| `a041a3820 Wire view_image into local image history` | Обновил architecture/plan, moved `FU-2026-002` to archive, added stages 003 and 004 |
| `23726d4c6 Preserve TUI image previews during replay` | Archived `FU-2026-001`, moved plan to archive, added stage 005 and summary |
| `bc35d2615 Anchor Kitty history images in scrollback` | Updated architecture, added `FU-2026-006` |
| `697bad938 Add configurable TUI image preview sizes` | Updated architecture, archived `FU-2026-006`, recorded `preview_size` and config/schema verification |

## Docs roots contract

### `docs/architecture`

Назначение: current architecture and implementation contracts for fork-specific
features.

Required shape for feature cards used here:

- `Статус`;
- `Кратко`;
- `Карта деталей`;
- `Назначение`;
- `Текущее устройство`;
- `Карта кода`;
- `Поток`;
- `Контракты`;
- `Инварианты`;
- `Runtime-заметки`;
- `Проверки`;
- `Связи`.

Rules:

- Явно говорить, что документы внутренние для fork, а не являются документацией продукта upstream.
- Link real code paths, tests, plans and follow-ups.
- For flow features, include ASCII map and Mermaid diagram when helpful.
- Keep architecture card as current system truth; plans/follow-ups record work
  route and deferred work.

### `docs/plans`

Назначение: work plans and archived plan summaries.

Current shape:

- `docs/plans/README.md` has `Текущие планы` and `Архив`;
- active table may be empty;
- archive table links completed plan summaries;
- completed plan lives under:

  ```text
  docs/plans/archive/2026/PLAN-TUI-ASSISTANT-IMAGES-001/
  ```

Plan folder contains:

- `plan.md`;
- `stages/001-open-questions-and-mvp.md`;
- `stages/002-local-image-history-cell.md`;
- `stages/003-wire-assistant-image-source.md`;
- `stages/004-wire-image-generation-saved-path.md`;
- `stages/005-replay-resize-and-terminal-verification.md`;
- `summary.md`.

The TUI image plan is `archived`; all stages are `completed`.

### `docs/follow-ups`

Назначение: small deferred work and archive.

Active follow-ups:

- `FU-2026-003`: configurable TUI rendering for local Markdown link labels in
  documentation tables;
- `FU-2026-004`: adaptive body-row separators for long wrapped Markdown tables
  in TUI renderer;
- `FU-2026-005`: policy for autonomous Spark subagents before changing
  `spawn_agent` policy/defaults.

Archived follow-ups:

- `FU-2026-001`: item-level replay/reflow implemented;
- `FU-2026-002`: controlled source path implemented;
- `FU-2026-006`: `preview_size` and config rows implemented.

Follow-up card shape:

- YAML frontmatter:
  - `id`;
  - `status`;
  - `priority`;
  - `kind`;
  - `tags`;
  - `created`;
  - `updated`;
  - `review_at`;
  - `architecture_refs`;
  - `invalid_if`.
- Body:
  - title;
  - `Кратко`;
  - `Наблюдение`;
  - `Почему это важно`;
  - `Что нужно сделать`;
  - return or closure conditions;
  - `Связи`.

### `docs/table-rendering-long-links-test.md`

Назначение: temporary visual test fixture for GitHub and terminal output.

The file intentionally contains long Markdown table rows with many local links.
It supports follow-ups `FU-2026-003` and `FU-2026-004`.

## Language and enum rules

1. Visible headings and labels are Russian:
   - `Кратко`;
   - `Статус`;
   - `Суть`;
   - `Почему важно`;
   - `Когда вернуться`;
   - `Следующий шаг`;
   - `Связи`.
2. Status/workflow enum values remain English and code-formatted:
   - `implemented`;
   - `archived`;
   - `completed`;
   - `accepted`;
   - `done`;
   - `decided`;
   - `deferred`.
3. Normal prose is Russian.
4. English remains only for protected technical tokens: paths, code identifiers,
   commands, config keys, enum/status values, link labels, API/protocol names.
5. Reference-style links are preferred in dense tables.
6. Do not shorten tables until they lose meaning. Local markdownlint allows
   longer lines and disables line-length checks for tables, but tables should
   still be overview-friendly.

## Markdownlint contract

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

Important behavior observed during inventory:

- `markdownlint-cli2 --config docs/.markdownlint-cli2.yaml` by itself linted
  zero files in one read-only run.
- Explicit globs linted 18 Markdown files and found existing committed issues.
- For `docs/FORK/*.md`, direct path invocation works:

  ```bash
  markdownlint-cli2 --config docs/.markdownlint-cli2.yaml docs/FORK/*.md
  ```

## Known existing lint issues in committed docs

Read-only inventory found existing issues in committed fork docs. They are not
introduced by `docs/FORK` cards, but must be preserved as cleanup risk:

- `docs/architecture/features/tui-history-image-previews.md:13`:
  - `MD038/no-space-in-code`;
  - `MD056/table-column-count`;
  - cause: pipe-like enum list inside a Markdown table cell.
- `docs/follow-ups/archive/2026/FU-2026-006-tui-history-image-preview-size.md:33`:
  - `MD056/table-column-count`;
  - same class of issue.
- `docs/follow-ups/README.md`:
  - unused reference definitions at lines found by markdownlint during
    inventory.
- `docs/plans/README.md`:
  - unused reference definition found by markdownlint during inventory.

Likely fix direction:

- rephrase `preview_size` enum lists in table cells as comma-separated text;
- remove or use unused reference definitions;
- rerun markdownlint with explicit globs.

This card does not fix those files because the current user request was to add
`docs/FORK` handoff cards, not cleanup committed docs.

## Пошаговое воспроизведение

### 1. Update `AGENTS.md`

Preserve upstream-docs prohibition, but add internal fork-doc exception for:

- `docs/architecture`;
- `docs/plans`;
- `docs/follow-ups`;
- `docs/backlog`.

### 2. Add local markdownlint config

Create `docs/.markdownlint-cli2.yaml` with scoped globs and `MD013` settings.

### 3. Create architecture root

Create `docs/architecture/README.md`:

- quick ASCII map;
- Mermaid diagram if flow-oriented docs are expected;
- one-minute explanation;
- links to feature cards, plans and follow-ups.

Create current feature owner card under `docs/architecture/features/`.

### 4. Create plans root

Create `docs/plans/README.md` with current and archive tables.

For completed work, archive plan under:

```text
docs/plans/archive/<year>/<PLAN-ID>/
```

Include `plan.md`, stage files and `summary.md`.

### 5. Create follow-ups root

Create `docs/follow-ups/README.md` with active and archive tables.

Create active `FU-*` files for deferred work. Move implemented follow-ups into
`docs/follow-ups/archive/<year>/`.

### 6. Keep indexes synchronized

After every move/archive:

- update `docs/follow-ups/README.md`;
- update `docs/plans/README.md`;
- update architecture links if current owner changes.

### 7. Preserve language rules

Run editorial pass:

- Russian visible headings/prose;
- English only for protected tokens;
- status enums remain English machine-readable values.

## Проверки

Read-only inventory actually ran:

- `git status --short`;
- `git diff --name-status rust-v0.137.0..HEAD -- docs`;
- `git diff --stat rust-v0.137.0..HEAD -- docs`;
- `git diff --check rust-v0.137.0..HEAD -- docs`;
- `markdownlint-cli2 --config docs/.markdownlint-cli2.yaml`;
- explicit markdownlint globs for docs roots.

For future cleanup or reproduction:

```bash
git diff --check rust-v0.137.0..HEAD -- docs
markdownlint-cli2 --config docs/.markdownlint-cli2.yaml "docs/architecture/**/*.md" "docs/plans/**/*.md" "docs/follow-ups/**/*.md" "docs/table-rendering-long-links-test.md"
```

For `docs/FORK` cards:

```bash
markdownlint-cli2 --config docs/.markdownlint-cli2.yaml docs/FORK/*.md
git diff --check
```

## Ограничения

- Не добавлять широкую документацию продукта upstream в `docs/`.
- Do not treat `docs/FORK` untracked cards as historical committed evidence for
  `rust-v0.137.0..HEAD`.
- Do not run workflow checker for these legacy internal roots unless the
  project migrates them to `docs/workflow`; current roots are `docs/architecture`,
  `docs/plans`, `docs/follow-ups`.
- Do not claim markdownlint is clean for committed docs until explicit globs
  pass.

## Риски

- Bare markdownlint invocation with config may lint zero files; use explicit
  globs or direct paths.
- Tables with `|` inside code spans can break markdownlint table parsing. Prefer
  comma-separated enum lists in table cells.
- Reference-style links can become unused after archive moves. Run lint and link
  review after moves.
- Architecture docs can drift from code. For example, TUI image feature docs
  contained test names that did not exist as test functions in committed code.

## Сводка покрытия

| Пункт | Статус | Где отражено |
| --- | --- | --- |
| Разрешить internal fork docs under `docs/` | перенесено | "Изменение правила в `AGENTS.md`" |
| Зафиксировать roots `architecture`, `plans`, `follow-ups` | перенесено | "Docs roots contract" |
| Зафиксировать markdownlint config | перенесено | "Markdownlint contract" |
| Зафиксировать language/status enum rules | перенесено | "Language and enum rules" |
| Зафиксировать existing lint issues | перенесено | "Known existing lint issues in committed docs" |
| Зафиксировать repeatable structure | перенесено | "Пошаговое воспроизведение" |
