// SPDX-License-Identifier: GPL-3.0-or-later
// src/ui/update.rs
//
// Application update loop: applies messages to the global model state.

use cosmic::{Action, Task};

use super::NoctuaApp;
use super::message::AppMessage;
use super::model::{AppMode, ViewMode};
use crate::application::commands::transform_document::{TransformDocumentCommand, TransformOperation};
use crate::application::commands::crop_document::CropDocumentCommand;
use crate::domain::document::core::document::Renderable;
use crate::ui::widgets::{CropSelection, DragHandle};

// =============================================================================
// Update Result
// =============================================================================

#[allow(dead_code)]
pub enum UpdateResult {
    None,
    Task(Task<Action<AppMessage>>),
}

// =============================================================================
// Main Update Function
// =============================================================================

pub fn update(app: &mut NoctuaApp, msg: &AppMessage) -> UpdateResult {
    match msg {
        // ---- File / navigation ----------------------------------------------------
        AppMessage::OpenPath(path) => {
            if let Err(e) = app.document_manager.open_document(path) {
                app.model.set_error(format!("Failed to open document: {e}"));
            } else {
                app.model.reset_pan();
                app.model.viewport.fit_mode = ViewMode::Fit;
                app.model.viewport.scale = 1.0;
                cache_render(&mut app.model, &mut app.document_manager);
            }
        }

        AppMessage::NextDocument => {
            // Ignore navigation in Crop mode
            if !matches!(app.model.mode, AppMode::Crop { .. })
                && let Some(_path) = app.document_manager.next_document()
            {
                // Reset zoom when navigating to new document
                app.model.viewport.scale = 1.0;
                app.model.viewport.fit_mode = ViewMode::ActualSize;
                app.model.reset_pan();
                cache_render(&mut app.model, &mut app.document_manager);
            }
        }

        AppMessage::PrevDocument => {
            // Ignore navigation in Crop mode
            if !matches!(app.model.mode, AppMode::Crop { .. })
                && let Some(_path) = app.document_manager.previous_document()
            {
                // Reset zoom when navigating to new document
                app.model.viewport.scale = 1.0;
                app.model.viewport.fit_mode = ViewMode::ActualSize;
                app.model.reset_pan();
                cache_render(&mut app.model, &mut app.document_manager);
            }
        }

        AppMessage::GotoPage(page) => {
            if let Some(doc) = app.document_manager.current_document_mut() {
                if let Err(e) = doc.go_to_page(*page) {
                    log::error!("Failed to navigate to page {page}: {e}");
                } else {
                    cache_render(&mut app.model, &mut app.document_manager);
                }
            }
        }

        // ---- Thumbnail generation -------------------------------------------------
        AppMessage::GenerateThumbnailPage(_page) => {
            // TODO: Thumbnail generation via DocumentManager
            // Currently handled by DocumentManager.open_document()
        }

        AppMessage::RefreshView => {
            app.model.tick += 1;
        }

        // ---- View / zoom ---------------------------------------------------------
        AppMessage::ZoomIn => {
            app.model.viewport.scale = (app.model.viewport.scale * 1.2).min(10.0);
            app.model.viewport.fit_mode = ViewMode::Custom;
        }

        AppMessage::ZoomOut => {
            app.model.viewport.scale = (app.model.viewport.scale / 1.2).max(0.1);
            app.model.viewport.fit_mode = ViewMode::Custom;
        }

        AppMessage::ZoomReset => {
            app.model.viewport.scale = 1.0;
            app.model.viewport.fit_mode = ViewMode::ActualSize;
            app.model.reset_pan();
        }

        AppMessage::ZoomFit => {
            app.model.viewport.fit_mode = ViewMode::Fit;
            app.model.reset_pan();
        }

        AppMessage::ViewerStateChanged {
            scale,
            offset_x,
            offset_y,
            canvas_size,
            image_size,
        } => {
            // Detect scale changes (zoom vs just pan)
            let old_scale = app.model.viewport.scale;

            // Update model from viewer state
            app.model.viewport.scale = *scale;
            app.model.viewport.pan_x = *offset_x;
            app.model.viewport.pan_y = *offset_y;
            app.model.viewport.canvas_size = *canvas_size;
            app.model.viewport.image_size = *image_size;

            // If scale changed, user zoomed -> switch to Custom mode and re-render
            // (Fit mode is only maintained when explicitly set via ZoomFit button)
            if (old_scale - *scale).abs() > 0.001 {
                app.model.viewport.fit_mode = ViewMode::Custom;
                cache_render(&mut app.model, &mut app.document_manager);
            }
        }

        // ---- Pan control ---------------------------------------------------------
        AppMessage::PanLeft => {
            app.model.viewport.pan_x -= 50.0;
        }
        AppMessage::PanRight => {
            app.model.viewport.pan_x += 50.0;
        }
        AppMessage::PanUp => {
            app.model.viewport.pan_y -= 50.0;
        }
        AppMessage::PanDown => {
            app.model.viewport.pan_y += 50.0;
        }
        AppMessage::PanReset => {
            app.model.reset_pan();
        }

        // ---- Tool modes ----------------------------------------------------------
        AppMessage::ToggleCropMode => {
            app.model.mode = match &app.model.mode {
                AppMode::Crop { .. } => AppMode::View,
                _ => AppMode::Crop {
                    selection: CropSelection::default(),
                },
            };
        }

        AppMessage::ToggleScaleMode => {
            // Scale mode -> Transform mode
            app.model.mode = match &app.model.mode {
                AppMode::Transform { .. } => AppMode::View,
                _ => AppMode::Transform {
                    paper_format: None,
                    orientation: Default::default(),
                },
            };
        }

        // ---- Crop operations -----------------------------------------------------
        AppMessage::StartCrop => {
            if app.document_manager.current_document().is_some() {
                app.model.mode = AppMode::Crop {
                    selection: CropSelection::default(),
                };
            }
        }

        AppMessage::CancelCrop => {
            // Only cancel if actually in Crop mode
            if matches!(app.model.mode, AppMode::Crop { .. }) {
                app.model.mode = AppMode::View;
            }
        }

        AppMessage::ApplyCrop => {
            if let AppMode::Crop { selection } = &app.model.mode {
                // Get crop selection region
                if let Some(crop_region) = selection.to_crop_region() {
                    // Create crop command from canvas selection
                    let pan_offset = cosmic::iced::Vector::new(
                        app.model.viewport.pan_x,
                        app.model.viewport.pan_y,
                    );

                    match CropDocumentCommand::from_canvas_selection(
                        &crop_region,
                        app.model.viewport.canvas_size,
                        app.model.viewport.image_size,
                        app.model.viewport.scale,
                        pan_offset,
                    ) {
                        Ok(cmd) => {
                            // Execute crop command
                            if let Err(e) = cmd.execute(&mut app.document_manager) {
                                app.model.set_error(format!("Crop failed: {e}"));
                            } else {
                                // Success - exit crop mode
                                app.model.mode = AppMode::View;
                                // Reset view to fit the cropped image
                                app.model.viewport.scale = 1.0;
                                app.model.viewport.fit_mode = ViewMode::Fit;
                                app.model.reset_pan();
                                cache_render(&mut app.model, &mut app.document_manager);
                            }
                        }
                        Err(e) => {
                            app.model.set_error(format!("Invalid crop region: {e}"));
                        }
                    }
                } else {
                    app.model.set_error("No crop region selected".to_string());
                }
            }
        }

        AppMessage::CropDragStart { x, y, handle } => {
            if let AppMode::Crop { selection } = &mut app.model.mode {
                if *handle == DragHandle::None {
                    selection.start_new_selection(*x, *y);
                } else {
                    selection.start_handle_drag(*handle, *x, *y);
                }
            }
        }

        AppMessage::CropDragMove { x, y, max_x, max_y } => {
            if let AppMode::Crop { selection } = &mut app.model.mode {
                selection.update_drag(*x, *y, *max_x, *max_y);
            }
        }

        AppMessage::CropDragEnd => {
            if let AppMode::Crop { selection } = &mut app.model.mode {
                selection.end_drag();
            }
        }

        // ---- Save operations -----------------------------------------------------
        AppMessage::SaveAs => {
            save_as(&mut app.model);
        }

        // ---- Document transformations --------------------------------------------
        AppMessage::FlipHorizontal => {
            // Ignore transformations in Crop mode (would invalidate selection)
            if !matches!(app.model.mode, AppMode::Crop { .. }) {
                let cmd = TransformDocumentCommand::new(TransformOperation::FlipHorizontal);
                if let Err(e) = cmd.execute(&mut app.document_manager) {
                    app.model.set_error(format!("Flip horizontal failed: {e}"));
                } else {
                    cache_render(&mut app.model, &mut app.document_manager);
                }
            }
        }

        AppMessage::FlipVertical => {
            // Ignore transformations in Crop mode (would invalidate selection)
            if !matches!(app.model.mode, AppMode::Crop { .. }) {
                let cmd = TransformDocumentCommand::new(TransformOperation::FlipVertical);
                if let Err(e) = cmd.execute(&mut app.document_manager) {
                    app.model.set_error(format!("Flip vertical failed: {e}"));
                } else {
                    cache_render(&mut app.model, &mut app.document_manager);
                }
            }
        }

        AppMessage::RotateCW => {
            // Ignore transformations in Crop mode (would invalidate selection)
            if !matches!(app.model.mode, AppMode::Crop { .. }) {
                let cmd = TransformDocumentCommand::new(TransformOperation::RotateCw);
                if let Err(e) = cmd.execute(&mut app.document_manager) {
                    app.model.set_error(format!("Rotate clockwise failed: {e}"));
                } else {
                    cache_render(&mut app.model, &mut app.document_manager);
                }
            }
        }

        AppMessage::RotateCCW => {
            // Ignore transformations in Crop mode (would invalidate selection)
            if !matches!(app.model.mode, AppMode::Crop { .. }) {
                let cmd = TransformDocumentCommand::new(TransformOperation::RotateCcw);
                if let Err(e) = cmd.execute(&mut app.document_manager) {
                    app.model.set_error(format!("Rotate CCW failed: {e}"));
                } else {
                    cache_render(&mut app.model, &mut app.document_manager);
                }
            }
        }

        // ---- Metadata ------------------------------------------------------------
        AppMessage::RefreshMetadata => {
            // Metadata is managed by DocumentManager
            // Nothing to do here - views access it directly
        }

        // ---- Format operations ---------------------------------------------------
        AppMessage::SetPaperFormat(format) => {
            if let AppMode::Transform { paper_format, .. } = &mut app.model.mode {
                *paper_format = Some(*format);
            }
        }

        AppMessage::SetOrientation(orientation) => {
            if let AppMode::Transform {
                orientation: ori, ..
            } = &mut app.model.mode
            {
                *ori = *orientation;
            }
        }

        // ---- Menu ----------------------------------------------------------------
        AppMessage::ToggleMainMenu => {
            app.model.menu_open = !app.model.menu_open;
        }

        // ---- Wallpaper -----------------------------------------------------------
        AppMessage::SetAsWallpaper => {
            if let Some(path) = app.document_manager.current_path() {
                log::info!("Setting wallpaper to: {}", path.display());
                crate::infrastructure::system::set_as_wallpaper(path);
            } else {
                app.model.set_error("No image loaded".to_string());
            }
        }

        // ---- Error handling ------------------------------------------------------
        AppMessage::ShowError(msg) => {
            app.model.set_error(msg.clone());
        }

        AppMessage::ClearError => {
            app.model.clear_error();
        }

        // ---- Handled elsewhere ---------------------------------------------------
        AppMessage::ToggleContextPage(_)
        | AppMessage::ToggleNavBar
        | AppMessage::OpenFormatPanel => {
            // These are handled in app.rs
        }

        AppMessage::NoOp => {}
    }

    UpdateResult::None
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Cache rendered image handle in viewport for view performance.
fn cache_render(
    model: &mut super::model::AppModel,
    manager: &mut crate::application::DocumentManager,
) {
    if let Some(doc) = manager.current_document_mut() {
        match doc.render(model.viewport.scale as f64) {
            Ok(output) => {
                model.viewport.cached_image_handle = Some(output.handle);
            }
            Err(e) => {
                log::error!("Failed to cache render: {e}");
                model.viewport.cached_image_handle = None;
            }
        }
    } else {
        model.viewport.cached_image_handle = None;
    }
}

fn save_as(model: &mut super::model::AppModel) {
    // TODO: Implement file dialog for save path
    // For now, show error that this needs UI integration
    model.set_error("Save As: File dialog not yet implemented".to_string());
}
