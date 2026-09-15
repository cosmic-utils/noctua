// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/view.rs
//
// Pure view: builds widgets from the application model. Never changes state.

use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::widget::scrollable::{Direction, Scrollbar};
use cosmic::iced::{Alignment, ContentFit, Length};
use cosmic::prelude::*;
use cosmic::widget::{self, icon, tab_bar};

use noctua_core::storage;

use crate::fl;
use crate::message::Message;
use crate::model::{
    AppModel, CurrentTarget, DocumentPreview, PREVIEW_SCROLL_ID, PageSlot, STRIP_SCROLL_ID,
    TabContent,
};

/// Strip tile size in logical pixels.
const TILE: f32 = 136.0;

/// Describes the interface based on the current state of the application model.
///
/// Application events will be processed through the view. Any messages emitted by
/// events received by widgets will be passed to the update method.
pub(crate) fn view(app: &AppModel) -> Element<'_, Message> {
    let space = cosmic::theme::spacing();

    let tab_strip = tab_bar::horizontal(&app.tab_model)
        .on_activate(Message::TabActivated)
        .on_close(Message::TabCloseRequested);

    let mut column = widget::column::with_capacity(3).spacing(space.space_xxs);
    column = column.push(tab_strip);

    if app.tabs.is_empty() {
        column = column.push(content_view(app));
    } else {
        let mut row = widget::row::with_capacity(2).spacing(space.space_xxs);
        if app.show_nav_panel {
            row = row.push(strip_view(app));
        }
        row = row.push(content_view(app));
        column = column.push(row.height(Length::Fill));
    }

    column = column.push(status_bar(app));

    widget::container(column)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The thumbnail strip of the active tab: image tiles, lazy thumbnails.
fn strip_view(app: &AppModel) -> Element<'_, Message> {
    let space = cosmic::theme::spacing();
    let Some(tab) = app.active_tab() else {
        return widget::container(widget::text::body("")).into();
    };
    let Some(state) = app.tab_ui.get(&tab) else {
        return widget::container(widget::text::body("")).into();
    };

    let mut column = widget::column::with_capacity(state.strip.len())
        .spacing(space.space_xxs)
        .padding(space.space_xxs);

    for (index, entry) in state.strip.iter().enumerate() {
        let selected = state.selected == Some(index);
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
            .width(Length::Fixed(TILE))
            .height(Length::Fixed(TILE))
            .padding(space.space_xxs)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .class(if selected {
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
        .id(widget::Id::new(STRIP_SCROLL_ID))
        .on_scroll(move |viewport| Message::StripScrolled {
            tab,
            offset_y: viewport.absolute_offset().y,
            viewport_height: viewport.bounds().height,
        });

    // The strip panel: fixed-width, scrollable thumbnail column.
    widget::container(strip_scroll)
        .width(Length::Fixed(168.0))
        .height(Length::Fill)
        .into()
}

/// The continuous multi-page preview of a PDF inside a folder tab.
fn preview_view<'a>(preview: &'a DocumentPreview) -> Element<'a, Message> {
    let space = cosmic::theme::spacing();

    let mut column = widget::column::with_capacity(preview.pages.len())
        .spacing(space.space_xs)
        .padding(space.space_m);

    for (index, slot) in preview.pages.iter().enumerate() {
        let (_, height) = preview
            .page_sizes
            .get(index)
            .copied()
            .unwrap_or((600.0, 800.0));
        let box_height = height * preview.zoom;
        let page: Element<'_, Message> = match slot {
            PageSlot::Empty => widget::space()
                .width(Length::Fill)
                .height(Length::Fixed(box_height))
                .into(),
            PageSlot::Thumb { handle } | PageSlot::Full { handle } => widget::container(
                widget::Image::new(handle.clone()).content_fit(ContentFit::ScaleDown),
            )
            .width(Length::Fill)
            .height(Length::Fixed(box_height))
            .align_x(Horizontal::Center)
            .into(),
        };
        column = column.push(page);
    }

    widget::scrollable(column)
        .id(widget::Id::new(PREVIEW_SCROLL_ID))
        .on_scroll(|viewport| Message::PreviewScrolled {
            path: preview.path.clone(),
            offset_y: viewport.absolute_offset().y,
            viewport_height: viewport.bounds().height,
        })
        .into()
}

/// A centered placeholder: an icon above a hint text.
fn empty_state(icon_name: &'static str, hint: String) -> Element<'static, Message> {
    let space = cosmic::theme::spacing();
    widget::container(
        widget::column::with_capacity(2)
            .spacing(space.space_m)
            .align_x(Alignment::Center)
            .push(widget::icon::from_name(icon_name).size(48))
            .push(widget::text::body(hint)),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Horizontal::Center)
    .align_y(Vertical::Center)
    .into()
}

/// The content area view.
fn content_view(app: &AppModel) -> Element<'_, Message> {
    let space = cosmic::theme::spacing();

    if app.tabs.is_empty() {
        let hint = match &app.start_error {
            Some(path) => fl!("start-error", path = path),
            None => fl!("open-folder-hint"),
        };
        return empty_state("folder-open-symbolic", hint);
    }

    // Continuous preview of the selected multi-page PDF.
    if let Some(tab) = app.active_tab()
        && let Some(state) = app.tab_ui.get(&tab)
        && let Some(preview) = state.preview.as_ref()
        && matches!(&app.current_target, Some(CurrentTarget::File { path }) if path == &preview.path)
    {
        return preview_view(preview);
    }

    match &app.current_image {
        Some(image) => {
            // Known upstream issue: the iced image atlas splits images
            // larger than its 2048 px layers into fragments and its
            // upload bounds check drops the bottom-right fragment, so
            // such images render with a missing block. Deliberately
            // not worked around — fix belongs in iced/libcosmic.
            let content: Element<'_, Message> = if (app.zoom - 1.0).abs() < f32::EPSILON {
                // Fit view: scale down images larger than the viewport,
                // keep smaller ones at their native size, never crop.
                // The mouse wheel zooms over the image.
                widget::mouse_area(
                    widget::container(
                        widget::Image::new(image.handle.clone()).content_fit(ContentFit::ScaleDown),
                    )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(space.space_m)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center),
                )
                .on_scroll(Message::WheelZoom)
                .into()
            } else {
                // Zoomed view: show native pixels at their rendered size.
                // ContentFit::None keeps the image from being re-fitted to the
                // viewport; the scrollable pans in both axes. The mouse area
                // sits inside the scrollable so the wheel zooms (it captures
                // the event before the scrollable would scroll).
                widget::scrollable(
                    widget::mouse_area(
                        widget::container(
                            widget::Image::new(image.handle.clone()).content_fit(ContentFit::None),
                        )
                        .padding(space.space_m),
                    )
                    .on_scroll(Message::WheelZoom),
                )
                .direction(Direction::Both {
                    vertical: Scrollbar::new(),
                    horizontal: Scrollbar::new(),
                })
                .scroller_width(8.0)
                .scrollbar_width(8.0)
                .scrollbar_padding(8.0)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
            };

            content
        }
        None => empty_state("image-x-generic-symbolic", fl!("select-hint")),
    }
}

/// The status bar view.
fn status_bar(app: &AppModel) -> Element<'_, Message> {
    let space = cosmic::theme::spacing();

    let nav = widget::row::with_capacity(2)
        .spacing(space.space_xxs)
        .push(
            widget::button::icon(icon::from_name("go-previous-symbolic"))
                .on_press(Message::PrevEntry),
        )
        .push(
            widget::button::icon(icon::from_name("go-next-symbolic")).on_press(Message::NextEntry),
        );

    let mut info = widget::row::with_capacity(4)
        .spacing(space.space_s)
        .align_y(Alignment::Center);

    let position = position_label(app);
    if !position.is_empty() {
        info = info.push(widget::text::body(position));
    }

    info = info.push(widget::text::body(format!("{:.0}%", app.zoom * 100.0)));

    if let Some(target) = &app.current_target {
        let path = match target {
            CurrentTarget::File { path } => path,
            CurrentTarget::Page { path, .. } => path,
        };
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            info = info.push(widget::text::body(name.to_string()));
        }
    }

    if let Some(size) = app.current_size {
        info = info.push(widget::text::body(storage::document::format_size(size)));
    }

    let row = widget::row::with_capacity(3)
        .spacing(space.space_s)
        .align_y(Alignment::Center)
        .push(nav)
        .push(widget::space().width(Length::Fill))
        .push(info);

    widget::container(row)
        .width(Length::Fill)
        .padding([
            space.space_xxs,
            space.space_s,
            space.space_xxs,
            space.space_s,
        ])
        .into()
}

/// Status bar text describing the current strip position.
fn position_label(app: &AppModel) -> String {
    let Some(tab) = app.active_tab() else {
        return String::new();
    };
    let Some(state) = app.tab_ui.get(&tab) else {
        return String::new();
    };
    let total = state.strip.len();
    let Some(position) = state.selected else {
        return String::new();
    };

    match app.tabs.get(&tab) {
        Some(TabContent::Document { .. }) => {
            let page = (position + 1) as u32;
            format!("{} / {total}", fl!("page-num", num = page))
        }
        _ => format!("{} / {total}", position + 1),
    }
}
