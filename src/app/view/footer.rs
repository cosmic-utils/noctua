// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/footer.rs
//
// Footer bar with zoom controls and document info.

use cosmic::iced::Alignment;
use cosmic::widget::{button, icon, row, text};
use cosmic::Element;

use crate::app::model::{AppModel, ViewMode};
use crate::app::AppMessage;

/// Build the footer element with zoom controls and document info.
pub fn view(model: &AppModel) -> Element<'_, AppMessage> {
    // Zoom level display.
    let zoom_text = match model.view_mode {
        ViewMode::Fit => "Fit".to_string(),
        ViewMode::ActualSize => "100%".to_string(),
        ViewMode::Custom(z) => format!("{}%", (z * 100.0).round() as i32),
    };

    // Document dimensions (if available).
    let doc_info = if let Some(ref doc) = model.document {
        let (w, h) = doc.dimensions();
        format!("{}×{}", w, h)
    } else {
        String::new()
    };

    // Navigation position (e.g., "3 / 42").
    let nav_info = if !model.folder_entries.is_empty() {
        let current = model.current_index.map(|i| i + 1).unwrap_or(0);
        let total = model.folder_entries.len();
        format!("{} / {}", current, total)
    } else {
        String::new()
    };

    row()
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([4, 12])
        // Zoom out button.
        .push(
            button::icon(icon::from_name("zoom-out-symbolic"))
                .on_press(AppMessage::ZoomOut)
                .padding(4),
        )
        // Zoom level display.
        .push(text::body(zoom_text))
        // Zoom in button.
        .push(
            button::icon(icon::from_name("zoom-in-symbolic"))
                .on_press(AppMessage::ZoomIn)
                .padding(4),
        )
        // Fit button.
        .push(
            button::icon(icon::from_name("zoom-fit-best-symbolic"))
                .on_press(AppMessage::ZoomFit)
                .padding(4),
        )
        // Spacer.
        .push(cosmic::widget::horizontal_space())
        // Document dimensions.
        .push(text::body(doc_info))
        // Separator.
        .push_maybe(if !model.folder_entries.is_empty() {
            Some(text::body("  |  "))
        } else {
            None
        })
        // Navigation position.
        .push(text::body(nav_info))
        .into()
}
