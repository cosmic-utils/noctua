// SPDX-License-Identifier: GPL-3.0-or-later
// src/ui/app/app.rs
//
// COSMIC application wiring and main app struct.

use super::message::AppMessage;
use super::model::AppModel;
use super::update;
use crate::ui::views;

use std::time::Duration;

use cosmic::app::{context_drawer, Core};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::keyboard::{self, key::Named, Key, Modifiers};
use cosmic::iced::time;
use cosmic::iced::window;
use cosmic::iced::Subscription;
use cosmic::widget::nav_bar;
use cosmic::{Action, Element, Task};

use crate::application::DocumentManager;
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
pub struct NoctuaApp {
    core: Core,
    pub model: AppModel,
    nav: nav_bar::Model,
    context_page: ContextPage,
    pub config: AppConfig,
    config_handler: Option<cosmic_config::Config>,
    pub document_manager: DocumentManager,
}

impl cosmic::Application for NoctuaApp {
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
        // Load persisted config.
        let (config, config_handler) =
            match cosmic_config::Config::new(Self::APP_ID, AppConfig::VERSION) {
                Ok(handler) => {
                    let config = AppConfig::get_entry(&handler).unwrap_or_default();
                    (config, Some(handler))
                }
                Err(_) => (AppConfig::default(), None),
            };

        let mut model = AppModel::new(config.clone());

        let Flags::Args(args) = flags;

        // Determine initial path: CLI argument takes priority.
        // Fall back to configured default directory only if it exists.
        let initial_path = args.file.or_else(|| {
            config
                .default_image_dir
                .as_ref()
                .filter(|p| p.exists())
                .cloned()
        });

        // Initialize document manager
        let mut document_manager = DocumentManager::new();

        // Load initial document if provided
        if let Some(path) = initial_path {
            if let Err(e) = document_manager.open_document(&path) {
                log::error!("Failed to open initial path {}: {}", path.display(), e);
            }
        }

        // Initialize nav bar model (required for COSMIC to show toggle icon).
        let nav = nav_bar::Model::default();

        // Apply persisted panel states.
        core.window.show_context = config.context_drawer_visible;
        core.nav_bar_set_toggled(config.nav_bar_visible);

        // Start thumbnail generation for initial document if applicable.
        let init_task = start_thumbnail_generation(&model);

        (
            Self {
                core,
                model,
                nav,
                context_page: ContextPage::default(),
                config,
                config_handler,
                document_manager,
            },
            init_task,
        )
    }

    fn on_close_requested(&self, _id: window::Id) -> Option<Self::Message> {
        None
    }

    fn update(&mut self, message: Self::Message) -> Task<Action<Self::Message>> {
        match &message {
            AppMessage::ToggleNavBar => {
                use crate::ui::model::LeftPanel;

                self.core.nav_bar_toggle();
                let is_visible = self.core.nav_bar_active();
                self.config.nav_bar_visible = is_visible;
                self.save_config();

                if is_visible {
                    // Opening nav bar - show thumbnails for multi-page docs
                    if let Some(doc) = self.document_manager.current_document()
                        && doc.is_multi_page()
                    {
                        self.model.panels.left = Some(LeftPanel::Thumbnails);
                    }
                } else {
                    // Closing nav bar - hide left panel
                    self.model.panels.left = None;
                }
                return Task::none();
            }

            AppMessage::OpenFormatPanel => {
                // Format panel is now part of Transform mode
                // Switch to Transform mode which shows format tools in right panel
                self.model.mode = crate::ui::model::AppMode::Transform {
                    paper_format: None,
                    orientation: crate::ui::model::Orientation::default(),
                };

                return Task::none();
            }

            AppMessage::ToggleContextPage(page) => {
                if self.context_page == *page {
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    self.context_page = *page;
                    self.core.window.show_context = true;
                }
                self.config.context_drawer_visible = self.core.window.show_context;
                self.save_config();
                return Task::none();
            }

            AppMessage::OpenPath(_) | AppMessage::NextDocument | AppMessage::PrevDocument => {
                let result = update::update(self, &message);
                let thumb_task = start_thumbnail_generation_task(&self.model);
                return match result {
                    update::UpdateResult::None => thumb_task,
                    update::UpdateResult::Task(task) => Task::batch([task, thumb_task]),
                };
            }

            _ => {}
        }

        match update::update(self, &message) {
            update::UpdateResult::None => Task::none(),
            update::UpdateResult::Task(task) => task,
        }
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        views::header::start(&self.model, &self.document_manager)
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        views::header::end(&self.model, &self.document_manager)
    }

    fn view(&self) -> Element<'_, Self::Message> {
        views::view(&self.model, &self.document_manager, &self.config)
    }

    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<'_, Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }
        Some(context_drawer::context_drawer(
            views::panels::view(&self.model, &self.document_manager),
            AppMessage::ToggleContextPage(ContextPage::Properties),
        ))
    }

    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    fn nav_bar(&self) -> Option<Element<'_, Action<Self::Message>>> {
        if !self.core.nav_bar_active() {
            return None;
        }
        views::nav_bar(&self.model, &self.document_manager)
    }

    fn footer(&self) -> Option<Element<'_, Self::Message>> {
        Some(views::footer::view(&self.model, &self.document_manager))
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch([
            keyboard::on_key_press(handle_key_press),
            thumbnail_refresh_subscription(self),
        ])
    }
}

impl NoctuaApp {
    /// Save current config to disk.
    fn save_config(&self) {
        if let Some(ref handler) = self.config_handler {
            let _ = self.config.write_entry(handler);
        }
    }
}

/// Map raw key presses + modifiers into high-level application messages.
fn handle_key_press(key: Key, modifiers: Modifiers) -> Option<AppMessage> {
    use AppMessage::{
        PanLeft, PanRight, PanUp, PanDown, OpenFormatPanel, NextDocument, PrevDocument,
        FlipHorizontal, FlipVertical, RotateCCW, RotateCW, ZoomIn, ZoomOut, ZoomReset, ZoomFit,
        ToggleCropMode, ToggleScaleMode, PanReset, ToggleContextPage, ToggleNavBar, SetAsWallpaper,
    };

    // Handle Ctrl + arrow keys for panning.
    if modifiers.control() && !modifiers.shift() && !modifiers.alt() && !modifiers.logo() {
        return match key.as_ref() {
            Key::Named(Named::ArrowLeft) => Some(PanLeft),
            Key::Named(Named::ArrowRight) => Some(PanRight),
            Key::Named(Named::ArrowUp) => Some(PanUp),
            Key::Named(Named::ArrowDown) => Some(PanDown),
            Key::Character(ch) if ch.eq_ignore_ascii_case("f") => Some(OpenFormatPanel),
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
        Key::Character("+" | "=") => Some(ZoomIn),
        Key::Character("-") => Some(ZoomOut),
        Key::Character("1") => Some(ZoomReset),
        Key::Character(ch) if ch.eq_ignore_ascii_case("f") => Some(ZoomFit),

        // Tool modes.
        Key::Character(ch) if ch.eq_ignore_ascii_case("c") => Some(ToggleCropMode),
        Key::Character(ch) if ch.eq_ignore_ascii_case("s") => Some(ToggleScaleMode),

        // Crop mode actions (Enter/Escape handled via key press, validated in update).
        Key::Named(Named::Enter) => Some(AppMessage::ApplyCrop),
        Key::Named(Named::Escape) => Some(AppMessage::CancelCrop),

        // Reset pan.
        Key::Character("0") => Some(PanReset),

        // Toggle panels.
        Key::Character(ch) if ch.eq_ignore_ascii_case("i") => {
            Some(ToggleContextPage(ContextPage::Properties))
        }
        Key::Character(ch) if ch.eq_ignore_ascii_case("n") => Some(ToggleNavBar),

        // Wallpaper.
        Key::Character(ch) if ch.eq_ignore_ascii_case("w") => Some(SetAsWallpaper),

        _ => None,
    }
}

// =============================================================================
// Thumbnail Helpers
// =============================================================================

fn start_thumbnail_generation(model: &AppModel) -> Task<Action<AppMessage>> {
    start_thumbnail_generation_task(model)
}

fn start_thumbnail_generation_task(_model: &AppModel) -> Task<Action<AppMessage>> {
    // TODO: Re-enable when document is synced from DocumentManager
    // if let Some(doc) = &model.document {
    //     let page_count = doc.page_count();
    //     if page_count > 0 && !doc.thumbnails_ready() {
    //         return Task::batch([
    //             Task::done(Action::App(AppMessage::GenerateThumbnailPage(0))),
    //             Task::done(Action::App(AppMessage::RefreshView)),
    //         ]);
    //     }
    // }
    Task::none()
}

fn thumbnail_refresh_subscription(_app: &NoctuaApp) -> Subscription<AppMessage> {
    // TODO: Re-enable when document is synced from DocumentManager
    let needs_refresh = false;
    // let needs_refresh = app
    //     .model
    //     .document
    //     .as_ref()
    //     .is_some_and(|doc| doc.is_multi_page() && !doc.thumbnails_ready());

    if needs_refresh {
        time::every(Duration::from_millis(100)).map(|_| AppMessage::RefreshView)
    } else {
        Subscription::none()
    }
}
