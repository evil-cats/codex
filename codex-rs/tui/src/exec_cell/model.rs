//! Data model for grouped exec-call history cells in the TUI transcript.
//!
//! An `ExecCell` can represent either a single command or an "exploring" group of related read/
//! list/search commands. The chat widget relies on stable `call_id` matching to route progress and
//! end events into the right cell, and it treats "call id not found" as a real signal (for
//! example, an orphan end that should render as a separate history entry).

use std::borrow::Cow;
use std::time::Duration;
use std::time::Instant;

use super::live_output::LiveCommandOutput;
use crate::history_cell::CoreToolActivityFileEntry;
use crate::history_cell::display_file_activity_detail;
use codex_app_server_protocol::CommandExecutionSource as ExecCommandSource;
use codex_app_server_protocol::CoreToolActivityKind;
use codex_app_server_protocol::CoreToolActivityStatus;
use codex_app_server_protocol::ThreadItem;
use codex_protocol::parse_command::ParsedCommand;
use itertools::Either;

const MAX_GROUPED_COMMANDS: usize = 32;

#[derive(Debug, Default)]
pub(crate) struct CommandOutput {
    pub(crate) exit_code: i32,
    /// The finalized, interleaved stderr and stdout that replaces any streamed preview.
    aggregated_output: String,
    /// The live preview while command-output deltas are still arriving.
    live_output: Option<LiveCommandOutput>,
}

impl CommandOutput {
    pub(crate) fn new(exit_code: i32, aggregated_output: String) -> Self {
        Self {
            exit_code,
            aggregated_output,
            live_output: None,
        }
    }

    /// Returns the total number of logical lines and the number retained for rendering.
    pub(super) fn line_counts(&self) -> (usize, usize) {
        match self.live_output.as_ref() {
            Some(output) => (output.total_lines(), output.retained_lines()),
            None => {
                let total = self.aggregated_output.lines().count();
                (total, total)
            }
        }
    }

    /// Returns retained preview lines with reverse traversal for efficient tail rendering.
    pub(super) fn lines(&self) -> impl DoubleEndedIterator<Item = Cow<'_, str>> {
        match self.live_output.as_ref() {
            Some(output) => Either::Left(output.lines()),
            None => Either::Right(self.aggregated_output.lines().map(Cow::Borrowed)),
        }
    }

    /// Returns lines for the expanded transcript, including any storage-level omission marker.
    pub(super) fn transcript_lines(&self) -> impl Iterator<Item = Cow<'_, str>> {
        match self.live_output.as_ref() {
            Some(output) => Either::Left(output.transcript_lines()),
            None => Either::Right(self.aggregated_output.lines().map(Cow::Borrowed)),
        }
    }
}

#[derive(Debug)]
pub(crate) struct ExecCall {
    pub(crate) call_id: String,
    pub(crate) command: Vec<String>,
    pub(crate) parsed: Vec<ParsedCommand>,
    pub(crate) output: Option<CommandOutput>,
    pub(crate) source: ExecCommandSource,
    pub(crate) start_time: Option<Instant>,
    pub(crate) duration: Option<Duration>,
    pub(crate) interaction_input: Option<String>,
    pub(crate) core_file_activity: Option<CoreFileActivity>,
}

#[derive(Debug, Clone)]
pub(crate) struct CoreFileActivity {
    pub(crate) detail: String,
    pub(crate) tool_name: String,
    pub(crate) status: CoreToolActivityStatus,
    pub(crate) error: Option<String>,
}

#[derive(Debug)]
pub(crate) struct ExecCell {
    pub(crate) calls: Vec<ExecCall>,
    animations_enabled: bool,
}

impl ExecCell {
    pub(crate) fn new(call: ExecCall, animations_enabled: bool) -> Self {
        Self {
            calls: vec![call],
            animations_enabled,
        }
    }

    pub(crate) fn add_call(
        &mut self,
        call_id: String,
        command: Vec<String>,
        parsed: Vec<ParsedCommand>,
        source: ExecCommandSource,
        interaction_input: Option<String>,
    ) -> bool {
        let call = ExecCall {
            call_id,
            command,
            parsed,
            output: None,
            source,
            start_time: Some(Instant::now()),
            duration: None,
            interaction_input,
            core_file_activity: None,
        };
        let has_failed_call = self.calls.iter().any(|existing| {
            existing
                .output
                .as_ref()
                .is_some_and(|output| output.exit_code != 0)
        });
        if (self.calls.len() >= MAX_GROUPED_COMMANDS && !self.is_active())
            || (!Self::is_groupable_source(call.source) && !self.is_active())
            || (has_failed_call && !self.is_active())
        {
            return false;
        }

        let continues_exploration = Self::is_exploring_call(&call)
            && (self.is_exploring_cell()
                || self.calls.last().is_some_and(|existing| {
                    existing.duration.is_none() && Self::is_exploring_call(existing)
                }))
            && (self.is_active()
                || self
                    .calls
                    .iter()
                    .all(|existing| Self::is_groupable_source(existing.source)));
        let continues_compact_group = self.calls.iter().all(|existing| {
            Self::is_groupable_source(existing.source)
                && existing.duration.is_some()
                && existing
                    .output
                    .as_ref()
                    .is_some_and(|output| output.exit_code == 0)
        });
        if continues_exploration || continues_compact_group {
            self.calls.push(call);
            true
        } else {
            false
        }
    }

    /// Marks the most recently matching call as finished and returns whether a call was found.
    ///
    /// Callers should treat `false` as a routing mismatch rather than silently ignoring it. The
    /// chat widget uses that signal to avoid attaching an orphan `exec_end` event to an unrelated
    /// active exploring cell, which would incorrectly collapse two transcript entries together.
    pub(crate) fn complete_call(
        &mut self,
        call_id: &str,
        output: CommandOutput,
        duration: Duration,
    ) -> bool {
        let Some(call) = self
            .calls
            .iter_mut()
            .rev()
            .find(|c| c.call_id == call_id && c.core_file_activity.is_none())
        else {
            return false;
        };
        call.output = Some(output);
        call.duration = Some(duration);
        call.start_time = None;
        true
    }

    pub(crate) fn try_add_core_file_activity(&mut self, item: &ThreadItem) -> bool {
        if !self.is_exploring_cell() {
            return false;
        }
        let Some(call) = ExecCall::from_core_file_activity(item) else {
            return false;
        };
        self.calls.push(call);
        true
    }

    pub(crate) fn add_core_file_activity_entry(&mut self, entry: CoreToolActivityFileEntry) {
        self.calls
            .push(ExecCall::from_core_file_activity_entry(entry));
    }

    pub(crate) fn complete_core_file_activity(&mut self, item: &ThreadItem) -> bool {
        let ThreadItem::CoreToolActivity {
            id,
            kind,
            status,
            error,
            duration_ms,
            ..
        } = item
        else {
            return false;
        };
        if *kind != CoreToolActivityKind::File {
            return false;
        }
        let Some(call) = self
            .calls
            .iter_mut()
            .rev()
            .find(|call| call.call_id == *id && call.core_file_activity.is_some())
        else {
            return false;
        };
        if let Some(activity) = call.core_file_activity.as_mut() {
            activity.status = *status;
            activity.error = error.clone();
        }
        call.start_time = None;
        call.duration = duration_ms.map(|ms| Duration::from_millis(ms.max(0) as u64));
        true
    }

    pub(crate) fn should_flush(&self) -> bool {
        if self.calls.iter().any(|call| {
            !Self::is_groupable_source(call.source)
                || call
                    .output
                    .as_ref()
                    .is_some_and(|output| output.exit_code != 0)
        }) {
            return !self.is_active();
        }

        if self.calls.len() >= MAX_GROUPED_COMMANDS {
            return !self.is_active();
        }

        if self.calls.iter().all(|call| {
            Self::is_groupable_source(call.source)
                && call.duration.is_some()
                && call
                    .output
                    .as_ref()
                    .is_some_and(|output| output.exit_code == 0)
        }) {
            return false;
        }

        !self.is_exploring_cell() && self.calls.iter().all(|c| c.duration.is_some())
    }

    pub(crate) fn mark_failed(&mut self) {
        for call in self.calls.iter_mut() {
            if let Some(activity) = call.core_file_activity.as_mut() {
                if matches!(activity.status, CoreToolActivityStatus::InProgress) {
                    let elapsed = call
                        .start_time
                        .map(|st| st.elapsed())
                        .unwrap_or_else(|| Duration::from_millis(0));
                    activity.status = CoreToolActivityStatus::Failed;
                    call.start_time = None;
                    call.duration = Some(elapsed);
                }
            } else if call.duration.is_none() {
                let elapsed = call
                    .start_time
                    .map(|st| st.elapsed())
                    .unwrap_or_else(|| Duration::from_millis(0));
                call.start_time = None;
                call.duration = Some(elapsed);
                call.output
                    .get_or_insert_with(CommandOutput::default)
                    .exit_code = 1;
            }
        }
    }

    pub(crate) fn is_exploring_cell(&self) -> bool {
        self.calls.iter().all(Self::is_exploring_call)
    }

    pub(crate) fn is_active(&self) -> bool {
        self.calls.iter().any(ExecCall::is_active)
    }

    pub(crate) fn active_start_time(&self) -> Option<Instant> {
        self.calls
            .iter()
            .find(|c| c.is_active())
            .and_then(|c| c.start_time)
    }

    pub(crate) fn animations_enabled(&self) -> bool {
        self.animations_enabled
    }

    pub(crate) fn iter_calls(&self) -> impl Iterator<Item = &ExecCall> {
        self.calls.iter()
    }

    pub(crate) fn append_output(&mut self, call_id: &str, chunk: &str) -> bool {
        if chunk.is_empty() {
            return false;
        }
        let Some(call) = self
            .calls
            .iter_mut()
            .rev()
            .find(|c| c.call_id == call_id && c.core_file_activity.is_none())
        else {
            return false;
        };
        let output = call.output.get_or_insert_with(CommandOutput::default);
        output
            .live_output
            .get_or_insert_with(LiveCommandOutput::default)
            .push_str(chunk);
        true
    }

    pub(super) fn is_exploring_call(call: &ExecCall) -> bool {
        call.core_file_activity.is_some()
            || (!matches!(call.source, ExecCommandSource::UserShell)
                && !call.parsed.is_empty()
                && call.parsed.iter().all(|p| {
                    matches!(
                        p,
                        ParsedCommand::Read { .. }
                            | ParsedCommand::ListFiles { .. }
                            | ParsedCommand::Search { .. }
                    )
                }))
    }

    fn is_groupable_source(source: ExecCommandSource) -> bool {
        matches!(
            source,
            ExecCommandSource::Agent | ExecCommandSource::UnifiedExecStartup
        )
    }
}

impl ExecCall {
    fn from_core_file_activity(item: &ThreadItem) -> Option<Self> {
        let ThreadItem::CoreToolActivity {
            id,
            tool_name,
            kind,
            detail,
            status,
            error,
            duration_ms,
            ..
        } = item
        else {
            return None;
        };
        if *kind != CoreToolActivityKind::File {
            return None;
        }
        Some(Self {
            call_id: id.clone(),
            command: Vec::new(),
            parsed: Vec::new(),
            output: None,
            source: ExecCommandSource::Agent,
            start_time: matches!(status, CoreToolActivityStatus::InProgress).then(Instant::now),
            duration: duration_ms.map(|ms| Duration::from_millis(ms.max(0) as u64)),
            interaction_input: None,
            core_file_activity: Some(CoreFileActivity {
                detail: display_file_activity_detail(detail),
                tool_name: tool_name.clone(),
                status: *status,
                error: error.clone(),
            }),
        })
    }

    fn from_core_file_activity_entry(entry: CoreToolActivityFileEntry) -> Self {
        Self {
            call_id: entry.id,
            command: Vec::new(),
            parsed: Vec::new(),
            output: None,
            source: ExecCommandSource::Agent,
            start_time: matches!(entry.status, CoreToolActivityStatus::InProgress)
                .then(Instant::now),
            duration: None,
            interaction_input: None,
            core_file_activity: Some(CoreFileActivity {
                detail: entry.detail,
                tool_name: entry.tool_name,
                status: entry.status,
                error: entry.error,
            }),
        }
    }

    fn is_active(&self) -> bool {
        if let Some(activity) = self.core_file_activity.as_ref() {
            matches!(activity.status, CoreToolActivityStatus::InProgress)
        } else {
            self.duration.is_none()
        }
    }

    fn is_complete(&self) -> bool {
        !self.is_active()
    }

    pub(crate) fn is_user_shell_command(&self) -> bool {
        matches!(self.source, ExecCommandSource::UserShell)
    }

    pub(crate) fn is_unified_exec_interaction(&self) -> bool {
        matches!(self.source, ExecCommandSource::UnifiedExecInteraction)
    }
}
