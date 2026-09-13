// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/view.rs
//
// Pure view: builds widgets from the application model. Never changes state.

use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, ContentFit, Length};
use cosmic::widget::{self, icon, nav_bar, tab_bar};
use cosmic::prelude::*;

use noctua_core::storage;

use crate::fl;
use crate::message::Message;
use crate::model::{AppModel, CurrentTarget, TabContent};

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
        column = column.push(app.content_view());
    } else {
        let mut row = widget::row::with_capacity(2).spacing(space.space_xxs);
        if app.show_nav_panel {
            row = row.push(
                nav_bar::nav_bar(&app.nav_model, Message::NavActivated)
                    .into_container()
                    .width(Length::Fixed(220.0))
                    .height(Length::Fill),
            );
        }
        row = row.push(app.content_view());
        column = column.push(row.height(Length::Fill));
    }

    column = column.push(app.status_bar());

    widget::container(column)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

impl AppModel {
    /// Status bar text describing the current nav position.
    fn position_label(&self) -> String {
        let (Some(position), Some(active)) = (self.current_position, self.active_tab()) else {
            return String::new();
        };
        let total = self.nav_model.len();

        match self.tabs.get(&active) {
            Some(TabContent::Document { .. }) => {
                format!("{} / {total}", fl!("page-num", num = position))
            }
            _ => format!("{position} / {total}"),
        }
    }

    /// The content area view.
    fn content_view(&self) -> Element<'_, Message> {
        let space = cosmic::theme::spacing();

        if self.tabs.is_empty() {
            return widget::container(
                widget::text::body(match &self.start_error {
                    Some(path) => fl!("start-error", path = path),
                    None => fl!("open-folder-hint"),
                })
                .size(18)
                .apply(widget::container)
                .width(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        }

        match &self.current_image {
            Some(image) => {
                // Known upstream issue: the iced image atlas splits images
                // larger than its 2048 px layers into fragments and its
                // upload bounds check drops the bottom-right fragment, so
                // such images render with a missing block. Deliberately
                // not worked around — fix belongs in iced/libcosmic.
                if (self.zoom - 1.0).abs() < f32::EPSILON {
                    // Fit view: scale down images larger than the viewport,
                    // keep smaller ones at their native size, never crop.
                    widget::container(
                        widget::Image::new(image.handle.clone()).content_fit(ContentFit::ScaleDown),
                    )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(space.space_m)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
                    .into()
                } else {
                    // Zoomed view: show native pixels and scroll.
                    widget::scrollable(
                        widget::container(widget::Image::new(image.handle.clone()))
                            .padding(space.space_m),
                    )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
                }
            }
            None => widget::container(
                widget::text::body(fl!("select-hint"))
                    .apply(widget::container)
                    .width(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        }
    }

    /// The status bar view.
    fn status_bar(&self) -> Element<'_, Message> {
        let space = cosmic::theme::spacing();

        let mut row = widget::row::with_capacity(8)
            .spacing(space.space_m)
            .align_y(Alignment::Center);

        row = row
            .push(
                widget::button::icon(icon::from_name("go-previous-symbolic"))
                    .on_press(Message::PrevEntry),
            )
            .push(
                widget::button::icon(icon::from_name("go-next-symbolic"))
                    .on_press(Message::NextEntry),
            );

        let position = self.position_label();
        if !position.is_empty() {
            row = row.push(widget::text::body(position));
        }

        row = row.push(widget::text::body(format!("{:.0}%", self.zoom * 100.0)));

        if let Some(target) = &self.current_target {
            let path = match target {
                CurrentTarget::File { path } => path,
                CurrentTarget::Page { path, .. } => path,
            };
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                row = row.push(widget::text::body(name.to_string()));
            }
        }

        if let Some(size) = self.current_size {
            row = row.push(widget::text::body(storage::document::format_size(size)));
        }

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
}
