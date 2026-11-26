// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/widget/strip.rs
//
// Shared parts of the thumbnail strips: panel width, tile frame, the preview
// inside a tile and the scrollable column. The folder strip and the annotation
// page strip use the same sizes, framing and selection style, and differ only
// in what a tile shows and which message it publishes.

use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{ContentFit, Length};
use cosmic::prelude::*;
use cosmic::widget;

use crate::message::Message;

/// Strip panel width in logical pixels.
pub(crate) const PANEL_WIDTH: f32 = 168.0;

/// File and annotation page tile size in logical pixels.
pub(crate) const TILE_SIZE: f32 = 136.0;

/// The framed tile: a preview in a selectable card.
pub(crate) fn tile<'a>(
    body: Element<'a, Message>,
    size: f32,
    selected: bool,
) -> Element<'a, Message> {
    let space = cosmic::theme::spacing();
    widget::container(body)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .padding(space.space_xxs)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .class(if selected {
            cosmic::style::Container::Primary
        } else {
            cosmic::style::Container::Card
        })
        .into()
}

/// The preview of a tile: its thumbnail, or `placeholder` while unloaded.
pub(crate) fn preview<'a>(
    thumb: Option<&widget::image::Handle>,
    placeholder: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    match thumb {
        Some(handle) => widget::Image::new(handle.clone())
            .content_fit(ContentFit::ScaleDown)
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        None => placeholder.into(),
    }
}

/// The scrollable, centered column of a strip; the caller pushes the tiles.
pub(crate) fn column<'a>(
    capacity: usize,
) -> widget::Column<'a, Message, cosmic::Theme, cosmic::Renderer> {
    let space = cosmic::theme::spacing();
    widget::column::with_capacity(capacity)
        .spacing(space.space_xxs)
        .padding(space.space_xxs)
        .align_x(Horizontal::Center)
}

/// The fixed-width strip panel around the column's scrollable.
pub(crate) fn panel<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    widget::container(content)
        .width(Length::Fixed(PANEL_WIDTH))
        .height(Length::Fill)
        .into()
}
