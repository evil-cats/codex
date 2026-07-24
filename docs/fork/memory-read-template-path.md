---
id: fork-memory-read-template-path
status: active
created: 2026-06-08
updated: 2026-07-21
source_scope: rust-v0.142.5..hermione-0.142.5
---

# Memory read template: встроенная политика обновления памяти

## Обзор

Эта карточка фиксирует fork-доработку Hermione для read-path memory prompt в
developer-инструкциях. Активная модель после правки 2026-07-08: нужная политика
обновления памяти живет прямо во встроенном runtime-шаблоне
`codex-rs/ext/memories/templates/memories/read_path.md`.

Исторически карточка вводила `[memories].read_template_path`, чтобы профиль мог
подставлять внешний Markdown-шаблон. Этот слой удален: он скрывал расхождение
upstream-шаблона и больше не нужен, потому что fork теперь меняет встроенный
шаблон напрямую.

| Поле | Значение |
| --- | --- |
| Статус | `active` |
| Исторический commit | `9c9af8853 Make memory read template configurable` |
| Текущая доправка | Удалить `[memories].read_template_path` и встроить политику обновления памяти Hermione в `read_path.md` |
| Runtime-владелец | `codex-rs/ext/memories/src/prompts.rs` |
| Канонический шаблон | `codex-rs/ext/memories/templates/memories/read_path.md` |
| Удаленный config key | `[memories].read_template_path` |
| Checkpoint перед исторической карточкой | Пропущен по явному разрешению пользователя от 2026-06-08 |

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
| `codex-rs/ext/memories/src/extension.rs` | Хранит только `codex_home`; не прокидывает template path |
| `codex-rs/ext/memories/src/tests.rs` | Создает `MemoriesExtensionConfig` без template path |
| `codex-rs/config/src/types.rs` | Не содержит `read_template_path` в `MemoriesToml` и `MemoriesConfig` |
| `codex-rs/core/config.schema.json` | Не экспортирует `read_template_path` в memories schema |
| `codex-rs/core/src/config/config_tests.rs` | Проверяет memories TOML/effective config без `read_template_path` |
| `codex-rs/memories/README.md` | Указывает, что undated runtime templates редактируются in-place |
| `docs/fork/migration-0.142.5.md` | Синхронизирует текущую migration-карту с удалением override-слоя |

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

## Порядок повторения при переносе

1. Найти живой runtime-владелец по
   `build_memory_tool_developer_instructions`. После merge `rust-v0.141.0` это
   `codex-rs/ext/memories/src/prompts.rs`; в более старых ветках код мог жить в
   `codex-rs/memories/read/src/prompts.rs`.
2. В `codex-rs/ext/memories/templates/memories/read_path.md` сохранить расширенный
   раздел `Updating memories` без старого запрета `only when explicitly asked by
   the user`.
3. Удалить `read_template_path` из `MemoriesToml`, `MemoriesConfig`,
   `Default for MemoriesConfig` и `From<MemoriesToml> for MemoriesConfig`.
4. Обновить `codex-rs/core/config.schema.json` через skill-owned владельца
   `fork generators`, чтобы schema больше не экспортировала удаленный key.
5. Удалить `read_template_path` из `MemoriesExtensionConfig` и из вызова prompt
   builder.
6. Упростить `build_memory_tool_developer_instructions`: убрать параметр
   template path и чтение configured template, оставить render встроенного
   шаблона.
7. Удалить configured-template test. Embedded-template test должен проверять, что
   prompt содержит новый безопасный путь обновления и не содержит старый запрет.
8. Обновить `codex-rs/memories/README.md`: не документировать
   `[memories].read_template_path`, оставить in-place editing undated runtime
   templates.
9. Синхронизировать текущую migration-карту, если она утверждает наличие
   `[memories].read_template_path`.

## Проверки

### Смысловое покрытие

Покрытие, которое должно присутствовать в diff:

- `codex-rs/ext/memories/templates/memories/read_path.md`:
  - содержит расширенный `Updating memories`;
  - не содержит старого запрета `only when explicitly asked by the user`;
  - описывает путь обновления ad-hoc карточки и `INDEX.md`.
- `codex-rs/config/src/types.rs`:
  - `MemoriesToml` не содержит `read_template_path`;
  - `MemoriesConfig` не содержит `read_template_path`;
  - default и conversion не упоминают удаленный key.
- `codex-rs/core/config.schema.json` не содержит schema entry
  `read_template_path`.
- `codex-rs/ext/memories/src/extension.rs` не хранит и не передает template
  path.
- `codex-rs/ext/memories/src/prompts.rs` не читает configured template и
  рендерит встроенный шаблон.
- `codex-rs/memories/README.md` не документирует config override.

Регрессионное покрытие:

- `codex-rs/core/src/config/config_tests.rs`:
  - `test_toml_parsing` продолжает проверять остальные memories settings без
    удаленного key;
  - итоговый `MemoriesConfig` сравнивается целиком.
- `codex-rs/ext/memories/src/prompts_tests.rs`:
  - embedded-template test проверяет подстановку summary;
  - embedded-template test проверяет новый текст `A direct "remember this"`;
  - embedded-template test проверяет отсутствие старого текста
    `only when explicitly asked by the user`.
  - embedded-template test проверяет, что unknown placeholder в embedded-шаблоне
    падает при lazy parse.
  - `build_memory_tool_developer_instructions_bounds_memory_summary` проверяет,
    что превышение токенного лимита заменяет середину summary маркером
    `tokens truncated`, сохраняя ограниченные prefix и suffix во фрагменте
    `DeveloperPolicy`.
  - `build_memory_tool_developer_instructions_skips_unusable_summary` проверяет
    отсутствие prompt при отсутствующем, пустом после trim и невалидном UTF-8
    `memory_summary.md`.
- `codex-rs/ext/memories/src/tests.rs`:
  - extension tests создают `MemoriesExtensionConfig` без удаленного поля;
  - prompt contribution по-прежнему добавляет developer-policy fragment.

### Владелец исполняемой карты

Проверки уровня карточки запускает `fork tests`. Внутренние argv живут в блоке
`fork-tests.v1` ниже и являются данными для skill-owned владельца, а не ручным
runbook для прямого запуска `cargo` или `just`.

```json
{
  "schema": "fork-tests.v1",
  "tests": [
    {
      "purpose": "core config",
      "argv": ["just", "test", "-p", "codex-core", "config"]
    },
    {
      "purpose": "memories extension",
      "argv": ["just", "test", "-p", "codex-memories-extension"]
    }
  ]
}
```

### Дополнительные gates

- Форму карточки, наличие strict-подразделов `Проверки` и связь с блоком
  `fork-tests.v1` проверяет skill-owned validator карточек.
- Удаление config key требует синхронизации `codex-rs/core/config.schema.json`
  через skill-owned generator gate.
- После изменения Rust-кода требуется repo formatting через skill-owned format
  gate.

### Исторические результаты

- Checkpoint перед исторической карточкой был пропущен по явному разрешению
  пользователя от 2026-06-08.
- Старый формат карточки вводил `[memories].read_template_path` как быстрый
  профильный override. Этот слой намеренно удален в доправке 2026-07-08.
- Старые migration-карты `0.140.0` и `0.141.0` сохраняют исторический факт, что
  на тех этапах переносилась именно доработка config-слоя.
- Текущая доправка 2026-07-08:
  - `fork generators`: `OK`;
  - `fork format --fix`: `OK`;
  - `fork tests --mode list --card docs/fork/memory-read-template-path.md`: `OK`,
    executable map содержит `core config` и `memories extension`;
  - `fork cards validate`: `OK`;
  - `fork tests --mode cards --card docs/fork/memory-read-template-path.md`: `OK`.
- Миграция на `0.144.5` после merge `rust-v0.144.5`:
  - ручная card-scoped сверка config types, defaults, conversion, schema,
    extension config, embedded prompt и regression tests: `OK`;
  - `read_template_path` в `codex-rs` отсутствует, встроенный `read_path.md`
    сохраняет безопасный ad-hoc путь обновления памяти и ограничения записи;
  - code-scoped правки не потребовались;
  - project-level gates оставлены общему проверочному проходу по правилам
    one-card миграции.
- Миграция на `0.144.6` после merge `rust-v0.144.6`:
  - ручная сверка в пределах карточки точного пути embedded-шаблона, типов
    конфигурации, JSON schema, добавления фрагмента `DeveloperPolicy` расширением,
    встроенной policy обновления памяти, токенного лимита, поведения при ошибках
    и регрессионных тестов: `OK`;
  - рабочий код и embedded `read_path.md` сохранили итоговый контракт без
    дополнительных правок;
  - в `prompts_tests.rs` добавлено регрессионное покрытие ограничения summary по
    токенам и отказа от prompt при отсутствующем, пустом или невалидном UTF-8
    summary;
  - общепроектные gates оставлены общему проверочному проходу по правилам
    миграции одной карточки.
- Миграция на `0.145.0` после merge `rust-v0.145.0`:
  - ручная сверка подтвердила прежний embedded template path и загрузку через
    `include_str!`, placeholders `base_path` и `memory_summary`, токенное
    ограничение summary и отказ от prompt для отсутствующего, пустого или
    невалидного UTF-8 `memory_summary.md`;
  - fork-шаблон сохраняет полный безопасный ad-hoc workflow вместо upstream
    режима с единственной update note, а extension по-прежнему добавляет
    результат как model-visible fragment в `PromptSlot::DeveloperPolicy`;
  - `read_template_path` отсутствует в config types, schema, extension config,
    runtime builder, тестах и README; code-scoped правки не потребовались;
  - общепроектные gates оставлены общему проверочному проходу по правилам
    миграции одной карточки.

### Известные падения и пропуски

- Профильный `~/.codex/hermione.config.toml` не входит в scope этой карточки.
  Если в нем остается `read_template_path`, новый бинарник с `deny_unknown_fields`
  будет считать этот key устаревшим, пока пользователь не уберет строку отдельно.
- Отдельный test на configured template удаляется вместе с самим поведением.
- Нельзя заменять встроенный шаблон внешним override только ради удобства
  локального профиля: это снова спрятало бы расхождение с upstream.

## Ограничения

- Не менять пользовательский `~/.codex/hermione.config.toml` в рамках этой
  карточки.
- Не восстанавливать `[memories].read_template_path` как deprecated no-op: это
  оставило бы мертвый config key и не помогло бы увидеть расхождение шаблона.
- Не переносить политику обновления памяти обратно в профильный файл как
  активный источник runtime prompt.
- Не запускать Rust/Cargo/`just` как нормативный шаг из карточки. Для fork
  workflow использовать skill-owned владельцев, а внутренние argv хранить только
  в `fork-tests.v1` или историческом подтверждении.

## Риски

- Upstream может снова изменить форму memory read-path prompt. При rebase нужно
  сравнить встроенный `read_path.md`, а не полагаться на профильный override.
- Удаление config key является ломающим изменением для локальных profile configs,
  где этот key еще указан. Эта карточка намеренно не редактирует пользовательский
  config.
- Если schema не обновить, редакторские подсказки и runtime-контракт config
  разойдутся.

## Проверка покрытия

| Пункт | Статус | Где отражено |
| --- | --- | --- |
| Встроить политику обновления памяти Hermione в embedded `read_path.md` | перенесено | "Итоговый контракт", "Порядок повторения при переносе", "Проверки" |
| Убрать старый запрет на самостоятельные memory updates | перенесено | "Итоговый контракт", "Смысловое покрытие" |
| Удалить `[memories].read_template_path` из активной config-области | перенесено | "Итоговый контракт", "Карта файлов", "Порядок повторения при переносе" |
| Сохранить `memory_summary.md` read/truncate behavior | перенесено | "Итоговый контракт" |
| Сохранить placeholders `base_path` и `memory_summary` для embedded-шаблона | перенесено | "Итоговый контракт" |
| Зафиксировать, что профильный `hermione.config.toml` не меняется | перенесено | "Известные падения и пропуски", "Ограничения" |
| Синхронизировать README и текущую migration-карту | перенесено | "Карта файлов", "Порядок повторения при переносе" |
| Зафиксировать тесты, gates и владельца исполняемой карты | перенесено | "Проверки" |
| Сохранить исторический контекст старого override без восстановления поведения | перенесено | "Обзор", "Исторические результаты" |
