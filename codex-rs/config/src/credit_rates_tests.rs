//! Регрессионные проверки внешней таблицы тарифов и fixed-point арифметики.

use super::*;
use pretty_assertions::assert_eq;

const VALID_RATES: &str = r#"
{
  "schema_version": 1,
  "unit": "credits_per_million_tokens",
  "source": "https://learn.chatgpt.com/docs/pricing",
  "models": {
    "gpt-5.6-sol": { "input": 100, "cached_input": 10, "output": 500 },
    "gpt-5.6-luna": { "input": 5, "cached_input": 0.5, "output": 30 }
  }
}
"#;

#[test]
/// Десятичные тарифы дают точные пикокредиты без округления через `float`.
fn credit_rates_compute_exact_costs() {
    let loaded = parse_credit_rates(VALID_RATES).expect("valid rate table");

    assert_eq!(loaded.invalid_model_entries, Vec::<String>::new());
    assert_eq!(
        loaded
            .rates
            .cost_for_model("gpt-5.6-sol", 3_450, 463_872, 2_573)
            .expect("Sol tariff")
            .picocredits(),
        6_270_220_000_000
    );
    assert_eq!(
        loaded
            .rates
            .cost_for_model("gpt-5.6-luna", 44_161, 156_672, 546)
            .expect("Luna tariff")
            .picocredits(),
        315_521_000_000
    );
}

#[test]
/// Повреждённая модель сообщается и пропускается без потери независимо корректных тарифов.
fn credit_rates_isolate_invalid_model_entries() {
    let loaded = parse_credit_rates(
        r#"
        {
          "schema_version": 1,
          "unit": "credits_per_million_tokens",
          "models": {
            "valid": { "input": 1, "cached_input": 0.5, "output": 2 },
            "negative": { "input": -1, "cached_input": 1, "output": 2 },
            "incomplete": { "input": 1, "cached_input": 1 }
          }
        }
        "#,
    )
    .expect("valid top-level document");

    assert_eq!(loaded.invalid_model_entries.len(), 2);
    assert!(
        loaded
            .invalid_model_entries
            .iter()
            .any(|entry| entry.contains("`negative`") && entry.contains("non-negative"))
    );
    assert!(
        loaded
            .invalid_model_entries
            .iter()
            .any(|entry| entry.contains("`incomplete`") && entry.contains("missing field"))
    );
    assert!(loaded.rates.cost_for_model("valid", 1, 1, 1).is_some());
    assert_eq!(loaded.rates.cost_for_model("negative", 1, 1, 1), None);
}

#[test]
/// Неподдерживаемый верхнеуровневый контракт отклоняет всю таблицу без догадок о смысле.
fn credit_rates_reject_unsupported_document_contracts() {
    for (document, expected) in [
        (
            r#"{"schema_version":2,"unit":"credits_per_million_tokens","models":{}}"#,
            "unsupported schema_version 2",
        ),
        (
            r#"{"schema_version":1,"unit":"dollars","models":{}}"#,
            "unsupported unit `dollars`",
        ),
    ] {
        let error = parse_credit_rates(document).expect_err("unsupported document");
        assert!(
            error.to_string().contains(expected),
            "unexpected error: {error}"
        );
    }
}

#[test]
/// Точность сверх fixed-point контракта отклоняет только соответствующую запись модели.
fn credit_rates_reject_excess_fractional_precision() {
    let loaded = parse_credit_rates(
        r#"
        {
          "schema_version": 1,
          "unit": "credits_per_million_tokens",
          "models": {
            "too-precise": { "input": 1.1234567, "cached_input": 1, "output": 2 }
          }
        }
        "#,
    )
    .expect("valid top-level document");

    assert_eq!(loaded.invalid_model_entries.len(), 1);
    assert!(loaded.invalid_model_entries[0].contains("fractional digits"));
}
