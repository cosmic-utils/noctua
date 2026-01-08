// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/update.rs
//
// Application update loop: applies messages to the global model state.

use super::document;
use super::message::AppMessage;
use super::model::{AppModel, ToolMode, ViewMode, PAN_STEP};

/// Central update function applying messages to the model.
///
/// This is the single place where application state is mutated.
pub fn update(model: &mut AppModel, msg: AppMessage) {
    println!("update(): received message: {:?}", msg);

    match msg {
        // ===== File / navigation ==========================================================
        AppMessage::OpenPath(path) => {
            document::file::open_single_file(model, &path);
        }

        AppMessage::NextDocument => {
            document::file::navigate_next(model);
        }

        AppMessage::PrevDocument => {
            document::file::navigate_prev(model);
        }

        // ===== Panels =====================================================================
        AppMessage::ToggleLeftPanel => {
            model.show_left_panel = !model.show_left_panel;
        }
        AppMessage::ToggleRightPanel => {
            model.show_right_panel = !model.show_right_panel;
        }

        // ===== View / zoom ===============================================================
        AppMessage::ZoomIn => zoom_in(model),
        AppMessage::ZoomOut => zoom_out(model),
        AppMessage::ZoomReset => {
            model.view_mode = ViewMode::ActualSize;
            model.reset_pan();
        }
        AppMessage::ZoomFit => {
            model.view_mode = ViewMode::Fit;
            model.reset_pan();
        }

        // ===== Pan control (Ctrl + arrow keys) ===========================================
        AppMessage::PanLeft => {
            model.pan_x -= PAN_STEP;
        }
        AppMessage::PanRight => {
            model.pan_x += PAN_STEP;
        }
        AppMessage::PanUp => {
            model.pan_y -= PAN_STEP;
        }
        AppMessage::PanDown => {
            model.pan_y += PAN_STEP;
        }
        AppMessage::PanReset => {
            model.reset_pan();
        }

        // ===== Tools =====================================================================
        AppMessage::ToggleCropMode => {
            model.tool_mode = if model.tool_mode == ToolMode::Crop {
                ToolMode::None
            } else {
                ToolMode::Crop
            };
        }
        AppMessage::ToggleScaleMode => {
            model.tool_mode = if model.tool_mode == ToolMode::Scale {
                ToolMode::None
            } else {
                ToolMode::Scale
            };
        }

        // ===== Document transformations ==================================================
        AppMessage::FlipHorizontal => {
            if let Some(doc) = &mut model.document {
                document::transform::flip_horizontal(doc);
            }
        }
        AppMessage::FlipVertical => {
            if let Some(doc) = &mut model.document {
                document::transform::flip_vertical(doc);
            }
        }
        AppMessage::RotateCW => {
            if let Some(doc) = &mut model.document {
                document::transform::rotate_cw(doc);
            }
        }
        AppMessage::RotateCCW => {
            if let Some(doc) = &mut model.document {
                document::transform::rotate_ccw(doc);
            }
        }

        // ===== Error handling ============================================================
        AppMessage::ShowError(msg) => {
            model.set_error(msg);
        }
        AppMessage::ClearError => {
            model.clear_error();
        }

        AppMessage::NoOp => {
            // Intentionally do nothing.
        }
    }
}

/// Increment zoom level by 10%.
fn zoom_in(model: &mut AppModel) {
    let current = current_zoom(model);
    let new_zoom = (current * 1.1).clamp(0.05, 20.0);
    model.view_mode = ViewMode::Custom(new_zoom);
}

/// Decrement zoom level by ~9% (inverse of 1.1).
fn zoom_out(model: &mut AppModel) {
    let current = current_zoom(model);
    let new_zoom = (current / 1.1).clamp(0.05, 20.0);
    model.view_mode = ViewMode::Custom(new_zoom);
}

/// Extract the current effective zoom factor from the view mode.
/// For `Fit` mode, we assume 1.0 as starting point when switching to custom zoom.
fn current_zoom(model: &AppModel) -> f32 {
    match model.view_mode {
        ViewMode::Fit => 1.0,
        ViewMode::ActualSize => 1.0,
        ViewMode::Custom(z) => z,
    }
}
