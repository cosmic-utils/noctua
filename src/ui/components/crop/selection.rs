// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/crop/selection.rs
//
// Crop selection state with direction-based drag handle system.

use cosmic::iced::{Point, Rectangle, Size};

/// Minimum selection size in pixels.
const MIN_SIZE: f32 = 1.0;

/// Represents a crop region in canvas coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CropRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl CropRegion {
    /// Create a new crop region.
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if region is valid (has positive dimensions).
    pub fn is_valid(&self) -> bool {
        self.width > 1.0 && self.height > 1.0
    }

    /// Convert to tuple representation (for backward compatibility).
    pub fn as_tuple(&self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.width, self.height)
    }

    /// Create from tuple representation.
    pub fn from_tuple(tuple: (f32, f32, f32, f32)) -> Self {
        Self::new(tuple.0, tuple.1, tuple.2, tuple.3)
    }

    /// Convert to Rectangle.
    pub fn as_rectangle(&self) -> Rectangle {
        Rectangle::new(
            Point::new(self.x, self.y),
            Size::new(self.width, self.height),
        )
    }

    /// Convert to pixel coordinates (for image operations).
    pub fn as_pixel_rect(&self) -> Option<(u32, u32, u32, u32)> {
        if self.is_valid() {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            Some((
                self.x as u32,
                self.y as u32,
                self.width as u32,
                self.height as u32,
            ))
        } else {
            None
        }
    }
}

/// Resize direction flags (can be combined for corners).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Direction {
    pub north: bool,
    pub south: bool,
    pub east: bool,
    pub west: bool,
}

impl Direction {
    pub const NONE: Self = Self {
        north: false,
        south: false,
        east: false,
        west: false,
    };
    pub const NORTH: Self = Self {
        north: true,
        south: false,
        east: false,
        west: false,
    };
    pub const SOUTH: Self = Self {
        north: false,
        south: true,
        east: false,
        west: false,
    };
    pub const EAST: Self = Self {
        north: false,
        south: false,
        east: true,
        west: false,
    };
    pub const WEST: Self = Self {
        north: false,
        south: false,
        east: false,
        west: true,
    };
    pub const NORTH_WEST: Self = Self {
        north: true,
        south: false,
        east: false,
        west: true,
    };
    pub const NORTH_EAST: Self = Self {
        north: true,
        south: false,
        east: true,
        west: false,
    };
    pub const SOUTH_WEST: Self = Self {
        north: false,
        south: true,
        east: false,
        west: true,
    };
    pub const SOUTH_EAST: Self = Self {
        north: false,
        south: true,
        east: true,
        west: false,
    };
}

/// Drag handle type for crop selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DragHandle {
    #[default]
    None,
    /// Resizing from an edge or corner (direction specifies which).
    Resize(Direction),
    /// Moving the entire selection.
    Move,
}

impl DragHandle {
    // Convenience constructors for backward compatibility
    pub const TOP_LEFT: Self = Self::Resize(Direction::NORTH_WEST);
    pub const TOP_RIGHT: Self = Self::Resize(Direction::NORTH_EAST);
    pub const BOTTOM_LEFT: Self = Self::Resize(Direction::SOUTH_WEST);
    pub const BOTTOM_RIGHT: Self = Self::Resize(Direction::SOUTH_EAST);
    pub const TOP: Self = Self::Resize(Direction::NORTH);
    pub const BOTTOM: Self = Self::Resize(Direction::SOUTH);
    pub const LEFT: Self = Self::Resize(Direction::WEST);
    pub const RIGHT: Self = Self::Resize(Direction::EAST);
}

/// Crop selection in screen coordinates (relative to canvas bounds).
#[derive(Debug, Clone, Default)]
pub struct CropSelection {
    pub region: Option<CropRegion>,
    pub is_dragging: bool,
    pub drag_handle: DragHandle,
    drag_start: Option<(f32, f32)>,
    drag_start_region: Option<CropRegion>,
    /// Canvas bounds (width, height) from last drag update
    pub canvas_bounds: Option<(f32, f32)>,
}

impl CropSelection {
    pub fn start_new_selection(&mut self, x: f32, y: f32) {
        self.region = Some(CropRegion::new(x, y, 0.0, 0.0));
        self.is_dragging = true;
        self.drag_handle = DragHandle::None;
        self.drag_start = Some((x, y));
        self.drag_start_region = None;
    }

    pub fn start_handle_drag(&mut self, handle: DragHandle, x: f32, y: f32) {
        self.is_dragging = true;
        self.drag_handle = handle;
        self.drag_start = Some((x, y));
        self.drag_start_region = self.region;
    }

    pub fn update_drag(&mut self, x: f32, y: f32, max_x: f32, max_y: f32) {
        if !self.is_dragging {
            return;
        }

        self.canvas_bounds = Some((max_x, max_y));

        match self.drag_handle {
            DragHandle::None => {
                // Creating new selection
                if let Some((start_x, start_y)) = self.drag_start {
                    let min_x = start_x.min(x).max(0.0);
                    let min_y = start_y.min(y).max(0.0);
                    let max_x_clamped = start_x.max(x).min(max_x);
                    let max_y_clamped = start_y.max(y).min(max_y);
                    self.region = Some(CropRegion::new(
                        min_x,
                        min_y,
                        max_x_clamped - min_x,
                        max_y_clamped - min_y,
                    ));
                }
            }
            DragHandle::Move => {
                // Moving entire selection
                if let (Some((start_x, start_y)), Some(region)) =
                    (self.drag_start, self.drag_start_region)
                {
                    let dx = x - start_x;
                    let dy = y - start_y;
                    let new_x = (region.x + dx).clamp(0.0, max_x - region.width);
                    let new_y = (region.y + dy).clamp(0.0, max_y - region.height);
                    self.region = Some(CropRegion::new(new_x, new_y, region.width, region.height));
                }
            }
            DragHandle::Resize(dir) => {
                // Resizing from edge/corner
                if let (Some((start_x, start_y)), Some(region)) =
                    (self.drag_start, self.drag_start_region)
                {
                    let dx = x - start_x;
                    let dy = y - start_y;
                    self.region = Some(CropRegion::from_tuple(resize_region(
                        region.x,
                        region.y,
                        region.width,
                        region.height,
                        dx,
                        dy,
                        dir,
                        max_x,
                        max_y,
                    )));
                }
            }
        }
    }

    pub fn end_drag(&mut self) {
        self.is_dragging = false;
        self.drag_start = None;
        self.drag_start_region = None;
    }

    pub fn reset(&mut self) {
        self.region = None;
        self.is_dragging = false;
        self.drag_handle = DragHandle::None;
        self.drag_start = None;
        self.drag_start_region = None;
        self.canvas_bounds = None;
    }

    pub fn has_selection(&self) -> bool {
        self.region.is_some_and(|r| r.is_valid())
    }

    /// Get the crop region (if any).
    pub fn get_region(&self) -> Option<CropRegion> {
        self.region
    }

    /// Returns the crop region as pixel coordinates (for saving).
    /// Note: This returns canvas coordinates, not image coordinates.
    /// Use with coordinate transformation for accurate image cropping.
    pub fn as_pixel_rect(&self) -> Option<(u32, u32, u32, u32)> {
        self.region.and_then(|r| r.as_pixel_rect())
    }
}

/// Resize a region based on drag delta and direction flags.
fn resize_region(
    rx: f32,
    ry: f32,
    rw: f32,
    rh: f32,
    dx: f32,
    dy: f32,
    dir: Direction,
    max_x: f32,
    max_y: f32,
) -> (f32, f32, f32, f32) {
    let mut new_x = rx;
    let mut new_y = ry;
    let mut new_w = rw;
    let mut new_h = rh;

    // Handle horizontal resize
    if dir.west {
        // Dragging left edge
        let proposed_x = (rx + dx).max(0.0);
        let proposed_w = (rx + rw) - proposed_x;
        if proposed_w >= MIN_SIZE {
            new_x = proposed_x;
            new_w = proposed_w;
        } else {
            new_x = (rx + rw) - MIN_SIZE;
            new_w = MIN_SIZE;
        }
    } else if dir.east {
        // Dragging right edge
        let proposed_right = (rx + rw + dx).min(max_x);
        new_w = (proposed_right - rx).max(MIN_SIZE);
    }

    // Handle vertical resize
    if dir.north {
        // Dragging top edge
        let proposed_y = (ry + dy).max(0.0);
        let proposed_h = (ry + rh) - proposed_y;
        if proposed_h >= MIN_SIZE {
            new_y = proposed_y;
            new_h = proposed_h;
        } else {
            new_y = (ry + rh) - MIN_SIZE;
            new_h = MIN_SIZE;
        }
    } else if dir.south {
        // Dragging bottom edge
        let proposed_bottom = (ry + rh + dy).min(max_y);
        new_h = (proposed_bottom - ry).max(MIN_SIZE);
    }

    (new_x, new_y, new_w, new_h)
}
