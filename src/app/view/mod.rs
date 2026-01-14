// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/view/mod.rs
//
// View module root, combining all view components.

mod canvas;
pub mod footer;
pub mod header;
pub mod panels;

use cosmic::Element;

use crate::app::{AppMessage, AppModel};

/// Main application view (canvas area).
pub fn view(model: &AppModel) -> Element<'_, AppMessage> {
    canvas::view(model)
}
