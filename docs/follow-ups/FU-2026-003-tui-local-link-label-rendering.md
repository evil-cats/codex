---
id: FU-2026-003
status: accepted
priority: medium
kind: feature
tags: [tui, markdown, local-links, documentation]
created: 2026-05-26
updated: 2026-05-26
review_at: после согласования правил рефакторинга документации и перед переписыванием таблиц
architecture_refs:
  - codex-rs/tui/src/markdown_render.rs
invalid_if:
  - таблицы документации перестают использовать local Markdown links в терминальном выводе Codex
---

# FU-2026-003: labels для local links в таблицах документации

## Кратко

| Поле | Значение |
| --- | --- |
| Статус | `accepted` |
| Суть | Сделать рендеринг local Markdown links в TUI настраиваемым, чтобы таблицы документации могли показывать labels, а не только target paths. |
| Почему важно | Reference-style links помогают source Markdown, но текущий rich display в TUI снова расширяет таблицы локальными путями. |
| Когда вернуться | После согласования правил рефакторинга и перед массовым переписыванием таблиц с reference-style links. |
| Когда закрыть | Если таблицы документации перестанут использовать local Markdown links в терминальном выводе Codex. |
| Следующий шаг | Спроектировать настройку display mode без хардкода и сохранить способ увидеть/copy target path. |
| Связи | [code:markdown-render], [follow-ups:image-history] |

## Наблюдение

Codex TUI сейчас рендерит local Markdown links как target path и подавляет
label. Для документационных таблиц это превращает короткие reference-style
links вроде `[feature:tui-history-image-previews]` в длинные локальные пути,
что ухудшает обзорность именно в терминальном просмотре.

## Почему это важно

Мы хотим использовать reference-style links, чтобы исходный Markdown оставался
коротким, grep-friendly и совместимым с `markdownlint`. Если TUI в чате и
terminal-rendered previews всегда показывают target path, таблицы снова
становятся широкими и хуже читаются.

## Что нужно сделать

- Обсудить эту фичу вместе с рефакторингом документации, а не делать сейчас.
- Спроектировать поведение через настройку, чтобы не хардкодить один режим для
  всех local links.
- Сохранить безопасный режим, в котором target path можно увидеть или
  скопировать, даже если обычный rich display показывает label.
- Проверить поведение для обычных local file links, reference-style links,
  таблиц и raw/copy mode.

## Когда вернуться

После согласования правил рефакторинга документации и перед тем, как массово
переписывать таблицы с reference-style links.

## Когда закрыть как неактуальное

Если таблицы документации перестанут использовать local Markdown links в
терминальном выводе Codex или будет выбран внешний просмотрщик, где labels
уже отображаются достаточно хорошо.

## Связи

- Код: [code:markdown-render]
- Контекст: [follow-ups:image-history]

[code:markdown-render]: ../../codex-rs/tui/src/markdown_render.rs
[follow-ups:image-history]: README.md
