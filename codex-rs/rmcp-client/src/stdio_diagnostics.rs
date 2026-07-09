//! Bounded diagnostics captured for MCP stdio launches.
//!
//! The rmcp transport owns JSON-RPC framing and must not expose stderr as tool
//! output. This module keeps only a small stderr tail and launch lifecycle flags
//! so higher-level recovery code can persist useful evidence in the rollout.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

const STDERR_TAIL_MAX_BYTES: usize = 8 * 1024;
static LAUNCH_COUNTER: AtomicUsize = AtomicUsize::new(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StdioDiagnosticSnapshot {
    pub(crate) launch_id: String,
    pub(crate) stderr_tail: Option<String>,
    pub(crate) stderr_truncated: bool,
}

pub(crate) struct StdioServerDiagnosticState {
    launch_id: String,
    stderr_tail: std::sync::Mutex<StderrTail>,
    dead: AtomicBool,
    dead_reported: AtomicBool,
}

impl StdioServerDiagnosticState {
    pub(crate) fn new() -> Arc<Self> {
        let index = LAUNCH_COUNTER.fetch_add(1, Ordering::Relaxed);
        Arc::new(Self {
            launch_id: format!("mcp-stdio-launch-{index}"),
            stderr_tail: std::sync::Mutex::new(StderrTail::default()),
            dead: AtomicBool::new(false),
            dead_reported: AtomicBool::new(false),
        })
    }

    pub(crate) fn push_stderr_line(&self, line: impl Into<String>) {
        let mut tail = self
            .stderr_tail
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        tail.push_line(line.into());
    }

    pub(crate) fn mark_dead(&self) {
        self.dead.store(true, Ordering::Release);
    }

    pub(crate) fn is_dead(&self) -> bool {
        self.dead.load(Ordering::Acquire)
    }

    pub(crate) fn take_dead_report_pending(&self) -> bool {
        self.is_dead() && !self.dead_reported.swap(true, Ordering::AcqRel)
    }

    pub(crate) fn snapshot(&self) -> StdioDiagnosticSnapshot {
        let tail = self
            .stderr_tail
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (stderr_tail, stderr_truncated) = tail.snapshot();
        StdioDiagnosticSnapshot {
            launch_id: self.launch_id.clone(),
            stderr_tail,
            stderr_truncated,
        }
    }
}

#[derive(Default)]
struct StderrTail {
    lines: VecDeque<String>,
    bytes: usize,
    truncated: bool,
}

impl StderrTail {
    fn push_line(&mut self, mut line: String) {
        let line_bytes = line.len().saturating_add(1);
        if line_bytes > STDERR_TAIL_MAX_BYTES {
            self.lines.clear();
            self.bytes = 0;
            self.truncated = true;
            line = keep_last_bytes_on_char_boundary(&line, STDERR_TAIL_MAX_BYTES);
        }

        self.bytes = self.bytes.saturating_add(line.len().saturating_add(1));
        self.lines.push_back(line);
        while self.bytes > STDERR_TAIL_MAX_BYTES {
            let Some(removed) = self.lines.pop_front() else {
                self.bytes = 0;
                break;
            };
            self.bytes = self.bytes.saturating_sub(removed.len().saturating_add(1));
            self.truncated = true;
        }
    }

    fn snapshot(&self) -> (Option<String>, bool) {
        let stderr_tail = if self.lines.is_empty() {
            None
        } else {
            Some(
                self.lines
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        };
        (stderr_tail, self.truncated)
    }
}

fn keep_last_bytes_on_char_boundary(line: &str, max_bytes: usize) -> String {
    let mut start = line.len().saturating_sub(max_bytes);
    while start < line.len() && !line.is_char_boundary(start) {
        start += 1;
    }
    line[start..].to_string()
}

#[cfg(test)]
#[path = "stdio_diagnostics_tests.rs"]
mod tests;
