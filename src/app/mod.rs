// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/mod.rs
//
// Application module root, re-exports, and COSMIC application wiring.

pub mod document;
pub mod message;
pub mod model;
pub mod update;

mod view;

use cosmic::app::Core;
use cosmic::iced::keyboard::{self, key::Named, Key, Modifiers};
use cosmic::iced::window;
use cosmic::iced::Subscription;
use cosmic::{Action, Element, Task};

pub use message::AppMessage;
pub use model::AppModel;

use crate::config::AppConfig;
use crate::Args;

/// Flags passed from `main` into the application.
#[derive(Debug, Clone)]
pub enum Flags {
    Args(Args),
}

/// Main application type.
pub struct Noctua {
    core: Core,
    pub model: AppModel,
}

impl cosmic::Application for Noctua {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = Flags;
    type Message = AppMessage;

    const APP_ID: &'static str = "org.codeberg.wfx.Noctua";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, flags: Self::Flags) -> (Self, Task<Action<Self::Message>>) {
        let config = AppConfig::default();
        let mut model = AppModel::new(config);

        // Use CLI arguments from `flags` to open initial file or folder.
        let Flags::Args(args) = flags;
        if let Some(path) = args.file {
            document::file::open_initial_path(&mut model, path);
        }

        (Self { core, model }, Task::none())
    }

    fn on_close_requested(&self, _id: window::Id) -> Option<Self::Message> {
        None
    }

    fn update(&mut self, message: Self::Message) -> Task<Action<Self::Message>> {
        update::update(&mut self.model, message);
        Task::none()
    }

    fn view(&self) -> Element<Self::Message> {
        view::view(&self.model)
    }

    fn view_window(&self, _id: window::Id) -> Element<Self::Message> {
        self.view()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        keyboard::on_key_press(handle_key_press)
    }
}

/// Map raw key presses + modifiers into high-level application messages.
fn handle_key_press(key: Key, modifiers: Modifiers) -> Option<AppMessage> {
    use AppMessage::*;

    // Handle Ctrl + arrow keys for panning.
    if modifiers.control() && !modifiers.shift() && !modifiers.alt() && !modifiers.logo() {
        return match key.as_ref() {
            Key::Named(Named::ArrowLeft) => Some(PanLeft),
            Key::Named(Named::ArrowRight) => Some(PanRight),
            Key::Named(Named::ArrowUp) => Some(PanUp),
            Key::Named(Named::ArrowDown) => Some(PanDown),
            _ => None,
        };
    }

    // Ignore key presses when command-style modifiers are pressed.
    if modifiers.command() || modifiers.alt() || modifiers.logo() || modifiers.control() {
        return None;
    }

    match key.as_ref() {
        // Navigation with arrow keys (no modifiers).
        Key::Named(Named::ArrowRight) => Some(NextDocument),
        Key::Named(Named::ArrowLeft) => Some(PrevDocument),

        // Transformations.
        Key::Character(ch) if ch.eq_ignore_ascii_case("h") => Some(FlipHorizontal),
        Key::Character(ch) if ch.eq_ignore_ascii_case("v") => Some(FlipVertical),
        Key::Character(ch) if ch.eq_ignore_ascii_case("r") => {
            if modifiers.shift() {
                Some(RotateCCW)
            } else {
                Some(RotateCW)
            }
        }

        // Zoom.
        Key::Character("+") | Key::Character("=") => Some(ZoomIn),
        Key::Character("-") => Some(ZoomOut),
        Key::Character("1") => Some(ZoomReset),
        Key::Character(ch) if ch.eq_ignore_ascii_case("f") => Some(ZoomFit),

        // Tool modes.
        Key::Character(ch) if ch.eq_ignore_ascii_case("c") => Some(ToggleCropMode),
        Key::Character(ch) if ch.eq_ignore_ascii_case("s") => Some(ToggleScaleMode),

        // Reset pan.
        Key::Character("0") => Some(PanReset),

        _ => None,
    }
}
