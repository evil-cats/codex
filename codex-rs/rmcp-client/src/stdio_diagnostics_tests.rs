//! Tests for bounded stdio MCP diagnostics.

use super::STDERR_TAIL_MAX_BYTES;
use super::StdioServerDiagnosticState;
use pretty_assertions::assert_eq;

#[test]
fn mcp_stderr_tail_flushes_on_transport_close() {
    let diagnostics = StdioServerDiagnosticState::new();

    diagnostics.push_stderr_line("first line");
    diagnostics.push_stderr_line("partial line at close");

    let snapshot = diagnostics.snapshot();
    assert_eq!(
        snapshot.stderr_tail.as_deref(),
        Some("first line\npartial line at close")
    );
    assert!(!snapshot.stderr_truncated);
}

#[test]
fn mcp_stderr_tail_is_bounded() {
    let diagnostics = StdioServerDiagnosticState::new();

    diagnostics.push_stderr_line("old line");
    diagnostics.push_stderr_line("x".repeat(STDERR_TAIL_MAX_BYTES + 64));

    let snapshot = diagnostics.snapshot();
    let tail = snapshot
        .stderr_tail
        .expect("tail should contain latest line");
    assert_eq!(tail.len(), STDERR_TAIL_MAX_BYTES);
    assert!(snapshot.stderr_truncated);
    assert!(tail.chars().all(|ch| ch == 'x'));
}
