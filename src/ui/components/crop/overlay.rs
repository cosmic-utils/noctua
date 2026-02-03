// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/crop/overlay.rs
//
// Crop overlay widget with selection UI (overlay, border, handles, grid).
// Works entirely in RELATIVE canvas coordinates - no transformations!

/// Crop overlay handle size in pixels (visual size of corner/edge handles).
const CROP_HANDLE_SIZE: f32 = 14.0;

/// Crop overlay handle hit area size in pixels (larger for easier interaction).
const CROP_HANDLE_HIT_SIZE: f32 = 28.0;

/// Crop overlay border width in pixels (selection rectangle outline).
const CROP_BORDER_WIDTH: f32 = 2.0;

/// Crop overlay grid line width in pixels (rule of thirds guide).
const CROP_GRID_WIDTH: f32 = 1.0;

use crate::{
    ui::{
        components::crop::{
            selection::{CropRegion, CropSelection, DragHandle},
            theme,
        },
        AppMessage,
    },
};
use cosmic::{
    Element, Renderer,
    iced::{
        Color, Length, Point, Rectangle, Size,
        advanced::{
            Clipboard, Layout, Shell, Widget,
            layout::{Limits, Node},
            renderer::{Quad, Renderer as QuadRenderer},
            widget::Tree,
        },
        event::{Event, Status},
        mouse::{self, Button, Cursor},
    },
};

pub struct CropOverlay {
    selection: CropSelection,
    show_grid: bool,
}

impl CropOverlay {
    pub fn new(selection: &CropSelection, show_grid: bool) -> Self {
        Self {
            selection: selection.clone(),
            show_grid,
        }
    }

    /// Hit-test handles in RELATIVE canvas coordinates.
    fn hit_test_handle(&self, rel_point: Point) -> DragHandle {
        let Some(region) = self.selection.region else {
            return DragHandle::None;
        };

        // All coordinates are relative - no conversion needed!
        let handles = [
            (Point::new(region.x, region.y), DragHandle::TOP_LEFT),
            (
                Point::new(region.x + region.width, region.y),
                DragHandle::TOP_RIGHT,
            ),
            (
                Point::new(region.x, region.y + region.height),
                DragHandle::BOTTOM_LEFT,
            ),
            (
                Point::new(region.x + region.width, region.y + region.height),
                DragHandle::BOTTOM_RIGHT,
            ),
            (
                Point::new(region.x + region.width / 2.0, region.y),
                DragHandle::TOP,
            ),
            (
                Point::new(region.x + region.width / 2.0, region.y + region.height),
                DragHandle::BOTTOM,
            ),
            (
                Point::new(region.x, region.y + region.height / 2.0),
                DragHandle::LEFT,
            ),
            (
                Point::new(region.x + region.width, region.y + region.height / 2.0),
                DragHandle::RIGHT,
            ),
        ];

        // Test handles
        for (pos, handle) in handles {
            if point_in_handle(rel_point, pos) {
                return handle;
            }
        }

        // Test if inside selection (move)
        if region.as_rectangle().contains(rel_point) {
            return DragHandle::Move;
        }

        DragHandle::None
    }

    fn cursor_for_handle(&self, handle: DragHandle) -> mouse::Interaction {
        match handle {
            DragHandle::Resize(dir) => {
                // Determine cursor based on direction flags
                let is_diagonal = (dir.north || dir.south) && (dir.east || dir.west);
                let is_nwse = (dir.north && dir.west) || (dir.south && dir.east);
                let is_nesw = (dir.north && dir.east) || (dir.south && dir.west);

                if is_diagonal && is_nwse {
                    mouse::Interaction::ResizingDiagonallyDown
                } else if is_diagonal && is_nesw {
                    mouse::Interaction::ResizingDiagonallyUp
                } else if dir.north || dir.south {
                    mouse::Interaction::ResizingVertically
                } else if dir.east || dir.west {
                    mouse::Interaction::ResizingHorizontally
                } else {
                    mouse::Interaction::Crosshair
                }
            }
            DragHandle::Move => mouse::Interaction::Grabbing,
            DragHandle::None => mouse::Interaction::Crosshair,
        }
    }

    fn draw_overlay_areas(
        &self,
        renderer: &mut Renderer,
        bounds: &Rectangle,
        region: CropRegion,
        overlay_color: Color,
    ) {
        let (rx, ry, rw, rh) = region.as_tuple();
        // Convert to absolute screen coordinates for drawing
        let sel_y = bounds.y + ry;

        // Top overlay (above selection)
        if ry > 0.0 {
            draw_quad(
                renderer,
                Rectangle::new(bounds.position(), Size::new(bounds.width, ry)),
                overlay_color,
            );
        }

        // Bottom overlay (below selection)
        let sel_bottom_rel = ry + rh;
        if sel_bottom_rel < bounds.height {
            draw_quad(
                renderer,
                Rectangle::new(
                    Point::new(bounds.x, bounds.y + sel_bottom_rel),
                    Size::new(bounds.width, bounds.height - sel_bottom_rel),
                ),
                overlay_color,
            );
        }

        // Left overlay
        if rx > 0.0 {
            draw_quad(
                renderer,
                Rectangle::new(Point::new(bounds.x, sel_y), Size::new(rx, rh)),
                overlay_color,
            );
        }

        // Right overlay
        let sel_right_rel = rx + rw;
        if sel_right_rel < bounds.width {
            draw_quad(
                renderer,
                Rectangle::new(
                    Point::new(bounds.x + sel_right_rel, sel_y),
                    Size::new(bounds.width - sel_right_rel, rh),
                ),
                overlay_color,
            );
        }
    }

    fn draw_border(
        &self,
        renderer: &mut Renderer,
        bounds: &Rectangle,
        region: CropRegion,
        border_color: Color,
    ) {
        let (rx, ry, rw, rh) = region.as_tuple();
        let border_width = CROP_BORDER_WIDTH;
        let x = bounds.x + rx;
        let y = bounds.y + ry;

        // Top border
        draw_quad(
            renderer,
            Rectangle::new(Point::new(x, y), Size::new(rw, border_width)),
            border_color,
        );

        // Bottom border
        draw_quad(
            renderer,
            Rectangle::new(
                Point::new(x, y + rh - border_width),
                Size::new(rw, border_width),
            ),
            border_color,
        );

        // Left border
        draw_quad(
            renderer,
            Rectangle::new(Point::new(x, y), Size::new(border_width, rh)),
            border_color,
        );

        // Right border
        draw_quad(
            renderer,
            Rectangle::new(
                Point::new(x + rw - border_width, y),
                Size::new(border_width, rh),
            ),
            border_color,
        );
    }

    fn draw_handles(
        &self,
        renderer: &mut Renderer,
        bounds: &Rectangle,
        region: CropRegion,
        handle_color: Color,
    ) {
        let (rx, ry, rw, rh) = region.as_tuple();
        let half = CROP_HANDLE_SIZE / 2.0;
        let x = bounds.x + rx;
        let y = bounds.y + ry;

        // 8 handle positions (4 corners + 4 edges)
        let handles = [
            (x, y),                 // Top-left
            (x + rw, y),            // Top-right
            (x, y + rh),            // Bottom-left
            (x + rw, y + rh),       // Bottom-right
            (x + rw / 2.0, y),      // Mid-top
            (x + rw / 2.0, y + rh), // Mid-bottom
            (x, y + rh / 2.0),      // Mid-left
            (x + rw, y + rh / 2.0), // Mid-right
        ];

        for (hx, hy) in handles {
            draw_quad(
                renderer,
                Rectangle::new(
                    Point::new(hx - half, hy - half),
                    Size::new(CROP_HANDLE_SIZE, CROP_HANDLE_SIZE),
                ),
                handle_color,
            );
        }
    }

    fn draw_grid(
        &self,
        renderer: &mut Renderer,
        bounds: &Rectangle,
        region: CropRegion,
        grid_color: Color,
    ) {
        if !self.show_grid || region.width <= 10.0 || region.height <= 10.0 {
            return;
        }

        let (rx, ry, rw, rh) = region.as_tuple();
        let x = bounds.x + rx;
        let y = bounds.y + ry;
        let grid_split_x = rw / 3.0;
        let grid_split_y = rh / 3.0;

        // Draw rule of thirds grid (2 vertical + 2 horizontal lines)
        for i in 1..3 {
            let offset_x = x + grid_split_x * i as f32;
            let offset_y = y + grid_split_y * i as f32;

            // Vertical line
            draw_quad(
                renderer,
                Rectangle::new(Point::new(offset_x, y), Size::new(CROP_GRID_WIDTH, rh)),
                grid_color,
            );

            // Horizontal line
            draw_quad(
                renderer,
                Rectangle::new(Point::new(x, offset_y), Size::new(rw, CROP_GRID_WIDTH)),
                grid_color,
            );
        }
    }
}

impl Widget<AppMessage, cosmic::Theme, Renderer> for CropOverlay {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(&self, _tree: &mut Tree, _renderer: &Renderer, limits: &Limits) -> Node {
        Node::new(limits.max())
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        theme: &cosmic::Theme,
        _style: &cosmic::iced::advanced::renderer::Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        // Early return if no selection
        let Some(region) = self.selection.region else {
            draw_quad(renderer, bounds, theme::overlay_color(theme));
            return;
        };

        // Check if selection is valid
        if !region.is_valid() {
            draw_quad(renderer, bounds, theme::overlay_color(theme));
            return;
        }

        // Get theme colors
        let overlay_color = theme::overlay_color(theme);
        let border_color = theme::border_color(theme);
        let handle_color = theme::handle_color(theme);
        let grid_color = theme::grid_color(theme);

        // Draw overlay areas (darkened regions)
        self.draw_overlay_areas(renderer, &bounds, region, overlay_color);

        // Draw border
        self.draw_border(renderer, &bounds, region, border_color);

        // Draw handles
        self.draw_handles(renderer, &bounds, region, handle_color);

        // Draw grid
        self.draw_grid(renderer, &bounds, region, grid_color);
    }

    fn on_event(
        &mut self,
        _tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, AppMessage>,
        _viewport: &Rectangle,
    ) -> Status {
        let bounds = layout.bounds();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(Button::Left)) => {
                // cursor.position_in(bounds) returns RELATIVE coordinates!
                if let Some(rel_pos) = cursor.position_in(bounds) {
                    let handle = self.hit_test_handle(rel_pos);

                    shell.publish(AppMessage::CropDragStart {
                        x: rel_pos.x,
                        y: rel_pos.y,
                        handle,
                    });
                    return Status::Captured;
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if self.selection.is_dragging
                    && let Some(rel_pos) = cursor.position_in(bounds)
                {
                    shell.publish(AppMessage::CropDragMove {
                        x: rel_pos.x,
                        y: rel_pos.y,
                        max_x: bounds.width,
                        max_y: bounds.height,
                    });
                    return Status::Captured;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(Button::Left)) => {
                if self.selection.is_dragging {
                    shell.publish(AppMessage::CropDragEnd);
                    return Status::Captured;
                }
            }
            _ => {}
        }

        Status::Ignored
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();

        if self.selection.is_dragging {
            return self.cursor_for_handle(self.selection.drag_handle);
        }

        if let Some(rel_pos) = cursor.position_in(bounds) {
            let handle = self.hit_test_handle(rel_pos);
            return self.cursor_for_handle(handle);
        }

        mouse::Interaction::None
    }
}

impl From<CropOverlay> for Element<'_, AppMessage> {
    fn from(overlay: CropOverlay) -> Self {
        Element::new(overlay)
    }
}

pub fn crop_overlay(selection: &CropSelection, show_grid: bool) -> CropOverlay {
    CropOverlay::new(selection, show_grid)
}

// === Helper functions ===

/// Check if a point is within the hit area of a handle.
fn point_in_handle(point: Point, handle_center: Point) -> bool {
    let half = CROP_HANDLE_HIT_SIZE / 2.0;
    point.x >= handle_center.x - half
        && point.x <= handle_center.x + half
        && point.y >= handle_center.y - half
        && point.y <= handle_center.y + half
}

/// Helper to draw a filled quad (reduces repetition).
fn draw_quad(renderer: &mut Renderer, bounds: Rectangle, color: Color) {
    renderer.fill_quad(
        Quad {
            bounds,
            ..Quad::default()
        },
        color,
    );
}
