// SPDX-License-Identifier: GPL-3.0-or-later
// src/ui/mod.rs
//
// UI layer: COSMIC application, views, and widgets.

pub mod app;
pub mod message;
pub mod model;
pub mod update;
pub mod components;
pub mod views;
pub mod widgets;

// Re-export main types
pub use app::NoctuaApp;
pub use message::AppMessage;
pub use model::AppModel;
