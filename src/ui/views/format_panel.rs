// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/format_panel.rs
//
// Format panel for paper format and orientation selection.

use cosmic::widget::{column, radio, text};
use cosmic::Element;

use crate::ui::model::{AppMode, AppModel, Orientation, PaperFormat};
use crate::ui::AppMessage;
use crate::fl;

/// Build the format panel view for the navigation bar.
pub fn view(model: &AppModel) -> Element<'static, AppMessage> {
    // Extract values from Transform mode
    let (paper_format, orientation) = match &model.mode {
        AppMode::Transform {
            paper_format,
            orientation,
        } => (*paper_format, *orientation),
        _ => (None, Orientation::default()),
    };

    let mut content = column::with_capacity(20).spacing(12).padding(16);

    // --- Format Section ---
    content = content
        .push(text::heading(fl!("format-section-title")))
        .push(text::caption(fl!("format-section-subtitle")));

    // US Letter
    content = content.push(
        radio(
            "US Letter (216 × 279 mm)",
            PaperFormat::UsLetter,
            paper_format,
            AppMessage::SetPaperFormat,
        )
        .size(16),
    );

    // ISO A formats
    content = content
        .push(text::body("ISO A"))
        .push(
            radio(
                PaperFormat::IsoA0.display_name(),
                PaperFormat::IsoA0,
                paper_format,
                AppMessage::SetPaperFormat,
            )
            .size(16),
        )
        .push(
            radio(
                PaperFormat::IsoA1.display_name(),
                PaperFormat::IsoA1,
                paper_format,
                AppMessage::SetPaperFormat,
            )
            .size(16),
        )
        .push(
            radio(
                PaperFormat::IsoA2.display_name(),
                PaperFormat::IsoA2,
                paper_format,
                AppMessage::SetPaperFormat,
            )
            .size(16),
        )
        .push(
            radio(
                PaperFormat::IsoA3.display_name(),
                PaperFormat::IsoA3,
                paper_format,
                AppMessage::SetPaperFormat,
            )
            .size(16),
        )
        .push(
            radio(
                PaperFormat::IsoA4.display_name(),
                PaperFormat::IsoA4,
                paper_format,
                AppMessage::SetPaperFormat,
            )
            .size(16),
        )
        .push(
            radio(
                PaperFormat::IsoA5.display_name(),
                PaperFormat::IsoA5,
                paper_format,
                AppMessage::SetPaperFormat,
            )
            .size(16),
        )
        .push(
            radio(
                PaperFormat::IsoA6.display_name(),
                PaperFormat::IsoA6,
                paper_format,
                AppMessage::SetPaperFormat,
            )
            .size(16),
        );

    // --- Orientation Section ---
    content = content
        .push(cosmic::widget::vertical_space().height(16))
        .push(text::heading(fl!("orientation-section-title")));

    // Horizontal
    content = content.push(
        radio(
            "Horizontal",
            Orientation::Horizontal,
            Some(orientation),
            AppMessage::SetOrientation,
        )
        .size(16),
    );

    // Vertical
    content = content.push(
        radio(
            "Vertical",
            Orientation::Vertical,
            Some(orientation),
            AppMessage::SetOrientation,
        )
        .size(16),
    );

    content.into()
}
