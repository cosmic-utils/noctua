// SPDX-License-Identifier: GPL-3.0-or-later
// src/domain/viewport/bounds.rs
//
// Bounding box calculations and intersection tests for viewport.

/// A rectangular bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// X coordinate of top-left corner.
    pub x: f32,
    /// Y coordinate of top-left corner.
    pub y: f32,
    /// Width of the bounds.
    pub width: f32,
    /// Height of the bounds.
    pub height: f32,
}

impl Bounds {
    /// Create a new bounds rectangle.
    #[must_use]
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Create bounds from two points (top-left and bottom-right).
    #[must_use]
    pub fn from_corners(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        let x = x1.min(x2);
        let y = y1.min(y2);
        let width = (x2 - x1).abs();
        let height = (y2 - y1).abs();

        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Create bounds centered at a point.
    #[must_use]
    pub fn centered(center_x: f32, center_y: f32, width: f32, height: f32) -> Self {
        Self {
            x: center_x - width / 2.0,
            y: center_y - height / 2.0,
            width,
            height,
        }
    }

    /// Get the right edge coordinate.
    #[must_use]
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Get the bottom edge coordinate.
    #[must_use]
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// Get the center point.
    #[must_use]
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Get the top-left corner.
    #[must_use]
    pub fn top_left(&self) -> (f32, f32) {
        (self.x, self.y)
    }

    /// Get the top-right corner.
    #[must_use]
    pub fn top_right(&self) -> (f32, f32) {
        (self.right(), self.y)
    }

    /// Get the bottom-left corner.
    #[must_use]
    pub fn bottom_left(&self) -> (f32, f32) {
        (self.x, self.bottom())
    }

    /// Get the bottom-right corner.
    #[must_use]
    pub fn bottom_right(&self) -> (f32, f32) {
        (self.right(), self.bottom())
    }

    /// Check if a point is inside this bounds.
    #[must_use]
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.right() && y >= self.y && y <= self.bottom()
    }

    /// Check if this bounds fully contains another bounds.
    #[must_use]
    pub fn contains_bounds(&self, other: &Self) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }

    /// Check if this bounds intersects with another bounds.
    #[must_use]
    pub fn intersects(&self, other: &Self) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    /// Calculate the intersection of two bounds.
    ///
    /// Returns None if the bounds don't intersect.
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        if !self.intersects(other) {
            return None;
        }

        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());

        Some(Self::new(x, y, right - x, bottom - y))
    }

    /// Calculate the union of two bounds (bounding box containing both).
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());

        Self::new(x, y, right - x, bottom - y)
    }

    /// Expand the bounds by a margin on all sides.
    #[must_use]
    pub fn expand(&self, margin: f32) -> Self {
        Self::new(
            self.x - margin,
            self.y - margin,
            self.width + 2.0 * margin,
            self.height + 2.0 * margin,
        )
    }

    /// Shrink the bounds by a margin on all sides.
    ///
    /// Returns None if the bounds would become invalid.
    #[must_use]
    pub fn shrink(&self, margin: f32) -> Option<Self> {
        let new_width = self.width - 2.0 * margin;
        let new_height = self.height - 2.0 * margin;

        if new_width <= 0.0 || new_height <= 0.0 {
            return None;
        }

        Some(Self::new(
            self.x + margin,
            self.y + margin,
            new_width,
            new_height,
        ))
    }

    /// Scale the bounds by a factor from center.
    #[must_use]
    pub fn scale(&self, factor: f32) -> Self {
        let (center_x, center_y) = self.center();
        let new_width = self.width * factor;
        let new_height = self.height * factor;

        Self::centered(center_x, center_y, new_width, new_height)
    }

    /// Translate the bounds by an offset.
    #[must_use]
    pub fn translate(&self, dx: f32, dy: f32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.width, self.height)
    }

    /// Get the area of the bounds.
    #[must_use]
    pub fn area(&self) -> f32 {
        self.width * self.height
    }

    /// Check if the bounds is empty (zero or negative area).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    /// Clamp this bounds to fit within another bounds.
    #[must_use]
    pub fn clamp_to(&self, container: &Self) -> Self {
        let x = self.x.max(container.x).min(container.right() - self.width);
        let y = self.y.max(container.y).min(container.bottom() - self.height);

        Self::new(x, y, self.width, self.height)
    }
}

impl Default for Bounds {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounds_creation() {
        let bounds = Bounds::new(10.0, 20.0, 100.0, 200.0);
        assert_eq!(bounds.x, 10.0);
        assert_eq!(bounds.y, 20.0);
        assert_eq!(bounds.width, 100.0);
        assert_eq!(bounds.height, 200.0);
    }

    #[test]
    fn test_bounds_from_corners() {
        let bounds = Bounds::from_corners(10.0, 20.0, 110.0, 220.0);
        assert_eq!(bounds.x, 10.0);
        assert_eq!(bounds.y, 20.0);
        assert_eq!(bounds.width, 100.0);
        assert_eq!(bounds.height, 200.0);
    }

    #[test]
    fn test_bounds_edges() {
        let bounds = Bounds::new(10.0, 20.0, 100.0, 200.0);
        assert_eq!(bounds.right(), 110.0);
        assert_eq!(bounds.bottom(), 220.0);
    }

    #[test]
    fn test_contains_point() {
        let bounds = Bounds::new(0.0, 0.0, 100.0, 100.0);
        assert!(bounds.contains_point(50.0, 50.0));
        assert!(bounds.contains_point(0.0, 0.0));
        assert!(bounds.contains_point(100.0, 100.0));
        assert!(!bounds.contains_point(-1.0, 50.0));
        assert!(!bounds.contains_point(50.0, 101.0));
    }

    #[test]
    fn test_intersection() {
        let a = Bounds::new(0.0, 0.0, 100.0, 100.0);
        let b = Bounds::new(50.0, 50.0, 100.0, 100.0);

        let intersection = a.intersection(&b).unwrap();
        assert_eq!(intersection.x, 50.0);
        assert_eq!(intersection.y, 50.0);
        assert_eq!(intersection.width, 50.0);
        assert_eq!(intersection.height, 50.0);
    }

    #[test]
    fn test_no_intersection() {
        let a = Bounds::new(0.0, 0.0, 100.0, 100.0);
        let b = Bounds::new(200.0, 200.0, 100.0, 100.0);

        assert!(!a.intersects(&b));
        assert!(a.intersection(&b).is_none());
    }

    #[test]
    fn test_union() {
        let a = Bounds::new(0.0, 0.0, 100.0, 100.0);
        let b = Bounds::new(50.0, 50.0, 100.0, 100.0);

        let union = a.union(&b);
        assert_eq!(union.x, 0.0);
        assert_eq!(union.y, 0.0);
        assert_eq!(union.width, 150.0);
        assert_eq!(union.height, 150.0);
    }

    #[test]
    fn test_expand_shrink() {
        let bounds = Bounds::new(10.0, 10.0, 100.0, 100.0);

        let expanded = bounds.expand(10.0);
        assert_eq!(expanded.x, 0.0);
        assert_eq!(expanded.width, 120.0);

        let shrunk = bounds.shrink(10.0).unwrap();
        assert_eq!(shrunk.x, 20.0);
        assert_eq!(shrunk.width, 80.0);
    }

    #[test]
    fn test_scale() {
        let bounds = Bounds::new(0.0, 0.0, 100.0, 100.0);
        let scaled = bounds.scale(2.0);

        assert_eq!(scaled.width, 200.0);
        assert_eq!(scaled.height, 200.0);
        assert_eq!(scaled.center(), bounds.center());
    }
}
