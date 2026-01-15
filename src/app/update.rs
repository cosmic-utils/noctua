// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/update.rs
//
// Application update loop: applies messages to the global model state.

use super::document;
use super::message::AppMessage;
use super::model::{AppModel, ToolMode, ViewMode, PAN_STEP};

/// Central update function applying messages to the model.
///
/// Panel toggle messages (ToggleContextPage) are handled directly in
/// `Noctua::update()` since they affect COSMIC's Core state.
pub fn update(model: &mut AppModel, msg: AppMessage) {
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
        AppMessage::ViewerStateChanged { scale, offset_x, offset_y } => {
            // Update model state from viewer (mouse interaction)
            model.view_mode = ViewMode::Custom(scale);
            model.pan_x = offset_x;
            model.pan_y = offset_y;
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

        // ===== Tool modes ================================================================
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

        // ===== Metadata ==================================================================
        AppMessage::RefreshMetadata => {
            refresh_metadata(model);
        }

        // ===== Wallpaper =================================================================
        AppMessage::SetAsWallpaper => {
            set_as_wallpaper(model);
        }

        // ===== Error handling ============================================================
        AppMessage::ShowError(msg) => {
            model.set_error(msg);
        }
        AppMessage::ClearError => {
            model.clear_error();
        }

        // ===== Handled elsewhere =========================================================
        AppMessage::ToggleContextPage(_) => {
            // Handled in Noctua::update() directly.
        }

        AppMessage::ToggleNavBar => {
            // Handled in Noctua::update() directly.
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
fn current_zoom(model: &AppModel) -> f32 {
    match model.view_mode {
        ViewMode::Fit => 1.0,
        ViewMode::ActualSize => 1.0,
        ViewMode::Custom(z) => z,
    }
}

/// Refresh metadata from the current document.
fn refresh_metadata(model: &mut AppModel) {
    model.metadata = model.document.as_ref().map(|doc| doc.extract_meta());
}

/// Set the current image as desktop wallpaper.
fn set_as_wallpaper(model: &mut AppModel) {
    let Some(path) = model.current_path.as_ref() else {
        model.set_error("No image loaded");
        return;
    };

    let path = path.clone();

    // Spawn async task to set wallpaper
    tokio::spawn(async move {
        document::set_as_wallpaper(&path);
    });
}
