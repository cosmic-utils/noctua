// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/app.rs
//
// Application shell: the cosmic::Application implementation. State lives in
// model.rs, message handling in update.rs and widgets in view.rs.

use std::collections::HashMap;

use cosmic::app::context_drawer;
use cosmic::iced::keyboard::key::Named;
use cosmic::widget::about::About;
use cosmic::widget::menu::action::MenuAction as _;
use cosmic::widget::segmented_button::SingleSelectModel;
use cosmic::widget::{self, menu};
use cosmic::{iced, prelude::*};

use noctua_core::render::worker::SharedWorker;
use noctua_core::storage;

use crate::fl;
use crate::message::{MenuAction, Message};
use crate::model::AppModel;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] =
    include_bytes!("../resources/icons/hicolor/scalable/apps/org.codeberg.wfx.Noctua.svg");

/// Create a COSMIC application from the app model.
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = crate::Args;

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "org.codeberg.wfx.Noctua";

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
            zoom_states: HashMap::new(),
            keyboard_modifiers: iced::keyboard::Modifiers::default(),
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
                        menu::Item::Button(fl!("zoom-fit"), None, MenuAction::ZoomToFit),
                        menu::Item::Button(fl!("zoom-100"), None, MenuAction::Zoom100),
                        menu::Item::Divider,
                        menu::Item::Button(fl!("fullscreen"), None, MenuAction::Fullscreen),
                        menu::Item::Button(fl!("show-nav-panel"), None, MenuAction::ToggleNavPanel),
                        menu::Item::Divider,
                        menu::Item::Button(fl!("about"), None, MenuAction::About),
                    ],
                ),
            ),
        ])
        .item_height(menu::ItemHeight::Dynamic(40))
        .item_width(menu::ItemWidth::Uniform(360));

        let nav = widget::row::with_capacity(2)
            .spacing(cosmic::theme::spacing().space_xxs)
            .push(
                widget::button::icon(widget::icon::from_name("go-previous-symbolic"))
                    .on_press(Message::PrevEntry),
            )
            .push(
                widget::button::icon(widget::icon::from_name("go-next-symbolic"))
                    .on_press(Message::NextEntry),
            );

        vec![menu_bar.into(), nav.into()]
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

    /// Maps the menu key binds to their actions. The menu bar only displays
    /// the shortcuts; this subscription is what actually triggers them.
    fn subscription(&self) -> iced::Subscription<Self::Message> {
        // A HashMap is not Hash, so pass the binds as a Vec for the
        // subscription state.
        let key_binds: Vec<_> = self.key_binds.clone().into_iter().collect();
        iced::keyboard::listen()
            .with(key_binds)
            .filter_map(|(key_binds, event)| match event {
                iced::keyboard::Event::ModifiersChanged(modifiers) => {
                    Some(Message::ModifiersChanged(modifiers))
                }
                iced::keyboard::Event::KeyPressed {
                    key,
                    modifiers,
                    physical_key,
                    ..
                } => {
                    if let Some((_, action)) = key_binds
                        .iter()
                        .find(|(bind, _)| bind.matches(modifiers, &key, Some(&physical_key)))
                    {
                        return Some(action.message());
                    }

                    match key {
                        iced::keyboard::Key::Named(Named::ArrowLeft) => Some(Message::PrevEntry),
                        iced::keyboard::Key::Named(Named::ArrowRight) => Some(Message::NextEntry),
                        iced::keyboard::Key::Named(Named::ArrowUp) => Some(Message::PrevPage),
                        iced::keyboard::Key::Named(Named::ArrowDown) => Some(Message::NextPage),
                        _ => None,
                    }
                }
                _ => None,
            })
    }

    /// Save the session when the app exits.
    fn on_app_exit(&mut self) -> Option<Self::Message> {
        self.save_session();
        None
    }
}
