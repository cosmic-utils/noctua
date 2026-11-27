// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/widget/text_format.rs
//
// Floating format bar shown while a free-text annotation is being entered. Two
// rows: font family and size on top; bold, italic, underline, strikethrough,
// background colour, text colour and alignment below. Every change publishes
// the whole style through `AnnotationStyleChanged`, so the edit is restyled in
// place.

use cosmic::prelude::*;
use cosmic::widget;

use noctua_core::pdfium_ops::model::{AnnotationColor, FontFamily, FreeTextStyle, TextAlign};

use crate::fl;
use crate::message::{AnnotationMessage, Message};

/// Free-text font size presets, in points.
pub(crate) const FONT_SIZE_PRESETS: [f32; 8] = [12.0, 16.0, 20.0, 24.0, 32.0, 48.0, 64.0, 96.0];

/// Standard font families and their display names.
const FONT_FAMILIES: [(FontFamily, &str); 3] = [
    (FontFamily::Helvetica, "Helvetica"),
    (FontFamily::Times, "Times"),
    (FontFamily::Courier, "Courier"),
];

/// Preset text colours (label, colour).
fn text_color_presets() -> Vec<(String, AnnotationColor)> {
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

/// Preset background colours; `None` means transparent.
fn background_presets() -> Vec<(String, Option<AnnotationColor>)> {
    let mut presets = vec![(fl!("no-background"), None)];
    presets.extend(
        text_color_presets()
            .into_iter()
            .map(|(label, color)| (label, Some(color))),
    );
    presets
}

/// The floating format bar for a free-text edit.
pub(crate) fn text_format_bar(style: FreeTextStyle) -> Element<'static, Message> {
    let space = cosmic::theme::spacing();

    let family_labels: Vec<String> = FONT_FAMILIES
        .iter()
        .map(|(_, name)| (*name).to_string())
        .collect();
    let selected_family = FONT_FAMILIES
        .iter()
        .position(|(family, _)| *family == style.family);
    let family_dropdown = widget::dropdown(family_labels, selected_family, move |index| {
        Message::Annotation(AnnotationMessage::AnnotationStyleChanged(FreeTextStyle {
            family: FONT_FAMILIES[index].0,
            ..style
        }))
    });

    let size_labels: Vec<String> = FONT_SIZE_PRESETS
        .iter()
        .map(|size| format!("{size:.0} pt"))
        .collect();
    let selected_size = FONT_SIZE_PRESETS
        .iter()
        .position(|size| (*size - style.font_size).abs() < f32::EPSILON);
    let size_dropdown = widget::dropdown(size_labels, selected_size, move |index| {
        Message::Annotation(AnnotationMessage::AnnotationStyleChanged(FreeTextStyle {
            font_size: FONT_SIZE_PRESETS[index],
            ..style
        }))
    });

    let colors = text_color_presets();
    let color_labels: Vec<String> = colors.iter().map(|(name, _)| name.clone()).collect();
    let selected_color = colors.iter().position(|(_, color)| *color == style.color);
    let color_dropdown = widget::dropdown(color_labels, selected_color, move |index| {
        Message::Annotation(AnnotationMessage::AnnotationStyleChanged(FreeTextStyle {
            color: colors[index].1,
            ..style
        }))
    });

    let backgrounds = background_presets();
    let background_labels: Vec<String> = backgrounds.iter().map(|(name, _)| name.clone()).collect();
    let selected_background = backgrounds
        .iter()
        .position(|(_, color)| *color == style.background);
    let background_dropdown =
        widget::dropdown(background_labels, selected_background, move |index| {
            Message::Annotation(AnnotationMessage::AnnotationStyleChanged(FreeTextStyle {
                background: backgrounds[index].1,
                ..style
            }))
        });

    let top = widget::row::with_capacity(2)
        .spacing(space.space_xxs)
        .push(family_dropdown)
        .push(size_dropdown);

    let bottom = widget::row::with_capacity(10)
        .spacing(space.space_xxs)
        .push(toggle(
            "format-text-bold-symbolic",
            fl!("bold"),
            style.bold,
            FreeTextStyle {
                bold: !style.bold,
                ..style
            },
        ))
        .push(toggle(
            "format-text-italic-symbolic",
            fl!("italic"),
            style.italic,
            FreeTextStyle {
                italic: !style.italic,
                ..style
            },
        ))
        .push(toggle(
            "format-text-underline-symbolic",
            fl!("underline"),
            style.underline,
            FreeTextStyle {
                underline: !style.underline,
                ..style
            },
        ))
        .push(toggle(
            "format-text-strikethrough-symbolic",
            fl!("strikethrough"),
            style.strikethrough,
            FreeTextStyle {
                strikethrough: !style.strikethrough,
                ..style
            },
        ))
        .push(background_dropdown)
        .push(color_dropdown)
        .push(align_button(style, TextAlign::Left))
        .push(align_button(style, TextAlign::Center))
        .push(align_button(style, TextAlign::Right))
        .push(align_button(style, TextAlign::Justify));

    widget::container(
        widget::column::with_capacity(2)
            .spacing(space.space_xxs)
            .push(top)
            .push(bottom),
    )
    .padding(space.space_xs)
    .class(cosmic::style::Container::Primary)
    .into()
}

/// A style toggle button (bold, italic, underline, strikethrough).
fn toggle(
    icon: &str,
    tooltip: String,
    selected: bool,
    style: FreeTextStyle,
) -> Element<'static, Message> {
    widget::button::icon(widget::icon::from_name(icon))
        .tooltip(tooltip)
        .selected(selected)
        .on_press(Message::Annotation(
            AnnotationMessage::AnnotationStyleChanged(style),
        ))
        .into()
}

/// One alignment button of the four-way group.
fn align_button(style: FreeTextStyle, align: TextAlign) -> Element<'static, Message> {
    let (icon, tooltip) = match align {
        TextAlign::Left => ("format-justify-left-symbolic", fl!("align-left")),
        TextAlign::Center => ("format-justify-center-symbolic", fl!("align-center")),
        TextAlign::Right => ("format-justify-right-symbolic", fl!("align-right")),
        TextAlign::Justify => ("format-justify-fill-symbolic", fl!("align-justify")),
    };
    widget::button::icon(widget::icon::from_name(icon))
        .tooltip(tooltip)
        .selected(style.align == align)
        .on_press(Message::Annotation(
            AnnotationMessage::AnnotationStyleChanged(FreeTextStyle { align, ..style }),
        ))
        .into()
}
