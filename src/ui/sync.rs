// SPDX-License-Identifier: GPL-3.0-or-later
// src/ui/sync.rs
//
// Synchronize UI model from DocumentManager state.

use crate::application::DocumentManager;
use crate::domain::document::core::document::Renderable;
use crate::ui::model::AppModel;

/// Synchronize AppModel from DocumentManager.
///
/// Updates UI state with current document info, but does NOT copy
/// the entire document (would break Clean Architecture).
/// Only caches render-related data for performance.
pub fn sync_model_from_manager(model: &mut AppModel, manager: &mut DocumentManager) {
    // Update cached render data
    if let Some(doc) = manager.current_document_mut() {
        // Cache image handle for rendering
        if let Ok(render_output) = doc.render(1.0) {
            model.current_image_handle = Some(render_output.handle);
        } else {
            model.current_image_handle = None;
        }

        // Cache dimensions
        let info = doc.info();
        model.current_dimensions = Some((info.width, info.height));

        // Cache page info
        model.current_page = Some(doc.current_page());
        model.page_count = Some(doc.page_count());
    } else {
        // No document loaded - clear cached data
        model.current_image_handle = None;
        model.current_dimensions = None;
        model.current_page = None;
        model.page_count = None;
    }

    // Update navigation state
    model.current_path = manager.current_path().map(|p| p.to_path_buf());
    model.folder_count = manager.folder_entries().len();
    model.current_index = manager.current_index();

    // Update metadata
    model.metadata = manager.current_metadata().cloned();
}

/// Synchronize only render data without full document info.
///
/// Useful when only the rendered image has changed (e.g., after transform).
pub fn sync_render_data(model: &mut AppModel, manager: &mut DocumentManager) {
    if let Some(doc) = manager.current_document_mut() {
        // Re-render at current scale to get updated image handle
        if let Ok(render_output) = doc.render(model.scale as f64) {
            model.current_image_handle = Some(render_output.handle);
        }

        // Update dimensions (may have changed after rotation)
        let info = doc.info();
        model.current_dimensions = Some((info.width, info.height));

        // Update page info (in case page changed)
        model.current_page = Some(doc.current_page());
    }
}

/// Synchronize only navigation state without render data.
///
/// Useful when switching documents in a folder.
#[allow(dead_code)]
pub fn sync_navigation(model: &mut AppModel, manager: &DocumentManager) {
    model.current_path = manager.current_path().map(|p| p.to_path_buf());
    model.current_index = manager.current_index();
    model.folder_count = manager.folder_entries().len();
}
