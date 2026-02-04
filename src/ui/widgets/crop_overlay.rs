// SPDX-License-Identifier: GPL-3.0-or-later
// src/ui/widgets/crop_overlay.rs
//
// Simple crop overlay widget.

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

use crate::ui::widgets::crop_model::{CropSelection, DragHandle};
use crate::ui::AppMessage;

// Visual constants
const HANDLE_SIZE: f32 = 12.0;
const HANDLE_HIT_SIZE: f32 = 24.0;
const OVERLAY_COLOR: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.5);
const HANDLE_COLOR: Color = Color::WHITE;
const BORDER_COLOR: Color = Color::WHITE;
const BORDER_WIDTH: f32 = 2.0;

/// Simple crop overlay widget.
/// 
/// Works with RELATIVE coordinates - selection.region is relative to bounds (0,0).
pub struct CropOverlay {
    selection: CropSelection,
    show_grid: bool,
    last_click: Option<std::time::Instant>,
}

impl CropOverlay {
    pub fn new(selection: &CropSelection, show_grid: bool) -> Self {
        Self {
            selection: selection.clone(),
            last_click: None,
            show_grid,
        }
    }

    /// Convert relative coords to absolute screen coords.
    fn to_screen(&self, x: f32, y: f32, bounds: &Rectangle) -> Point {
        Point::new(bounds.x + x, bounds.y + y)
    }

    /// Convert absolute screen coords to relative coords.
    fn to_relative(&self, point: Point, bounds: &Rectangle) -> Point {
        Point::new(point.x - bounds.x, point.y - bounds.y)
    }

    /// Hit test for handles (in relative coordinates).
    fn hit_test_handle(&self, rel_point: Point) -> DragHandle {
        let Some((x, y, w, h)) = self.selection.region else {
            return DragHandle::None;
        };

        // 8 handle positions (relative coordinates)
        let handles = [
            (Point::new(x, y), DragHandle::TopLeft),
            (Point::new(x + w, y), DragHandle::TopRight),
            (Point::new(x, y + h), DragHandle::BottomLeft),
            (Point::new(x + w, y + h), DragHandle::BottomRight),
            (Point::new(x + w / 2.0, y), DragHandle::Top),
            (Point::new(x + w / 2.0, y + h), DragHandle::Bottom),
            (Point::new(x, y + h / 2.0), DragHandle::Left),
            (Point::new(x + w, y + h / 2.0), DragHandle::Right),
        ];

        // Test handles
        for (pos, handle) in handles {
            if point_in_handle(rel_point, pos) {
                return handle;
            }
        }

        // Test if inside selection (move)
        if rel_point.x >= x && rel_point.x <= x + w && rel_point.y >= y && rel_point.y <= y + h {
            return DragHandle::Move;
        }

        DragHandle::None
    }

    /// Draw darkened overlay (4 rectangles around selection).
    fn draw_overlay(&self, renderer: &mut Renderer, bounds: Rectangle) {
        let Some((x, y, w, h)) = self.selection.region else {
            // No selection - darken entire canvas
            draw_quad(renderer, bounds, OVERLAY_COLOR);
            return;
        };

        // Convert to absolute screen coordinates
        let sel_x = bounds.x + x;
        let sel_y = bounds.y + y;
        let sel_right = sel_x + w;
        let sel_bottom = sel_y + h;

        // Clamp to bounds
        let sel_x = sel_x.max(bounds.x);
        let sel_y = sel_y.max(bounds.y);
        let sel_right = sel_right.min(bounds.x + bounds.width);
        let sel_bottom = sel_bottom.min(bounds.y + bounds.height);

        // Draw 4 overlay rectangles
        // Top
        if sel_y > bounds.y {
            draw_quad(
                renderer,
                Rectangle::new(
                    Point::new(bounds.x, bounds.y),
                    Size::new(bounds.width, sel_y - bounds.y),
                ),
                OVERLAY_COLOR,
            );
        }

        // Bottom
        if sel_bottom < bounds.y + bounds.height {
            draw_quad(
                renderer,
                Rectangle::new(
                    Point::new(bounds.x, sel_bottom),
                    Size::new(bounds.width, bounds.y + bounds.height - sel_bottom),
                ),
                OVERLAY_COLOR,
            );
        }

        // Left
        if sel_x > bounds.x {
            draw_quad(
                renderer,
                Rectangle::new(
                    Point::new(bounds.x, sel_y),
                    Size::new(sel_x - bounds.x, sel_bottom - sel_y),
                ),
                OVERLAY_COLOR,
            );
        }

        // Right
        if sel_right < bounds.x + bounds.width {
            draw_quad(
                renderer,
                Rectangle::new(
                    Point::new(sel_right, sel_y),
                    Size::new(bounds.x + bounds.width - sel_right, sel_bottom - sel_y),
                ),
                OVERLAY_COLOR,
            );
        }
    }

    /// Draw border (4 lines).
    fn draw_border(&self, renderer: &mut Renderer, bounds: Rectangle) {
        let Some((x, y, w, h)) = self.selection.region else {
            return;
        };

        // Convert to absolute screen coordinates
        let sx = bounds.x + x;
        let sy = bounds.y + y;

        // Top
        draw_quad(
            renderer,
            Rectangle::new(Point::new(sx, sy), Size::new(w, BORDER_WIDTH)),
            BORDER_COLOR,
        );

        // Bottom
        draw_quad(
            renderer,
            Rectangle::new(
                Point::new(sx, sy + h - BORDER_WIDTH),
                Size::new(w, BORDER_WIDTH),
            ),
            BORDER_COLOR,
        );

        // Left
        draw_quad(
            renderer,
            Rectangle::new(Point::new(sx, sy), Size::new(BORDER_WIDTH, h)),
            BORDER_COLOR,
        );

        // Right
        draw_quad(
            renderer,
            Rectangle::new(
                Point::new(sx + w - BORDER_WIDTH, sy),
                Size::new(BORDER_WIDTH, h),
            ),
            BORDER_COLOR,
        );
    }

    /// Draw handles (8 squares).
    fn draw_handles(&self, renderer: &mut Renderer, bounds: Rectangle) {
        let Some((x, y, w, h)) = self.selection.region else {
            return;
        };

        let half = HANDLE_SIZE / 2.0;

        // 8 handle positions (relative, then convert to screen)
        let handles = [
            self.to_screen(x, y, &bounds),
            self.to_screen(x + w, y, &bounds),
            self.to_screen(x, y + h, &bounds),
            self.to_screen(x + w, y + h, &bounds),
            self.to_screen(x + w / 2.0, y, &bounds),
            self.to_screen(x + w / 2.0, y + h, &bounds),
            self.to_screen(x, y + h / 2.0, &bounds),
            self.to_screen(x + w, y + h / 2.0, &bounds),
        ];

        for pos in handles {
            draw_quad(
                renderer,
                Rectangle::new(
                    Point::new(pos.x - half, pos.y - half),
                    Size::new(HANDLE_SIZE, HANDLE_SIZE),
                ),
                HANDLE_COLOR,
            );
        }
    }

    /// Draw rule-of-thirds grid.
    fn draw_grid(&self, renderer: &mut Renderer, bounds: Rectangle) {
        if !self.show_grid {
            return;
        }

        let Some((x, y, w, h)) = self.selection.region else {
            return;
        };

        if w <= 10.0 || h <= 10.0 {
            return;
        }

        // Convert to absolute screen coordinates
        let sx = bounds.x + x;
        let sy = bounds.y + y;

        let grid_color = Color::from_rgba(1.0, 1.0, 1.0, 0.3);
        let third_w = w / 3.0;
        let third_h = h / 3.0;

        // 2 vertical lines
        for i in 1..3 {
            let line_x = sx + third_w * i as f32;
            draw_quad(
                renderer,
                Rectangle::new(Point::new(line_x, sy), Size::new(1.0, h)),
                grid_color,
            );
        }

        // 2 horizontal lines
        for i in 1..3 {
            let line_y = sy + third_h * i as f32;
            draw_quad(
                renderer,
                Rectangle::new(Point::new(sx, line_y), Size::new(w, 1.0)),
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
        _theme: &cosmic::Theme,
        _style: &cosmic::iced::advanced::renderer::Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        self.draw_overlay(renderer, bounds);
        self.draw_border(renderer, bounds);
        self.draw_handles(renderer, bounds);
        self.draw_grid(renderer, bounds);
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
                if let Some(screen_pos) = cursor.position_in(bounds) {
                    let rel_pos = self.to_relative(screen_pos, &bounds);
                    let handle = self.hit_test_handle(rel_pos);

                    shell.publish(AppMessage::CropDragStart {
                        x: rel_pos.x,
                        y: rel_pos.y,
                        handle,
                    });
                    return Status::Captured;
                
                // Check for double-click on Move handle
                if handle == DragHandle::Move {
                    use std::time::{Duration, Instant};
                    let now = Instant::now();
                    if let Some(last) = self.last_click {
                        if now.duration_since(last) < Duration::from_millis(400) {
                            // Double-click detected - apply crop
                            shell.publish(AppMessage::ApplyCrop);
                            self.last_click = None;
                            return Status::Captured;
                        }
                    }
                    self.last_click = Some(now);
                }
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if self.selection.is_dragging {
                    if let Some(screen_pos) = cursor.position_in(bounds) {
                        let rel_pos = self.to_relative(screen_pos, &bounds);
                        shell.publish(AppMessage::CropDragMove {
                            x: rel_pos.x,
                            y: rel_pos.y,
                            max_x: bounds.width,
                            max_y: bounds.height,
                        });
                        return Status::Captured;
                    }
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

        if let Some(screen_pos) = cursor.position_in(bounds) {
            let rel_pos = self.to_relative(screen_pos, &bounds);
            let handle = self.hit_test_handle(rel_pos);
            return match handle {
                DragHandle::TopLeft | DragHandle::BottomRight => {
                    mouse::Interaction::ResizingDiagonallyDown
                }
                DragHandle::TopRight | DragHandle::BottomLeft => {
                    mouse::Interaction::ResizingDiagonallyUp
                }
                DragHandle::Top | DragHandle::Bottom => mouse::Interaction::ResizingVertically,
                DragHandle::Left | DragHandle::Right => mouse::Interaction::ResizingHorizontally,
                DragHandle::Move => mouse::Interaction::Grabbing,
                DragHandle::None => mouse::Interaction::Crosshair,
            };
        }

        mouse::Interaction::None
    }
}

impl<'a> From<CropOverlay> for Element<'a, AppMessage> {
    fn from(widget: CropOverlay) -> Self {
        Element::new(widget)
    }
}

/// Helper: Check if point is within handle hit area.
fn point_in_handle(point: Point, handle_center: Point) -> bool {
    let half = HANDLE_HIT_SIZE / 2.0;
    point.x >= handle_center.x - half
        && point.x <= handle_center.x + half
        && point.y >= handle_center.y - half
        && point.y <= handle_center.y + half
}

/// Helper: Draw a filled quad.
fn draw_quad(renderer: &mut Renderer, bounds: Rectangle, color: Color) {
    renderer.fill_quad(
        Quad {
            bounds,
            ..Quad::default()
        },
        color,
    );
}

/// Public constructor.
pub fn crop_overlay<'a>(selection: &CropSelection, show_grid: bool) -> Element<'a, AppMessage> {
    CropOverlay::new(selection, show_grid).into()
}
