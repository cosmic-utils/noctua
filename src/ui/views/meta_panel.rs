// SPDX-License-Identifier: GPL-3.0-or-later
// src/ui/views/meta_panel.rs
//
// Metadata and properties panel for document information.

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, column, divider, horizontal_space, icon, row, text};
use cosmic::Element;

use crate::application::DocumentManager;
use crate::domain::document::core::document::Renderable;
use crate::ui::{AppMessage, AppModel};
use crate::fl;

/// Build the metadata/properties panel view.
pub fn view(_model: &AppModel, manager: &DocumentManager) -> Element<'static, AppMessage> {
    let mut content = column::with_capacity(16).spacing(8).padding(12);

    // Header with action icons
    content = content.push(panel_header(manager));

    // Display document metadata if available
    if let Some(meta) = manager.current_metadata() {
        // --- Basic Information Section ---
        content = content
            .push(section_header(fl!("meta-section-file")))
            .push(meta_row(fl!("meta-filename"), meta.basic.file_name.clone()))
            .push(meta_row(fl!("meta-format"), meta.basic.format.clone()));

        // Show dimensions - original from metadata, current if transformed
        let original_dims = (meta.basic.width, meta.basic.height);
        let current_dims = if let Some(doc) = manager.current_document() {
            let info = doc.info();
            (info.width, info.height)
        } else {
            (0, 0)
        };

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
            .push(meta_row_small(
                fl!("meta-path"),
                meta.basic.file_path.clone(),
            ));
    } else {
        // No document loaded
        content = content
            .push(vertical_space())
            .push(text::body(fl!("no-document")))
            .push(vertical_space());
    }

    content.into()
}

// =============================================================================
// Helper Components
// =============================================================================

/// Panel header with title and action buttons.
fn panel_header(manager: &DocumentManager) -> Element<'static, AppMessage> {
    let has_doc = manager.current_document().is_some();

    row::with_capacity(5)
        .spacing(4)
        .align_y(Alignment::Center)
        .padding([0, 0, 8, 0])
        .push(text::title4(fl!("panel-properties")))
        .push(horizontal_space().width(Length::Fill))
        .push(
            button::icon(icon::from_name("image-x-generic-symbolic"))
                .tooltip(fl!("action-set-wallpaper"))
                .padding(4)
                .on_press_maybe(has_doc.then_some(AppMessage::SetAsWallpaper)),
        )
        .into()
}

/// Section header for grouping metadata.
fn section_header(label: String) -> Element<'static, AppMessage> {
    text::heading(label).size(14).into()
}

/// Key-value metadata row.
fn meta_row(label: String, value: String) -> Element<'static, AppMessage> {
    column::with_capacity(2)
        .spacing(2)
        .push(text::caption(format!("{}:", label)))
        .push(text::body(value))
        .into()
}

/// Less prominent metadata row (smaller text).
fn meta_row_small(label: String, value: String) -> Element<'static, AppMessage> {
    column::with_capacity(2)
        .spacing(2)
        .push(text::caption(format!("{}:", label)))
        .push(text::caption(value))
        .into()
}

/// Vertical spacer helper.
fn vertical_space() -> Element<'static, AppMessage> {
    cosmic::widget::vertical_space()
        .height(Length::Fixed(32.0))
        .into()
}
