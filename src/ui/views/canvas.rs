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
use crate::ui::model::{ToolMode, ViewMode};
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
    if let Some(handle) = &model.current_image_handle {
        let content_fit = match model.view_mode {
            ViewMode::Fit => ContentFit::Contain,
            ViewMode::ActualSize | ViewMode::Custom => ContentFit::None,
        };

        let img_viewer = Viewer::new(handle)
            .with_state(model.scale, model.pan_x, model.pan_y)
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
            .disable_pan(model.tool_mode == ToolMode::Crop);

        // Overlay crop UI when in crop mode
        if model.tool_mode == ToolMode::Crop {
            let overlay = crop_overlay(&model.crop_selection, config.crop_show_grid);
            stack![img_viewer, overlay].into()
        } else {
            container(img_viewer)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
    } else {
        container(text(fl!("no-document")))
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .into()
    }
}
