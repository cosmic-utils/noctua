// SPDX-License-Identifier: GPL-3.0-or-later
// src/ui/views/panels.rs
//
// Panel router - delegates to specific panel views.

use cosmic::Element;

use crate::application::DocumentManager;
use crate::ui::model::{AppModel, RightPanel};
use crate::ui::AppMessage;

use super::{format_panel, meta_panel};

/// Build the right panel view based on current panel state.
///
/// Returns the appropriate panel content:
/// - `RightPanel::Properties`: Metadata and document properties (default)
/// - `RightPanel::CropTools`: Crop tool controls (TODO)
/// - `RightPanel::TransformTools`: Transform/export controls
///
/// Defaults to Properties panel if no panel is explicitly set.
pub fn view(model: &AppModel, manager: &DocumentManager) -> Element<'static, AppMessage> {
    match model.panels.right.as_ref() {
        Some(RightPanel::Properties) | None => meta_panel::view(model, manager),
        Some(RightPanel::CropTools) => crop_tools_panel(model, manager),
        Some(RightPanel::TransformTools) => format_panel::view(model),
    }
}

/// Crop tools panel (TODO: implement dedicated crop controls).
fn crop_tools_panel(_model: &AppModel, _manager: &DocumentManager) -> Element<'static, AppMessage> {
    use cosmic::widget::{column, text};

    column::with_capacity(4)
        .spacing(12)
        .padding(12)
        .push(text::title4("Crop Tools"))
        .push(text::body("Crop controls will be implemented here."))
        .push(text::caption(
            "For now, use the crop overlay on the canvas.",
        ))
        .into()
}
