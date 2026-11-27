// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/widget/text_format.rs
//
// Floating format bar shown while a free-text annotation is being entered:
// font size and colour presets. Reused by the annotation view.

use cosmic::prelude::*;
use cosmic::widget;

use noctua_core::pdfium_ops::model::AnnotationColor;

use crate::fl;
use crate::message::{AnnotationMessage, Message};

/// Free-text font size presets, in points.
pub(crate) const FONT_SIZE_PRESETS: [f32; 8] = [12.0, 16.0, 20.0, 24.0, 32.0, 48.0, 64.0, 96.0];

/// Preset text colours for the free-text tool (label, colour).
pub(crate) fn text_color_presets() -> Vec<(String, AnnotationColor)> {
    use noctua_core::pdfium_ops::model::palette;
    vec![
        (fl!("color-black"), AnnotationColor::rgb(0, 0, 0)),
        (fl!("color-white"), AnnotationColor::rgb(255, 255, 255)),
        (fl!("color-red"), palette::RED),
        (fl!("color-orange"), palette::ORANGE),
        (fl!("color-yellow"), palette::YELLOW),
        (fl!("color-green"), palette::GREEN),
        (fl!("color-blue"), palette::BLUE),
    ]
}

/// The floating format bar: font size and colour presets. Publishing goes
/// through the existing `AnnotationFontSizeChanged`/`AnnotationColorChanged`
/// messages, so the free-text edit is restyled in place.
pub(crate) fn text_format_bar(font_size: f32, color: AnnotationColor) -> Element<'static, Message> {
    let space = cosmic::theme::spacing();

    let size_labels: Vec<String> = FONT_SIZE_PRESETS
        .iter()
        .map(|size| format!("{size:.0} pt"))
        .collect();
    let selected_size = FONT_SIZE_PRESETS
        .iter()
        .position(|size| (*size - font_size).abs() < f32::EPSILON);

    let colors = text_color_presets();
    let color_labels: Vec<String> = colors.iter().map(|(name, _)| name.clone()).collect();
    let selected_color = colors.iter().position(|(_, preset)| *preset == color);

    widget::row::with_capacity(2)
        .spacing(space.space_xxs)
        .push(widget::dropdown(size_labels, selected_size, |index| {
            Message::Annotation(AnnotationMessage::AnnotationFontSizeChanged(
                FONT_SIZE_PRESETS[index],
            ))
        }))
        .push(widget::dropdown(
            color_labels,
            selected_color,
            move |index| {
                Message::Annotation(AnnotationMessage::AnnotationColorChanged(colors[index].1))
            },
        ))
        .into()
}
