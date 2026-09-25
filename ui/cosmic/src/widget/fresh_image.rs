// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/widget/fresh_image.rs
//
// Wrapper that self-drives a few extra redraws after the image handle changes.
// iced uploads raster images larger than 2 MB asynchronously and does not
// reliably redraw once the upload lands, so a freshly set image can stay
// invisible until the next input event. Requesting a handful of extra frames
// here makes it appear without any input, without the app knowing about it.

use cosmic::iced::advanced::layout;
use cosmic::iced::advanced::renderer;
use cosmic::iced::advanced::widget::tree::Tree;
use cosmic::iced::advanced::widget::{Widget, tree};
use cosmic::iced::advanced::{Clipboard, Layout, Shell};
use cosmic::iced::core::image::Id as ImageId;
use cosmic::iced::mouse;
use cosmic::iced::{Event, Length, Rectangle, Size, window};
use cosmic::{Element, Renderer, Theme};

/// Number of extra frames to request after the handle changes. Roughly 100 ms
/// at 60 Hz; long enough for the asynchronous GPU upload to land.
const EXTRA_FRAMES: u8 = 6;

/// Wraps an image element and requests a few redraws after its handle changes
/// so the freshly uploaded image becomes visible without an input event.
pub(crate) struct FreshImage<'a, Message> {
    inner: Element<'a, Message>,
    handle_id: ImageId,
}

#[derive(Default)]
struct State {
    last_id: Option<ImageId>,
    frames_left: u8,
}

impl<'a, Message> FreshImage<'a, Message> {
    pub(crate) fn new(inner: impl Into<Element<'a, Message>>, handle_id: ImageId) -> Self {
        Self {
            inner: inner.into(),
            handle_id,
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for FreshImage<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.inner)]
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.inner));
    }

    fn size(&self) -> Size<Length> {
        self.inner.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.inner
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
        let state = tree.state.downcast_mut::<State>();

        // A changed handle means the upload may still be in flight; pull a few
        // frames until it lands.
        if state.last_id != Some(self.handle_id) {
            state.last_id = Some(self.handle_id);
            state.frames_left = EXTRA_FRAMES;
        }

        if let Event::Window(window::Event::RedrawRequested(_)) = event
            && state.frames_left > 0
        {
            state.frames_left -= 1;
            shell.request_redraw();
        }

        self.inner.as_widget_mut().update(
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
        self.inner.as_widget().mouse_interaction(
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
        self.inner.as_widget().draw(
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

impl<'a, Message: 'a> From<FreshImage<'a, Message>> for Element<'a, Message> {
    fn from(fresh: FreshImage<'a, Message>) -> Self {
        Element::new(fresh)
    }
}
