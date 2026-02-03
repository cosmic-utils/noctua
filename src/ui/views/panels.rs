// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/panels.rs
//
// Properties panel content for COSMIC context drawer.

use cosmic::iced::Length;
use cosmic::widget::{button, column, divider, horizontal_space, icon, row, text};
use cosmic::Element;

use crate::ui::{AppMessage, AppModel};
use crate::fl;
use crate::application::DocumentManager;

/// Build the properties panel view.
pub fn view(model: &AppModel, manager: &DocumentManager) -> Element<'static, AppMessage> {
    let mut content = column::with_capacity(16).spacing(8);

    // Header with action icons
    content = content.push(panel_header(model, manager));

    // Display document metadata if available (cached in model).
    if let Some(meta) = manager.current_metadata() {
        // --- Basic Information Section ---
        content = content
            .push(section_header(fl!("meta-section-file")))
            .push(meta_row(fl!("meta-filename"), meta.basic.file_name.clone()))
            .push(meta_row(fl!("meta-format"), meta.basic.format.clone()));

        // Show dimensions - original from metadata, current if transformed
        let original_dims = (meta.basic.width, meta.basic.height);
        let current_dims = model.current_dimensions.unwrap_or((0, 0));

        if original_dims != current_dims && current_dims != (0, 0) {
            // Dimensions changed (e.g., rotation) - show both
            content = content.push(meta_row(
                fl!("meta-dimensions"),
                format!(
                    "{} × {} (original: {} × {})",
                    current_dims.0, current_dims.1, original_dims.0, original_dims.1
                ),
            ));
        } else {
            // No transformation or no document loaded yet
            content = content.push(meta_row(
                fl!("meta-dimensions"),
                meta.basic.resolution_display(),
            ));
        }

        content = content
            .push(meta_row(
                fl!("meta-filesize"),
                meta.basic.file_size_display(),
            ))
            .push(meta_row(
                fl!("meta-colortype"),
                meta.basic.color_type.clone(),
            ));

        // --- EXIF Section (if available) ---
        if let Some(ref exif) = meta.exif {
            let has_exif_data = exif.camera_display().is_some()
                || exif.date_time.is_some()
                || exif.exposure_time.is_some()
                || exif.f_number.is_some()
                || exif.iso.is_some()
                || exif.focal_length.is_some()
                || exif.gps_display().is_some();

            if has_exif_data {
                content = content
                    .push(divider::horizontal::light())
                    .push(section_header(fl!("meta-section-exif")));

                if let Some(camera) = exif.camera_display() {
                    content = content.push(meta_row(fl!("meta-camera"), camera));
                }

                if let Some(ref date) = exif.date_time {
                    content = content.push(meta_row(fl!("meta-datetime"), date.clone()));
                }

                if let Some(ref exposure) = exif.exposure_time {
                    content = content.push(meta_row(fl!("meta-exposure"), exposure.clone()));
                }

                if let Some(ref fnumber) = exif.f_number {
                    content = content.push(meta_row(fl!("meta-aperture"), fnumber.clone()));
                }

                if let Some(iso) = exif.iso {
                    content = content.push(meta_row(fl!("meta-iso"), fl!("meta-iso", iso: iso)));
                }

                if let Some(ref focal) = exif.focal_length {
                    content = content.push(meta_row(fl!("meta-focal"), focal.clone()));
                }

                if let Some(gps) = exif.gps_display() {
                    content = content.push(meta_row(fl!("meta-gps"), gps));
                }
            }
        }

        // --- File Path (at the bottom, less prominent) ---
        content = content
            .push(divider::horizontal::light())
            .push(meta_row_small(
                fl!("meta-path"),
                meta.basic.file_path.clone(),
            ));
    } else {
        content = content.push(text::body(fl!("no-document")));
    }

    content.into()
}

/// Section header for grouping metadata.
fn section_header(label: String) -> Element<'static, AppMessage> {
    text::body(label).into()
}

/// Helper to create a key-value metadata row.
fn meta_row(label: String, value: String) -> Element<'static, AppMessage> {
    row::with_capacity(2)
        .spacing(8)
        .push(text::body(format!("{label}:")))
        .push(text::body(value))
        .into()
}

/// Helper for less prominent metadata (smaller text, e.g., file path).
fn meta_row_small(label: String, value: String) -> Element<'static, AppMessage> {
    column::with_capacity(2)
        .spacing(2)
        .push(text::caption(format!("{label}:")))
        .push(text::caption(value))
        .into()
}

/// Panel header with title and action icon buttons.
fn panel_header(model: &AppModel, _manager: &DocumentManager) -> Element<'static, AppMessage> {
    let has_doc = model.current_image_handle.is_some();

    row::with_capacity(5)
        .spacing(4)
        .align_y(cosmic::iced::Alignment::Center)
        .push(text::title4(fl!("panel-properties")))
        .push(horizontal_space().width(Length::Fill))
        .push(
            button::icon(icon::from_name("image-x-generic-symbolic"))
                .tooltip(fl!("action-set-wallpaper"))
                .on_press_maybe(has_doc.then_some(AppMessage::SetAsWallpaper)),
        )
        // .push(
        //     button::icon(icon::from_name("system-run-symbolic"))
        //         .on_press_maybe(has_doc.then_some(AppMessage::NoOp)) // TODO: Implement
        //         .tooltip(fl!("action-open-with"))
        // )
        // .push(
        //     button::icon(icon::from_name("system-file-manager-symbolic"))
        //         .on_press_maybe(has_doc.then_some(AppMessage::NoOp)) // TODO: Implement
        //         .tooltip(fl!("action-show-in-folder"))
        // )
        .into()
}
