// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/widget/viewer_frame.rs
//
// Frames a rendered page for the app: wires a `Viewer` to the view state, the
// keyboard modifiers and the interaction callbacks, and pads it inside the
// content area. Shared by single images and annotation pages.

use std::cell::Cell;
use std::rc::Rc;

use cosmic::iced::keyboard::Modifiers;
use cosmic::iced::mouse;
use cosmic::iced::{ContentFit, Length, Point, Size};
use cosmic::prelude::*;
use cosmic::widget;

use crate::message::Message;
use crate::model::ViewState;
use crate::widget::image_viewer::{TextCaret, Viewer};

/// Build the fitted/zoomable viewer for a rendered image.
///
/// `on_click` is the click position in native image pixels plus the same click
/// in the viewer's own coordinates; `caret` is an optional insertion marker.
//
// The arguments mirror the widget's builder; bundling them into a struct would
// only move the noise to the call sites.
#[allow(clippy::too_many_arguments)]
pub(crate) fn viewer_frame(
    handle: &widget::image::Handle,
    view: ViewState,
    modifiers: Modifiers,
    reported_bounds: Rc<Cell<Size>>,
    cursor: mouse::Interaction,
    on_state_change: impl Fn(f32, f32, f32) -> Message + 'static,
    on_click: Option<Box<dyn Fn(Point, Point) -> Message>>,
    on_double_click: Option<Box<dyn Fn(Point, Point) -> Message>>,
    hover_cursor: Option<Box<dyn Fn(Point) -> Option<mouse::Interaction>>>,
    caret: Option<TextCaret>,
    caret_position: Option<Rc<Cell<Option<Point>>>>,
) -> Element<'static, Message> {
    let space = cosmic::theme::spacing();
    let content_fit = if view.fit {
        ContentFit::ScaleDown
    } else {
        ContentFit::None
    };
    let (scale, offset_x, offset_y) = if view.fit {
        (1.0, 0.0, 0.0)
    } else {
        (view.scale, view.offset_x, view.offset_y)
    };

    let mut viewer = Viewer::new(handle.clone())
        .content_fit(content_fit)
        .modifiers(modifiers)
        .cursor(cursor)
        .reported_bounds(reported_bounds)
        .width(Length::Fill)
        .height(Length::Fill)
        .with_state(scale, offset_x, offset_y)
        .on_state_change(on_state_change)
        .text_caret(caret);
    if let Some(caret_position) = caret_position {
        viewer = viewer.caret_position(caret_position);
    }
    if let Some(hover) = hover_cursor {
        viewer = viewer.hover_cursor(hover);
    }
    if let Some(on_click) = on_click {
        viewer = viewer.on_click(on_click);
    }
    if let Some(on_double_click) = on_double_click {
        viewer = viewer.on_double_click(on_double_click);
    }

    widget::container(viewer).padding(space.space_m).into()
}
