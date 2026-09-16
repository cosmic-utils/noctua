// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/widget/empty_state.rs
//
// Reusable centered placeholder: an icon above a hint text.

use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget;

use crate::message::Message;

/// Build a centered icon + hint placeholder.
pub(crate) fn empty_state(icon_name: &'static str, hint: String) -> Element<'static, Message> {
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
