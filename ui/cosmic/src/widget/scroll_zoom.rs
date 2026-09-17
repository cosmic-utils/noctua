// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/widget/scroll_zoom.rs
//
// Wraps scrollable content to turn Ctrl+wheel into a zoom message while
// leaving plain wheel to the surrounding scrollable.

use cosmic::iced::advanced::layout;
use cosmic::iced::advanced::renderer;
use cosmic::iced::advanced::widget::Widget;
use cosmic::iced::advanced::widget::tree::Tree;
use cosmic::iced::advanced::{Clipboard, Layout, Shell};
use cosmic::iced::keyboard::Modifiers;
use cosmic::iced::mouse;
use cosmic::iced::{Event, Length, Rectangle, Size};
use cosmic::{Element, Renderer, Theme};

use crate::message::Message;

/// Wraps a widget and intercepts Ctrl+wheel, publishing a zoom message. Other
/// events (including plain wheel) pass through to the wrapped content.
pub(crate) struct ScrollZoom<'a> {
    content: Element<'a, Message>,
    /// Current keyboard modifiers, synced from the app.
    modifiers: Modifiers,
    on_zoom: Option<Box<dyn Fn(mouse::ScrollDelta) -> Message + 'a>>,
}

impl<'a> ScrollZoom<'a> {
    /// Creates a new [`ScrollZoom`] around the given content.
    pub(crate) fn new(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            modifiers: Modifiers::default(),
            on_zoom: None,
        }
    }

    /// Sets the current keyboard modifiers, synced from the app.
    pub(crate) fn modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = modifiers;
        self
    }

    /// Sets the callback notified on Ctrl+wheel.
    pub(crate) fn on_zoom<F>(mut self, f: F) -> Self
    where
        F: 'a + Fn(mouse::ScrollDelta) -> Message,
    {
        self.on_zoom = Some(Box::new(f));
        self
    }
}

impl Widget<Message, Theme, Renderer> for ScrollZoom<'_> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event
            && self.modifiers.control()
            && cursor.position_over(layout.bounds()).is_some()
        {
            if let Some(on_zoom) = &self.on_zoom {
                shell.publish(on_zoom(*delta));
                shell.capture_event();
            }
            return;
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }
}

impl<'a> From<ScrollZoom<'a>> for Element<'a, Message> {
    fn from(scroll_zoom: ScrollZoom<'a>) -> Self {
        Element::new(scroll_zoom)
    }
}
