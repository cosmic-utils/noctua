// SPDX-License-Identifier: GPL-3.0-or-later
// src/application/commands/crop_document.rs
//
// Crop document command: crop the current document to a specified region.

use cosmic::iced::{ContentFit, Size, Vector};

use crate::application::DocumentManager;
use crate::domain::document::core::content::DocumentKind;
use crate::domain::document::core::document::DocResult;
use crate::ui::components::crop::CropRegion;

/// Crop document command.
///
/// Crops the current document to the specified rectangular region.
/// The coordinates are in image pixels (not canvas/screen coordinates).
pub struct CropDocumentCommand {
    /// X coordinate of the crop region (top-left corner).
    pub x: u32,
    /// Y coordinate of the crop region (top-left corner).
    pub y: u32,
    /// Width of the crop region in pixels.
    pub width: u32,
    /// Height of the crop region in pixels.
    pub height: u32,
}

impl CropDocumentCommand {
    /// Create a new crop document command.
    #[must_use]
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Create a crop command from canvas coordinates.
    ///
    /// Converts canvas-space coordinates to image-space pixels based on
    /// the current view state (scale, pan, content fit).
    ///
    /// # Errors
    ///
    /// Returns an error if the crop region is invalid or outside image bounds.
    pub fn from_canvas_selection(
        crop_region: &CropRegion,
        canvas_size: Size,
        image_size: Size,
        scale: f32,
        pan_offset: Vector,
    ) -> Result<Self, String> {
        let canvas_rect = crop_region.as_tuple();

        // Convert canvas coordinates to image pixel coordinates
        let image_rect = Self::canvas_rect_to_image_rect(
            canvas_rect,
            canvas_size,
            image_size,
            scale,
            pan_offset,
            ContentFit::Contain,
        )
        .ok_or_else(|| "Invalid crop region".to_string())?;

        Ok(Self {
            x: image_rect.0,
            y: image_rect.1,
            width: image_rect.2,
            height: image_rect.3,
        })
    }

    /// Convert canvas rectangle to image pixel rectangle.
    ///
    /// This is the core coordinate transformation logic that maps from
    /// canvas/screen coordinates to actual image pixel coordinates.
    fn canvas_rect_to_image_rect(
        canvas_rect: (f32, f32, f32, f32),
        canvas_size: Size,
        image_size: Size,
        scale: f32,
        offset: Vector,
        content_fit: ContentFit,
    ) -> Option<(u32, u32, u32, u32)> {
        let (cx, cy, cw, ch) = canvas_rect;

        if cw <= 1.0 || ch <= 1.0 {
            return None;
        }

        // Transform top-left and bottom-right corners
        let (x1, y1) = Self::canvas_to_image_coords(
            cx,
            cy,
            canvas_size,
            image_size,
            scale,
            offset,
            content_fit,
        );
        let (x2, y2) = Self::canvas_to_image_coords(
            cx + cw,
            cy + ch,
            canvas_size,
            image_size,
            scale,
            offset,
            content_fit,
        );

        // Clamp to image boundaries
        let img_x = x1.max(0.0).min(image_size.width);
        let img_y = y1.max(0.0).min(image_size.height);
        let img_w = (x2 - x1).max(1.0).min(image_size.width - img_x);
        let img_h = (y2 - y1).max(1.0).min(image_size.height - img_y);

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Some((
            img_x.round() as u32,
            img_y.round() as u32,
            img_w.round() as u32,
            img_h.round() as u32,
        ))
    }

    /// Convert a single point from canvas coordinates to image coordinates.
    fn canvas_to_image_coords(
        cx: f32,
        cy: f32,
        canvas_size: Size,
        image_size: Size,
        scale: f32,
        offset: Vector,
        content_fit: ContentFit,
    ) -> (f32, f32) {
        // Calculate displayed image dimensions based on ContentFit
        let (display_w, display_h) = match content_fit {
            ContentFit::Contain => {
                let aspect = image_size.width / image_size.height;
                let canvas_aspect = canvas_size.width / canvas_size.height;

                if aspect > canvas_aspect {
                    // Limited by width
                    (canvas_size.width, canvas_size.width / aspect)
                } else {
                    // Limited by height
                    (canvas_size.height * aspect, canvas_size.height)
                }
            }
            _ => (image_size.width, image_size.height),
        };

        // Apply scale
        let scaled_w = display_w * scale;
        let scaled_h = display_h * scale;

        // Center in canvas
        let center_x = (canvas_size.width - scaled_w) / 2.0;
        let center_y = (canvas_size.height - scaled_h) / 2.0;

        // Convert canvas coords to scaled image coords
        let img_x = (cx - center_x - offset.x) / scale;
        let img_y = (cy - center_y - offset.y) / scale;

        // Scale from display space to actual image pixel space
        let pixel_x = (img_x / display_w) * image_size.width;
        let pixel_y = (img_y / display_h) * image_size.height;

        (pixel_x, pixel_y)
    }

    /// Execute the crop command on the document manager.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No document is currently open
    /// - The document type doesn't support cropping
    /// - The crop region is invalid
    /// - The crop operation fails
    pub fn execute(&self, manager: &mut DocumentManager) -> DocResult<()> {
        let doc = manager
            .current_document_mut()
            .ok_or_else(|| anyhow::anyhow!("No document open"))?;

        // Only raster images support cropping
        if doc.kind() != DocumentKind::Raster {
            return Err(anyhow::anyhow!(
                "Crop operation is only supported for raster images"
            ));
        }

        // Get the raster document and apply crop
        if let crate::domain::document::core::content::DocumentContent::Raster(raster) = doc {
            raster
                .crop(self.x, self.y, self.width, self.height)
                .map_err(|e| anyhow::anyhow!("Crop failed: {}", e))?;
        }

        Ok(())
    }

    /// Check if the command can be executed.
    #[must_use]
    pub fn can_execute(&self, manager: &DocumentManager) -> bool {
        manager
            .current_document()
            .map_or(false, |doc| doc.kind() == DocumentKind::Raster)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_creation() {
        let cmd = CropDocumentCommand::new(10, 20, 100, 150);
        assert_eq!(cmd.x, 10);
        assert_eq!(cmd.y, 20);
        assert_eq!(cmd.width, 100);
        assert_eq!(cmd.height, 150);
    }
}
