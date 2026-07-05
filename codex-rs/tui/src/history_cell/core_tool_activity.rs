//! Core function-tool activity history cells.

use super::*;
use codex_app_server_protocol::CoreToolActivityKind;
use codex_app_server_protocol::CoreToolActivityStatus;
use codex_app_server_protocol::ThreadItem;

#[derive(Debug)]
pub(crate) struct CoreToolActivityCell {
    kind: CoreToolActivityKind,
    entries: Vec<CoreToolActivityEntry>,
    start_time: Instant,
    animations_enabled: bool,
}

#[derive(Debug, Clone)]
struct CoreToolActivityEntry {
    id: String,
    detail: String,
    tool_name: String,
    status: CoreToolActivityStatus,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct CoreToolActivityFileEntry {
    pub(crate) id: String,
    pub(crate) detail: String,
    pub(crate) tool_name: String,
    pub(crate) status: CoreToolActivityStatus,
    pub(crate) error: Option<String>,
}

impl CoreToolActivityEntry {
    fn new(
        id: String,
        kind: CoreToolActivityKind,
        detail: String,
        tool_name: String,
        status: CoreToolActivityStatus,
        error: Option<String>,
    ) -> Self {
        Self {
            id,
            detail: display_detail(kind, detail),
            tool_name,
            status,
            error,
        }
    }
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
            kind,
            entries: vec![CoreToolActivityEntry::new(
                id, kind, detail, tool_name, status, error,
            )],
            start_time: Instant::now(),
            animations_enabled,
        }
    }

    fn entry_from_item(item: ThreadItem) -> Option<(CoreToolActivityKind, CoreToolActivityEntry)> {
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
        let entry = CoreToolActivityEntry {
            id,
            detail: display_detail(kind, detail),
            tool_name,
            status,
            error,
        };
        Some((kind, entry))
    }

    pub(crate) fn contains_call_id(&self, id: &str) -> bool {
        self.entries.iter().any(|entry| entry.id == id)
    }

    pub(crate) fn try_add_file_activity(&mut self, item: ThreadItem) -> bool {
        if self.kind != CoreToolActivityKind::File {
            return false;
        }
        let Some((kind, entry)) = Self::entry_from_item(item) else {
            return false;
        };
        if kind != CoreToolActivityKind::File {
            return false;
        }
        self.entries.push(entry);
        true
    }

    pub(crate) fn file_activity_entries(&self) -> Option<Vec<CoreToolActivityFileEntry>> {
        if self.kind != CoreToolActivityKind::File {
            return None;
        }
        Some(
            self.entries
                .iter()
                .map(|entry| CoreToolActivityFileEntry {
                    id: entry.id.clone(),
                    detail: entry.detail.clone(),
                    tool_name: entry.tool_name.clone(),
                    status: entry.status,
                    error: entry.error.clone(),
                })
                .collect(),
        )
    }

    pub(crate) fn complete(
        &mut self,
        id: &str,
        status: CoreToolActivityStatus,
        error: Option<String>,
    ) -> bool {
        let Some(entry) = self.entries.iter_mut().rev().find(|entry| entry.id == id) else {
            return false;
        };
        entry.status = status;
        entry.error = error;
        true
    }

    pub(crate) fn should_flush_on_complete(&self) -> bool {
        self.kind != CoreToolActivityKind::File
    }

    fn is_file_activity(&self) -> bool {
        self.kind == CoreToolActivityKind::File
    }

    fn details(&self) -> Vec<String> {
        if self.is_file_activity() {
            let mut details: Vec<String> = Vec::new();
            for entry in &self.entries {
                let detail = entry.detail.trim();
                if !detail.is_empty() && !details.iter().any(|existing| existing.as_str() == detail)
                {
                    details.push(detail.to_string());
                }
            }
            details
        } else {
            self.entries
                .first()
                .map(|entry| entry.detail.trim().to_string())
                .filter(|detail| !detail.is_empty())
                .into_iter()
                .collect()
        }
    }

    fn has_failed(&self) -> bool {
        self.entries
            .iter()
            .any(|entry| matches!(entry.status, CoreToolActivityStatus::Failed))
    }

    fn is_active(&self) -> bool {
        self.entries
            .iter()
            .any(|entry| matches!(entry.status, CoreToolActivityStatus::InProgress))
    }

    fn heading(&self) -> &'static str {
        self.heading_for_active(self.is_active())
    }

    fn heading_for_status(&self, status: CoreToolActivityStatus) -> &'static str {
        self.heading_for_active(matches!(status, CoreToolActivityStatus::InProgress))
    }

    fn heading_for_active(&self, active: bool) -> &'static str {
        match (self.kind, active) {
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
        if self.is_active() {
            activity_indicator(
                Some(self.start_time),
                MotionMode::from_animations_enabled(self.animations_enabled),
                ReducedMotionIndicator::StaticBullet,
            )
            .unwrap_or_else(|| "•".dim())
        } else if self.has_failed() {
            "•".red().bold()
        } else {
            "•".dim()
        }
    }

    fn action_line(&self) -> Line<'static> {
        let mut spans = vec![self.action_label().cyan()];
        let details = self.details();
        if !details.is_empty() {
            spans.push(" ".into());
            for (index, detail) in details.into_iter().enumerate() {
                if index > 0 {
                    spans.push(", ".dim());
                }
                spans.push(detail.into());
            }
        }
        Line::from(spans)
    }
}

fn display_detail(kind: CoreToolActivityKind, detail: String) -> String {
    if kind == CoreToolActivityKind::File {
        display_file_activity_detail(&detail)
    } else {
        detail
    }
}

pub(crate) fn display_file_activity_detail(detail: &str) -> String {
    short_display_path(strip_line_range_suffix(detail.trim()))
}

fn strip_line_range_suffix(detail: &str) -> &str {
    let Some((prefix, suffix)) = detail.rsplit_once(':') else {
        return detail;
    };
    if is_line_range(suffix) {
        prefix
    } else {
        detail
    }
}

fn is_line_range(value: &str) -> bool {
    let Some((start, end)) = value.split_once('-') else {
        return false;
    };
    !start.is_empty()
        && !end.is_empty()
        && start.chars().all(|ch| ch.is_ascii_digit())
        && end.chars().all(|ch| ch.is_ascii_digit())
}

fn short_display_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let trimmed = normalized.trim_end_matches('/');
    let mut parts = trimmed.split('/').rev().filter(|part| {
        !part.is_empty()
            && *part != "build"
            && *part != "dist"
            && *part != "node_modules"
            && *part != "src"
    });
    parts
        .next()
        .map(str::to_string)
        .unwrap_or_else(|| trimmed.to_string())
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
        self.entries
            .iter()
            .map(|entry| {
                let mut line = format!(
                    "{} {}",
                    self.heading_for_status(entry.status),
                    self.action_label()
                );
                if !entry.detail.trim().is_empty() {
                    line.push(' ');
                    line.push_str(&entry.detail);
                }
                line.push_str(&format!(" ({})", entry.tool_name));
                if let Some(error) = entry
                    .error
                    .as_ref()
                    .filter(|error| !error.trim().is_empty())
                {
                    line.push_str(": ");
                    line.push_str(error);
                }
                Line::from(line)
            })
            .collect()
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
