// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/widget/thumbnail_strip.rs
//
// Reusable thumbnail strip: a fixed-width, scrollable column of tiles that
// selects (click) or opens (double-click) an entry.

use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{ContentFit, Length};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic::widget::segmented_button::Entity;

use crate::message::Message;
use crate::model::StripEntry;

/// Strip tile size in logical pixels.
const TILE_SIZE: f32 = 136.0;

/// Strip panel width in logical pixels.
const PANEL_WIDTH: f32 = 168.0;

/// Stable id of the strip scrollable.
const SCROLL_ID: &str = "strip-scroll";

/// Build the thumbnail strip for one tab. Emits `Message::StripActivated`,
/// `Message::StripDoubleClicked` and `Message::StripScrolled`.
pub(crate) fn thumbnail_strip<'a>(
    entries: &[StripEntry],
    selected: Option<usize>,
    tab: Entity,
) -> Element<'a, Message> {
    let space = cosmic::theme::spacing();

    let mut column = widget::column::with_capacity(entries.len())
        .spacing(space.space_xxs)
        .padding(space.space_xxs);

    for (index, entry) in entries.iter().enumerate() {
        let is_selected = selected == Some(index);
        let tile: Element<'_, Message> = match &entry.thumb {
            Some(handle) => widget::Image::new(handle.clone())
                .content_fit(ContentFit::ScaleDown)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            // Unloaded tiles show the file name until the thumb arrives.
            None => widget::text::caption(entry.name.clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        };

        let tile = widget::container(tile)
            .width(Length::Fixed(TILE_SIZE))
            .height(Length::Fixed(TILE_SIZE))
            .padding(space.space_xxs)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .class(if is_selected {
                cosmic::style::Container::Primary
            } else {
                cosmic::style::Container::Card
            });
        column = column.push(
            widget::mouse_area(tile)
                .on_press(Message::StripActivated(index))
                .on_double_click(Message::StripDoubleClicked(index)),
        );
    }

    let strip_scroll = widget::scrollable(column)
        .id(widget::Id::new(SCROLL_ID))
        .on_scroll(move |viewport| Message::StripScrolled {
            tab,
            offset_y: viewport.absolute_offset().y,
            viewport_height: viewport.bounds().height,
        });

    // The strip panel: fixed-width, scrollable thumbnail column.
    widget::container(strip_scroll)
        .width(Length::Fixed(PANEL_WIDTH))
        .height(Length::Fill)
        .into()
}
