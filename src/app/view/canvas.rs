// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/canvas.rs
//
// Renders the center canvas area with the current document.

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{container, image, text, Column, Row};
use cosmic::Element;

use crate::app::model::ViewMode;
use crate::app::{AppMessage, AppModel};
use crate::fl;

/// Render the center canvas area with the current document.
pub fn view(model: &AppModel) -> Element<'_, AppMessage> {
    if let Some(doc) = &model.document {
        let handle = doc.handle();

        let img_widget = match &model.view_mode {
            ViewMode::Fit => {
                // Fit mode: image scales to fill container while preserving aspect ratio.
                image::Image::new(handle)
                    .width(Length::Fill)
                    .height(Length::Fill)
            }
            ViewMode::ActualSize => {
                // 1:1 pixel size.
                let (native_w, native_h) = doc.dimensions();
                image::Image::new(handle)
                    .width(Length::Fixed(native_w as f32))
                    .height(Length::Fixed(native_h as f32))
            }
            ViewMode::Custom(zoom) => {
                // Custom zoom factor applied to native size.
                let (native_w, native_h) = doc.dimensions();
                let scaled_w = (native_w as f32 * zoom).round();
                let scaled_h = (native_h as f32 * zoom).round();
                image::Image::new(handle)
                    .width(Length::Fixed(scaled_w))
                    .height(Length::Fixed(scaled_h))
            }
        };

        // Center the image both horizontally and vertically.
        Column::new()
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .push(
                Row::new()
                    .push(img_widget)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_y(Alignment::Center),
            )
            .into()
    } else {
        // No document loaded placeholder.
        container(text(fl!("no-document")))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
