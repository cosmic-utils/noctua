// SPDX-License-Identifier: GPL-3.0-or-later
// src/ui/views/footer.rs
//
// Footer bar with zoom controls and document info.

use cosmic::iced::Alignment;
use cosmic::widget::{button, icon, row, text};
use cosmic::Element;

use crate::ui::model::{AppModel, ViewMode};
use crate::ui::AppMessage;
use crate::application::DocumentManager;
use crate::domain::document::core::document::Renderable;
use crate::fl;

/// Build the footer element with zoom controls and document info.
pub fn view<'a>(model: &'a AppModel, manager: &'a DocumentManager) -> Element<'a, AppMessage> {
    // Zoom level display
    let zoom_text = if model.viewport.fit_mode == ViewMode::Fit {
        fl!("status-zoom-fit")
    } else {
        let percent = (model.viewport.scale * 100.0).round() as i32;
        fl!("status-zoom-percent", percent: percent)
    };

    // Document dimensions (from DocumentManager)
    let doc_info = if let Some(doc) = manager.current_document() {
        let info = doc.info();
        fl!("status-doc-dimensions", width: info.width, height: info.height)
    } else {
        String::new()
    };

    // Navigation position (from DocumentManager)
    let folder_count = manager.folder_entries().len();
    let nav_info = if folder_count == 0 {
        String::new()
    } else {
        let current = manager.current_index().map_or(0, |i| i + 1);
        let total = folder_count;
        fl!("status-nav-position", current: current, total: total)
    };

    row()
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([4, 12])
        // Zoom out button
        .push(
            button::icon(icon::from_name("zoom-out-symbolic"))
                .on_press(AppMessage::ZoomOut)
                .padding(4),
        )
        // Zoom level text
        .push(text(zoom_text))
        // Zoom in button
        .push(
            button::icon(icon::from_name("zoom-in-symbolic"))
                .on_press(AppMessage::ZoomIn)
                .padding(4),
        )
        // Zoom reset button
        .push(
            button::icon(icon::from_name("zoom-original-symbolic"))
                .on_press(AppMessage::ZoomReset)
                .padding(4),
        )
        // Zoom fit button
        .push(
            button::icon(icon::from_name("zoom-fit-best-symbolic"))
                .on_press(AppMessage::ZoomFit)
                .padding(4),
        )
        // Document dimensions
        .push_maybe(if !doc_info.is_empty() {
            Some(text(doc_info))
        } else {
            None
        })
        // Navigation info
        .push_maybe(if folder_count == 0 {
            None
        } else {
            Some(text(nav_info))
        })
        .into()
}
