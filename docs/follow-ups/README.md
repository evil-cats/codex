# Отложенные работы

## Активные

| ID | Статус | Тема | Когда вернуться | Связи |
| --- | --- | --- | --- | --- |
| [follow-up:FU-2026-001] | accepted | Re-emit bitmap previews during resize/reflow and replay | перед расширением image history beyond normal insertion | [plan:PLAN-TUI-ASSISTANT-IMAGES-001], [feature:tui-history-image-previews] |
| [follow-up:FU-2026-002] | accepted | Controlled assistant/tool image source path | при старте source-backed assistant/tool image previews | [stage:PLAN-TUI-ASSISTANT-IMAGES-001:002], [feature:tui-history-image-previews] |
| [follow-up:FU-2026-003] | accepted | TUI local-link labels for documentation tables | после правил doc-refactor и перед массовым переписыванием таблиц | [code:markdown-render], [follow-up:FU-2026-004] |
| [follow-up:FU-2026-004] | accepted | Adaptive row separators for long wrapped TUI tables | после doc-refactor и оценки реальных таблиц | [code:markdown-render], [test:long-link-table-rendering] |
| [follow-up:FU-2026-005] | accepted | Autonomous Spark subagents policy | перед изменением `spawn_agent` policy или Hermione multi-agent defaults | [code:multi-agents-spec], [code:multi-agent-config], [code:feature-configs] |

## Архив

| ID | Итог | Архив | Причина закрытия |
| --- | --- | --- | --- |

[code:markdown-render]: ../../codex-rs/tui/src/markdown_render.rs
[code:feature-configs]: ../../codex-rs/features/src/feature_configs.rs
[code:multi-agent-config]: ../../codex-rs/core/src/config/mod.rs
[code:multi-agents-spec]: ../../codex-rs/core/src/tools/handlers/multi_agents_spec.rs
[feature:tui-history-image-previews]: ../architecture/features/tui-history-image-previews.md
[follow-up:FU-2026-001]: FU-2026-001-tui-history-image-reflow-reemit.md
[follow-up:FU-2026-002]: FU-2026-002-tui-assistant-tool-image-source.md
[follow-up:FU-2026-003]: FU-2026-003-tui-local-link-label-rendering.md
[follow-up:FU-2026-004]: FU-2026-004-tui-adaptive-table-row-separators.md
[follow-up:FU-2026-005]: FU-2026-005-autonomous-spark-subagents-policy.md
[plan:PLAN-TUI-ASSISTANT-IMAGES-001]: ../plans/PLAN-TUI-ASSISTANT-IMAGES-001/plan.md
[stage:PLAN-TUI-ASSISTANT-IMAGES-001:002]: ../plans/PLAN-TUI-ASSISTANT-IMAGES-001/stages/002-local-image-history-cell.md
[test:long-link-table-rendering]: ../table-rendering-long-links-test.md
