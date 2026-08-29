---
id: fork-memory-read-template-path
status: active
created: 2026-06-08
updated: 2026-08-28
---

# Memory read template: встроенная политика обновления памяти

## Обзор

Эта карточка фиксирует fork-доработку Hermione для read-path memory prompt в
developer-инструкциях. Политика обновления памяти живёт в каноническом
встроенном шаблоне
`codex-rs/ext/memories/templates/memories/read_path.md`;
внешний config override `[memories].read_template_path` намеренно отсутствует.

## Зачем это нужно

Hermione-профиль должен получать инструкции чтения памяти, которые разрешают
агенту самостоятельно создавать и обслуживать ad-hoc карточки памяти, когда это
разрешено активными `developer` instructions, profile policy или явной просьбой
пользователя.

Старый подход с внешним `read_template_path` решал задачу быстро, но создавал
лишний override-слой. При следующем upstream update изменение штатного
`read_path.md` могло пройти незамеченным, потому что активный профиль продолжал
читать текст из `~/.codex/policies/memory-read-template.md`.

Новая модель делает встроенный шаблон источником истины. Если upstream изменит
этот prompt, rebase или diff покажет изменение в кодовом файле, а не спрячет за
профильной настройкой.

## Карта файлов

| Файл | Роль |
| --- | --- |
| `codex-rs/ext/memories/templates/memories/read_path.md` | Содержит канонический read-path prompt и расширенный раздел `Updating memories` |
| `codex-rs/ext/memories/src/prompts.rs` | Рендерит только встроенный шаблон с `{{ base_path }}` и `{{ memory_summary }}` |
| `codex-rs/ext/memories/src/prompts_tests.rs` | Проверяет встроенный шаблон, подстановку summary и отсутствие старого запрета на самостоятельные memory updates |
| `codex-rs/ext/memories/src/extension.rs` | Хранит runtime-флаги и `codex_home`; не принимает и не прокидывает template path |
| `codex-rs/ext/memories/src/tests.rs` | Создает `MemoriesExtensionConfig` без template path |
| `codex-rs/config/src/types.rs` | Не содержит `read_template_path` в `MemoriesToml` и `MemoriesConfig` |
| `codex-rs/core/config.schema.json` | Не экспортирует `read_template_path` в memories schema |
| `codex-rs/core/src/config/config_tests.rs` | Проверяет memories TOML/effective config без `read_template_path` |
| `codex-rs/memories/README.md` | Указывает, что undated runtime templates редактируются in-place |

## Итоговый контракт

1. Read-path memory prompt в developer-инструкциях всегда строится из
   встроенного шаблона
   `codex-rs/ext/memories/templates/memories/read_path.md`.
2. `[memories].read_template_path` отсутствует в Rust config types, effective
   `MemoriesConfig`, JSON schema и config tests.
3. `MemoriesExtensionConfig` не хранит template path и передает в
   `build_memory_tool_developer_instructions` только `codex_home`.
4. `build_memory_tool_developer_instructions` читает
   `${codex_home}/memories/memory_summary.md`, trim'ит summary, обрезает ее по
   `MEMORY_TOOL_DEVELOPER_INSTRUCTIONS_SUMMARY_TOKEN_LIMIT` и рендерит
   встроенный шаблон.
5. Если `memory_summary.md` отсутствует, пустой после trim или не читается,
   prompt не добавляется.
6. Встроенный шаблон поддерживает только placeholders `{{ base_path }}` и
   `{{ memory_summary }}`. Unknown placeholders остаются ошибкой встроенного
   шаблона, потому что такой template должен падать при lazy parse, а не
   подменяться внешним файлом.
7. Раздел `Updating memories` во встроенном шаблоне разрешает обновления памяти,
   когда это позволяют активные `developer` instructions, profile policy или
   явная просьба пользователя. Прямая команда `"remember this"` не обязательна,
   если активная policy уже разрешает запись.
8. Безопасный путь записи памяти по умолчанию:
   - создавать маленькие Markdown-карточки в
     `${codex_home}/memories/extensions/ad_hoc/notes/`;
   - редактировать существующие ad-hoc карточки, когда нужно исправить durable
     knowledge, `status`, `updated`, `confidence` или условия применимости;
   - обновлять `${codex_home}/memories/extensions/ad_hoc/INDEX.md` в том же
     изменении, когда добавляются карточки, меняется статус или переносится
     ответственность.
9. Шаблон прямо запрещает сохранять секреты, учетные данные, большие сырые логи,
   сгенерированные артефакты и временный вывод команд как memory.
10. Шаблон запрещает редактировать consolidated/generated memory artifacts
    вроде `MEMORY.md`, `memory_summary.md`, rollout summaries и skill files без
    более высокого разрешения.
11. При конфликте memory candidate, возможной чувствительности данных или
    неясном scope агент должен спросить пользователя перед записью.

## Архитектурное решение

Канонический read-path prompt является встроенным asset расширения `memories`;
`prompts.rs` владеет чтением ограниченной summary и рендерингом шаблона. Config
и extension намеренно не принимают внешний template path, поэтому разные
установки не могут незаметно получить разный memory contract. Config schema и
tests закрепляют отсутствие удалённого override-слоя.

Базовое чтение и ограничение `memory_summary.md`, встроенный `read_path.md` и
отсутствие `[memories].read_template_path` уже обеспечивает upstream.
Fork-дельта ограничена расширенной политикой `Updating memories`, явной
проверкой разрешённых placeholders в `prompts.rs`, проверками этой политики и
уточнением владельца runtime-шаблона в `codex-rs/memories/README.md`.

## Порядок повторения при переносе

1. Найти живого runtime-владельца по
   `build_memory_tool_developer_instructions`. В текущей структуре это
   `codex-rs/ext/memories/src/prompts.rs`; не восстанавливать прежний отдельный
   memory-read модуль механически.
2. В `codex-rs/ext/memories/templates/memories/read_path.md` сохранить расширенный
   раздел `Updating memories` без старого запрета `only when explicitly asked by
   the user`.
3. Проверить, что `read_template_path` отсутствует в `MemoriesToml`,
   `MemoriesConfig`, `Default for MemoriesConfig` и
   `From<MemoriesToml> for MemoriesConfig`. Если upstream вернул этот key,
   удалить его, а не сохранять как deprecated no-op.
4. Если config types пришлось менять, обновить
   `codex-rs/core/config.schema.json` через skill-owned владельца
   `fork generators`, чтобы schema не экспортировала удалённый key.
5. Проверить, что `MemoriesExtensionConfig` не хранит template path, а вызов
   prompt builder передаёт только `codex_home`.
6. Проверить `build_memory_tool_developer_instructions`: он не должен принимать
   template path или читать configured template и должен рендерить встроенный
   шаблон.
7. Проверить, что configured-template test отсутствует. Embedded-template test
   должен проверять, что prompt содержит новый безопасный путь обновления и не
   содержит старый запрет.
8. Обновить `codex-rs/memories/README.md`: не документировать
   `[memories].read_template_path`, оставить in-place editing undated runtime
   templates.

## Проверки

Исполняемая карта card-level regression tests:

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "read_template_path отсутствует в TOML и итоговом MemoriesConfig",
      "argv": ["just", "test", "-p", "codex-core", "test_toml_parsing"]
    },
    {
      "purpose": "embedded read-path prompt, bounded summary и safe update policy",
      "argv": ["just", "test", "-p", "codex-memories-extension"]
    }
  ]
}
```

Если при переносе пришлось удалять `read_template_path` из config types,
дополнительно обязателен `fork generators`, чтобы синхронно обновить config
schema. Если upstream уже не содержит key, а config types и schema не менялись,
этот gate для карточки не нужен.

Точный фильтр `test_toml_parsing` запускает только владеющий тест TOML и
`MemoriesConfig`, не включая посторонние тесты Code Mode по совпадению слова
`config` в имени.

## Риски и ограничения

### Ограничения

- Не менять пользовательский `~/.codex/hermione.config.toml` в рамках этой
  карточки.
- Не восстанавливать `[memories].read_template_path` как deprecated no-op: это
  оставило бы мертвый config key и не помогло бы увидеть расхождение шаблона.
- Не переносить политику обновления памяти обратно в профильный файл как
  активный источник runtime prompt.

### Риски

- Upstream может снова изменить форму memory read-path prompt. При rebase нужно
  сравнить встроенный `read_path.md`, а не полагаться на профильный override.
- Удаление config key является ломающим изменением для локальных profile configs,
  где этот key еще указан. Эта карточка намеренно не редактирует пользовательский
  config.
- Если schema не обновить, редакторские подсказки и runtime-контракт config
  разойдутся.
