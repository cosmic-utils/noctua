// SPDX-License-Identifier: MPL-2.0 OR Apache-2.0
// src/app/update.rs
//
// Application update loop: applies messages to the global model state.

use std::fs;
use std::path::{Path, PathBuf};

use super::document;
use super::message::AppMessage;
use super::model::{AppModel, ToolMode, ViewMode, PAN_STEP};

/// Central update function applying messages to the model.
///
/// This is the single place where application state is mutated.
pub fn update(model: &mut AppModel, msg: AppMessage) {
    // Debug output: log every received message.
    println!("update(): received message: {:?}", msg);

    match msg {
        // ===== File / navigation ==========================================================
        AppMessage::OpenPath(path) => {
            open_single_path(model, path);
        }

        AppMessage::NextDocument => {
            go_to_next_document(model);
        }

        AppMessage::PrevDocument => {
            go_to_prev_document(model);
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
            model.zoom = 1.0;
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

/// Open a single path, refreshing navigation context.
fn open_single_path(model: &mut AppModel, path: PathBuf) {
    // Try to load the concrete document type (raster/vector/portable).
    match document::file::open_document(path.clone()) {
        Ok(doc) => {
            // Update current document.
            model.document = Some(doc);
            model.current_path = Some(path.clone());
            model.clear_error();

            // Reset view state for new document.
            model.reset_pan();
            model.zoom = 1.0;
            model.view_mode = ViewMode::Fit;

            // Refresh folder listing based on parent directory.
            if let Some(parent) = path.parent() {
                refresh_folder_entries(model, parent, &path);
            }
        }
        Err(err) => {
            model.document = None;
            model.current_path = None;
            model.set_error(err.to_string());
        }
    }
}

/// Refresh the `folder_entries` list and current index.
fn refresh_folder_entries(model: &mut AppModel, folder: &Path, current: &Path) {
    let mut entries: Vec<PathBuf> = Vec::new();

    if let Ok(read_dir) = fs::read_dir(folder) {
        for entry in read_dir.flatten() {
            let path = entry.path();

            // Only keep files that are recognized as supported documents.
            if document::DocumentKind::from_path(&path).is_some() {
                entries.push(path);
            }
        }
    }

    entries.sort();

    // Determine current index.
    let current_index = entries.iter().position(|p| p == current);

    model.folder_entries = entries;
    model.current_index = current_index;
}

/// Go to next document in the current folder, if any.
fn go_to_next_document(model: &mut AppModel) {
    let len = model.folder_entries.len();
    let Some(idx) = model.current_index else {
        return;
    };
    if len == 0 {
        return;
    }

    let next_idx = (idx + 1) % len;
    if let Some(path) = model.folder_entries.get(next_idx).cloned() {
        model.current_index = Some(next_idx);
        open_single_path(model, path);
    }
}

/// Go to previous document in the current folder, if any.
fn go_to_prev_document(model: &mut AppModel) {
    let len = model.folder_entries.len();
    let Some(idx) = model.current_index else {
        return;
    };
    if len == 0 {
        return;
    }

    let prev_idx = (idx + len - 1) % len;
    if let Some(path) = model.folder_entries.get(prev_idx).cloned() {
        model.current_index = Some(prev_idx);
        open_single_path(model, path);
    }
}

/// Increment zoom level.
fn zoom_in(model: &mut AppModel) {
    let factor = 1.1_f32;
    let new_zoom = (model.zoom * factor).clamp(0.05, 20.0);

    model.zoom = new_zoom;
    model.view_mode = ViewMode::Custom(new_zoom);
}

/// Decrement zoom level.
fn zoom_out(model: &mut AppModel) {
    let factor = 1.0 / 1.1_f32;
    let new_zoom = (model.zoom * factor).clamp(0.05, 20.0);

    model.zoom = new_zoom;
    model.view_mode = ViewMode::Custom(new_zoom);
}
