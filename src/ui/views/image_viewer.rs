// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/image_viewer.rs
//
// Zoom and pan image viewer widget with external state control.
// Forked from cosmic::iced to support external state control.

use cosmic::iced::advanced::image as img_renderer;
use cosmic::iced::advanced::layout;
use cosmic::iced::advanced::renderer;
use cosmic::iced::advanced::widget::tree::{self, Tree};
use cosmic::iced::advanced::widget::Widget;
use cosmic::iced::advanced::{Clipboard, Layout, Shell};
use cosmic::iced::event::{self, Event};
use cosmic::iced::mouse;
use cosmic::iced::widget::image::FilterMethod;
use cosmic::iced::{ContentFit, Element, Length, Pixels, Point, Radians, Rectangle, Size, Vector};

/// Tolerance for scale comparisons in widget state synchronization.
const SCALE_EPSILON: f32 = 0.0001;

/// Tolerance for offset comparisons in widget state synchronization.
const OFFSET_EPSILON: f32 = 0.01;

/// Callback type for notifying viewer state changes (scale, `offset_x`, `offset_y`, `canvas_size`, `image_size`).
type StateChangeCallback<Message> = Box<dyn Fn(f32, f32, f32, Size, Size) -> Message>;

/// A frame that displays an image with the ability to zoom in/out and pan.
#[allow(missing_debug_implementations)]
pub struct Viewer<Handle, Message> {
    padding: f32,
    width: Length,
    height: Length,
    min_scale: f32,
    max_scale: f32,
    scale_step: f32,
    handle: Handle,
    filter_method: FilterMethod,
    content_fit: ContentFit,
    /// Optional external state to override internal state (scale, offset)
    external_state: Option<(f32, Vector)>,
    /// Optional callback to notify state changes
    on_state_change: Option<StateChangeCallback<Message>>,
    /// Disable pan interaction (for crop mode)
    disable_pan: bool,
}

impl<Handle, Message> Viewer<Handle, Message> {
    /// Creates a new [`Viewer`] with the given handle.
    pub fn new<T: Into<Handle>>(handle: T) -> Self {
        Viewer {
            handle: handle.into(),
            padding: 0.0,
            width: Length::Shrink,
            height: Length::Shrink,
            min_scale: 0.25,
            max_scale: 10.0,
            scale_step: 0.10,
            filter_method: FilterMethod::default(),
            content_fit: ContentFit::default(),
            external_state: None,
            on_state_change: None,
            disable_pan: false,
        }
    }

    /// Set external state to control zoom and pan from outside.
    /// This allows keyboard/button controls to override the internal state.
    pub fn with_state(mut self, scale: f32, offset_x: f32, offset_y: f32) -> Self {
        self.external_state = Some((scale, Vector::new(offset_x, offset_y)));
        self
    }

    /// Set a callback to be notified when the state changes (for mouse interaction).
    pub fn on_state_change<F>(mut self, f: F) -> Self
    where
        F: 'static + Fn(f32, f32, f32, Size, Size) -> Message,
    {
        self.on_state_change = Some(Box::new(f));
        self
    }

    /// Disable pan interaction (useful when overlaying crop tools).
    pub fn disable_pan(mut self, disable: bool) -> Self {
        self.disable_pan = disable;
        self
    }

    /// Sets the [`FilterMethod`] of the [`Viewer`].
    pub fn filter_method(mut self, filter_method: FilterMethod) -> Self {
        self.filter_method = filter_method;
        self
    }

    /// Sets the [`ContentFit`] of the [`Viewer`].
    pub fn content_fit(mut self, content_fit: ContentFit) -> Self {
        self.content_fit = content_fit;
        self
    }

    /// Sets the padding of the [`Viewer`].
    pub fn padding(mut self, padding: impl Into<Pixels>) -> Self {
        self.padding = padding.into().0;
        self
    }

    /// Sets the width of the [`Viewer`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Viewer`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the max scale applied to the image of the [`Viewer`].
    ///
    /// Default is `10.0`
    pub fn max_scale(mut self, max_scale: f32) -> Self {
        self.max_scale = max_scale;
        self
    }

    /// Sets the min scale applied to the image of the [`Viewer`].
    ///
    /// Default is `0.25`
    pub fn min_scale(mut self, min_scale: f32) -> Self {
        self.min_scale = min_scale;
        self
    }

    /// Sets the percentage the image of the [`Viewer`] will be scaled by
    /// when zoomed in / out.
    ///
    /// Default is `0.10`
    pub fn scale_step(mut self, scale_step: f32) -> Self {
        self.scale_step = scale_step;
        self
    }
}

impl<Message, Theme, Renderer, Handle> Widget<Message, Theme, Renderer> for Viewer<Handle, Message>
where
    Renderer: img_renderer::Renderer<Handle = Handle>,
    Handle: Clone,
    Message: Clone,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        let mut state = State::new();
        // Apply external state if provided at creation
        if let Some((scale, offset)) = self.external_state {
            state.scale = scale;
            state.current_offset = offset;
            state.starting_offset = offset;
        }
        tree::State::new(state)
    }

    fn diff(&mut self, tree: &mut Tree) {
        // Sync external state into internal state when user is not dragging
        if let Some((ext_scale, ext_offset)) = self.external_state {
            let state = tree.state.downcast_mut::<State>();

            // Only apply external state if user is not currently dragging
            if !state.is_cursor_grabbed() {
                // Check if external state differs significantly from current state
                let scale_changed = (state.scale - ext_scale).abs() > SCALE_EPSILON;
                let offset_changed = (state.current_offset.x - ext_offset.x).abs() > OFFSET_EPSILON
                    || (state.current_offset.y - ext_offset.y).abs() > OFFSET_EPSILON;

                if scale_changed || offset_changed {
                    state.scale = ext_scale;
                    state.current_offset = ext_offset;
                    state.starting_offset = ext_offset;
                }
            }
        }
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &self,
        _tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let image_size = renderer.measure_image(&self.handle);
        let image_size = Size::new(image_size.width as f32, image_size.height as f32);

        let raw_size = limits.resolve(self.width, self.height, image_size);
        let full_size = self.content_fit.fit(image_size, raw_size);

        let final_size = Size {
            width: match self.width {
                Length::Shrink => f32::min(raw_size.width, full_size.width),
                _ => raw_size.width,
            },
            height: match self.height {
                Length::Shrink => f32::min(raw_size.height, full_size.height),
                _ => raw_size.height,
            },
        };

        layout::Node::new(final_size)
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        let bounds = layout.bounds();

        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let Some(cursor_position) = cursor.position_over(bounds) else {
                    return event::Status::Ignored;
                };

                match delta {
                    mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => {
                        let state = tree.state.downcast_mut::<State>();
                        let previous_scale = state.scale;

                        if y < 0.0 && previous_scale > self.min_scale
                            || y > 0.0 && previous_scale < self.max_scale
                        {
                            state.scale = (if y > 0.0 {
                                state.scale * (1.0 + self.scale_step)
                            } else {
                                state.scale / (1.0 + self.scale_step)
                            })
                            .clamp(self.min_scale, self.max_scale);

                            let scale_factor = state.scale / previous_scale;

                            // Cursor position relative to the image center (not bounds center)
                            // The image is centered in bounds, so bounds.center() is correct
                            let cursor_to_center = cursor_position - bounds.center();

                            // Transform offset so the point under cursor stays stationary
                            // Formula: new_offset = old_offset * scale_factor + cursor_to_center * (scale_factor - 1)
                            let new_offset = Vector::new(
                                state.current_offset.x * scale_factor
                                    + cursor_to_center.x * (scale_factor - 1.0),
                                state.current_offset.y * scale_factor
                                    + cursor_to_center.y * (scale_factor - 1.0),
                            );

                            // Clamp offset to valid range
                            let scaled_size = scaled_image_size(
                                renderer,
                                &self.handle,
                                state,
                                bounds.size(),
                                self.content_fit,
                            );

                            state.current_offset =
                                clamp_offset(new_offset, bounds.size(), scaled_size);

                            // Notify state change
                            if let Some(ref on_change) = self.on_state_change {
                                let image_size = renderer.measure_image(&self.handle);
                                let image_size =
                                    Size::new(image_size.width as f32, image_size.height as f32);
                                shell.publish(on_change(
                                    state.scale,
                                    state.current_offset.x,
                                    state.current_offset.y,
                                    bounds.size(),
                                    image_size,
                                ));
                            }
                        }
                    }
                }

                event::Status::Captured
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if self.disable_pan {
                    return event::Status::Ignored;
                }

                let Some(cursor_position) = cursor.position_over(bounds) else {
                    return event::Status::Ignored;
                };

                let state = tree.state.downcast_mut::<State>();
                state.cursor_grabbed_at = Some(cursor_position);
                state.starting_offset = state.current_offset;

                event::Status::Captured
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if self.disable_pan {
                    return event::Status::Ignored;
                }

                let state = tree.state.downcast_mut::<State>();

                if state.cursor_grabbed_at.is_some() {
                    state.cursor_grabbed_at = None;

                    // Notify final state after drag ends
                    if let Some(ref on_change) = self.on_state_change {
                        let image_size = renderer.measure_image(&self.handle);
                        let image_size =
                            Size::new(image_size.width as f32, image_size.height as f32);
                        shell.publish(on_change(
                            state.scale,
                            state.current_offset.x,
                            state.current_offset.y,
                            bounds.size(),
                            image_size,
                        ));
                    }

                    event::Status::Captured
                } else {
                    event::Status::Ignored
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if self.disable_pan {
                    return event::Status::Ignored;
                }

                let state = tree.state.downcast_mut::<State>();

                if let Some(origin) = state.cursor_grabbed_at {
                    let scaled_size = scaled_image_size(
                        renderer,
                        &self.handle,
                        state,
                        bounds.size(),
                        self.content_fit,
                    );

                    let delta = position - origin;

                    // Pan: subtract delta from starting offset
                    let new_offset = Vector::new(
                        state.starting_offset.x - delta.x,
                        state.starting_offset.y - delta.y,
                    );

                    state.current_offset = clamp_offset(new_offset, bounds.size(), scaled_size);

                    // Notify state change during pan
                    if let Some(ref on_change) = self.on_state_change {
                        let image_size = renderer.measure_image(&self.handle);
                        let image_size =
                            Size::new(image_size.width as f32, image_size.height as f32);
                        shell.publish(on_change(
                            state.scale,
                            state.current_offset.x,
                            state.current_offset.y,
                            bounds.size(),
                            image_size,
                        ));
                    }

                    event::Status::Captured
                } else {
                    event::Status::Ignored
                }
            }
            _ => event::Status::Ignored,
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let is_mouse_over = cursor.is_over(bounds);

        if state.is_cursor_grabbed() {
            mouse::Interaction::Grabbing
        } else if is_mouse_over {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();

        let scaled_size = scaled_image_size(
            renderer,
            &self.handle,
            state,
            bounds.size(),
            self.content_fit,
        );

        // Calculate translation to center the image and apply offset
        let translation = {
            // How much space is left after placing the scaled image
            let diff_w = bounds.width - scaled_size.width;
            let diff_h = bounds.height - scaled_size.height;

            // Base position: center the image in the viewport
            // For images smaller than viewport: center them (diff > 0)
            // For images larger than viewport: they extend beyond bounds (diff < 0)
            let center_offset = Vector::new(diff_w / 2.0, diff_h / 2.0);

            // Apply pan offset (offset moves the "camera", so subtract it)
            // Positive offset = looking at right/bottom part = image moves left/up
            center_offset - state.current_offset
        };

        let drawing_bounds = Rectangle::new(bounds.position(), scaled_size);

        let render = |renderer: &mut Renderer| {
            renderer.with_translation(translation, |renderer| {
                renderer.draw_image(
                    self.handle.clone(),
                    self.filter_method,
                    drawing_bounds,
                    Radians(0.0),
                    1.0,
                    [0.0; 4],
                );
            });
        };

        renderer.with_layer(bounds, render);
    }
}

/// The local state of a [`Viewer`].
#[derive(Debug, Clone, Copy)]
pub struct State {
    scale: f32,
    starting_offset: Vector,
    current_offset: Vector,
    cursor_grabbed_at: Option<Point>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            scale: 1.0,
            starting_offset: Vector::default(),
            current_offset: Vector::default(),
            cursor_grabbed_at: None,
        }
    }
}

impl State {
    /// Creates a new [`State`].
    pub fn new() -> Self {
        State::default()
    }

    /// Returns if the cursor is currently grabbed by the [`Viewer`].
    pub fn is_cursor_grabbed(&self) -> bool {
        self.cursor_grabbed_at.is_some()
    }
}

/// Clamps the offset to keep the image within reasonable bounds.
///
/// The offset represents how far the viewport's center is displaced from the image's center.
/// - offset (0, 0) = image centered
/// - positive offset = viewing right/bottom part of image
/// - negative offset = viewing left/top part of image
fn clamp_offset(offset: Vector, viewport_size: Size, image_size: Size) -> Vector {
    // Maximum allowed offset in each direction
    // When image is larger than viewport, allow panning up to image edge
    // When image is smaller than viewport, no panning needed (clamp to 0)
    let max_offset_x = ((image_size.width - viewport_size.width) / 2.0).max(0.0);
    let max_offset_y = ((image_size.height - viewport_size.height) / 2.0).max(0.0);

    Vector::new(
        offset.x.clamp(-max_offset_x, max_offset_x),
        offset.y.clamp(-max_offset_y, max_offset_y),
    )
}

impl<'a, Message, Theme, Renderer, Handle> From<Viewer<Handle, Message>>
    for Element<'a, Message, Theme, Renderer>
where
    Renderer: 'a + img_renderer::Renderer<Handle = Handle>,
    Message: 'a + Clone,
    Handle: Clone + 'a,
{
    fn from(viewer: Viewer<Handle, Message>) -> Element<'a, Message, Theme, Renderer> {
        Element::new(viewer)
    }
}

/// Returns the scaled size of the image given current state.
/// Calculate the scaled image size after applying content fit and zoom.
///
/// This is the canonical implementation used by the viewer widget.
/// A simplified version exists in `document::utils::scaled_image_size`.
pub fn scaled_image_size<Renderer>(
    renderer: &Renderer,
    handle: &<Renderer as img_renderer::Renderer>::Handle,
    state: &State,
    bounds: Size,
    content_fit: ContentFit,
) -> Size
where
    Renderer: img_renderer::Renderer,
{
    let Size { width, height } = renderer.measure_image(handle);
    let image_size = Size::new(width as f32, height as f32);

    let adjusted_fit = match content_fit {
        ContentFit::None => image_size,
        _ => content_fit.fit(image_size, bounds),
    };

    Size::new(
        adjusted_fit.width * state.scale,
        adjusted_fit.height * state.scale,
    )
}
