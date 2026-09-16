// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/widget/thumbnail_strip.rs
//
// Reusable thumbnail strip: a fixed-width, scrollable column of tiles that
// selects (click) or expands (double-click) an entry. Expanded multi-page
// PDFs list their pages as indented, smaller, page-numbered tiles.

use std::path::PathBuf;

use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{ContentFit, Length};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic::widget::segmented_button::Entity;

use crate::message::Message;
use crate::model::{NavEntry, StripEntry};

/// File tile size in logical pixels.
const TILE_SIZE: f32 = 136.0;

/// Page tile size in logical pixels (smaller than files).
const PAGE_TILE_SIZE: f32 = 96.0;

/// Left indentation of page entries in logical pixels.
const INDENT: u16 = 20;

/// Strip panel width in logical pixels.
const PANEL_WIDTH: f32 = 168.0;

/// Stable id of the strip scrollable.
const SCROLL_ID: &str = "strip-scroll";

/// Build the thumbnail strip for one tab. Emits `Message::StripActivated`,
/// `Message::StripDoubleClicked` and `Message::StripScrolled`.
pub(crate) fn thumbnail_strip<'a>(
    entries: &[StripEntry],
    selected: Option<usize>,
    expanded: Option<&PathBuf>,
    tab: Entity,
) -> Element<'a, Message> {
    let space = cosmic::theme::spacing();

    let mut column = widget::column::with_capacity(entries.len())
        .spacing(space.space_xxs)
        .padding(space.space_xxs)
        .align_x(Horizontal::Center);

    for (index, entry) in entries.iter().enumerate() {
        let is_selected = selected == Some(index);
        let is_page = matches!(entry.target, NavEntry::Page { .. });
        let is_expanded =
            matches!(&entry.target, NavEntry::File { path } if expanded == Some(path));
        let tile_size = if is_page { PAGE_TILE_SIZE } else { TILE_SIZE };

        let body: Element<'_, Message> = match &entry.thumb {
            Some(handle) => widget::Image::new(handle.clone())
                .content_fit(ContentFit::ScaleDown)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            // Unloaded file tiles show the name; unloaded page tiles stay
            // blank because the page number is shown below the tile.
            None if is_page => widget::space()
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            None => widget::text::caption(entry.name.clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        };

        let tile = widget::container(body)
            .width(Length::Fixed(tile_size))
            .height(Length::Fixed(tile_size))
            .padding(space.space_xxs)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .class(if is_selected {
                cosmic::style::Container::Primary
            } else {
                cosmic::style::Container::Card
            });

        // Page entries are indented and carry their page number; expandable
        // files show a chevron to hint at the double-click.
        let entry_widget: Element<'_, Message> = if is_page {
            widget::container(
                widget::column::with_capacity(2)
                    .spacing(space.space_xxs)
                    .push(tile)
                    .push(widget::text::caption(entry.name.clone())),
            )
            .padding([0, 0, 0, INDENT])
            .into()
        } else if entry.expandable {
            let chevron: &'static str = if is_expanded { "▾" } else { "▸" };
            widget::column::with_capacity(2)
                .spacing(space.space_xxs)
                .push(tile)
                .push(widget::text::caption(chevron))
                .into()
        } else {
            tile.into()
        };

        column = column.push(
            widget::mouse_area(entry_widget)
                .on_press(Message::StripActivated(index))
                .on_double_click(Message::StripDoubleClicked(index)),
        );
    }

    let strip_scroll = widget::scrollable(column)
        .id(widget::Id::new(SCROLL_ID))
        // Reserve the scrollbar width instead of overlaying the thumbnails,
        // so the gap to the scrollbar matches the gap to the panel edge.
        .spacing(0.0)
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
