//! Подсчёт строк текущего resumed thread и применение общего row budget в `App`.

use super::App;
use crate::history_cell::HistoryCellDisplayItem;
use crate::history_cell::HistoryRowCalculator;
use crate::history_cell::SessionInfoCell;

impl App {
    pub(super) fn history_row_calculator(&self, width: u16) -> HistoryRowCalculator {
        HistoryRowCalculator::new(
            width,
            self.chat_widget.history_render_mode(),
            self.config.history_image_preview,
        )
    }

    pub(super) fn current_thread_history_rows(&self, width: u16) -> usize {
        let start = self
            .transcript_cells
            .iter()
            .rposition(|cell| cell.as_any().is::<SessionInfoCell>())
            .map_or(/*default*/ 0, |index| index.saturating_add(/*rhs*/ 1));
        self.history_row_calculator(width).appended_cells_rows(
            self.transcript_cells[start..]
                .iter()
                .map(std::convert::AsRef::as_ref),
            /*initial_rows*/ 0,
        )
    }

    pub(super) fn trim_display_items_to_row_cap(
        items: &mut Vec<HistoryCellDisplayItem>,
        max_rows: usize,
        row_calculator: HistoryRowCalculator,
    ) -> bool {
        let mut remaining_rows = row_calculator.display_items_rows(items);
        if remaining_rows <= max_rows {
            return false;
        }

        let mut trimmed_items = 0;
        for item in items.iter().take(items.len().saturating_sub(1)) {
            if remaining_rows <= max_rows {
                break;
            }
            remaining_rows = remaining_rows.saturating_sub(row_calculator.display_item_rows(item));
            trimmed_items += 1;
        }
        if trimmed_items != 0 {
            items.drain(..trimmed_items);
        }
        true
    }
}
