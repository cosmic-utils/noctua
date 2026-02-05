// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/mod.rs
//
// View module exports.

pub mod canvas;
pub mod footer;
pub mod format_panel;
pub mod header;
pub mod meta_panel;
pub mod pages_panel;
pub mod panels;

use cosmic::iced::Length;
use cosmic::widget::container;
use cosmic::{Action, Element};

use crate::ui::model::LeftPanel;
use crate::ui::{AppMessage, AppModel};
use crate::application::DocumentManager;
use crate::config::AppConfig;

/// Main application view (canvas area).
pub fn view<'a>(
    model: &'a AppModel,
    manager: &'a DocumentManager,
    config: &'a AppConfig,
) -> Element<'a, AppMessage> {
    canvas::view(model, manager, config)
}

/// Navigation bar content (left panel).
///
/// Shows different panels based on panel state:
/// - `LeftPanel::Thumbnails`: Page thumbnails (multi-page documents)
/// - `None`: Hidden
pub fn nav_bar<'a>(
    model: &'a AppModel,
    manager: &'a DocumentManager,
) -> Option<Element<'a, Action<AppMessage>>> {
    match model.panels.left {
        None => None,
        Some(LeftPanel::Thumbnails) => pages_panel::view(model, manager).map(|panel| {
            container(panel.map(Action::App))
                .width(Length::Shrink)
                .height(Length::Fill)
                .max_width(250)
                .into()
        }),
    }
}
