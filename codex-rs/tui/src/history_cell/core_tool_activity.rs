//! Core function-tool activity history cells.

use super::*;
use codex_app_server_protocol::CoreToolActivityKind;
use codex_app_server_protocol::CoreToolActivityStatus;
use codex_app_server_protocol::ThreadItem;

#[derive(Debug)]
pub(crate) struct CoreToolActivityCell {
    id: String,
    kind: CoreToolActivityKind,
    detail: String,
    tool_name: String,
    status: CoreToolActivityStatus,
    error: Option<String>,
    start_time: Instant,
    animations_enabled: bool,
}

impl CoreToolActivityCell {
    fn new(
        id: String,
        kind: CoreToolActivityKind,
        detail: String,
        tool_name: String,
        status: CoreToolActivityStatus,
        error: Option<String>,
        animations_enabled: bool,
    ) -> Self {
        Self {
            id,
            kind,
            detail,
            tool_name,
            status,
            error,
            start_time: Instant::now(),
            animations_enabled,
        }
    }

    pub(crate) fn call_id(&self) -> &str {
        &self.id
    }

    pub(crate) fn complete(&mut self, status: CoreToolActivityStatus, error: Option<String>) {
        self.status = status;
        self.error = error;
    }

    fn is_active(&self) -> bool {
        matches!(self.status, CoreToolActivityStatus::InProgress)
    }

    fn heading(&self) -> &'static str {
        match (self.kind, self.is_active()) {
            (CoreToolActivityKind::File, true) => "Exploring",
            (CoreToolActivityKind::File, false) => "Explored",
            (CoreToolActivityKind::ThreadInfo | CoreToolActivityKind::SystemTime, true) => {
                "Inspecting"
            }
            (CoreToolActivityKind::ThreadInfo | CoreToolActivityKind::SystemTime, false) => {
                "Inspected"
            }
        }
    }

    fn action_label(&self) -> &'static str {
        match self.kind {
            CoreToolActivityKind::File => "File",
            CoreToolActivityKind::ThreadInfo => "Thread info",
            CoreToolActivityKind::SystemTime => "System time",
        }
    }

    fn bullet(&self) -> Span<'static> {
        match self.status {
            CoreToolActivityStatus::InProgress => activity_indicator(
                Some(self.start_time),
                MotionMode::from_animations_enabled(self.animations_enabled),
                ReducedMotionIndicator::StaticBullet,
            )
            .unwrap_or_else(|| "•".dim()),
            CoreToolActivityStatus::Completed => "•".dim(),
            CoreToolActivityStatus::Failed => "•".red().bold(),
        }
    }

    fn action_line(&self) -> Line<'static> {
        let mut spans = vec![self.action_label().cyan(), " ".into()];
        if self.detail.trim().is_empty() {
            let _ = spans.pop();
        } else {
            spans.push(self.detail.clone().into());
        }
        Line::from(spans)
    }
}

impl HistoryCell for CoreToolActivityCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let mut lines = vec![vec![self.bullet(), " ".into(), self.heading().bold()].into()];
        let wrap_width = (width as usize).saturating_sub(4).max(1);
        let action_line = self.action_line();
        let wrapped = adaptive_wrap_line(
            &action_line,
            RtOptions::new(wrap_width)
                .initial_indent("".into())
                .subsequent_indent(" ".repeat(self.action_label().len() + 1).into()),
        );
        let body_lines: Vec<Line<'static>> = wrapped.iter().map(line_to_static).collect();
        lines.extend(prefix_lines(body_lines, "  └ ".dim(), "    ".into()));
        lines
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        let mut line = format!("{} {}", self.heading(), self.action_label());
        if !self.detail.trim().is_empty() {
            line.push(' ');
            line.push_str(&self.detail);
        }
        line.push_str(&format!(" ({})", self.tool_name));
        if let Some(error) = self.error.as_ref().filter(|error| !error.trim().is_empty()) {
            line.push_str(": ");
            line.push_str(error);
        }
        vec![Line::from(line)]
    }

    fn transcript_animation_tick(&self) -> Option<u64> {
        if !self.animations_enabled || !self.is_active() {
            return None;
        }
        Some((self.start_time.elapsed().as_millis() / 50) as u64)
    }
}

pub(crate) fn new_core_tool_activity_cell(
    item: ThreadItem,
    animations_enabled: bool,
) -> Option<CoreToolActivityCell> {
    let ThreadItem::CoreToolActivity {
        id,
        tool_name,
        kind,
        detail,
        status,
        error,
        ..
    } = item
    else {
        return None;
    };
    Some(CoreToolActivityCell::new(
        id,
        kind,
        detail,
        tool_name,
        status,
        error,
        animations_enabled,
    ))
}
