// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/key_binds.rs
//
// Menu key bindings: the map from key chord to menu action, in one place.

use std::collections::HashMap;

use cosmic::iced::keyboard::key::{Key, Named};
use cosmic::widget::menu;
use cosmic::widget::menu::key_bind::Modifier;

use crate::message::MenuAction;

/// The menu key bindings.
pub(crate) fn key_binds() -> HashMap<menu::KeyBind, MenuAction> {
    let bind = |modifiers: &[Modifier], key: Key, action: MenuAction| {
        (
            menu::KeyBind {
                modifiers: modifiers.to_vec(),
                key,
            },
            action,
        )
    };

    HashMap::from([
        bind(
            &[Modifier::Ctrl],
            Key::Character("o".into()),
            MenuAction::OpenFolder,
        ),
        bind(
            &[Modifier::Ctrl],
            Key::Character("s".into()),
            MenuAction::SavePdf,
        ),
        bind(
            &[Modifier::Ctrl, Modifier::Shift],
            Key::Character("s".into()),
            MenuAction::SavePdfAs,
        ),
        bind(
            &[Modifier::Ctrl],
            Key::Character("w".into()),
            MenuAction::CloseTab,
        ),
        // Zoom in: register every `=`/`+` combination because the two keys
        // differ per layout (e.g. on German QWERTZ `+` is its own key and
        // `=` is Shift+`0`).
        bind(
            &[Modifier::Ctrl],
            Key::Character("=".into()),
            MenuAction::ZoomIn,
        ),
        bind(
            &[Modifier::Ctrl, Modifier::Shift],
            Key::Character("=".into()),
            MenuAction::ZoomIn,
        ),
        bind(
            &[Modifier::Ctrl],
            Key::Character("+".into()),
            MenuAction::ZoomIn,
        ),
        bind(
            &[Modifier::Ctrl, Modifier::Shift],
            Key::Character("+".into()),
            MenuAction::ZoomIn,
        ),
        bind(
            &[Modifier::Ctrl],
            Key::Character("-".into()),
            MenuAction::ZoomOut,
        ),
        bind(
            &[Modifier::Ctrl],
            Key::Character("1".into()),
            MenuAction::Zoom100,
        ),
        bind(
            &[Modifier::Ctrl],
            Key::Character("0".into()),
            MenuAction::ZoomToFit,
        ),
        bind(
            &[Modifier::Ctrl],
            Key::Character("r".into()),
            MenuAction::RotateClockwise,
        ),
        bind(
            &[Modifier::Ctrl, Modifier::Shift],
            Key::Character("r".into()),
            MenuAction::RotateCounterClockwise,
        ),
        bind(
            &[Modifier::Ctrl],
            Key::Character("h".into()),
            MenuAction::FlipHorizontal,
        ),
        bind(
            &[Modifier::Ctrl],
            Key::Character("v".into()),
            MenuAction::FlipVertical,
        ),
        bind(&[], Key::Named(Named::F11), MenuAction::Fullscreen),
        bind(
            &[Modifier::Ctrl],
            Key::Character("q".into()),
            MenuAction::Quit,
        ),
    ])
}
