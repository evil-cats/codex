---
id: FU-2026-005
status: accepted
priority: medium
kind: enhancement
tags: [codex, multi-agent, delegation, spark, config]
created: 2026-05-26
updated: 2026-05-26
review_at: before changing spawn_agent policy or Hermione multi-agent defaults
architecture_refs:
  - codex-rs/core/src/tools/handlers/multi_agents_spec.rs
  - codex-rs/core/src/config/mod.rs
  - codex-rs/features/src/feature_configs.rs
invalid_if:
  - subagent delegation remains explicit-only by product policy
  - Hermione profile uses config-only usage hints and no Rust change is needed
---

# FU-2026-005: политика автономных Spark subagents

## Кратко

| Поле | Значение |
| --- | --- |
| Статус | принят (`accepted`) |
| Суть | Оценить доработку, позволяющую Hermione автономно запускать bounded routine subagents на `gpt-5.3-codex-spark`. |
| Почему важно | У Spark отдельные лимиты, поэтому простую sidecar работу можно вынести из основного агента без траты сильной модели. |
| Когда вернуться | Перед изменением `spawn_agent` policy, `multi_agent_v2` defaults или Hermione profile delegation guidance. |
| Когда закрыть | Если subagent delegation остается строго explicit-only или достаточно config-only `usage_hint_text` без Rust-доработки. |
| Следующий шаг | Выбрать между config-only экспериментом и first-class `delegation_policy`, затем обновить tool description и tests. |
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

Если Hermione сможет автономно запускать ограниченные Spark-субагенты по
понятным критериям, основной агент будет меньше тратить дорогую модель на
рутинный поиск, механическую сверку и узкую валидацию. Но это нельзя делать
размытым prompt-правилом: subagent запуск потребляет отдельные ресурсы и может
создавать лишнюю параллельную работу.

## Что нужно сделать

- Сначала попробовать config-only вариант через `features.multi_agent_v2.usage_hint_text` или Hermione profile guidance.
- Если поведение нужно закрепить в коде, добавить явную политику вроде `delegation_policy = "explicit_only" | "bounded_autonomous"`.
- Для `bounded_autonomous` описать критерии: routine, independent, bounded, low-risk, easy to verify, no full `fork_context` unless needed.
- Сделать `gpt-5.3-codex-spark` предпочтительной моделью для такой рутинной delegation, если она доступна.
- Сохранить explicit-only как default для обычного Codex behavior.
- Обновить tests на `spawn_agent` description и при необходимости config schema.

## Когда вернуться

Перед изменением `spawn_agent` policy, `multi_agent_v2` defaults, Hermione
profile delegation guidance или перед отдельным планом по экономии основной
модели за счет Spark.

## Когда закрыть как неактуальное

Если продуктовая политика остается строго explicit-only для subagent
delegation или если для Hermione достаточно config-only `usage_hint_text`
без Rust-доработки.

## Связи

- Tool description: [code:multi-agents-spec]
- Multi-agent config: [code:multi-agent-config]
- TOML-конфиг feature flags: [code:feature-configs]

[code:feature-configs]: ../../codex-rs/features/src/feature_configs.rs
[code:multi-agent-config]: ../../codex-rs/core/src/config/mod.rs
[code:multi-agents-spec]: ../../codex-rs/core/src/tools/handlers/multi_agents_spec.rs
