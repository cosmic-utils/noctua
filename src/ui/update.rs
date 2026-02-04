// SPDX-License-Identifier: GPL-3.0-or-later
// src/ui/app/update.rs
//
// Application update loop: applies messages to the global model state.

use cosmic::{Action, Task};

use super::NoctuaApp;
use super::message::AppMessage;
use super::model::{AppModel, ToolMode, ViewMode};
use crate::application::commands::transform_document::{TransformDocumentCommand, TransformOperation};
use crate::application::commands::crop_document::CropDocumentCommand;

use crate::ui::widgets::DragHandle;

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
                app.model.view_mode = ViewMode::Fit;
                app.model.scale = 1.0;
                // Sync model from document manager
                crate::ui::sync::sync_model_from_manager(&mut app.model, &mut app.document_manager);
            }
        }

        AppMessage::NextDocument => {
            // Ignore navigation in Crop mode
            if app.model.tool_mode != ToolMode::Crop
                && let Some(_path) = app.document_manager.next_document()
            {
                // Reset zoom when navigating to new document
                app.model.scale = 1.0;
                app.model.view_mode = ViewMode::ActualSize;
                app.model.reset_pan();
                // Sync model from document manager
                crate::ui::sync::sync_model_from_manager(&mut app.model, &mut app.document_manager);
            }
        }

        AppMessage::PrevDocument => {
            // Ignore navigation in Crop mode
            if app.model.tool_mode != ToolMode::Crop
                && let Some(_path) = app.document_manager.previous_document()
            {
                // Reset zoom when navigating to new document
                app.model.scale = 1.0;
                app.model.view_mode = ViewMode::ActualSize;
                app.model.reset_pan();
                // Sync model from document manager
                crate::ui::sync::sync_model_from_manager(&mut app.model, &mut app.document_manager);
            }
        }

        AppMessage::GotoPage(page) => {
            if let Some(doc) = app.document_manager.current_document_mut() {
                if let Err(e) = doc.go_to_page(*page) {
                    log::error!("Failed to navigate to page {page}: {e}");
                } else {
                    // Sync render data after page change
                    crate::ui::sync::sync_render_data(&mut app.model, &mut app.document_manager);
                }
            }
        }

        // ---- Thumbnail generation -------------------------------------------------
        AppMessage::GenerateThumbnailPage(_page) => {
            // TODO: Re-enable when model.document is synced from DocumentManager
            // Currently disabled because DocumentContent doesn't implement Clone
            // if let Some(doc) = &mut model.document {
            //     if let Ok(()) = doc.generate_thumbnail_page(*page) {
            //         return UpdateResult::Task(Task::batch([
            //             Task::done(Action::App(AppMessage::RefreshView)),
            //         ]));
            //     }
            // }
        }

        AppMessage::RefreshView => {
            app.model.tick += 1;
        }

        // ---- View / zoom ---------------------------------------------------------
        AppMessage::ZoomIn => {
            let current = app.model.scale;
            let new_zoom =
                (current * app.config.scale_step).clamp(app.config.min_scale, app.config.max_scale);
            app.model.scale = new_zoom;
            app.model.view_mode = ViewMode::Custom;
        }

        AppMessage::ZoomOut => {
            let current = app.model.scale;
            let new_zoom =
                (current / app.config.scale_step).clamp(app.config.min_scale, app.config.max_scale);
            app.model.scale = new_zoom;
            app.model.view_mode = ViewMode::Custom;
        }

        AppMessage::ZoomReset => {
            app.model.scale = 1.0;
            app.model.view_mode = ViewMode::ActualSize;
            app.model.reset_pan();
        }

        AppMessage::ZoomFit => {
            app.model.view_mode = ViewMode::Fit;
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
            let old_scale = app.model.scale;

            // Update model from viewer state
            app.model.scale = *scale;
            app.model.pan_x = *offset_x;
            app.model.pan_y = *offset_y;
            app.model.canvas_size = *canvas_size;
            app.model.image_size = *image_size;

            // If scale changed, user zoomed -> switch to Custom mode
            // (Fit mode is only maintained when explicitly set via ZoomFit button)
            if old_scale != *scale {
                app.model.view_mode = ViewMode::Custom;
            }
        }

        // ---- Pan control ---------------------------------------------------------
        AppMessage::PanLeft => {
            app.model.pan_x -= app.config.pan_step;
        }
        AppMessage::PanRight => {
            app.model.pan_x += app.config.pan_step;
        }
        AppMessage::PanUp => {
            app.model.pan_y -= app.config.pan_step;
        }
        AppMessage::PanDown => {
            app.model.pan_y += app.config.pan_step;
        }
        AppMessage::PanReset => {
            app.model.reset_pan();
        }

        // ---- Tool modes ----------------------------------------------------------
        AppMessage::ToggleCropMode => {
            app.model.tool_mode = if app.model.tool_mode == ToolMode::Crop {
                ToolMode::None
            } else {
                ToolMode::Crop
            };
        }
        AppMessage::ToggleScaleMode => {
            app.model.tool_mode = if app.model.tool_mode == ToolMode::Scale {
                ToolMode::None
            } else {
                ToolMode::Scale
            };
        }

        // ---- Crop operations -----------------------------------------------------
        AppMessage::StartCrop => {
            if app.document_manager.current_document().is_some() {
                app.model.tool_mode = ToolMode::Crop;
                app.model.crop_selection.reset();
            }
        }
        AppMessage::CancelCrop => {
            // Only cancel if actually in Crop mode
            if app.model.tool_mode == ToolMode::Crop {
                app.model.tool_mode = ToolMode::None;
                app.model.crop_selection.reset();
            }
        }
        AppMessage::ApplyCrop => {
            if app.model.tool_mode == ToolMode::Crop {
                // Get crop selection region
                if let Some(crop_region) = app.model.crop_selection.to_crop_region() {
                    // Create crop command from canvas selection
                    let pan_offset = cosmic::iced::Vector::new(app.model.pan_x, app.model.pan_y);

                    match CropDocumentCommand::from_canvas_selection(
                        &crop_region,
                        app.model.canvas_size,
                        app.model.image_size,
                        app.model.scale,
                        pan_offset,
                    ) {
                        Ok(cmd) => {
                            // Execute crop command
                            if let Err(e) = cmd.execute(&mut app.document_manager) {
                                app.model.set_error(format!("Crop failed: {e}"));
                            } else {
                                // Success - exit crop mode and reset selection
                                app.model.tool_mode = ToolMode::None;
                                app.model.crop_selection.reset();
                                // Reset view to fit the cropped image
                                app.model.scale = 1.0;
                                app.model.view_mode = ViewMode::Fit;
                                app.model.reset_pan();
                                // Sync model after crop
                                crate::ui::sync::sync_model_from_manager(
                                    &mut app.model,
                                    &mut app.document_manager,
                                );
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
            if app.model.tool_mode == ToolMode::Crop {
                if *handle == DragHandle::None {
                    app.model.crop_selection.start_new_selection(*x, *y);
                } else {
                    app.model.crop_selection.start_handle_drag(*handle, *x, *y);
                }
            }
        }
        AppMessage::CropDragMove { x, y, max_x, max_y } => {
            if app.model.tool_mode == ToolMode::Crop {
                app.model.crop_selection.update_drag(*x, *y, *max_x, *max_y);
            }
        }
        AppMessage::CropDragEnd => {
            if app.model.tool_mode == ToolMode::Crop {
                app.model.crop_selection.end_drag();
            }
        }

        // ---- Save operations -----------------------------------------------------
        AppMessage::SaveAs => {
            save_as(&mut app.model);
        }

        // ---- Document transformations --------------------------------------------
        AppMessage::FlipHorizontal => {
            // Ignore transformations in Crop mode (would invalidate selection)
            if app.model.tool_mode != ToolMode::Crop {
                let cmd = TransformDocumentCommand::new(TransformOperation::FlipHorizontal);
                if let Err(e) = cmd.execute(&mut app.document_manager) {
                    app.model.set_error(format!("Flip horizontal failed: {e}"));
                } else {
                    // Sync render data after transform
                    crate::ui::sync::sync_render_data(&mut app.model, &mut app.document_manager);
                }
            }
        }
        AppMessage::FlipVertical => {
            // Ignore transformations in Crop mode (would invalidate selection)
            if app.model.tool_mode != ToolMode::Crop {
                let cmd = TransformDocumentCommand::new(TransformOperation::FlipVertical);
                if let Err(e) = cmd.execute(&mut app.document_manager) {
                    app.model.set_error(format!("Flip vertical failed: {e}"));
                } else {
                    // Sync render data after transform
                    crate::ui::sync::sync_render_data(&mut app.model, &mut app.document_manager);
                }
            }
        }
        AppMessage::RotateCW => {
            // Ignore transformations in Crop mode (would invalidate selection)
            if app.model.tool_mode != ToolMode::Crop {
                let cmd = TransformDocumentCommand::new(TransformOperation::RotateCw);
                if let Err(e) = cmd.execute(&mut app.document_manager) {
                    app.model.set_error(format!("Rotate clockwise failed: {e}"));
                } else {
                    // Sync render data after transform
                    crate::ui::sync::sync_render_data(&mut app.model, &mut app.document_manager);
                }
            }
        }
        AppMessage::RotateCCW => {
            // Ignore transformations in Crop mode (would invalidate selection)
            if app.model.tool_mode != ToolMode::Crop {
                let cmd = TransformDocumentCommand::new(TransformOperation::RotateCcw);
                if let Err(e) = cmd.execute(&mut app.document_manager) {
                    app.model.set_error(format!("Rotate CCW failed: {e}"));
                } else {
                    // Sync render data after transform
                    crate::ui::sync::sync_render_data(&mut app.model, &mut app.document_manager);
                }
            }
        }

        // ---- Metadata ------------------------------------------------------------
        AppMessage::RefreshMetadata => {
            // Metadata is already synced via DocumentManager
            // Nothing to do here
        }

        // ---- Wallpaper -----------------------------------------------------------
        AppMessage::SetAsWallpaper => {
            set_as_wallpaper(&mut app.model, &app.document_manager);
        }

        // ---- Format operations ---------------------------------------------------
        AppMessage::SetPaperFormat(format) => {
            app.model.paper_format = Some(*format);
        }

        AppMessage::SetOrientation(orientation) => {
            app.model.orientation = *orientation;
        }

        // ---- Menu ----------------------------------------------------------------
        AppMessage::ToggleMainMenu => {
            app.model.menu_open = !app.model.menu_open;
        }

        // ---- Format Panel --------------------------------------------------------
        AppMessage::OpenFormatPanel => {
            // Close menu if open
            app.model.menu_open = false;
            // This is also handled in app.rs for nav bar toggling
        }

        // ---- Error handling ------------------------------------------------------
        AppMessage::ShowError(msg) => {
            app.model.set_error(msg.clone());
        }
        AppMessage::ClearError => {
            app.model.clear_error();
        }

        // ---- Handled elsewhere ---------------------------------------------------
        AppMessage::ToggleContextPage(_) | AppMessage::ToggleNavBar => {}

        AppMessage::NoOp => {}
    }

    UpdateResult::None
}

// =============================================================================
// Helper Functions
// =============================================================================

fn set_as_wallpaper(model: &mut AppModel, manager: &crate::application::DocumentManager) {
    let Some(path) = manager.current_path() else {
        model.set_error("No image loaded".to_string());
        return;
    };

    log::info!("Setting wallpaper to: {}", path.display());
    crate::infrastructure::system::set_as_wallpaper(path);
}

fn save_as(model: &mut AppModel) {
    // TODO: Implement file dialog for save path
    // For now, show error that this needs UI integration
    model.set_error("Save As: File dialog not yet implemented".to_string());
}
