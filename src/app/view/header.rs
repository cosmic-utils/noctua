// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/header.rs
//
// Header bar buttons (navigation, rotation, flip).

use cosmic::iced::Length;
use cosmic::widget::{button, horizontal_space, icon};
use cosmic::Element;

use crate::app::message::AppMessage;
use crate::app::model::AppModel;
use crate::app::ContextPage;

/// Build the left side of the header bar.
pub fn header_start(model: &AppModel) -> Vec<Element<AppMessage>> {
    let has_doc = model.document.is_some();

    vec![
        // Nav bar toggle
        button::icon(icon::from_name("view-sidebar-start-symbolic"))
            .on_press(AppMessage::ToggleNavBar)
            .into(),
        // Spacer
        horizontal_space().width(Length::Fixed(12.0)).into(),
        // Navigation: previous / next
        button::icon(icon::from_name("go-previous-symbolic"))
            .on_press_maybe(has_doc.then_some(AppMessage::PrevDocument))
            .into(),
        button::icon(icon::from_name("go-next-symbolic"))
            .on_press_maybe(has_doc.then_some(AppMessage::NextDocument))
            .into(),
        // Spacer
        horizontal_space().width(Length::Fixed(12.0)).into(),
        // Rotation: counter-clockwise / clockwise
        button::icon(icon::from_name("object-rotate-left-symbolic"))
            .on_press_maybe(has_doc.then_some(AppMessage::RotateCCW))
            .into(),
        button::icon(icon::from_name("object-rotate-right-symbolic"))
            .on_press_maybe(has_doc.then_some(AppMessage::RotateCW))
            .into(),
        // Spacer
        horizontal_space().width(Length::Fixed(12.0)).into(),
        // Flip: horizontal / vertical
        button::icon(icon::from_name("object-flip-horizontal-symbolic"))
            .on_press_maybe(has_doc.then_some(AppMessage::FlipHorizontal))
            .into(),
        button::icon(icon::from_name("object-flip-vertical-symbolic"))
            .on_press_maybe(has_doc.then_some(AppMessage::FlipVertical))
            .into(),
    ]
}

/// Build the right side of the header bar.
pub fn header_end(model: &AppModel) -> Vec<Element<AppMessage>> {
    vec![button::icon(icon::from_name("dialog-information-symbolic"))
        .on_press(AppMessage::ToggleContextPage(ContextPage::Properties))
        .into()]
}
