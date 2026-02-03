// SPDX-License-Identifier: GPL-3.0-or-later
// src/ui/mod.rs
//
// UI layer: COSMIC application, views, and components.

pub mod app;
pub mod message;
pub mod model;
pub mod update;
pub mod components;
pub mod views;

// Internal module for syncing model from DocumentManager
pub(crate) mod sync;

// Re-export main types
pub use app::NoctuaApp;
pub use message::AppMessage;
pub use model::AppModel;
