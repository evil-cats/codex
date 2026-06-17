use super::*;
use pretty_assertions::assert_eq;

fn thread_id(value: &str) -> ThreadId {
    ThreadId::from_string(value).expect("valid thread id")
}

#[test]
fn parse_requested_thread_id_defaults_to_current() {
    let current = thread_id("00000000-0000-0000-0000-000000000001");

    assert_eq!(parse_requested_thread_id(None, current).unwrap(), current);
}

#[test]
fn parse_requested_thread_id_parses_explicit_id() {
    let current = thread_id("00000000-0000-0000-0000-000000000001");
    let requested = "00000000-0000-0000-0000-000000000002".to_string();

    assert_eq!(
        parse_requested_thread_id(Some(requested), current).unwrap(),
        thread_id("00000000-0000-0000-0000-000000000002")
    );
}

#[test]
fn parse_requested_thread_id_rejects_empty_id() {
    let current = thread_id("00000000-0000-0000-0000-000000000001");

    let err = parse_requested_thread_id(Some("   ".to_string()), current)
        .expect_err("empty thread id should fail");

    assert!(
        matches!(err, FunctionCallError::RespondToModel(message) if message.contains("non-empty UUID"))
    );
}
