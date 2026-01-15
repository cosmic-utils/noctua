// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/panels.rs
//
// Panel content for COSMIC context drawer.

use cosmic::iced::Length;
use cosmic::widget::{button, column, divider, horizontal_space, icon, row, text};
use cosmic::Element;

use crate::app::{AppMessage, AppModel};
use crate::fl;

/// Content for the right-side properties panel (context drawer).
pub fn properties_panel(model: &AppModel) -> Element<'static, AppMessage> {
    let mut content = column::with_capacity(16).spacing(8);

    // Header with action icons
    content = content.push(panel_header(model));

    // Display document metadata if available.
    if let Some(ref doc) = model.document {
        // Use the unified interface to extract metadata.
        let meta = doc.extract_meta();

        // --- Basic Information Section ---
        content = content
            .push(section_header(fl!("meta-section-file")))
            .push(meta_row(fl!("meta-filename"), meta.basic.file_name.clone()))
            .push(meta_row(fl!("meta-format"), meta.basic.format.clone()))
            .push(meta_row(
                fl!("meta-dimensions"),
                meta.basic.resolution_display(),
            ))
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
                    content = content.push(meta_row(fl!("meta-iso"), format!("ISO {}", iso)));
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
            .push(meta_row_small(fl!("meta-path"), meta.basic.file_path.clone()));
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
        .push(text::body(format!("{}:", label)))
        .push(text::body(value))
        .into()
}

/// Helper for less prominent metadata (smaller text, e.g., file path).
fn meta_row_small(label: String, value: String) -> Element<'static, AppMessage> {
    column::with_capacity(2)
        .spacing(2)
        .push(text::caption(format!("{}:", label)))
        .push(text::caption(value))
        .into()
}

/// Panel header with title and action icon buttons.
fn panel_header(model: &AppModel) -> Element<'static, AppMessage> {
    let has_doc = model.document.is_some();

    row::with_capacity(5)
        .spacing(4)
        .align_y(cosmic::iced::Alignment::Center)
        .push(text::title4(fl!("panel-properties")))
        .push(horizontal_space().width(Length::Fill))
        .push(
            button::icon(icon::from_name("image-x-generic-symbolic"))
                .on_press_maybe(has_doc.then_some(AppMessage::SetAsWallpaper))
                .tooltip(fl!("action-set-wallpaper"))
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
