//! History cell for controlled assistant/tool local image previews.

use super::*;

#[derive(Debug)]
pub(crate) struct LocalImageHistoryCell {
    path: PathBuf,
    caption: Option<String>,
    preview_size: ImagePreviewSize,
}

impl LocalImageHistoryCell {
    pub(crate) fn new(
        path: PathBuf,
        caption: Option<String>,
        preview_size: ImagePreviewSize,
    ) -> Self {
        Self {
            path,
            caption,
            preview_size,
        }
    }

    fn fallback_text(&self) -> String {
        match self
            .caption
            .as_deref()
            .map(str::trim)
            .filter(|caption| !caption.is_empty())
        {
            Some(caption) => format!("[Image: {}]", caption.replace(['\r', '\n'], " ")),
            None => "[Image]".to_string(),
        }
    }

    fn fallback_line(&self) -> Line<'static> {
        Line::from(self.fallback_text())
    }
}

impl HistoryCell for LocalImageHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let wrap_width = width.saturating_sub(/*prefix*/ 2).max(1);
        adaptive_wrap_lines(
            [self.fallback_line()],
            RtOptions::new(usize::from(wrap_width))
                .initial_indent("• ".dim().into())
                .subsequent_indent("  ".into()),
        )
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        vec![self.fallback_line()]
    }

    fn display_items_for_mode(
        &self,
        width: u16,
        mode: HistoryRenderMode,
    ) -> Vec<HistoryCellDisplayItem> {
        match mode {
            HistoryRenderMode::Rich => {
                let mut items = self
                    .display_lines(width)
                    .into_iter()
                    .map(HistoryCellDisplayItem::from)
                    .collect::<Vec<_>>();
                items.push(HistoryCellDisplayItem::LocalImage {
                    path: self.path.clone(),
                    preview_size: self.preview_size,
                });
                items
            }
            HistoryRenderMode::Raw => self
                .raw_lines()
                .into_iter()
                .map(HistoryCellDisplayItem::from)
                .collect(),
        }
    }
}

pub(crate) fn new_local_image(
    path: PathBuf,
    caption: Option<String>,
    preview_size: ImagePreviewSize,
) -> LocalImageHistoryCell {
    LocalImageHistoryCell::new(path, caption, preview_size)
}
