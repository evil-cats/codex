//! Модульные тесты строгого разбора аргументов и формирования ответа `get_system_time`.
//!
//! Проверки фиксируют видимые модели ошибки, допустимые смещения и форму метаданных,
//! не привязываясь к конкретному текущему времени хоста.

use super::*;
use chrono::TimeZone;
use pretty_assertions::assert_eq;

#[test]
fn defaults_to_local_offset_and_short_time_format() {
    let args = SystemTimeArgs::default();
    let response = system_time_response(args).expect("system time response");

    let SystemTimeResponse::Short(response) = response else {
        panic!("default response should be short");
    };
    assert!(
        response.formatted.len() == 5 && response.formatted.as_bytes()[2] == b':',
        "formatted default time should use %H:%M, got {}",
        response.formatted
    );
}

#[test]
fn full_response_includes_metadata() {
    let response = system_time_response(SystemTimeArgs {
        format: Some(DEFAULT_SYSTEM_TIME_FORMAT.to_string()),
        offset: Some(DEFAULT_SYSTEM_TIME_OFFSET.to_string()),
        full: true,
    })
    .expect("system time response");

    let SystemTimeResponse::Full(response) = response else {
        panic!("full response should include metadata");
    };

    assert_eq!(response.format, DEFAULT_SYSTEM_TIME_FORMAT);
    assert_eq!(response.offset, DEFAULT_SYSTEM_TIME_OFFSET);
    assert!(
        matches!(
            response.resolved_offset.as_bytes().first(),
            Some(b'+' | b'-')
        ),
        "resolved offset should be concrete, got {}",
        response.resolved_offset
    );
}

#[test]
fn full_response_formats_with_chrono_strftime_syntax() {
    let format = "%Y-%m-%d %H:%M %:z".to_string();
    let format_items = StrftimeItems::new(&format)
        .parse_to_owned()
        .expect("valid format");
    let offset = FixedOffset::east_opt(3 * 60 * 60).expect("valid offset");
    let selected = offset
        .with_ymd_and_hms(2026, 6, 8, 23, 16, 0)
        .single()
        .expect("valid datetime");
    let utc = selected.with_timezone(&Utc);
    let formatted = selected
        .format_with_items(format_items.iter().cloned())
        .to_string();

    let response = build_full_response(formatted, format, "+03:00".to_string(), selected, utc);

    assert_eq!(response.formatted, "2026-06-08 23:16 +03:00");
    assert_eq!(response.offset, "+03:00");
    assert_eq!(response.resolved_offset, "+03:00");
    assert_eq!(response.resolved_offset_seconds, 10_800);
    assert_eq!(response.unix_seconds, utc.timestamp());
    assert_eq!(response.utc_rfc3339, "2026-06-08T20:16:00.000Z");
}

#[test]
fn utc_offset_is_case_insensitive() {
    let offset = parse_requested_offset(Some("UTC")).expect("parse UTC");

    assert_eq!(
        offset,
        RequestedOffset::Fixed {
            label: "utc".to_string(),
            offset: FixedOffset::east_opt(0).expect("valid offset")
        }
    );
}

/// Проверяет, что пробельная строка после `trim()` выбирает локальное время.
#[test]
fn whitespace_offset_selects_local_time() {
    let offset = parse_requested_offset(Some(" \n\t ")).expect("пробельная строка означает local");

    assert_eq!(offset, RequestedOffset::Local);
}

/// Проверяет обе разрешённые границы фиксированного смещения без чтения текущего времени.
#[test]
fn parses_fixed_offsets() {
    let max_offset_seconds = 23 * 60 * 60 + 59 * 60;
    let offsets = [
        parse_requested_offset(Some("+23:59")).expect("положительная граница допустима"),
        parse_requested_offset(Some("-23:59")).expect("отрицательная граница допустима"),
    ];

    assert_eq!(
        offsets,
        [
            RequestedOffset::Fixed {
                label: "+23:59".to_string(),
                offset: FixedOffset::east_opt(max_offset_seconds)
                    .expect("положительная граница chrono допустима"),
            },
            RequestedOffset::Fixed {
                label: "-23:59".to_string(),
                offset: FixedOffset::west_opt(max_offset_seconds)
                    .expect("отрицательная граница chrono допустима"),
            },
        ]
    );
}

/// Проверяет все структурные классы ошибки парсера и обе числовые границы диапазона.
#[test]
fn rejects_malformed_incomplete_and_out_of_range_fixed_offsets() {
    let invalid_offsets = [
        "+1:00", "+01:0", "x01:00", "+01-00", "+0x:00", "+24:00", "+00:60",
    ];

    let actual_messages = invalid_offsets
        .iter()
        .copied()
        .map(|value| {
            let error = parse_requested_offset(Some(value))
                .expect_err("ошибочное фиксированное смещение должно быть отклонено");
            let FunctionCallError::RespondToModel(message) = error else {
                panic!("ошибка для `{value}` должна быть видна модели");
            };
            message
        })
        .collect::<Vec<_>>();
    let expected_messages = invalid_offsets
        .iter()
        .copied()
        .map(|value| {
            format!(
                "invalid offset `{value}`; use `local`, `utc`, or a fixed offset in `+HH:MM`/`-HH:MM` form"
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(actual_messages, expected_messages);
}

#[test]
fn rejects_iana_timezone_names() {
    let err = parse_requested_offset(Some("Europe/Moscow")).expect_err("IANA zones unsupported");

    let FunctionCallError::RespondToModel(message) = err else {
        panic!("expected model-facing error");
    };
    assert!(message.contains("invalid offset `Europe/Moscow`"));
    assert!(message.contains("`+HH:MM`/`-HH:MM`"));
}

#[test]
fn rejects_invalid_format_strings() {
    let err = system_time_response(SystemTimeArgs {
        format: Some("%Q".to_string()),
        offset: Some("utc".to_string()),
        full: false,
    })
    .expect_err("invalid format should fail");

    let FunctionCallError::RespondToModel(message) = err else {
        panic!("expected model-facing error");
    };
    assert!(message.contains("invalid chrono strftime format"));
}
