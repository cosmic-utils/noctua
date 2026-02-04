// SPDX-License-Identifier: GPL-3.0-or-later
// src/ui/widgets/crop_widget.rs
//
// Self-contained crop widget (based on Cupola, adapted for Noctua).

use cosmic::{
    Element, Renderer,
    iced::{
        Color, Length, Point, Rectangle, Size,
        advanced::{
            Clipboard, Layout, Shell, Widget,
            image::Renderer as ImageRenderer,
            layout::{Limits, Node},
            renderer::{Quad, Renderer as QuadRenderer},
            widget::Tree,
        },
        event::{Event, Status},
        mouse::{self, Button, Cursor},
    },
    widget::image::Handle,
};

use crate::ui::widgets::{CropSelection, DragHandle};
use crate::ui::AppMessage;

// Visual constants
const HANDLE_SIZE: f32 = 12.0;
const HANDLE_HIT_SIZE: f32 = 24.0;
const OVERLAY_COLOR: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.5);
const HANDLE_COLOR: Color = Color::WHITE;
const BORDER_COLOR: Color = Color::WHITE;
const BORDER_WIDTH: f32 = 2.0;

/// Self-contained crop widget that renders image and crop UI together.
/// 
/// All coordinates are handled internally - no transformation needed!
/// This is much simpler than the old overlay approach.
pub struct CropWidget {
    handle: Handle,
    img_width: u32,
    img_height: u32,
    selection: CropSelection,
}

impl CropWidget {
    pub fn new(handle: Handle, img_width: u32, img_height: u32, selection: &CropSelection) -> Self {
        Self {
            handle,
            img_width,
            img_height,
            selection: selection.clone(),
        }
    }

    /// Calculate image rectangle within bounds (centered, scaled to fit).
    fn calculate_image_rect(&self, bounds: Rectangle) -> (Rectangle, f32) {
        let scale_x = bounds.width / self.img_width as f32;
        let scale_y = bounds.height / self.img_height as f32;
        let scale = scale_x.min(scale_y).min(1.0); // Don't upscale

        let img_w = self.img_width as f32 * scale;
        let img_h = self.img_height as f32 * scale;

        let img_x = bounds.x + (bounds.width - img_w) / 2.0;
        let img_y = bounds.y + (bounds.height - img_h) / 2.0;

        (
            Rectangle::new(Point::new(img_x, img_y), Size::new(img_w, img_h)),
            scale,
        )
    }

    /// Convert screen coordinates to image coordinates.
    fn screen_to_image(&self, screen_point: Point, img_rect: Rectangle, scale: f32) -> (f32, f32) {
        let rel_x = (screen_point.x - img_rect.x) / scale;
        let rel_y = (screen_point.y - img_rect.y) / scale;
        (rel_x, rel_y)
    }

    /// Convert image coordinates to screen coordinates.
    fn image_to_screen(&self, img_x: f32, img_y: f32, img_rect: Rectangle, scale: f32) -> Point {
        Point::new(
            img_rect.x + img_x * scale,
            img_rect.y + img_y * scale,
        )
    }

    /// Hit-test to find which handle (if any) is under the cursor.
    fn hit_test_handle(&self, screen_point: Point, img_rect: Rectangle, scale: f32) -> DragHandle {
        let Some((x, y, w, h)) = self.selection.region else {
            return DragHandle::None;
        };

        // Convert handle positions to screen coordinates
        let handles = [
            (self.image_to_screen(x, y, img_rect, scale), DragHandle::TopLeft),
            (self.image_to_screen(x + w, y, img_rect, scale), DragHandle::TopRight),
            (self.image_to_screen(x, y + h, img_rect, scale), DragHandle::BottomLeft),
            (self.image_to_screen(x + w, y + h, img_rect, scale), DragHandle::BottomRight),
            (self.image_to_screen(x + w / 2.0, y, img_rect, scale), DragHandle::Top),
            (self.image_to_screen(x + w / 2.0, y + h, img_rect, scale), DragHandle::Bottom),
            (self.image_to_screen(x, y + h / 2.0, img_rect, scale), DragHandle::Left),
            (self.image_to_screen(x + w, y + h / 2.0, img_rect, scale), DragHandle::Right),
        ];

        // Check handles
        for (pos, handle) in handles {
            if point_in_handle(screen_point, pos) {
                return handle;
            }
        }

        // Check if inside selection (move)
        let sel_rect = Rectangle::new(
            self.image_to_screen(x, y, img_rect, scale),
            Size::new(w * scale, h * scale),
        );
        if sel_rect.contains(screen_point) {
            return DragHandle::Move;
        }

        DragHandle::None
    }

    /// Draw the darkened overlay outside the selection.
    fn draw_overlay(&self, renderer: &mut Renderer, bounds: Rectangle, img_rect: Rectangle, scale: f32) {
        let Some((x, y, w, h)) = self.selection.region else {
            // No selection - darken entire image
            draw_quad(renderer, img_rect, OVERLAY_COLOR);
            return;
        };

        // Convert selection to screen coordinates
        let sel_screen = Rectangle::new(
            self.image_to_screen(x, y, img_rect, scale),
            Size::new(w * scale, h * scale),
        );

        // Draw 4 overlay rectangles around the selection
        // Top
        if sel_screen.y > img_rect.y {
            draw_quad(
                renderer,
                Rectangle::new(
                    Point::new(img_rect.x, img_rect.y),
                    Size::new(img_rect.width, sel_screen.y - img_rect.y),
                ),
                OVERLAY_COLOR,
            );
        }

        // Bottom
        let sel_bottom = sel_screen.y + sel_screen.height;
        let img_bottom = img_rect.y + img_rect.height;
        if sel_bottom < img_bottom {
            draw_quad(
                renderer,
                Rectangle::new(
                    Point::new(img_rect.x, sel_bottom),
                    Size::new(img_rect.width, img_bottom - sel_bottom),
                ),
                OVERLAY_COLOR,
            );
        }

        // Left
        if sel_screen.x > img_rect.x {
            draw_quad(
                renderer,
                Rectangle::new(
                    Point::new(img_rect.x, sel_screen.y),
                    Size::new(sel_screen.x - img_rect.x, sel_screen.height),
                ),
                OVERLAY_COLOR,
            );
        }

        // Right
        let sel_right = sel_screen.x + sel_screen.width;
        let img_right = img_rect.x + img_rect.width;
        if sel_right < img_right {
            draw_quad(
                renderer,
                Rectangle::new(
                    Point::new(sel_right, sel_screen.y),
                    Size::new(img_right - sel_right, sel_screen.height),
                ),
                OVERLAY_COLOR,
            );
        }
    }

    /// Draw selection border.
    fn draw_border(&self, renderer: &mut Renderer, img_rect: Rectangle, scale: f32) {
        let Some((x, y, w, h)) = self.selection.region else {
            return;
        };

        let sel_screen = Rectangle::new(
            self.image_to_screen(x, y, img_rect, scale),
            Size::new(w * scale, h * scale),
        );

        // Draw 4 border lines
        let sx = sel_screen.x;
        let sy = sel_screen.y;
        let sw = sel_screen.width;
        let sh = sel_screen.height;

        // Top
        draw_quad(
            renderer,
            Rectangle::new(Point::new(sx, sy), Size::new(sw, BORDER_WIDTH)),
            BORDER_COLOR,
        );

        // Bottom
        draw_quad(
            renderer,
            Rectangle::new(
                Point::new(sx, sy + sh - BORDER_WIDTH),
                Size::new(sw, BORDER_WIDTH),
            ),
            BORDER_COLOR,
        );

        // Left
        draw_quad(
            renderer,
            Rectangle::new(Point::new(sx, sy), Size::new(BORDER_WIDTH, sh)),
            BORDER_COLOR,
        );

        // Right
        draw_quad(
            renderer,
            Rectangle::new(
                Point::new(sx + sw - BORDER_WIDTH, sy),
                Size::new(BORDER_WIDTH, sh),
            ),
            BORDER_COLOR,
        );
    }

    /// Draw resize handles.
    fn draw_handles(&self, renderer: &mut Renderer, img_rect: Rectangle, scale: f32) {
        let Some((x, y, w, h)) = self.selection.region else {
            return;
        };

        let half = HANDLE_SIZE / 2.0;

        // 8 handle positions
        let handles = [
            self.image_to_screen(x, y, img_rect, scale),
            self.image_to_screen(x + w, y, img_rect, scale),
            self.image_to_screen(x, y + h, img_rect, scale),
            self.image_to_screen(x + w, y + h, img_rect, scale),
            self.image_to_screen(x + w / 2.0, y, img_rect, scale),
            self.image_to_screen(x + w / 2.0, y + h, img_rect, scale),
            self.image_to_screen(x, y + h / 2.0, img_rect, scale),
            self.image_to_screen(x + w, y + h / 2.0, img_rect, scale),
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
}

impl Widget<AppMessage, cosmic::Theme, Renderer> for CropWidget {
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
        let (img_rect, scale) = self.calculate_image_rect(bounds);

        // Draw image
        renderer.draw_image(
            self.handle.clone(),
            cosmic::iced::widget::image::FilterMethod::Linear,
            img_rect,
            cosmic::iced::Radians(0.0),
            1.0,
            [0.0; 4],
        );

        // Draw crop UI
        self.draw_overlay(renderer, bounds, img_rect, scale);
        self.draw_border(renderer, img_rect, scale);
        self.draw_handles(renderer, img_rect, scale);
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
        let (img_rect, scale) = self.calculate_image_rect(bounds);

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(Button::Left)) => {
                if let Some(screen_pos) = cursor.position_in(bounds) {
                    // Only handle clicks inside image area
                    if !img_rect.contains(screen_pos) {
                        return Status::Ignored;
                    }

                    let handle = self.hit_test_handle(screen_pos, img_rect, scale);
                    let (img_x, img_y) = self.screen_to_image(screen_pos, img_rect, scale);

                    shell.publish(AppMessage::CropDragStart {
                        x: img_x,
                        y: img_y,
                        handle,
                    });
                    return Status::Captured;
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if self.selection.is_dragging {
                    if let Some(screen_pos) = cursor.position_in(bounds) {
                        let (img_x, img_y) = self.screen_to_image(screen_pos, img_rect, scale);
                        shell.publish(AppMessage::CropDragMove {
                            x: img_x,
                            y: img_y,
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
        let (img_rect, scale) = self.calculate_image_rect(bounds);

        if let Some(screen_pos) = cursor.position_in(bounds) {
            if img_rect.contains(screen_pos) {
                let handle = self.hit_test_handle(screen_pos, img_rect, scale);
                return match handle {
                    DragHandle::TopLeft | DragHandle::BottomRight => {
                        mouse::Interaction::ResizingDiagonallyDown
                    }
                    DragHandle::TopRight | DragHandle::BottomLeft => {
                        mouse::Interaction::ResizingDiagonallyUp
                    }
                    DragHandle::Top | DragHandle::Bottom => {
                        mouse::Interaction::ResizingVertically
                    }
                    DragHandle::Left | DragHandle::Right => {
                        mouse::Interaction::ResizingHorizontally
                    }
                    DragHandle::Move => mouse::Interaction::Grabbing,
                    DragHandle::None => mouse::Interaction::Crosshair,
                };
            }
        }

        mouse::Interaction::None
    }
}

impl<'a> From<CropWidget> for Element<'a, AppMessage> {
    fn from(widget: CropWidget) -> Self {
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

/// Public constructor function (convenience).
pub fn crop_widget<'a>(
    handle: Handle,
    img_width: u32,
    img_height: u32,
    selection: &CropSelection,
) -> Element<'a, AppMessage> {
    CropWidget::new(handle, img_width, img_height, selection).into()
}
