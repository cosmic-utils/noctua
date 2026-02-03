// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/mod.rs
//
// View module exports.

pub mod canvas;
pub mod footer;
pub mod format_panel;
pub mod header;
pub mod image_viewer;
pub mod pages_panel;
pub mod panels;

use cosmic::iced::Length;
use cosmic::widget::container;
use cosmic::{Action, Element};

use crate::ui::model::NavPanel;
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
/// Shows different panels based on `active_nav_panel` state:
/// - `NavPanel::Format`: Format and orientation selection
/// - `NavPanel::Pages`: Page thumbnails (multi-page documents)
/// - `NavPanel::None`: Hidden
pub fn nav_bar<'a>(
    model: &'a AppModel,
    manager: &'a DocumentManager,
) -> Option<Element<'a, Action<AppMessage>>> {
    match model.active_nav_panel {
        NavPanel::None => None,
        NavPanel::Format => {
            let panel = format_panel::view(model);
            Some(
                container(panel.map(Action::App))
                    .width(Length::Shrink)
                    .height(Length::Fill)
                    .max_width(250)
                    .into(),
            )
        }
        NavPanel::Pages => {
            // Check if document has multiple pages using cached data
            if model.page_count.unwrap_or(1) <= 1 {
                return None;
            }

            pages_panel::view(model, manager).map(|panel| {
                container(panel.map(Action::App))
                    .width(Length::Shrink)
                    .height(Length::Fill)
                    .max_width(200)
                    .into()
            })
        }
    }
}
