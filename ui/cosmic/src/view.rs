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
use crate::model::{AppModel, CurrentTarget};
use crate::widget::document_preview::document_preview;
use crate::widget::empty_state::empty_state;
use crate::widget::thumbnail_strip::thumbnail_strip;

/// Describes the interface based on the current state of the application model.
///
/// Application events will be processed through the view. Any messages emitted by
/// events received by widgets will be passed to the update method.
pub(crate) fn view(app: &AppModel) -> Element<'_, Message> {
    let space = cosmic::theme::spacing();

    let mut column = widget::column::with_capacity(3).spacing(space.space_xxs);

    // A single tab is the whole window; only show the tab bar with more,
    // matching COSMIC Terminal.
    if app.tabs.len() > 1 {
        let tab_strip = tab_bar::horizontal(&app.tab_model)
            .on_activate(Message::TabActivated)
            .on_close(Message::TabCloseRequested)
            .button_height(32)
            .button_spacing(space.space_xxs);
        column = column.push(tab_strip);
    }

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
    let Some(tab) = app.active_tab() else {
        return widget::container(widget::text::body("")).into();
    };
    let Some(state) = app.tab_ui.get(&tab) else {
        return widget::container(widget::text::body("")).into();
    };

    thumbnail_strip(&state.strip, state.selected, state.expanded.as_ref(), tab)
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
        return document_preview(preview);
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

    format!("{} / {total}", position + 1)
}
