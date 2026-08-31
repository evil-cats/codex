//! Применяет exec spill к итоговому тексту Code Mode непосредственно перед ответом модели.

use codex_protocol::models::FunctionCallOutputContentItem;
use codex_utils_output_truncation::approx_token_count;

use super::ExecContext;
use crate::unified_exec::output_spill::effective_inline_output_max_tokens;
use crate::unified_exec::output_spill::maybe_spill_exec_command_output;
use crate::unified_exec::output_spill::render_output_spill;
use crate::unified_exec::resolve_max_tokens;

/// Идентифицирует внешний ответ инструмента, которому принадлежит файл Code Mode.
pub(super) struct RuntimeOutputIdentity<'a> {
    pub call_id: &'a str,
    pub cell_id: &'a str,
}

/// Различает обычный результат и уже ограниченный построчный spill.
pub(super) enum RuntimeOutputDelivery {
    Inline(Vec<FunctionCallOutputContentItem>),
    Spilled {
        items: Vec<FunctionCallOutputContentItem>,
        excerpt_token_count: usize,
    },
}

/// Сохраняет полное текстовое представление большого результата и заменяет только
/// его текстовые элементы ограниченным сообщением для модели. Мультимедийные
/// элементы сохраняют исходный порядок и остаются под общей политикой Code Mode.
pub(super) async fn maybe_spill_runtime_output(
    exec: &ExecContext,
    items: Vec<FunctionCallOutputContentItem>,
    identity: RuntimeOutputIdentity<'_>,
    max_output_tokens: Option<usize>,
) -> RuntimeOutputDelivery {
    let projection = text_projection(&items);
    let original_token_count = approx_token_count(&projection);
    let inline_limit = effective_inline_output_max_tokens(
        exec.turn.config.exec_inline_output_max_tokens,
        Some(resolve_max_tokens(max_output_tokens)),
        exec.turn.model_info().truncation_policy.into(),
    );
    let thread_id = exec.session.thread_id().to_string();
    let Some(spill) = maybe_spill_exec_command_output(
        &exec.turn.config.codex_home,
        &thread_id,
        identity.call_id,
        identity.cell_id,
        projection.as_bytes(),
        original_token_count,
        inline_limit,
    )
    .await
    else {
        return RuntimeOutputDelivery::Inline(items);
    };

    let rendered = render_output_spill(projection.as_bytes(), &spill);
    RuntimeOutputDelivery::Spilled {
        items: replace_text_items(
            items,
            FunctionCallOutputContentItem::InputText {
                text: rendered.body,
            },
        ),
        excerpt_token_count: rendered.excerpt_token_count,
    }
}

/// Объединяет текстовые элементы тем же одиночным `\n`, который использует усечение.
fn text_projection(items: &[FunctionCallOutputContentItem]) -> String {
    items
        .iter()
        .filter_map(|item| match item {
            FunctionCallOutputContentItem::InputText { text } => Some(text.as_str()),
            FunctionCallOutputContentItem::InputImage { .. }
            | FunctionCallOutputContentItem::InputAudio { .. }
            | FunctionCallOutputContentItem::EncryptedContent { .. } => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Вставляет единое spill-сообщение вместо первого текста и сохраняет порядок медиа.
fn replace_text_items(
    items: Vec<FunctionCallOutputContentItem>,
    replacement: FunctionCallOutputContentItem,
) -> Vec<FunctionCallOutputContentItem> {
    let mut replacement = Some(replacement);
    items
        .into_iter()
        .filter_map(|item| match item {
            FunctionCallOutputContentItem::InputText { .. } => replacement.take(),
            FunctionCallOutputContentItem::InputImage { .. }
            | FunctionCallOutputContentItem::InputAudio { .. }
            | FunctionCallOutputContentItem::EncryptedContent { .. } => Some(item),
        })
        .collect()
}

#[cfg(test)]
#[path = "output_spill_tests.rs"]
mod tests;
