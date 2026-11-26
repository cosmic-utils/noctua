// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/widget/focus_scope.rs
//
// Wraps content and, while active, keeps a set of named keys for that content.
//
// A captured key is published by the scope itself: `iced::keyboard::listen`
// only reports events with status `Ignored`, so the application's keyboard
// subscription no longer sees the key. Capturing also stops libcosmic's
// `keyboard_nav` subscription, which would otherwise move the widget focus on
// Tab while the content is being edited.

use cosmic::iced::advanced::layout;
use cosmic::iced::advanced::renderer;
use cosmic::iced::advanced::widget::tree::Tree;
use cosmic::iced::advanced::widget::{Operation, Widget};
use cosmic::iced::advanced::{Clipboard, Layout, Shell, overlay};
use cosmic::iced::keyboard::key::Named;
use cosmic::iced::keyboard::{self, Key};
use cosmic::iced::mouse;
use cosmic::iced::{Event, Length, Rectangle, Size, Vector};
use cosmic::{Element, Renderer, Theme};

use crate::message::Message;

/// A wrapper that hands selected keys to its content while it is active.
pub(crate) struct FocusScope<'a> {
    content: Element<'a, Message>,
    active: bool,
    keys: Vec<Named>,
    on_key: Option<Box<dyn Fn(Key) -> Message + 'a>>,
}

/// Wraps `content` in a [`FocusScope`]; inactive until [`FocusScope::active`]
/// is set.
pub(crate) fn focus_scope<'a>(content: impl Into<Element<'a, Message>>) -> FocusScope<'a> {
    FocusScope {
        content: content.into(),
        active: false,
        keys: Vec::new(),
        on_key: None,
    }
}

impl<'a> FocusScope<'a> {
    /// Enables or disables capturing.
    pub(crate) fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Sets the named keys that belong to the content while it is active.
    pub(crate) fn capture(mut self, keys: &[Named]) -> Self {
        self.keys = keys.to_vec();
        self
    }

    /// Sets the callback that receives every captured key.
    pub(crate) fn on_key<F>(mut self, f: F) -> Self
    where
        F: 'a + Fn(Key) -> Message,
    {
        self.on_key = Some(Box::new(f));
        self
    }
}

/// Whether a key press belongs to the scope's key set.
fn captures(keys: &[Named], key: &Key) -> bool {
    matches!(key, Key::Named(named) if keys.contains(named))
}

impl Widget<Message, Theme, Renderer> for FocusScope<'_> {
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

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
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
        if self.active
            && let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event
            && captures(&self.keys, key)
            && let Some(on_key) = &self.on_key
        {
            shell.publish(on_key(key.clone()));
            shell.capture_event();
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

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a> From<FocusScope<'a>> for Element<'a, Message> {
    fn from(scope: FocusScope<'a>) -> Self {
        Element::new(scope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_listed_named_keys_are_captured() {
        let keys = [Named::Tab, Named::Escape];

        assert!(captures(&keys, &Key::Named(Named::Tab)));
        assert!(captures(&keys, &Key::Named(Named::Escape)));
        assert!(!captures(&keys, &Key::Named(Named::Enter)));
        assert!(!captures(&keys, &Key::Character("a".into())));
    }
}
