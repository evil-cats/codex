---
id: FU-2026-005
status: accepted
priority: medium
kind: enhancement
tags: [codex, multi-agent, delegation, spark, config]
created: 2026-05-26
updated: 2026-05-26
review_at: перед изменением политики spawn_agent или Hermione multi-agent defaults
architecture_refs:
  - codex-rs/core/src/tools/handlers/multi_agents_spec.rs
  - codex-rs/core/src/config/mod.rs
  - codex-rs/features/src/feature_configs.rs
invalid_if:
  - subagent delegation остается explicit-only по продуктовой политике
  - профиль Hermione использует только config usage hints и Rust-доработка не нужна
---

# FU-2026-005: политика автономных Spark-субагентов

## Кратко

| Поле | Значение |
| --- | --- |
| Статус | `accepted` |
| Суть | Оценить доработку, позволяющую Hermione автономно запускать ограниченных рутинных субагентов на `gpt-5.3-codex-spark`. |
| Почему важно | У Spark отдельные лимиты, поэтому простую sidecar-работу можно вынести из основного агента без траты сильной модели. |
| Когда вернуться | Перед изменением `spawn_agent` policy, defaults `multi_agent_v2` или правил delegation профиля Hermione. |
| Когда закрыть | Если subagent delegation остается строго explicit-only или достаточно `usage_hint_text` только через config без Rust-доработки. |
| Следующий шаг | Выбрать между экспериментом только через config и полноценной `delegation_policy`, затем обновить описание tool и tests. |
| Связи | [code:multi-agents-spec], [code:multi-agent-config], [code:feature-configs] |

## Наблюдение

`spawn_agent` сейчас описывает модельное правило explicit-only: агент должен
использовать субагентов только когда пользователь явно просит sub-agents,
delegation или parallel agent work. При этом runtime в основном валидирует
модель, `reasoning_effort`, `service_tier`, depth/concurrency и схему вызова,
а не намерение пользователя.

У пользователя есть `gpt-5.3-codex-spark` с отдельными лимитами. Для рутинных,
независимых и легко проверяемых задач это хороший кандидат на дешевую
sidecar-модель.

## Почему это важно

Если Hermione сможет автономно запускать ограниченных Spark-субагентов по
понятным критериям, основной агент будет меньше тратить дорогую модель на
рутинный поиск, механическую сверку и узкую валидацию. Но это нельзя делать
размытым prompt-правилом: запуск субагента потребляет отдельные ресурсы и может
создавать лишнюю параллельную работу.

## Что нужно сделать

- Сначала попробовать вариант только через config:
  `features.multi_agent_v2.usage_hint_text` или guidance профиля Hermione.
- Если поведение нужно закрепить в коде, добавить явную политику вроде `delegation_policy = "explicit_only" | "bounded_autonomous"`.
- Для `bounded_autonomous` описать критерии: routine, independent, bounded,
  low-risk, easy to verify, no full `fork_context` unless needed.
- Сделать `gpt-5.3-codex-spark` предпочтительной моделью для такой рутинной delegation, если она доступна.
- Сохранить explicit-only как default для обычного поведения Codex.
- Обновить тесты на описание `spawn_agent` и при необходимости config schema.

## Когда вернуться

Перед изменением `spawn_agent` policy, defaults `multi_agent_v2`, правил
delegation профиля Hermione или перед отдельным планом по экономии основной
модели за счет Spark.

## Когда закрыть как неактуальное

Если продуктовая политика остается строго explicit-only для subagent
delegation или если для Hermione достаточно `usage_hint_text` только через config
без Rust-доработки.

## Связи

- Описание tool: [code:multi-agents-spec]
- Multi-agent config: [code:multi-agent-config]
- TOML-конфиг feature flags: [code:feature-configs]

[code:feature-configs]: ../../codex-rs/features/src/feature_configs.rs
[code:multi-agent-config]: ../../codex-rs/core/src/config/mod.rs
[code:multi-agents-spec]: ../../codex-rs/core/src/tools/handlers/multi_agents_spec.rs
