//! Выбирает политику сохранения tool output в model-visible history.

use crate::tools::handlers::read_file_spec::READ_FILE_TOOL_NAME;
use codex_protocol::models::ResponseItem;

/// Определяет, требуется ли общая model-default обработка tool output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToolOutputHistoryPolicy {
    ModelDefault,
    AlreadyBounded,
}

impl ToolOutputHistoryPolicy {
    /// Определяет политику по фактической typed-паре в текущей history.
    ///
    /// `read_file` уже ограничивает содержимое собственным config budget, поэтому
    /// повторное усечение нарушило бы его `complete=yes`. Поиск по history, а не
    /// отдельный cache, сохраняет одинаковый результат после replay, rollback и
    /// замены history при compaction.
    pub(super) fn for_item<'a>(
        item: &ResponseItem,
        history: impl DoubleEndedIterator<Item = &'a ResponseItem>,
    ) -> Self {
        let ResponseItem::FunctionCallOutput { call_id, .. } = item else {
            return Self::ModelDefault;
        };

        history
            .rev()
            .find_map(|history_item| match history_item {
                ResponseItem::FunctionCall {
                    name,
                    call_id: candidate_call_id,
                    ..
                } if candidate_call_id == call_id => Some(if name == READ_FILE_TOOL_NAME {
                    Self::AlreadyBounded
                } else {
                    Self::ModelDefault
                }),
                _ => None,
            })
            .unwrap_or(Self::ModelDefault)
    }
}
