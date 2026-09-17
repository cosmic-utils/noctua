// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/widget/zoom_controls.rs
//
// Footer zoom controls: zoom out/in buttons and a percentage menu with
// presets, in the style of a typical image viewer.

use std::collections::HashMap;

use cosmic::prelude::*;
use cosmic::widget::{self, icon, menu};

use crate::fl;
use crate::message::{MenuAction, Message};

/// Build the zoom controls. `fit` and `scale` describe the current zoom of the
/// active single image; the percentage menu emits the preset actions.
pub(crate) fn zoom_controls<'a>(
    fit: bool,
    scale: f32,
    key_binds: &HashMap<menu::KeyBind, MenuAction>,
) -> Element<'a, Message> {
    let space = cosmic::theme::spacing();

    let label = if fit {
        fl!("fit")
    } else {
        format!("{:.0}%", scale * 100.0)
    };

    let percentage_menu = menu::bar(vec![menu::Tree::with_children(
        menu::root(label).apply(Element::from),
        menu::items(
            key_binds,
            vec![
                menu::Item::Button(fl!("fit"), None, MenuAction::ZoomToFit),
                menu::Item::Button("50 %".to_string(), None, MenuAction::Zoom50),
                menu::Item::Button(fl!("zoom-100"), None, MenuAction::Zoom100),
                menu::Item::Button("200 %".to_string(), None, MenuAction::Zoom200),
                menu::Item::Button("400 %".to_string(), None, MenuAction::Zoom400),
            ],
        ),
    )]);

    widget::row::with_capacity(3)
        .spacing(space.space_xxs)
        .push(widget::button::icon(icon::from_name("zoom-out-symbolic")).on_press(Message::ZoomOut))
        .push(percentage_menu)
        .push(widget::button::icon(icon::from_name("zoom-in-symbolic")).on_press(Message::ZoomIn))
        .into()
}
