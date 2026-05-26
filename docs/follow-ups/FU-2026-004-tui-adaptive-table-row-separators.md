---
id: FU-2026-004
status: accepted
priority: medium
kind: feature
tags: [tui, markdown, tables, rendering]
created: 2026-05-26
updated: 2026-05-26
review_at: after documentation refactor rules are agreed and documentation table shape is understood
architecture_refs:
  - codex-rs/tui/src/markdown_render.rs
  - codex-rs/tui/src/markdown_render_tests.rs
  - docs/table-rendering-long-links-test.md
invalid_if:
  - documentation tables avoid long wrapped cells or a different terminal Markdown renderer is chosen
---

# FU-2026-004: adaptive row separators в TUI Markdown tables

## Кратко

| Поле | Значение |
| --- | --- |
| Статус | принят (`accepted`) |
| Суть | Добавить adaptive body row separators для длинных wrapped Markdown tables в TUI renderer. |
| Почему важно | Короткие таблицы должны оставаться компактными, но многострочные rows без разделителей плохо читаются. |
| Когда вернуться | После doc-refactor, когда будет понятно, какие таблицы реально остаются и как они wrapping-rendered. |
| Когда закрыть | Если длинные wrapped cells исчезнут, будет выбран внешний renderer или тест покажет, что separators не помогают. |
| Следующий шаг | Оценить реальные таблицы, затем добавить compact и wrapped regression tests. |
| Связи | [code:markdown-render], [code:markdown-render-tests], [test:long-link-table-rendering], [follow-up:FU-2026-003] |

## Наблюдение

Codex TUI сейчас рисует Markdown-таблицы Unicode box-таблицей с верхней
границей, разделителем заголовка и нижней границей. Между body rows
горизонтальных разделителей нет. Для коротких таблиц это компактно и хорошо,
но длинные ячейки с большим количеством ссылок переносятся на несколько
визуальных строк, и становится трудно увидеть границу между логическими
строками таблицы.

## Почему это важно

При рефакторинге документации мы хотим оставлять таблицы там, где они остаются
обзорными, и не терять ссылки или контекст ради line-length lint. Тестовый
документ `docs/table-rendering-long-links-test.md` показал, что широкие таблицы
могут быть приемлемы по ширине, но для wrapped rows нужны визуальные разделители
между логическими строками.

## Что нужно сделать

- После рефакторинга документации оценить реальные таблицы, которые остаются в
  `docs/`.
- Спроектировать адаптивный режим: не добавлять body row separators для мелких
  таблиц, но включать их, если хотя бы одна body cell переносится на несколько
  rendered lines или превышает выбранный порог длины.
- Не делать простой always-on separator, чтобы короткие таблицы не стали
  визуально тяжелее.
- Добавить regression tests в `codex-rs/tui/src/markdown_render_tests.rs`:
  короткая таблица остается compact, длинная/wrapped таблица получает
  horizontal separators между body rows.
- Проверить, что fallback pipe rendering для слишком узкого терминала не ломается.

## Когда вернуться

После согласования и выполнения рефакторинга документации, когда будет понятно,
какие таблицы реально остаются и какие из них рендерятся в терминале как
многострочные.

## Когда закрыть как неактуальное

Если документационные таблицы перестанут иметь длинные wrapped cells, если
будет выбран внешний terminal Markdown renderer с подходящим table style, или
если table rendering test покажет, что separators не улучшают читаемость.

## Связи

- Код: [code:markdown-render]
- Тесты: [code:markdown-render-tests]
- Визуальный тест: [test:long-link-table-rendering]
- Связанный follow-up: [follow-up:FU-2026-003]

[code:markdown-render]: ../../codex-rs/tui/src/markdown_render.rs
[code:markdown-render-tests]: ../../codex-rs/tui/src/markdown_render_tests.rs
[follow-up:FU-2026-003]: FU-2026-003-tui-local-link-label-rendering.md
[test:long-link-table-rendering]: ../table-rendering-long-links-test.md
