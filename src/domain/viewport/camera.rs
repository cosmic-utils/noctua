// SPDX-License-Identifier: GPL-3.0-or-later
// src/domain/viewport/camera.rs
//
// Camera controls and transformations for viewport navigation.

use super::viewport::Viewport;

/// Camera pan direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanDirection {
    /// Pan left.
    Left,
    /// Pan right.
    Right,
    /// Pan up.
    Up,
    /// Pan down.
    Down,
}

/// Camera movement speed presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanSpeed {
    /// Slow pan (10% of viewport).
    Slow,
    /// Normal pan (25% of viewport).
    Normal,
    /// Fast pan (50% of viewport).
    Fast,
}

impl PanSpeed {
    /// Get the multiplier for this speed.
    #[must_use]
    pub fn multiplier(self) -> f32 {
        match self {
            Self::Slow => 0.1,
            Self::Normal => 0.25,
            Self::Fast => 0.5,
        }
    }
}

impl Default for PanSpeed {
    fn default() -> Self {
        Self::Normal
    }
}

/// Camera controller for viewport navigation.
///
/// Provides high-level camera operations like directional panning,
/// smooth zooming, and bounds checking.
pub struct Camera {
    /// Default pan speed.
    pan_speed: PanSpeed,
    /// Zoom step multiplier.
    zoom_step: f32,
}

impl Camera {
    /// Create a new camera controller with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pan_speed: PanSpeed::default(),
            zoom_step: 1.25,
        }
    }

    /// Set the default pan speed.
    pub fn set_pan_speed(&mut self, speed: PanSpeed) {
        self.pan_speed = speed;
    }

    /// Set the zoom step multiplier.
    pub fn set_zoom_step(&mut self, step: f32) {
        self.zoom_step = step.max(1.01);
    }

    /// Pan the viewport in a specific direction.
    ///
    /// The pan amount is calculated as a percentage of the canvas size
    /// based on the current pan speed.
    pub fn pan(&self, viewport: &mut Viewport, direction: PanDirection) {
        self.pan_with_speed(viewport, direction, self.pan_speed);
    }

    /// Pan with a specific speed.
    pub fn pan_with_speed(
        &self,
        viewport: &mut Viewport,
        direction: PanDirection,
        speed: PanSpeed,
    ) {
        let (canvas_width, canvas_height) = viewport.canvas_size();
        let multiplier = speed.multiplier();

        let (dx, dy) = match direction {
            PanDirection::Left => (canvas_width * multiplier, 0.0),
            PanDirection::Right => (-canvas_width * multiplier, 0.0),
            PanDirection::Up => (0.0, canvas_height * multiplier),
            PanDirection::Down => (0.0, -canvas_height * multiplier),
        };

        viewport.pan_by(dx, dy);
    }

    /// Zoom in using the default zoom step.
    pub fn zoom_in(&self, viewport: &mut Viewport) {
        viewport.zoom_in(self.zoom_step);
    }

    /// Zoom out using the default zoom step.
    pub fn zoom_out(&self, viewport: &mut Viewport) {
        viewport.zoom_out(self.zoom_step);
    }

    /// Zoom to a specific scale factor.
    pub fn zoom_to(&self, viewport: &mut Viewport, scale: f32) {
        viewport.set_scale(scale);
    }

    /// Center the document in the viewport.
    pub fn center(&self, viewport: &mut Viewport) {
        viewport.reset_pan();
    }

    /// Calculate pan delta to center a specific point in the viewport.
    ///
    /// Returns (dx, dy) to apply to pan offset.
    #[must_use]
    pub fn calculate_pan_to_center_point(
        &self,
        viewport: &Viewport,
        doc_x: f32,
        doc_y: f32,
    ) -> (f32, f32) {
        let (canvas_width, canvas_height) = viewport.canvas_size();
        let _scale = viewport.scale();

        // Convert document point to screen space
        let (screen_x, screen_y) = viewport.document_to_screen(doc_x, doc_y);

        // Calculate delta to center point
        let center_x = canvas_width / 2.0;
        let center_y = canvas_height / 2.0;

        (center_x - screen_x, center_y - screen_y)
    }

    /// Pan to center a specific document point in the viewport.
    pub fn pan_to_center_point(&self, viewport: &mut Viewport, doc_x: f32, doc_y: f32) {
        let (dx, dy) = self.calculate_pan_to_center_point(viewport, doc_x, doc_y);
        viewport.pan_by(dx, dy);
    }

    /// Zoom to a specific point (zoom centered on that point).
    pub fn zoom_at_point(
        &self,
        viewport: &mut Viewport,
        screen_x: f32,
        screen_y: f32,
        zoom_factor: f32,
    ) {
        // Convert screen point to document coordinates before zoom
        let (doc_x, doc_y) = viewport.screen_to_document(screen_x, screen_y);

        // Apply zoom
        let old_scale = viewport.scale();
        let new_scale = old_scale * zoom_factor;
        viewport.set_scale(new_scale);

        // Convert document point back to screen coordinates after zoom
        let (new_screen_x, new_screen_y) = viewport.document_to_screen(doc_x, doc_y);

        // Calculate pan adjustment to keep point under cursor
        let dx = screen_x - new_screen_x;
        let dy = screen_y - new_screen_y;

        viewport.pan_by(dx, dy);
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_creation() {
        let camera = Camera::new();
        assert_eq!(camera.pan_speed, PanSpeed::Normal);
        assert_eq!(camera.zoom_step, 1.25);
    }

    #[test]
    fn test_pan_speed_multiplier() {
        assert_eq!(PanSpeed::Slow.multiplier(), 0.1);
        assert_eq!(PanSpeed::Normal.multiplier(), 0.25);
        assert_eq!(PanSpeed::Fast.multiplier(), 0.5);
    }

    #[test]
    fn test_pan_direction() {
        let camera = Camera::new();
        let mut viewport = Viewport::new();
        viewport.set_canvas_size(800.0, 600.0);

        camera.pan(&mut viewport, PanDirection::Right);
        let (pan_x, _) = viewport.pan_offset();
        assert!(pan_x < 0.0); // Right pan moves content left

        camera.pan(&mut viewport, PanDirection::Left);
        let (pan_x, _) = viewport.pan_offset();
        assert_eq!(pan_x, 0.0); // Should cancel out
    }

    #[test]
    fn test_zoom() {
        let camera = Camera::new();
        let mut viewport = Viewport::new();
        viewport.set_scale(1.0);

        camera.zoom_in(&mut viewport);
        assert_eq!(viewport.scale(), 1.25);

        camera.zoom_out(&mut viewport);
        assert_eq!(viewport.scale(), 1.0);
    }
}
