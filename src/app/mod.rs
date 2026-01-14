// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/mod.rs
//
// Application module root, re-exports, and COSMIC application wiring.

pub mod document;
pub mod message;
pub mod model;
pub mod update;

mod view;

use cosmic::app::{context_drawer, Core};
use cosmic::iced::keyboard::{self, key::Named, Key, Modifiers};
use cosmic::iced::window;
use cosmic::iced::Subscription;
use cosmic::widget::{button, icon, nav_bar};
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

/// Context page displayed in right drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    Properties,
}

/// Main application type.
pub struct Noctua {
    core: Core,
    pub model: AppModel,
    nav: nav_bar::Model,
    context_page: ContextPage,
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

    fn init(mut core: Core, flags: Self::Flags) -> (Self, Task<Action<Self::Message>>) {
        let config = AppConfig::default();
        let mut model = AppModel::new(config);

        // Use CLI arguments from `flags` to open initial file or folder.
        let Flags::Args(args) = flags;
        if let Some(path) = args.file {
            document::file::open_initial_path(&mut model, path);
        }

        // Initialize empty nav bar (for folder/thumbnail navigation later).
        let nav = nav_bar::Model::default();

        // Context drawer hidden by default.
        core.window.show_context = false;

        (
            Self {
                core,
                model,
                nav,
                context_page: ContextPage::default(),
            },
            Task::none(),
        )
    }

    fn on_close_requested(&self, _id: window::Id) -> Option<Self::Message> {
        None
    }

    fn update(&mut self, message: Self::Message) -> Task<Action<Self::Message>> {
        // Handle panel toggle messages.
        if let AppMessage::ToggleContextPage(page) = &message {
            if self.context_page == *page {
                self.core.window.show_context = !self.core.window.show_context;
            } else {
                self.context_page = *page;
                self.core.window.show_context = true;
            }
            return Task::none();
        }

        update::update(&mut self.model, message);
        Task::none()
    }

    fn view(&self) -> Element<Self::Message> {
        view::view(&self.model)
    }

    fn view_window(&self, _id: window::Id) -> Element<Self::Message> {
        self.view()
    }

    /// Header end items (right side of header bar).
    fn header_end(&self) -> Vec<Element<Self::Message>> {
        vec![
            // Properties panel toggle button.
            button::icon(icon::from_name("document-properties-symbolic"))
                .on_press(AppMessage::ToggleContextPage(ContextPage::Properties))
                .into(),
        ]
    }

    /// Right-side context drawer (properties panel).
    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(context_drawer::context_drawer(
            view::panels::properties_panel(&self.model),
            AppMessage::ToggleContextPage(ContextPage::Properties),
        ))
    }

    /// Nav bar model for left panel.
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    /// Footer with zoom controls and document info.
    fn footer(&self) -> Option<Element<Self::Message>> {
        Some(view::footer::view(&self.model))
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

        // Toggle properties panel with 'i' for info.
        Key::Character(ch) if ch.eq_ignore_ascii_case("i") => {
            Some(ToggleContextPage(ContextPage::Properties))
        }

        _ => None,
    }
}
