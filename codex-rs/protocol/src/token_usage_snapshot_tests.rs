//! Regression coverage for deterministic model ordering in token snapshots.

use super::compare_model_slugs;
use pretty_assertions::assert_eq;

#[test]
fn model_token_usage_order_is_numeric_and_tier_aware() {
    let mut models = vec![
        "other-model",
        "gpt-5.6-luna",
        "gpt-5.5",
        "gpt-5.6-terra",
        "gpt-6",
        "gpt-5.6-sol",
        "gpt-5.6-preview",
    ];

    models.sort_by(|left, right| compare_model_slugs(left, right));

    assert_eq!(
        models,
        vec![
            "gpt-6",
            "gpt-5.6-sol",
            "gpt-5.6-terra",
            "gpt-5.6-luna",
            "gpt-5.6-preview",
            "gpt-5.5",
            "other-model",
        ]
    );
}
