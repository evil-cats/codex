//! Проверяет текстовое представление и сохранение мультимедиа при внешнем spill Code Mode.

use codex_protocol::models::FunctionCallOutputContentItem;
use pretty_assertions::assert_eq;

use super::replace_text_items;
use super::text_projection;

/// Представление объединяет только текстовые элементы в исходном порядке через один `\n`.
#[test]
fn text_projection_preserves_text_item_order() {
    let items = vec![
        FunctionCallOutputContentItem::InputText {
            text: "first".to_string(),
        },
        FunctionCallOutputContentItem::InputImage {
            image_url: "data:image/png;base64,AA==".to_string(),
            detail: None,
        },
        FunctionCallOutputContentItem::InputText {
            text: "second".to_string(),
        },
    ];

    assert_eq!(text_projection(&items), "first\nsecond");
}

/// Единое spill-сообщение заменяет весь текст на позиции первого и не двигает мультимедиа.
#[test]
fn replacement_preserves_media_item_order() {
    let image = FunctionCallOutputContentItem::InputImage {
        image_url: "data:image/png;base64,AA==".to_string(),
        detail: None,
    };
    let audio = FunctionCallOutputContentItem::InputAudio {
        audio_url: "data:audio/wav;base64,AA==".to_string(),
    };
    let replacement = FunctionCallOutputContentItem::InputText {
        text: "spill".to_string(),
    };

    assert_eq!(
        replace_text_items(
            vec![
                image.clone(),
                FunctionCallOutputContentItem::InputText {
                    text: "first".to_string(),
                },
                audio.clone(),
                FunctionCallOutputContentItem::InputText {
                    text: "second".to_string(),
                },
            ],
            replacement.clone(),
        ),
        vec![image, replacement, audio]
    );
}
