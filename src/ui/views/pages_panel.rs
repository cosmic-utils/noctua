// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/pages_panel.rs
//
// Page navigation panel for multi-page documents (PDF, multi-page TIFF, etc.).

/// Maximum width in pixels for page navigation thumbnails.
const THUMBNAIL_MAX_WIDTH: f32 = 100.0;

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, column, container, scrollable, text};
use cosmic::widget::image as cosmic_image;

use cosmic::Element;

use crate::application::DocumentManager;
use crate::ui::{AppMessage, AppModel};
use crate::fl;

/// Build the page navigation panel view.
/// Returns None if the current document doesn't support multiple pages.
pub fn view<'a>(
    model: &'a AppModel,
    manager: &'a DocumentManager,
) -> Option<Element<'a, AppMessage>> {
    // Only show for multi-page documents.
    let page_count = model.page_count?;
    if page_count <= 1 {
        return None;
    }

    let current_page = model.current_page.unwrap_or(0);

    // Get document for thumbnail loading status
    let doc = manager.current_document()?;
    let loaded = doc.thumbnails_loaded();

    let mut content = column::with_capacity(page_count + 1)
        .spacing(12)
        .padding([12, 8])
        .align_x(Alignment::Center)
        .width(Length::Fill);

    // Show loading progress if not all thumbnails are ready.
    if !doc.thumbnails_ready() {
        let loading_msg = fl!("loading-thumbnails", current: loaded, total: page_count);
        content = content.push(text::caption(loading_msg));
    }

    // Build thumbnail list for pages that are already loaded.
    for page_index in 0..loaded {
        let is_current = page_index == current_page;

        // Get cached thumbnail handle (read-only access).
        let thumbnail_element: Element<'static, AppMessage> =
            if let Some(handle) = manager.get_thumbnail_handle(page_index) {
                // Display the thumbnail image.
                cosmic_image::Image::new(handle)
                    .width(Length::Fixed(THUMBNAIL_MAX_WIDTH))
                    .into()
            } else {
                // Fallback: show page number if thumbnail not yet loaded.
                container(text(format!("Page {}", page_index + 1)))
                    .width(Length::Fixed(THUMBNAIL_MAX_WIDTH))
                    .height(Length::Fixed(THUMBNAIL_MAX_WIDTH * 1.4))
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into()
            };

        // Page number label.
        let page_label = text::caption(format!("{}", page_index + 1));

        // Combine thumbnail and label in a column.
        let page_content = column::with_capacity(2)
            .spacing(4)
            .align_x(Alignment::Center)
            .push(thumbnail_element)
            .push(page_label);

        // Wrap in button for navigation.
        let page_button = if is_current {
            // Current page: highlighted style.
            button::custom(page_content)
                .class(cosmic::theme::Button::Suggested)
                .padding(4)
        } else {
            // Other pages: clickable with standard style.
            button::custom(page_content)
                .class(cosmic::theme::Button::Standard)
                .padding(4)
                .on_press(AppMessage::GotoPage(page_index))
        };

        content = content.push(page_button);
    }

    // Wrap in scrollable container.
    Some(
        scrollable(content)
            .width(Length::Shrink)
            .height(Length::Fill)
            .into(),
    )
}
