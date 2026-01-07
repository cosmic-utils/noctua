// SPDX-License-Identifier: MPL-2.0
// src/app/view/mod.rs
//
// Root layout for the main application window.

pub mod canvas;
pub mod panels;

use cosmic::Element;
use cosmic::iced::Length;
use cosmic::widget::{Column, Container, Row};

use crate::app::{AppMessage, AppModel};

/// Main window layout (header, center row, footer).
pub fn view(model: &AppModel) -> Element<'_, AppMessage> {
    let header = panels::header(model);
    let footer = panels::footer(model);
    let left_panel = panels::left_panel(model);
    let right_panel = panels::right_panel(model);
    let canvas = canvas::view(model);

    // Build middle row step by step to handle optional panels.
    let mut middle_row = Row::new().spacing(8).height(Length::Fill);

    if let Some(left) = left_panel {
        middle_row = middle_row.push(left);
    }

    middle_row = middle_row.push(canvas);

    if let Some(right) = right_panel {
        middle_row = middle_row.push(right);
    }

    let content = Column::new()
        .spacing(8)
        .padding(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .push(header)
        .push(middle_row)
        .push(footer);

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
