// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/panels.rs
//
// Header, footer, and side panels composing the main layout.

use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, Column, Container, Row, Text};

use crate::fl;
use crate::app::model::ViewMode;
use crate::app::{AppMessage, AppModel};

/// Top header bar (global actions, toggles).
pub fn header(_model: &AppModel) -> Element<'_, AppMessage> {
    let content = Row::new().spacing(8).align_y(Alignment::Center);
    // In a real implementation, add more buttons/actions here.

    Container::new(content)
        .width(Length::Fill)
        .padding([4, 8])
        .into()
}

/// Bottom footer bar (navigation & zoom).
pub fn footer(model: &AppModel) -> Element<'_, AppMessage> {
    let nav = Row::new()
        .spacing(4)
        .align_y(Alignment::Center)
        .push(widget::button::standard("<").on_press(AppMessage::PrevDocument))
        .push(widget::button::standard(">").on_press(AppMessage::NextDocument));

    let zoom_text = match model.view_mode {
        ViewMode::Fit => "Fit".to_string(),
        ViewMode::ActualSize => "100%".to_string(),
        ViewMode::Custom(zoom_factor) => format!("{:.0}%", zoom_factor * 100.0),
    };

    let zoom_info = Text::new(format!("Zoom: {}", zoom_text));

    let content = Row::new()
        .spacing(16)
        .align_y(Alignment::Center)
        .push(nav)
        .push(zoom_info);

    Container::new(content)
        .width(Length::Fill)
        .padding([4, 8])
        .into()
}

/// Optional left panel (tools).
pub fn left_panel(model: &AppModel) -> Option<Element<'_, AppMessage>> {
    if !model.show_left_panel {
        return None;
    }

    let tools = Column::new()
        .spacing(4)
        .push(Text::new(fl!("tools")))
        .push(widget::button::standard(fl!("crop")).on_press(AppMessage::ToggleCropMode))
        .push(widget::button::standard(fl!("scale")).on_press(AppMessage::ToggleScaleMode));

    let panel = Container::new(tools)
        .width(Length::Fixed(180.0))
        .height(Length::Fill)
        .padding(8);

    Some(panel.into())
}

/// Optional right panel (metadata, info).
pub fn right_panel(model: &AppModel) -> Option<Element<'_, AppMessage>> {
    if !model.show_right_panel {
        return None;
    }

    let meta = Column::new()
        .spacing(4)
        .push(Text::new("Metadata"))
        .push(Text::new(format!(
            "Current index: {:?}",
            model.current_index
        )));

    let panel = Container::new(meta)
        .width(Length::Fixed(220.0))
        .height(Length::Fill)
        .padding(8);

    Some(panel.into())
}
