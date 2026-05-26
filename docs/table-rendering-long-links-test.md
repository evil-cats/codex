# Тест рендера длинных строк таблицы

Документ создан как временный визуальный тест для GitHub и терминального вывода.
Строки таблицы ниже намеренно длинные: последняя колонка содержит много
Markdown-ссылок и в исходнике должна быть около 500 символов или длиннее.

| ID | Сценарий | Ссылки |
|---|---|---|
| ROW-001 | Много ссылок на пользовательскую документацию | [getting-started](getting-started.md), [install](install.md), [config](config.md), [example-config](example-config.md), [authentication](authentication.md), [agents-md](agents_md.md), [skills](skills.md), [slash-commands](slash_commands.md), [exec](exec.md), [exec-policy](execpolicy.md), [sandbox](sandbox.md), [contributing](contributing.md), [CLA](CLA.md), [license](license.md), [open-source-fund](open-source-fund.md), [getting-started-again](getting-started.md), [config-again](config.md), [skills-again](skills.md), [sandbox-again](sandbox.md) |
| ROW-002 | Повторяем длинную ссылочную ячейку для проверки высоты | [config](config.md), [authentication](authentication.md), [exec](exec.md), [exec-policy](execpolicy.md), [sandbox](sandbox.md), [agents-md](agents_md.md), [skills](skills.md), [slash-commands](slash_commands.md), [install](install.md), [getting-started](getting-started.md), [example-config](example-config.md), [contributing](contributing.md), [CLA](CLA.md), [license](license.md), [open-source-fund](open-source-fund.md), [config-second-pass](config.md), [exec-second-pass](exec.md), [agents-second-pass](agents_md.md) |
| ROW-003 | Смешиваем короткие и длинные labels в одной ячейке | [docs-config](config.md), [docs-example-config](example-config.md), [docs-authentication](authentication.md), [docs-getting-started](getting-started.md), [docs-install](install.md), [docs-agents-md](agents_md.md), [docs-skills](skills.md), [docs-slash-commands](slash_commands.md), [docs-exec](exec.md), [docs-exec-policy](execpolicy.md), [docs-sandbox](sandbox.md), [docs-contributing](contributing.md), [docs-open-source-fund](open-source-fund.md), [docs-license](license.md), [docs-cla](CLA.md), [docs-config-repeat](config.md) |
