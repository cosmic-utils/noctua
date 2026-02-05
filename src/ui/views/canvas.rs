// SPDX-License-Identifier: GPL-3.0-or-later
// src/ui/views/canvas.rs
//
// Render the center canvas area with the current document.

use cosmic::iced::widget::image::FilterMethod;
use cosmic::iced::{ContentFit, Length};
use cosmic::iced_widget::stack;
use cosmic::widget::{container, text};
use cosmic::Element;

use crate::ui::widgets::{crop_overlay, Viewer};
use crate::ui::model::{AppMode, ViewMode};
use crate::ui::{AppMessage, AppModel};
use crate::application::DocumentManager;
use crate::config::AppConfig;
use crate::fl;

/// Render the center canvas area with the current document.
pub fn view<'a>(
    model: &'a AppModel,
    _manager: &'a DocumentManager,
    config: &'a AppConfig,
) -> Element<'a, AppMessage> {
    // Use cached image handle from viewport
    if let Some(handle) = &model.viewport.cached_image_handle {
        // Determine content fit mode
        let content_fit = match model.viewport.fit_mode {
            ViewMode::Fit => ContentFit::Contain,
            ViewMode::ActualSize | ViewMode::Custom => ContentFit::None,
        };

        // Check if we're in crop mode (to disable pan)
        let disable_pan = matches!(model.mode, AppMode::Crop { .. });

        // Create image viewer
        let img_viewer = Viewer::new(handle.clone())
            .with_state(
                model.viewport.scale,
                model.viewport.pan_x,
                model.viewport.pan_y,
            )
            .on_state_change(|scale, offset_x, offset_y, canvas_size, image_size| {
                AppMessage::ViewerStateChanged {
                    scale,
                    offset_x,
                    offset_y,
                    canvas_size,
                    image_size,
                }
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(content_fit)
            .filter_method(FilterMethod::Nearest)
            .min_scale(config.min_scale)
            .max_scale(config.max_scale)
            .scale_step(config.scale_step - 1.0)
            .disable_pan(disable_pan);

        // Overlay crop UI when in crop mode
        if let AppMode::Crop { selection } = &model.mode {
            let overlay = crop_overlay(selection, config.crop_show_grid);
            stack![img_viewer, overlay].into()
        } else {
            container(img_viewer)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
    } else {
        // No document loaded
        container(text(fl!("no-document")))
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .into()
    }
}
