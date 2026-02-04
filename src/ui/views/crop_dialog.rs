// SPDX-License-Identifier: GPL-3.0-or-later
// src/ui/views/crop_dialog.rs
//
// Crop dialog view with CropWidget.

use cosmic::widget::{button, column, container, horizontal_space, icon, row};
use cosmic::iced::Length;
use cosmic::{Element, theme};

use crate::ui::widgets::crop_widget;
use crate::ui::{AppMessage, AppModel};
use crate::fl;

/// Render crop dialog as modal overlay.
///
/// Shows the crop widget with header (title + close) and footer (apply/cancel buttons).
pub fn view<'a>(model: &'a AppModel) -> Option<Element<'a, AppMessage>> {
    // Only show if in crop mode and have an image
    if model.tool_mode != crate::ui::model::ToolMode::Crop {
        return None;
    }

    let Some(handle) = &model.current_image_handle else {
        return None;
    };

    let (img_width, img_height) = model.current_dimensions.unwrap_or((800, 600));

    let spacing = theme::active().cosmic().spacing;

    // Header with title and close button
    let close_btn = button::icon(icon::from_name("window-close-symbolic"))
        .on_press(AppMessage::CancelCrop)
        .padding(spacing.space_xs);

    let header = row()
        .push(
            container(cosmic::widget::text("Crop Image"))
                .padding(spacing.space_xs)
        )
        .push(horizontal_space())
        .push(close_btn)
        .width(Length::Fill)
        .padding(spacing.space_xs);

    // Crop widget (self-contained, handles all crop UI)
    let crop = crop_widget(
        handle.clone(),
        img_width,
        img_height,
        &model.crop_selection,
    );

    // Footer with action buttons
    let cancel_btn = button::standard("Cancel")
        .on_press(AppMessage::CancelCrop);

    let apply_btn = if model.crop_selection.has_selection() {
        button::suggested("Apply")
            .on_press(AppMessage::ApplyCrop)
    } else {
        button::suggested("Apply")
    };

    let footer = row()
        .push(horizontal_space())
        .push(cancel_btn)
        .push(apply_btn)
        .spacing(spacing.space_xs)
        .padding(spacing.space_xs);

    // Full layout
    let content = column()
        .push(header)
        .push(crop)
        .push(footer)
        .width(Length::Fill)
        .height(Length::Fill);

    Some(
        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .into()
    )
}
