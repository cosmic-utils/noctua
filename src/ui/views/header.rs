// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/header.rs
//
// Header bar content (navigation, rotation, flip).

use cosmic::iced::Length;
use cosmic::widget::{button, horizontal_space, icon, row};
use cosmic::Element;

use crate::ui::message::AppMessage;
use crate::ui::model::AppModel;
use crate::ui::app::ContextPage;
use crate::application::DocumentManager;
use crate::fl;

/// Build the start (left) side of the header bar.
pub fn start<'a>(
    model: &'a AppModel,
    _manager: &'a DocumentManager,
) -> Vec<Element<'a, AppMessage>> {
    let has_doc = model.current_image_handle.is_some();

    // Left section: Panel toggle + Menu + Navigation
    let left_controls = row()
        .spacing(4)
        .push(
            button::icon(icon::from_name("view-sidebar-start-symbolic"))
                .on_press(AppMessage::ToggleNavBar)
                .tooltip(fl!("tooltip-nav-toggle")),
        )
        .push(
            button::icon(icon::from_name("open-menu-symbolic"))
                .on_press(AppMessage::ToggleMainMenu)
                .tooltip(fl!("menu-main")),
        )
        .push(
            button::icon(icon::from_name("go-previous-symbolic"))
                .on_press_maybe(has_doc.then_some(AppMessage::PrevDocument))
                .tooltip(fl!("tooltip-nav-previous")),
        )
        .push(
            button::icon(icon::from_name("go-next-symbolic"))
                .on_press_maybe(has_doc.then_some(AppMessage::NextDocument))
                .tooltip(fl!("tooltip-nav-next")),
        );

    // Center section: Transformations
    let center_controls = row()
        .spacing(4)
        .push(
            button::icon(icon::from_name("object-rotate-left-symbolic"))
                .on_press_maybe(has_doc.then_some(AppMessage::RotateCCW))
                .tooltip(fl!("tooltip-rotate-ccw")),
        )
        .push(
            button::icon(icon::from_name("object-rotate-right-symbolic"))
                .on_press_maybe(has_doc.then_some(AppMessage::RotateCW))
                .tooltip(fl!("tooltip-rotate-cw")),
        )
        .push(horizontal_space().width(Length::Fixed(12.0)))
        .push(
            button::icon(icon::from_name("object-flip-horizontal-symbolic"))
                .on_press_maybe(has_doc.then_some(AppMessage::FlipHorizontal))
                .tooltip(fl!("tooltip-flip-horizontal")),
        )
        .push(
            button::icon(icon::from_name("object-flip-vertical-symbolic"))
                .on_press_maybe(has_doc.then_some(AppMessage::FlipVertical))
                .tooltip(fl!("tooltip-flip-vertical")),
        );

    vec![
        left_controls.into(),
        center_controls.into(),
        horizontal_space().width(Length::Fill).into(),
    ]
}

/// Build the end (right) side of the header bar.
pub fn end<'a>(
    _model: &'a AppModel,
    _manager: &'a DocumentManager,
) -> Vec<Element<'a, AppMessage>> {
    vec![
        // Info panel toggle
        button::icon(icon::from_name("dialog-information-symbolic"))
            .on_press(AppMessage::ToggleContextPage(ContextPage::Properties))
            .tooltip(fl!("tooltip-info-panel"))
            .into(),
    ]
}
