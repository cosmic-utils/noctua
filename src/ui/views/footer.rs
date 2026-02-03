// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/footer.rs
//
// Footer bar with zoom controls and document info.

use cosmic::iced::Alignment;
use cosmic::widget::{button, icon, row, text};
use cosmic::Element;

use crate::ui::model::{AppModel, ViewMode};
use crate::ui::AppMessage;
use crate::application::DocumentManager;
use crate::fl;

/// Build the footer element with zoom controls and document info.
pub fn view<'a>(model: &'a AppModel, _manager: &'a DocumentManager) -> Element<'a, AppMessage> {
    // Zoom level display - use scale as single source of truth.
    let zoom_text = if model.view_mode == ViewMode::Fit {
        fl!("status-zoom-fit")
    } else {
        // Use scale directly for accurate zoom display
        let percent = (model.scale * 100.0).round() as i32;
        fl!("status-zoom-percent", percent: percent)
    };

    // Document dimensions (current after transformations).
    let doc_info = if let Some((w, h)) = model.current_dimensions {
        fl!("status-doc-dimensions", width: w, height: h)
    } else {
        String::new()
    };

    // Navigation position (e.g., "3 / 42").
    let nav_info = if model.folder_count == 0 {
        String::new()
    } else {
        let current = model.current_index.map_or(0, |i| i + 1);
        let total = model.folder_count;
        fl!("status-nav-position", current: current, total: total)
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
        .push_maybe(if model.folder_count == 0 {
            None
        } else {
            Some(text::body(fl!("status-separator")))
        })
        // Navigation position.
        .push(text::body(nav_info))
        .into()
}
