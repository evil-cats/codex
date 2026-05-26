# Отложенные работы

## Активные

| ID | Статус | Тема | Когда вернуться | Связи |
| --- | --- | --- | --- | --- |
| [follow-up:FU-2026-003] | `accepted` | Labels для local Markdown links в таблицах документации | после правил рефакторинга документации и перед массовым переписыванием таблиц | [code:markdown-render], [follow-up:FU-2026-004] |
| [follow-up:FU-2026-004] | `accepted` | Адаптивные разделители строк для длинных TUI-таблиц с переносами | после рефакторинга документации и оценки реальных таблиц | [code:markdown-render], [test:long-link-table-rendering] |
| [follow-up:FU-2026-005] | `accepted` | Политика автономных Spark-субагентов | перед изменением `spawn_agent` policy или defaults Hermione multi-agent | [code:multi-agents-spec], [code:multi-agent-config], [code:feature-configs] |

## Архив

| ID | Итог | Архив | Причина закрытия |
| --- | --- | --- | --- |
| [follow-up:FU-2026-001] | Item-level replay/reflow реализован в stage 005 | [archive:FU-2026-001] | Bitmap marker сохраняется до `prepare_history_insert_items` |
| [follow-up:FU-2026-002] | Контролируемый путь источника реализован в stage 002 | [archive:FU-2026-002] | Следующий вызывающий production-код вынесен в stage 003 |

[code:markdown-render]: ../../codex-rs/tui/src/markdown_render.rs
[code:feature-configs]: ../../codex-rs/features/src/feature_configs.rs
[code:multi-agent-config]: ../../codex-rs/core/src/config/mod.rs
[code:multi-agents-spec]: ../../codex-rs/core/src/tools/handlers/multi_agents_spec.rs
[feature:tui-history-image-previews]: ../architecture/features/tui-history-image-previews.md
[archive:FU-2026-001]: archive/2026/FU-2026-001-tui-history-image-reflow-reemit.md
[archive:FU-2026-002]: archive/2026/FU-2026-002-tui-assistant-tool-image-source.md
[follow-up:FU-2026-001]: archive/2026/FU-2026-001-tui-history-image-reflow-reemit.md
[follow-up:FU-2026-002]: archive/2026/FU-2026-002-tui-assistant-tool-image-source.md
[follow-up:FU-2026-003]: FU-2026-003-tui-local-link-label-rendering.md
[follow-up:FU-2026-004]: FU-2026-004-tui-adaptive-table-row-separators.md
[follow-up:FU-2026-005]: FU-2026-005-autonomous-spark-subagents-policy.md
[plan:PLAN-TUI-ASSISTANT-IMAGES-001]: ../plans/archive/2026/PLAN-TUI-ASSISTANT-IMAGES-001/plan.md
[stage:PLAN-TUI-ASSISTANT-IMAGES-001:005]: ../plans/archive/2026/PLAN-TUI-ASSISTANT-IMAGES-001/stages/005-replay-resize-and-terminal-verification.md
[test:long-link-table-rendering]: ../table-rendering-long-links-test.md
