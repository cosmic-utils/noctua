// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/widget/image_viewer.rs
//
// Zoom and pan image viewer with external state control.

use cosmic::iced::advanced::image as img_renderer;
use cosmic::iced::advanced::layout;
use cosmic::iced::advanced::renderer;
use cosmic::iced::advanced::widget::Widget;
use cosmic::iced::advanced::widget::tree::{self, Tree};
use cosmic::iced::advanced::{Clipboard, Layout, Shell};
use cosmic::iced::core::Image;
use cosmic::iced::core::border;
use cosmic::iced::core::image::FilterMethod;
use cosmic::iced::keyboard::{self, Modifiers};
use cosmic::iced::mouse;
use cosmic::iced::{ContentFit, Element, Event, Length, Point, Radians, Rectangle, Size, Vector};

use crate::model::{MAX_SCALE, MIN_SCALE, ZOOM_STEP};

/// Tolerance for scale comparisons in widget state synchronization.
const SCALE_EPSILON: f32 = 0.0001;

/// Tolerance for offset comparisons in widget state synchronization.
const OFFSET_EPSILON: f32 = 0.01;

/// Callback for viewer state changes (scale, `offset_x`, `offset_y`).
type StateChangeCallback<Message> = Box<dyn Fn(f32, f32, f32) -> Message>;

/// A frame that displays an image with the ability to zoom in/out and pan.
pub(crate) struct Viewer<Handle, Message> {
    width: Length,
    height: Length,
    handle: Handle,
    content_fit: ContentFit,
    /// Current keyboard modifiers, synced from the app.
    modifiers: Modifiers,
    /// Optional external state to override the internal state (scale, offset).
    external_state: Option<(f32, Vector)>,
    /// Optional callback to notify the app of state changes.
    on_state_change: Option<StateChangeCallback<Message>>,
}

impl<Handle, Message> Viewer<Handle, Message> {
    /// Creates a new [`Viewer`] with the given handle.
    pub(crate) fn new<T: Into<Handle>>(handle: T) -> Self {
        Viewer {
            handle: handle.into(),
            width: Length::Shrink,
            height: Length::Shrink,
            content_fit: ContentFit::default(),
            modifiers: Modifiers::default(),
            external_state: None,
            on_state_change: None,
        }
    }

    /// Sets the external zoom/pan state, used to drive the viewer from the
    /// menu, keyboard or per-image restore.
    pub(crate) fn with_state(mut self, scale: f32, offset_x: f32, offset_y: f32) -> Self {
        self.external_state = Some((scale, Vector::new(offset_x, offset_y)));
        self
    }

    /// Sets a callback notified when the user zooms or pans with the mouse.
    pub(crate) fn on_state_change<F>(mut self, f: F) -> Self
    where
        F: 'static + Fn(f32, f32, f32) -> Message,
    {
        self.on_state_change = Some(Box::new(f));
        self
    }

    /// Sets the [`ContentFit`] of the viewer.
    pub(crate) fn content_fit(mut self, content_fit: ContentFit) -> Self {
        self.content_fit = content_fit;
        self
    }

    /// Sets the current keyboard modifiers used to initialize the widget state.
    pub(crate) fn modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = modifiers;
        self
    }

    /// Sets the width of the viewer.
    pub(crate) fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the viewer.
    pub(crate) fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
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
        let mut state = State {
            keyboard_modifiers: self.modifiers,
            ..State::default()
        };
        if let Some((scale, offset)) = self.external_state {
            state.scale = scale;
            state.current_offset = offset;
            state.starting_offset = offset;
        }
        tree::State::new(state)
    }

    fn diff(&mut self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State>();

        // Modifiers are independent of grab state and must stay current.
        state.keyboard_modifiers = self.modifiers;

        if let Some((ext_scale, ext_offset)) = self.external_state {
            // Never fight the user while they are dragging.
            if !state.is_cursor_grabbed() {
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
        &mut self,
        _tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let image_size = renderer.measure_image(&self.handle).unwrap_or_default();
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

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                tree.state.downcast_mut::<State>().keyboard_modifiers = *modifiers;
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let Some(cursor_position) = cursor.position_over(bounds) else {
                    return;
                };

                let state = tree.state.downcast_mut::<State>();

                if state.keyboard_modifiers.control() {
                    // Ctrl+wheel: zoom proportionally, keeping the cursor stable.
                    let steps = match *delta {
                        mouse::ScrollDelta::Lines { y, .. } => y,
                        mouse::ScrollDelta::Pixels { y, .. } => y / 50.0,
                    };

                    if steps != 0.0 {
                        let previous_scale = state.scale;
                        let new_scale =
                            (previous_scale * ZOOM_STEP.powf(steps)).clamp(MIN_SCALE, MAX_SCALE);

                        if (new_scale - previous_scale).abs() > f32::EPSILON {
                            state.scale = new_scale;

                            let scaled_size = scaled_image_size(
                                renderer,
                                &self.handle,
                                state,
                                bounds.size(),
                                self.content_fit,
                            );

                            let factor = new_scale / previous_scale - 1.0;
                            let cursor_to_center = cursor_position - bounds.center();
                            let adjustment =
                                cursor_to_center * factor + state.current_offset * factor;

                            state.current_offset = Vector::new(
                                if scaled_size.width > bounds.width {
                                    state.current_offset.x + adjustment.x
                                } else {
                                    0.0
                                },
                                if scaled_size.height > bounds.height {
                                    state.current_offset.y + adjustment.y
                                } else {
                                    0.0
                                },
                            );

                            if let Some(ref on_change) = self.on_state_change {
                                shell.publish(on_change(
                                    native_scale(renderer, &self.handle, scaled_size),
                                    state.current_offset.x,
                                    state.current_offset.y,
                                ));
                            }
                        }
                    }

                    shell.request_redraw();
                    shell.capture_event();
                } else {
                    // Plain wheel pans vertically; Shift swaps to horizontal.
                    let (mut dx, mut dy) = match *delta {
                        mouse::ScrollDelta::Lines { x, y, .. } => (x * 60.0, y * 60.0),
                        mouse::ScrollDelta::Pixels { x, y, .. } => (x, y),
                    };
                    if state.keyboard_modifiers.shift() {
                        std::mem::swap(&mut dx, &mut dy);
                    }
                    let scaled_size = scaled_image_size(
                        renderer,
                        &self.handle,
                        state,
                        bounds.size(),
                        self.content_fit,
                    );
                    let hidden_width = (scaled_size.width - bounds.width / 2.0).max(0.0).round();
                    let hidden_height = (scaled_size.height - bounds.height / 2.0).max(0.0).round();

                    let previous = state.current_offset;
                    let x = if bounds.width < scaled_size.width {
                        (state.current_offset.x - dx).clamp(-hidden_width, hidden_width)
                    } else {
                        0.0
                    };
                    let y = if bounds.height < scaled_size.height {
                        (state.current_offset.y - dy).clamp(-hidden_height, hidden_height)
                    } else {
                        0.0
                    };
                    state.current_offset = Vector::new(x, y);

                    // Only notify the app when the offset actually changed.
                    if ((state.current_offset.x - previous.x).abs() > OFFSET_EPSILON
                        || (state.current_offset.y - previous.y).abs() > OFFSET_EPSILON)
                        && let Some(ref on_change) = self.on_state_change
                    {
                        shell.publish(on_change(
                            native_scale(renderer, &self.handle, scaled_size),
                            state.current_offset.x,
                            state.current_offset.y,
                        ));
                    }

                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(cursor_position) = cursor.position_over(bounds) else {
                    return;
                };

                let state = tree.state.downcast_mut::<State>();
                state.cursor_grabbed_at = Some(cursor_position);
                state.starting_offset = state.current_offset;

                shell.capture_event();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let state = tree.state.downcast_mut::<State>();
                let was_grabbed = state.cursor_grabbed_at.is_some();
                state.cursor_grabbed_at = None;

                // Publish the final state once per drag, only if it moved.
                let moved = (state.current_offset.x - state.starting_offset.x).abs()
                    > OFFSET_EPSILON
                    || (state.current_offset.y - state.starting_offset.y).abs() > OFFSET_EPSILON;
                if was_grabbed && moved {
                    let scaled_size = scaled_image_size(
                        renderer,
                        &self.handle,
                        state,
                        bounds.size(),
                        self.content_fit,
                    );
                    if let Some(ref on_change) = self.on_state_change {
                        shell.publish(on_change(
                            native_scale(renderer, &self.handle, scaled_size),
                            state.current_offset.x,
                            state.current_offset.y,
                        ));
                    }
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let state = tree.state.downcast_mut::<State>();

                if let Some(origin) = state.cursor_grabbed_at {
                    let scaled_size = scaled_image_size(
                        renderer,
                        &self.handle,
                        state,
                        bounds.size(),
                        self.content_fit,
                    );
                    let hidden_width = (scaled_size.width - bounds.width / 2.0).max(0.0).round();
                    let hidden_height = (scaled_size.height - bounds.height / 2.0).max(0.0).round();

                    let delta = *position - origin;

                    let x = if bounds.width < scaled_size.width {
                        (state.starting_offset.x - delta.x).clamp(-hidden_width, hidden_width)
                    } else {
                        0.0
                    };

                    let y = if bounds.height < scaled_size.height {
                        (state.starting_offset.y - delta.y).clamp(-hidden_height, hidden_height)
                    } else {
                        0.0
                    };

                    state.current_offset = Vector::new(x, y);

                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            _ => {}
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

        if state.is_cursor_grabbed() {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
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
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();

        let final_size = scaled_image_size(
            renderer,
            &self.handle,
            state,
            bounds.size(),
            self.content_fit,
        );

        let translation = {
            let diff_w = bounds.width - final_size.width;
            let diff_h = bounds.height - final_size.height;

            let image_top_left = match self.content_fit {
                ContentFit::None => Vector::new(diff_w.max(0.0) / 2.0, diff_h.max(0.0) / 2.0),
                _ => Vector::new(diff_w / 2.0, diff_h / 2.0),
            };

            image_top_left - state.offset(bounds, final_size)
        };

        let drawing_bounds = Rectangle::new(bounds.position(), final_size);

        let render = |renderer: &mut Renderer| {
            renderer.with_translation(translation, |renderer| {
                renderer.draw_image(
                    Image {
                        handle: self.handle.clone(),
                        border_radius: border::Radius::default(),
                        filter_method: FilterMethod::default(),
                        rotation: Radians(0.0),
                        opacity: 1.0,
                        snap: true,
                    },
                    drawing_bounds,
                    *viewport - translation,
                );
            });
        };

        renderer.with_layer(bounds, render);
    }
}

/// The local state of a [`Viewer`].
#[derive(Debug, Clone, Copy)]
struct State {
    scale: f32,
    starting_offset: Vector,
    current_offset: Vector,
    cursor_grabbed_at: Option<Point>,
    keyboard_modifiers: Modifiers,
}

impl Default for State {
    fn default() -> Self {
        Self {
            scale: 1.0,
            starting_offset: Vector::default(),
            current_offset: Vector::default(),
            cursor_grabbed_at: None,
            keyboard_modifiers: Modifiers::default(),
        }
    }
}

impl State {
    /// The current offset clamped to the visible image bounds.
    fn offset(&self, bounds: Rectangle, image_size: Size) -> Vector {
        let hidden_width = (image_size.width - bounds.width / 2.0).max(0.0).round();
        let hidden_height = (image_size.height - bounds.height / 2.0).max(0.0).round();

        Vector::new(
            self.current_offset.x.clamp(-hidden_width, hidden_width),
            self.current_offset.y.clamp(-hidden_height, hidden_height),
        )
    }

    /// Whether the cursor is currently grabbed by the [`Viewer`].
    fn is_cursor_grabbed(&self) -> bool {
        self.cursor_grabbed_at.is_some()
    }
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

/// The scaled size of the image given the current viewer state.
fn scaled_image_size<Renderer>(
    renderer: &Renderer,
    handle: &<Renderer as img_renderer::Renderer>::Handle,
    state: &State,
    bounds: Size,
    content_fit: ContentFit,
) -> Size
where
    Renderer: img_renderer::Renderer,
{
    let Size { width, height } = renderer.measure_image(handle).unwrap_or_default();
    let image_size = Size::new(width as f32, height as f32);

    let adjusted_fit = content_fit.fit(image_size, bounds);

    Size::new(
        adjusted_fit.width * state.scale,
        adjusted_fit.height * state.scale,
    )
}

/// The scale relative to the native image size for a displayed size. The
/// internal scale is relative to the fitted size in fit mode, so the reported
/// scale is derived from the displayed size.
fn native_scale<Renderer>(
    renderer: &Renderer,
    handle: &<Renderer as img_renderer::Renderer>::Handle,
    scaled_size: Size,
) -> f32
where
    Renderer: img_renderer::Renderer,
{
    let Size { width, .. } = renderer.measure_image(handle).unwrap_or_default();
    if width == 0 {
        1.0
    } else {
        scaled_size.width / width as f32
    }
}
