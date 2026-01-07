// SPDX-License-Identifier: GPL-3.0-or-later
// src/app/mod.rs
//
// Application module root, re-exports, and COSMIC application wiring.

pub mod document;
pub mod message;
pub mod model;
pub mod update;

// UI is kept as an internal detail of this module.
mod view;

use std::fs;
use std::path::{Path, PathBuf};

use cosmic::app::Core;
use cosmic::iced::keyboard::{self, Key, Modifiers};
use cosmic::iced::keyboard::key::Named;
use cosmic::iced::window;
use cosmic::iced::Subscription;
use cosmic::{Action, Element, Task};

pub use message::AppMessage;
pub use model::AppModel;

use crate::config::AppConfig;
use crate::Args;

/// Flags passed from `main` into the application.
/// Currently we only forward the parsed CLI `Args`.
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
        // Load persistent configuration at startup.
        let config = AppConfig::default();

        // Create initial application model from configuration.
        let mut model = AppModel::new(config);

        // Use CLI arguments from `flags` to open initial file or folder.
        let Flags::Args(args) = flags;
        if let Some(path) = args.file {
            open_initial_path(&mut model, path);
        }

        (Self { core, model }, Task::none())
    }

    fn on_close_requested(&self, _id: window::Id) -> Option<Self::Message> {
        // Return a message here if you want to handle close requests in update().
        None
    }

    fn update(&mut self, message: Self::Message) -> Task<Action<Self::Message>> {
        // Delegate to the domain update logic.
        update::update(&mut self.model, message);
        Task::none()
    }

    fn view(&self) -> Element<Self::Message> {
        // Main application window view.
        view::view(&self.model)
    }

    fn view_window(&self, _id: window::Id) -> Element<Self::Message> {
        // For now, we only have a single window, so reuse the main view.
        self.view()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        // Global keyboard handler: maps key presses to AppMessage.
        keyboard::on_key_press(handle_key_press)
    }
}

/// Open the initial path passed on the command line.
///
/// If `path` is a directory, this will collect supported documents inside it,
/// open the first one, and initialize navigation state. If it is a file, the
/// file is opened directly and the surrounding folder is scanned.
fn open_initial_path(model: &mut AppModel, path: PathBuf) {
    if path.is_dir() {
        open_from_directory(model, &path);
    } else {
        open_single_file(model, &path);
    }
}

/// Open the first supported document from the given directory and
/// populate folder navigation state.
fn open_from_directory(model: &mut AppModel, dir: &Path) {
    let mut entries: Vec<PathBuf> = Vec::new();

    if let Ok(read_dir) = fs::read_dir(dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();

            // Only keep regular files that are recognized as supported documents.
            if path.is_file() && document::DocumentKind::from_path(&path).is_some() {
                entries.push(path);
            }
        }
    }

    entries.sort();

    let first = match entries.first().cloned() {
        Some(path) => path,
        None => {
            model.set_error(format!(
                "No supported documents found in directory: {}",
                dir.display()
            ));
            return;
        }
    };

    model.folder_entries = entries;
    model.current_index = Some(0);

    open_single_file(model, &first);
}

/// Open a single file, update current path and refresh folder entries.
fn open_single_file(model: &mut AppModel, path: &Path) {
    match document::file::open_document(path.to_path_buf()) {
        Ok(doc) => {
            model.document = Some(doc);
            model.current_path = Some(path.to_path_buf());
            model.clear_error();

            // Reset view state for new document.
            model.reset_pan();
            model.zoom = 1.0;
            model.view_mode = model::ViewMode::Fit;

            // Refresh folder listing based on parent directory.
            if let Some(parent) = path.parent() {
                refresh_folder_entries(model, parent, path);
            }
        }
        Err(err) => {
            model.document = None;
            model.current_path = None;
            model.set_error(err.to_string());
        }
    }
}

/// Refresh the `folder_entries` list and current index based on the
/// given folder and currently active file.
fn refresh_folder_entries(model: &mut AppModel, folder: &Path, current: &Path) {
    let mut entries: Vec<PathBuf> = Vec::new();

    if let Ok(read_dir) = fs::read_dir(folder) {
        for entry in read_dir.flatten() {
            let path = entry.path();

            // Only keep regular files that are recognized as supported documents.
            if path.is_file() && document::DocumentKind::from_path(&path).is_some() {
                entries.push(path);
            }
        }
    }

    entries.sort();

    // Determine current index.
    let current_index = entries.iter().position(|p| p == current);

    model.folder_entries = entries;
    model.current_index = current_index;
}

/// Map raw key presses + modifiers into high-level application messages.
///
/// This function is used by `keyboard::on_key_press` and must be a plain
/// function pointer (no captures).
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

    // Ignore key presses when other "command-style" modifiers are pressed,
    // so we do not conflict with system- / desktop-level shortcuts.
    if modifiers.command() || modifiers.alt() || modifiers.logo() || modifiers.control() {
        return None;
    }

    match key.as_ref() {
        // Navigation with arrow keys (no modifiers).
        Key::Named(Named::ArrowRight) => Some(NextDocument),
        Key::Named(Named::ArrowLeft) => Some(PrevDocument),

        // Character keys (case-insensitive where it makes sense).
        Key::Character(ch) if ch.eq_ignore_ascii_case("h") => Some(FlipHorizontal),
        Key::Character(ch) if ch.eq_ignore_ascii_case("v") => Some(FlipVertical),

        Key::Character(ch) if ch.eq_ignore_ascii_case("r") => {
            // "r" without Shift => RotateCW
            // "r" with Shift    => RotateCCW
            if modifiers.shift() {
                Some(RotateCCW)
            } else {
                Some(RotateCW)
            }
        }

        // Zoom
        Key::Character("+") | Key::Character("=") => Some(ZoomIn),
        Key::Character("-") => Some(ZoomOut),
        Key::Character("1") => Some(ZoomReset),
        Key::Character(ch) if ch.eq_ignore_ascii_case("f") => Some(ZoomFit),

        // Tool modes
        Key::Character(ch) if ch.eq_ignore_ascii_case("c") => Some(ToggleCropMode),
        Key::Character(ch) if ch.eq_ignore_ascii_case("s") => Some(ToggleScaleMode),

        // Reset pan with "0"
        Key::Character("0") => Some(PanReset),

        _ => None,
    }
}
