// SPDX-License-Identifier: GPL-3.0-or-later
// src/ui/views/canvas.rs
//
// Canvas view using standard widgets (no custom viewer needed).

use cosmic::iced::widget::scrollable::{Direction, Scrollbar};
use cosmic::widget::Id;
use cosmic::iced::{ContentFit, Length};
use cosmic::widget::{container, image, scrollable, text};
use cosmic::{Element, widget::responsive};

use crate::ui::model::ViewMode;
use crate::ui::{AppMessage, AppModel};
use crate::application::DocumentManager;
use crate::config::AppConfig;
use crate::fl;

/// Render the center canvas area with the current document.
/// 
/// Uses standard cosmic widgets:
/// - `image()` for display
/// - `responsive()` for size calculation based on available space
/// - `scrollable()` for panning when image is larger than viewport
/// 
/// The Domain renders images at scale=1.0, and UI scales them for display.
/// This allows smooth zooming without re-rendering from Domain.
pub fn view<'a>(
    model: &'a AppModel,
    _manager: &'a DocumentManager,
    _config: &'a AppConfig,
) -> Element<'a, AppMessage> {
    // Check if we have an image to display
    let Some(handle) = &model.current_image_handle else {
        return container(text(fl!("no-document")))
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .into();
    };

    // Get image dimensions (from cached data)
    let (img_width, img_height) = model.current_dimensions
        .map(|(w, h)| (w as f32, h as f32))
        .unwrap_or((800.0, 600.0)); // Fallback if dimensions not available

    // Clone values for move closure in responsive()
    let handle_clone = handle.clone();
    let scale = model.scale;
    let view_mode = model.view_mode;

    // Use responsive() to calculate sizes based on available viewport space
    // This ensures proper scaling regardless of window size
    container(responsive(move |size| {
        let available_width = size.width;
        let available_height = size.height;

        // Calculate effective zoom based on view mode
        let effective_zoom = match view_mode {
            ViewMode::Fit => {
                // Calculate zoom to fit image in viewport (maintain aspect ratio)
                let zoom_x = available_width / img_width;
                let zoom_y = available_height / img_height;
                zoom_x.min(zoom_y).min(1.0) // Don't zoom in beyond 100%
            }
            ViewMode::ActualSize => 1.0,
            ViewMode::Custom => scale,
        };

        // Calculate scaled dimensions for display
        let scaled_width = img_width * effective_zoom;
        let scaled_height = img_height * effective_zoom;

        // Create image widget with calculated size
        // ContentFit::Fill ensures the image fills the specified dimensions
        let image_widget = image(handle_clone.clone())
            .content_fit(ContentFit::Fill)
            .width(Length::Fixed(scaled_width))
            .height(Length::Fixed(scaled_height));

        // If image is larger than viewport, wrap in scrollable for panning
        if scaled_width > available_width || scaled_height > available_height {
            // Calculate padding to center the image when not scrolled
            let pad_x = ((available_width - scaled_width) / 2.0).max(0.0);
            let pad_y = ((available_height - scaled_height) / 2.0).max(0.0);

            // Scrollable provides automatic panning via scrollbars/mouse drag
            container(
                scrollable(
                    container(image_widget)
                        .width(Length::Shrink)
                        .height(Length::Shrink)
                        .padding([pad_y, pad_x]),
                )
                .id(Id::new("canvas-scroll"))
                .direction(Direction::Both {
                    vertical: Scrollbar::default(),
                    horizontal: Scrollbar::default(),
                })
                .width(Length::Fill)
                .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        } else {
            // Image fits in viewport - just center it
            container(image_widget)
                .width(Length::Fill)
                .height(Length::Fill)
                .center(Length::Fill)
                .into()
        }
    }))
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
