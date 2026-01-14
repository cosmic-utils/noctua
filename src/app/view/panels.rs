// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/panels.rs
//
// Panel content for COSMIC context drawer.

use cosmic::widget::{column, row, text};
use cosmic::Element;

use crate::app::document::DocumentContent;
use crate::app::{AppMessage, AppModel};
use crate::fl;

/// Content for the right-side properties panel (context drawer).
pub fn properties_panel(model: &AppModel) -> Element<'static, AppMessage> {
    let mut content = column::with_capacity(6).spacing(12);

    // Header.
    let header = fl!("panel-properties");
    content = content.push(text::title4(header));

    // Display document metadata if available.
    if let Some(ref doc) = model.document {
        match doc {
            DocumentContent::Raster(raster) => {
                let (w, h) = raster.dimensions();
                let format_str = raster
                    .path
                    .as_ref()
                    .and_then(|p| p.extension())
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
                    .to_uppercase();

                let lbl_dim = fl!("meta-dimensions");
                let lbl_fmt = fl!("meta-format");

                content = content
                    .push(meta_row(lbl_dim, format!("{}×{}", w, h)))
                    .push(meta_row(lbl_fmt, format_str));
            }
            DocumentContent::Vector(vector) => {
                let (w, h) = vector.dimensions();

                let lbl_dim = fl!("meta-dimensions");
                let lbl_fmt = fl!("meta-format");

                content = content
                    .push(meta_row(lbl_dim, format!("{}×{}", w, h)))
                    .push(meta_row(lbl_fmt, "SVG".to_string()));
            }
            DocumentContent::Portable(portable) => {
                let lbl_pages = fl!("meta-pages");
                let lbl_current = fl!("meta-current-page");

                content = content
                    .push(meta_row(lbl_pages, portable.page_count.to_string()))
                    .push(meta_row(
                        lbl_current,
                        (portable.current_page + 1).to_string(),
                    ));
            }
        }
    } else {
        let no_doc = fl!("no-document");
        content = content.push(text::body(no_doc));
    }

    content.into()
}

/// Helper to create a key-value metadata row.
fn meta_row(label: String, value: String) -> Element<'static, AppMessage> {
    row::with_capacity(2)
        .spacing(8)
        .push(text::body(format!("{}:", label)))
        .push(text::body(value))
        .into()
}
