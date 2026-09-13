// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/app.rs
//
// Application shell: the cosmic::Application implementation. State lives in
// model.rs, message handling in update.rs and widgets in view.rs.

use std::collections::HashMap;

use cosmic::app::context_drawer;
use cosmic::widget::about::About;
use cosmic::widget::segmented_button::SingleSelectModel;
use cosmic::widget::{self, menu};
use cosmic::{iced, prelude::*};

use noctua_core::render::worker::SharedWorker;
use noctua_core::storage;

use crate::fl;
use crate::message::{MenuAction, Message};
use crate::model::AppModel;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

/// Create a COSMIC application from the app model.
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = crate::Args;

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "io.codeberg.wfx.Noctua";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(core: cosmic::Core, args: Self::Flags) -> (Self, iced::Task<cosmic::Action<Message>>) {
        // Create the about widget
        let about = About::default()
            .name(fl!("app-title"))
            .icon(widget::icon::from_svg_bytes(APP_ICON))
            .version(env!("CARGO_PKG_VERSION"))
            .links([(fl!("repository"), REPOSITORY)])
            .license(env!("CARGO_PKG_LICENSE"));

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            about,
            key_binds: AppModel::key_binds(),
            tab_model: SingleSelectModel::default(),
            tabs: HashMap::new(),
            tab_ui: HashMap::new(),
            current_target: None,
            current_image: None,
            zoom: 1.0,
            show_nav_panel: true,
            worker: SharedWorker::spawn(),
            current_size: None,
            pending_select: None,
            start_error: None,
        };

        let mut command = iced::Task::none();

        // A start path wins over session restore.
        if let Some(path) = args.path {
            command = app.open_start_path(path);
        } else if let Ok(Some(name)) = storage::session::last()
            && let Ok(session) = storage::session::load(&name)
        {
            command = app.restore_session(session);
        }

        let title = app.update_title();
        command = cosmic::task::batch(vec![command, title]);
        (app, command)
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        let menu_bar = menu::bar(vec![
            menu::Tree::with_children(
                menu::root(fl!("file")).apply(Element::from),
                menu::items(
                    &self.key_binds,
                    vec![
                        menu::Item::Button(fl!("open-folder"), None, MenuAction::OpenFolder),
                        menu::Item::Button(fl!("close-tab"), None, MenuAction::CloseTab),
                        menu::Item::Divider,
                        menu::Item::Button(fl!("quit"), None, MenuAction::Quit),
                    ],
                ),
            ),
            menu::Tree::with_children(
                menu::root(fl!("view")).apply(Element::from),
                menu::items(
                    &self.key_binds,
                    vec![
                        menu::Item::Button(fl!("zoom-in"), None, MenuAction::ZoomIn),
                        menu::Item::Button(fl!("zoom-out"), None, MenuAction::ZoomOut),
                        menu::Item::Button(fl!("zoom-100"), None, MenuAction::Zoom100),
                        menu::Item::Divider,
                        menu::Item::Button(fl!("show-nav-panel"), None, MenuAction::ToggleNavPanel),
                        menu::Item::Divider,
                        menu::Item::Button(fl!("about"), None, MenuAction::About),
                    ],
                ),
            ),
        ]);

        vec![menu_bar.into()]
    }

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<'_, Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(context_drawer::about(
            &self.about,
            |url| Message::LaunchUrl(url.to_string()),
            Message::ToggleAbout,
        ))
    }

    /// Describes the interface based on the current state of the application model.
    fn view(&self) -> Element<'_, Self::Message> {
        crate::view::view(self)
    }

    /// Handles messages emitted by the application and its widgets.
    fn update(&mut self, message: Self::Message) -> iced::Task<cosmic::Action<Self::Message>> {
        self.handle_message(message)
    }

    /// Save the session when the app exits.
    fn on_app_exit(&mut self) -> Option<Self::Message> {
        self.save_session();
        None
    }
}
