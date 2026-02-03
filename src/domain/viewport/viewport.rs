// SPDX-License-Identifier: GPL-3.0-or-later
// src/domain/viewport/viewport.rs
//
// Viewport state and transformations for document viewing.

/// View mode for document display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Fit entire document in viewport.
    Fit,
    /// Display at actual size (1:1 pixel ratio).
    ActualSize,
    /// Custom zoom level.
    Custom,
}

impl Default for ViewMode {
    fn default() -> Self {
        Self::Fit
    }
}

/// Viewport state for document display.
///
/// Manages pan, zoom, and view mode transformations.
#[derive(Debug, Clone, PartialEq)]
pub struct Viewport {
    /// Current view mode.
    view_mode: ViewMode,
    /// Pan offset X (in screen pixels).
    pan_x: f32,
    /// Pan offset Y (in screen pixels).
    pan_y: f32,
    /// Current scale factor.
    scale: f32,
    /// Canvas dimensions (viewport size).
    canvas_width: f32,
    canvas_height: f32,
    /// Document dimensions (content size).
    document_width: f32,
    document_height: f32,
}

impl Viewport {
    /// Create a new viewport with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            view_mode: ViewMode::Fit,
            pan_x: 0.0,
            pan_y: 0.0,
            scale: 1.0,
            canvas_width: 0.0,
            canvas_height: 0.0,
            document_width: 0.0,
            document_height: 0.0,
        }
    }

    /// Set the canvas (viewport) dimensions.
    pub fn set_canvas_size(&mut self, width: f32, height: f32) {
        self.canvas_width = width;
        self.canvas_height = height;
        self.update_scale_if_fit();
    }

    /// Set the document dimensions.
    pub fn set_document_size(&mut self, width: f32, height: f32) {
        self.document_width = width;
        self.document_height = height;
        self.update_scale_if_fit();
    }

    /// Get the current view mode.
    #[must_use]
    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }

    /// Set the view mode.
    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.view_mode = mode;
        match mode {
            ViewMode::Fit => {
                self.reset_pan();
                self.update_scale_if_fit();
            }
            ViewMode::ActualSize => {
                self.reset_pan();
                self.scale = 1.0;
            }
            ViewMode::Custom => {
                // Keep current scale and pan
            }
        }
    }

    /// Get the current scale factor.
    #[must_use]
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Set the scale factor (switches to Custom mode).
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale.max(0.01); // Minimum scale
        self.view_mode = ViewMode::Custom;
    }

    /// Zoom in by a factor.
    pub fn zoom_in(&mut self, factor: f32) {
        self.set_scale(self.scale * factor);
    }

    /// Zoom out by a factor.
    pub fn zoom_out(&mut self, factor: f32) {
        self.set_scale(self.scale / factor);
    }

    /// Get pan offset.
    #[must_use]
    pub fn pan_offset(&self) -> (f32, f32) {
        (self.pan_x, self.pan_y)
    }

    /// Set pan offset.
    pub fn set_pan(&mut self, x: f32, y: f32) {
        self.pan_x = x;
        self.pan_y = y;
        if self.view_mode == ViewMode::Fit {
            self.view_mode = ViewMode::Custom;
        }
    }

    /// Pan by a delta.
    pub fn pan_by(&mut self, dx: f32, dy: f32) {
        self.pan_x += dx;
        self.pan_y += dy;
        if self.view_mode == ViewMode::Fit {
            self.view_mode = ViewMode::Custom;
        }
    }

    /// Reset pan to center.
    pub fn reset_pan(&mut self) {
        self.pan_x = 0.0;
        self.pan_y = 0.0;
    }

    /// Get canvas dimensions.
    #[must_use]
    pub fn canvas_size(&self) -> (f32, f32) {
        (self.canvas_width, self.canvas_height)
    }

    /// Get document dimensions.
    #[must_use]
    pub fn document_size(&self) -> (f32, f32) {
        (self.document_width, self.document_height)
    }

    /// Get scaled document dimensions.
    #[must_use]
    pub fn scaled_document_size(&self) -> (f32, f32) {
        (
            self.document_width * self.scale,
            self.document_height * self.scale,
        )
    }

    /// Calculate the scale to fit the document in the viewport.
    #[must_use]
    pub fn calculate_fit_scale(&self) -> f32 {
        if self.document_width == 0.0 || self.document_height == 0.0 {
            return 1.0;
        }

        let width_scale = self.canvas_width / self.document_width;
        let height_scale = self.canvas_height / self.document_height;

        width_scale.min(height_scale)
    }

    /// Update scale to fit mode if currently in fit mode.
    fn update_scale_if_fit(&mut self) {
        if self.view_mode == ViewMode::Fit {
            self.scale = self.calculate_fit_scale();
        }
    }

    /// Convert screen coordinates to document coordinates.
    #[must_use]
    pub fn screen_to_document(&self, screen_x: f32, screen_y: f32) -> (f32, f32) {
        let (scaled_width, scaled_height) = self.scaled_document_size();

        // Calculate document position in canvas
        let doc_x = (self.canvas_width - scaled_width) / 2.0 + self.pan_x;
        let doc_y = (self.canvas_height - scaled_height) / 2.0 + self.pan_y;

        // Convert screen to document coordinates
        let rel_x = screen_x - doc_x;
        let rel_y = screen_y - doc_y;

        (rel_x / self.scale, rel_y / self.scale)
    }

    /// Convert document coordinates to screen coordinates.
    #[must_use]
    pub fn document_to_screen(&self, doc_x: f32, doc_y: f32) -> (f32, f32) {
        let (scaled_width, scaled_height) = self.scaled_document_size();

        // Calculate document position in canvas
        let offset_x = (self.canvas_width - scaled_width) / 2.0 + self.pan_x;
        let offset_y = (self.canvas_height - scaled_height) / 2.0 + self.pan_y;

        (
            offset_x + doc_x * self.scale,
            offset_y + doc_y * self.scale,
        )
    }

    /// Get the visible bounds of the document in document coordinates.
    ///
    /// Returns (x, y, width, height) of the visible region.
    #[must_use]
    pub fn visible_bounds(&self) -> (f32, f32, f32, f32) {
        let (top_left_x, top_left_y) = self.screen_to_document(0.0, 0.0);
        let (bottom_right_x, bottom_right_y) =
            self.screen_to_document(self.canvas_width, self.canvas_height);

        let x = top_left_x.max(0.0);
        let y = top_left_y.max(0.0);
        let width = (bottom_right_x - top_left_x).min(self.document_width - x);
        let height = (bottom_right_y - top_left_y).min(self.document_height - y);

        (x, y, width, height)
    }

    /// Reset viewport to default state.
    pub fn reset(&mut self) {
        self.view_mode = ViewMode::Fit;
        self.pan_x = 0.0;
        self.pan_y = 0.0;
        self.update_scale_if_fit();
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_creation() {
        let viewport = Viewport::new();
        assert_eq!(viewport.view_mode(), ViewMode::Fit);
        assert_eq!(viewport.scale(), 1.0);
        assert_eq!(viewport.pan_offset(), (0.0, 0.0));
    }

    #[test]
    fn test_fit_scale_calculation() {
        let mut viewport = Viewport::new();
        viewport.set_canvas_size(800.0, 600.0);
        viewport.set_document_size(1600.0, 1200.0);

        assert_eq!(viewport.calculate_fit_scale(), 0.5);
    }

    #[test]
    fn test_zoom() {
        let mut viewport = Viewport::new();
        viewport.set_scale(1.0);

        viewport.zoom_in(2.0);
        assert_eq!(viewport.scale(), 2.0);
        assert_eq!(viewport.view_mode(), ViewMode::Custom);

        viewport.zoom_out(2.0);
        assert_eq!(viewport.scale(), 1.0);
    }

    #[test]
    fn test_coordinate_conversion() {
        let mut viewport = Viewport::new();
        viewport.set_canvas_size(800.0, 600.0);
        viewport.set_document_size(400.0, 300.0);
        viewport.set_scale(1.0);

        // Document should be centered in canvas
        let (screen_x, screen_y) = viewport.document_to_screen(0.0, 0.0);
        assert_eq!(screen_x, 200.0); // (800 - 400) / 2
        assert_eq!(screen_y, 150.0); // (600 - 300) / 2
    }
}
