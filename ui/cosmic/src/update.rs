// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/update.rs
//
// Update logic: message handling and all state changes of the UI model.
// Produces tasks; never builds widgets and never touches the filesystem
// directly (all I/O goes through noctua-core).

use std::collections::HashMap;
use std::path::PathBuf;

use cosmic::iced::keyboard::Key;
use cosmic::widget::icon;
use cosmic::widget::menu;
use cosmic::widget::menu::key_bind::Modifier;
use cosmic::widget::nav_bar;
use cosmic::widget::segmented_button::Entity;
use cosmic::{iced, prelude::*};

use noctua_core::render;
use noctua_core::render::worker::{Job, JobResult, Priority};
use noctua_core::session::Session;
use noctua_core::storage;
use noctua_core::storage::thumbcache::ThumbSize;

use crate::fl;
use crate::message::{MenuAction, Message};
use crate::model::{
    AppModel, CurrentTarget, CurrentImage, NavEntry, TabContent, ThumbRequest, ThumbResult,
    INITIAL_THUMBS, PAGE_THUMB_ZOOM, SESSION_NAME, ZOOM_STEP,
};

impl AppModel {
    /// Register the menu key bindings.
    pub(crate) fn key_binds() -> HashMap<menu::KeyBind, MenuAction> {
        let bind = |modifiers: &[Modifier], key: Key, action: MenuAction| {
            (
                menu::KeyBind {
                    modifiers: modifiers.to_vec(),
                    key,
                },
                action,
            )
        };

        HashMap::from([
            bind(
                &[Modifier::Ctrl],
                Key::Character("o".into()),
                MenuAction::OpenFolder,
            ),
            bind(
                &[Modifier::Ctrl],
                Key::Character("w".into()),
                MenuAction::CloseTab,
            ),
            bind(
                &[Modifier::Ctrl],
                Key::Character("=".into()),
                MenuAction::ZoomIn,
            ),
            bind(
                &[Modifier::Ctrl],
                Key::Character("-".into()),
                MenuAction::ZoomOut,
            ),
            bind(
                &[Modifier::Ctrl],
                Key::Character("1".into()),
                MenuAction::Zoom100,
            ),
            bind(
                &[Modifier::Ctrl],
                Key::Character("b".into()),
                MenuAction::ToggleNavPanel,
            ),
            bind(
                &[Modifier::Ctrl],
                Key::Character("q".into()),
                MenuAction::Quit,
            ),
        ])
    }

    /// The currently active tab entity, if any tab is open.
    pub(crate) fn active_tab(&self) -> Option<Entity> {
        let entity = self.tab_model.active();
        self.tabs.contains_key(&entity).then_some(entity)
    }

    /// Rebuild the nav bar for the active tab and request thumbnails.
    /// Returns the thumbnail tasks to run in the background.
    fn rebuild_nav(&mut self) -> Vec<iced::Task<cosmic::Action<Message>>> {
        let mut nav = nav_bar::Model::default();
        let mut thumb_requests: Vec<ThumbRequest> = Vec::new();

        if let Some(entity) = self.active_tab() {
            match &self.tabs[&entity] {
                TabContent::Folder { entries, .. } => {
                    for (index, entry) in entries.iter().enumerate() {
                        let target = NavEntry::File {
                            path: entry.path.clone(),
                        };
                        let id = nav.insert().text(entry.name.clone()).data(target).id();
                        if index < INITIAL_THUMBS {
                            thumb_requests.push(ThumbRequest::File {
                                entity: id,
                                path: entry.path.clone(),
                            });
                        }
                    }
                }
                TabContent::Document { path, pages } => {
                    for page in 1..=*pages {
                        let target = NavEntry::Page {
                            path: path.clone(),
                            page,
                        };
                        let id = nav
                            .insert()
                            .text(fl!("page-num", num = page))
                            .data(target)
                            .id();
                        if (page as usize) <= INITIAL_THUMBS {
                            thumb_requests.push(ThumbRequest::Page {
                                entity: id,
                                path: path.clone(),
                                page,
                            });
                        }
                    }
                }
            }
        }

        self.nav_model = nav;

        if thumb_requests.is_empty() {
            Vec::new()
        } else {
            vec![self.thumbs_task(thumb_requests)]
        }
    }

    /// Select the first usable nav entry and render it. PDFs are skipped:
    /// diving in is an explicit user action.
    fn activate_tab(&mut self) -> iced::Task<cosmic::Action<Message>> {
        self.current_target = None;
        self.current_image = None;
        self.current_size = None;
        self.current_position = None;

        let mut tasks = self.rebuild_nav();

        if let Some(first) = self.nav_model.entity_at(0) {
            self.nav_model.activate(first);
            self.current_position = Some(1);
            let entry = self.nav_model.data::<NavEntry>(first).cloned();
            let dives = matches!(
                &entry,
                Some(NavEntry::File { path }) if storage::document::is_pdf(path).unwrap_or(false)
            );
            if !dives && let Some(entry) = entry {
                tasks.push(self.activate_target(entry));
            }
        }

        if tasks.is_empty() {
            iced::Task::none()
        } else {
            cosmic::task::batch(tasks)
        }
    }

    /// Show or render the given nav entry.
    fn activate_target(&mut self, entry: NavEntry) -> iced::Task<cosmic::Action<Message>> {
        match entry {
            NavEntry::File { path } => {
                if storage::document::is_pdf(&path).unwrap_or(false) {
                    return self.dive_into(path);
                }
                self.current_target = Some(CurrentTarget::File { path: path.clone() });
                self.current_size = storage::document::metadata(&path)
                    .ok()
                    .map(|meta| meta.size_bytes);
                self.render_file_task(path)
            }
            NavEntry::Page { path, page } => {
                self.current_target = Some(CurrentTarget::Page {
                    path: path.clone(),
                    page,
                });
                self.current_size = storage::document::metadata(&path)
                    .ok()
                    .map(|meta| meta.size_bytes);
                self.render_page_task(path, page)
            }
        }
    }

    /// Open a document tab for the given PDF and count its pages on the worker.
    fn dive_into(&mut self, path: PathBuf) -> iced::Task<cosmic::Action<Message>> {
        let title = storage::browser::display_name(&path);
        let tab = self.tab_model.insert().text(title).closable().id();
        self.tabs.insert(
            tab,
            TabContent::Document {
                path: path.clone(),
                pages: 0,
            },
        );
        self.tab_model.activate(tab);
        let activate = self.activate_tab();

        let worker = self.worker.clone();
        let count = cosmic::task::future(async move {
            let pages = tokio::task::spawn_blocking(move || {
                match worker.execute(Priority::VisiblePage, Job::FilePageCount { path }) {
                    JobResult::PageCount(count) => Some(count),
                    _ => None,
                }
            })
            .await
            .ok()
            .flatten();
            Message::PagesKnown { tab, pages }
        });

        cosmic::task::batch(vec![activate, count])
    }

    /// Generate all requested thumbnails serially on a background thread
    /// and return them as one message. Page thumbnails are grouped per
    /// PDF so the worker opens each document only once.
    fn thumbs_task(&self, requests: Vec<ThumbRequest>) -> iced::Task<cosmic::Action<Message>> {
        let worker = self.worker.clone();
        cosmic::task::future(async move {
            let thumbs = tokio::task::spawn_blocking(move || {
                let mut results: Vec<ThumbResult> = Vec::new();
                let mut pages_by_path: Vec<(PathBuf, Vec<(nav_bar::Id, u32)>)> = Vec::new();

                for request in requests {
                    match request {
                        ThumbRequest::File { entity, path } => {
                            let thumb = worker.thumbnail(&path, ThumbSize::Normal);
                            results.push((entity, thumb));
                        }
                        ThumbRequest::Page { entity, path, page } => {
                            match pages_by_path.iter_mut().find(|(p, _)| p == &path) {
                                Some((_, pages)) => pages.push((entity, page)),
                                None => pages_by_path.push((path, vec![(entity, page)])),
                            }
                        }
                    }
                }

                for (path, pages) in pages_by_path {
                    let page_numbers: Vec<u32> = pages.iter().map(|(_, page)| *page).collect();
                    match worker.page_thumbs(&path, &page_numbers, PAGE_THUMB_ZOOM) {
                        Some(thumbs) => {
                            for (entity, page) in pages {
                                let rgba = thumbs
                                    .iter()
                                    .find(|(p, ..)| *p == page)
                                    .map(|(_, w, h, data)| (*w, *h, data.clone()));
                                results.push((entity, rgba));
                            }
                        }
                        None => {
                            for (entity, _) in pages {
                                results.push((entity, None));
                            }
                        }
                    }
                }

                results
            })
            .await
            .ok()
            .unwrap_or_default();
            Message::ThumbsReady(thumbs)
        })
    }

    /// Render a raster or SVG file into the content area.
    fn render_file_task(&self, path: PathBuf) -> iced::Task<cosmic::Action<Message>> {
        let zoom = self.zoom;
        let render_path = path.clone();
        cosmic::task::future(async move {
            let rgba = tokio::task::spawn_blocking(move || {
                render::render_path(&render_path, zoom)
                    .ok()
                    .map(|rendered| (rendered.width, rendered.height, rendered.rgba_data))
            })
            .await
            .ok()
            .flatten();
            Message::FileRendered { path, zoom, rgba }
        })
    }

    /// Render a PDF page into the content area via the worker.
    fn render_page_task(&self, path: PathBuf, page: u32) -> iced::Task<cosmic::Action<Message>> {
        let worker = self.worker.clone();
        let zoom = self.zoom;
        let render_path = path.clone();
        cosmic::task::future(async move {
            let rgba =
                tokio::task::spawn_blocking(move || worker.render_page(&render_path, page, zoom))
                    .await
                    .ok()
                    .flatten();
            Message::PageRendered {
                path,
                page,
                zoom,
                rgba,
            }
        })
    }

    /// Re-render the current target at the current zoom.
    fn render_current(&self) -> iced::Task<cosmic::Action<Message>> {
        match &self.current_target {
            Some(CurrentTarget::File { path }) => self.render_file_task(path.clone()),
            Some(CurrentTarget::Page { path, page }) => self.render_page_task(path.clone(), *page),
            None => iced::Task::none(),
        }
    }

    /// Emit [`Message::RepaintTick`] after a short delay so the image just
    /// applied gets a follow-up frame (see the variant's doc comment).
    fn repaint_task() -> iced::Task<cosmic::Action<Message>> {
        cosmic::task::future(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            Message::RepaintTick
        })
    }

    /// Open the system folder dialog.
    fn open_folder_dialog(&self) -> iced::Task<cosmic::Action<Message>> {
        cosmic::task::future(async move {
            let path = cosmic::dialog::file_chooser::open::Dialog::new()
                .open_folder()
                .await
                .ok()
                .and_then(|response| response.url().to_file_path().ok());
            Message::FolderChosen(path)
        })
    }

    /// Start listing a folder and open a tab for it.
    fn open_folder(&mut self, dir: PathBuf) -> iced::Task<cosmic::Action<Message>> {
        let title = storage::browser::display_name(&dir);
        let tab = self.tab_model.insert().text(title).closable().id();
        self.tabs.insert(
            tab,
            TabContent::Folder {
                path: dir.clone(),
                entries: Vec::new(),
            },
        );
        self.tab_model.activate(tab);
        let activate = self.activate_tab();

        let list_dir = dir.clone();
        let list = cosmic::task::future(async move {
            let result = match tokio::task::spawn_blocking(move || {
                storage::browser::list_documents(&list_dir)
            })
            .await
            {
                Ok(Ok(entries)) => Ok(entries),
                Ok(Err(e)) => Err(e.to_string()),
                Err(e) => Err(e.to_string()),
            };
            Message::FolderListed { dir, result }
        });

        cosmic::task::batch(vec![activate, list])
    }

    /// Open a tab for a start path: folders list, PDFs dive, other files
    /// open their parent folder with the file pre-selected.
    pub(crate) fn open_start_path(&mut self, path: PathBuf) -> iced::Task<cosmic::Action<Message>> {
        // A missing path is a user error; surface it instead of opening
        // the parent folder silently.
        if !path.exists() {
            self.start_error = Some(path.display().to_string());
            return iced::Task::none();
        }

        if path.is_dir() {
            return self.open_folder(path);
        }

        if storage::document::is_pdf(&path).unwrap_or(false) {
            return self.dive_into(path);
        }

        match path.parent().map(PathBuf::from) {
            Some(parent) if !parent.as_os_str().is_empty() => {
                self.pending_select = Some(path);
                return self.open_folder(parent);
            }
            _ => {
                self.start_error = Some(path.display().to_string());
            }
        }

        iced::Task::none()
    }

    /// Open the tabs of a restored session.
    pub(crate) fn restore_session(&mut self, session: Session) -> iced::Task<cosmic::Action<Message>> {
        let mut tasks: Vec<iced::Task<cosmic::Action<Message>>> = Vec::new();

        for (index, path) in session.browser_tabs.into_iter().enumerate() {
            if path.is_dir() {
                let title = storage::browser::display_name(&path);
                let tab = self.tab_model.insert().text(title).closable().id();
                self.tabs.insert(
                    tab,
                    TabContent::Folder {
                        path: path.clone(),
                        entries: Vec::new(),
                    },
                );
                let dir = path.clone();
                let list_dir = dir.clone();
                tasks.push(cosmic::task::future(async move {
                    let result = match tokio::task::spawn_blocking(move || {
                        storage::browser::list_documents(&list_dir)
                    })
                    .await
                    {
                        Ok(Ok(entries)) => Ok(entries),
                        Ok(Err(e)) => Err(e.to_string()),
                        Err(e) => Err(e.to_string()),
                    };
                    Message::FolderListed { dir, result }
                }));
            } else if storage::document::is_pdf(&path).unwrap_or(false) {
                let title = storage::browser::display_name(&path);
                let tab = self.tab_model.insert().text(title).closable().id();
                self.tabs.insert(
                    tab,
                    TabContent::Document {
                        path: path.clone(),
                        pages: 0,
                    },
                );
                let worker = self.worker.clone();
                let dir = path.clone();
                tasks.push(cosmic::task::future(async move {
                    let pages = tokio::task::spawn_blocking(move || {
                        match worker.execute(Priority::Low, Job::FilePageCount { path: dir }) {
                            JobResult::PageCount(count) => Some(count),
                            _ => None,
                        }
                    })
                    .await
                    .ok()
                    .flatten();
                    Message::PagesKnown { tab, pages }
                }));
            } else {
                // Browser tabs hold folders or PDFs; anything else is stale.
                tracing::warn!("restore: skipping non-document path {path:?}");
            }

            if index == session.active_tab {
                let entity = self.tab_model.entity_at(index as u16);
                if let Some(entity) = entity {
                    self.tab_model.activate(entity);
                }
            }
        }

        let command = self.activate_tab();
        if tasks.is_empty() {
            command
        } else {
            tasks.push(command);
            cosmic::task::batch(tasks)
        }
    }

    /// Find the nav entry for a file path and show it.
    fn select_nav_by_path(&mut self, path: &PathBuf) -> iced::Task<cosmic::Action<Message>> {
        let mut found: Option<(Entity, NavEntry)> = None;
        for position in 0..self.nav_model.len() {
            if let Some(entity) = self.nav_model.entity_at(position as u16)
                && let Some(entry) = self.nav_model.data::<NavEntry>(entity)
                && matches!(entry, NavEntry::File { path: p } if p == path)
            {
                found = Some((entity, entry.clone()));
                break;
            }
        }

        if let Some((entity, entry)) = found {
            self.nav_model.activate(entity);
            self.current_position = Some(
                (0..self.nav_model.len())
                    .find_map(|position| {
                        self.nav_model
                            .entity_at(position as u16)
                            .filter(|candidate| *candidate == entity)
                            .map(|_| position + 1)
                    })
                    .unwrap_or(1),
            );
            return self.activate_target(entry);
        }
        iced::Task::none()
    }

    /// Close a tab; activates a neighbor when the active tab was closed.
    fn close_tab(&mut self, tab: Entity) -> iced::Task<cosmic::Action<Message>> {
        let was_active = self.active_tab() == Some(tab);
        self.tab_model.remove(tab);
        self.tabs.remove(&tab);

        if !was_active {
            return iced::Task::none();
        }

        if let Some(first) = self.tab_model.entity_at(0) {
            self.tab_model.activate(first);
        }
        self.activate_tab()
    }

    /// Build the session from the open tabs, in tab order.
    fn build_session(&mut self) -> Session {
        let mut browser_tabs = Vec::new();
        let mut active_tab = 0;
        let active = self.tab_model.active();

        let mut index = 0usize;
        for position in 0..self.tab_model.len() {
            if let Some(entity) = self.tab_model.entity_at(position as u16)
                && let Some(content) = self.tabs.get(&entity)
            {
                if entity == active {
                    active_tab = index;
                }
                browser_tabs.push(content.path().clone());
                index += 1;
            }
        }

        Session {
            browser_tabs,
            annotation_tabs: Vec::new(),
            active_tab,
        }
    }

    /// Save the current session as the last one.
    pub(crate) fn save_session(&mut self) {
        let session = self.build_session();
        if let Err(e) = storage::session::save(SESSION_NAME, &session) {
            tracing::error!("failed to save session: {e}");
        }

        if let Err(e) = storage::session::set_last(SESSION_NAME) {
            tracing::error!("failed to mark last session: {e}");
        }
    }

    /// Update the window title from the active tab.
    pub(crate) fn update_title(&mut self) -> iced::Task<cosmic::Action<Message>> {
        let mut window_title = fl!("app-title");

        if let Some(entity) = self.active_tab()
            && let Some(content) = self.tabs.get(&entity)
        {
            let path = content.path();
            let name = storage::browser::display_name(path);
            window_title.push_str(" — ");
            window_title.push_str(&name);
        }

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            iced::Task::none()
        }
    }

    fn apply_image(&mut self, rgba: Option<(u32, u32, Vec<u8>)>) {
        self.current_image = rgba.map(|(width, height, rgba)| CurrentImage {
            handle: cosmic::widget::image::Handle::from_rgba(width, height, rgba),
        });
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    pub(crate) fn handle_message(
        &mut self,
        message: Message,
    ) -> iced::Task<cosmic::Action<Message>> {
        match message {
            Message::TabActivated(id) => {
                self.tab_model.activate(id);
                let command = self.activate_tab();
                let title = self.update_title();
                return cosmic::task::batch(vec![command, title]);
            }

            Message::TabCloseRequested(id) => {
                let command = self.close_tab(id);
                let title = self.update_title();
                return cosmic::task::batch(vec![command, title]);
            }

            Message::CloseTab => {
                if let Some(tab) = self.active_tab() {
                    let command = self.close_tab(tab);
                    let title = self.update_title();
                    return cosmic::task::batch(vec![command, title]);
                }
            }

            Message::NavActivated(id) => {
                self.nav_model.activate(id);
                for position in 0..self.nav_model.len() {
                    if let Some(entity) = self.nav_model.entity_at(position as u16)
                        && entity == id
                    {
                        self.current_position = Some(position + 1);
                        break;
                    }
                }
                if let Some(entry) = self.nav_model.data::<NavEntry>(id).cloned() {
                    return self.activate_target(entry);
                }
            }

            Message::PrevEntry => {
                let position = self.current_position.unwrap_or(0);
                if position >= 2
                    && let Some(prev) = self.nav_model.entity_at(position as u16 - 2)
                {
                    self.nav_model.activate(prev);
                    self.current_position = Some(position - 1);
                    if let Some(entry) = self.nav_model.data::<NavEntry>(prev).cloned() {
                        return self.activate_target(entry);
                    }
                }
            }

            Message::NextEntry => {
                let position = self.current_position.unwrap_or(0);
                if position >= 1
                    && let Some(next) = self.nav_model.entity_at(position as u16)
                {
                    self.nav_model.activate(next);
                    self.current_position = Some(position + 1);
                    if let Some(entry) = self.nav_model.data::<NavEntry>(next).cloned() {
                        return self.activate_target(entry);
                    }
                }
            }

            Message::OpenFolder => {
                return self.open_folder_dialog();
            }

            Message::FolderChosen(path) => {
                if let Some(dir) = path {
                    return self.open_folder(dir);
                }
            }

            Message::FolderListed { dir, result } => {
                let tab = self.tabs.iter().find_map(|(tab, content)| {
                    matches!(content, TabContent::Folder { path, .. } if path == &dir)
                        .then_some(*tab)
                });

                match (tab, result) {
                    (Some(tab), Ok(entries)) => {
                        self.tabs
                            .insert(tab, TabContent::Folder { path: dir, entries });
                        let active = self.active_tab() == Some(tab);
                        if active {
                            let pending = self.pending_select.take();
                            let command = self.activate_tab();
                            let select = pending
                                .map(|path| self.select_nav_by_path(&path))
                                .unwrap_or_else(iced::Task::none);
                            return cosmic::task::batch(vec![command, select]);
                        }
                    }
                    (Some(_), Err(e)) => {
                        tracing::error!("failed to list folder: {e}");
                    }
                    (None, _) => {}
                }
            }

            Message::PagesKnown { tab, pages } => match (self.tabs.get(&tab), pages) {
                (Some(TabContent::Document { path, .. }), Some(pages)) => {
                    self.tabs.insert(
                        tab,
                        TabContent::Document {
                            path: path.clone(),
                            pages,
                        },
                    );
                    if self.active_tab() == Some(tab) {
                        return self.activate_tab();
                    }
                }
                (_, None) => {
                    tracing::error!("failed to count pages of document tab");
                }
                _ => {}
            },

            Message::ThumbsReady(thumbs) => {
                for (entity, rgba) in thumbs {
                    if let Some((width, height, rgba)) = rgba {
                        self.nav_model
                            .icon_set(entity, icon::from_raster_pixels(width, height, rgba).icon());
                    }
                }
            }

            Message::FileRendered { path, zoom, rgba } => {
                // Ignore renders that raced with a zoom change.
                if self.current_target == Some(CurrentTarget::File { path })
                    && (self.zoom - zoom).abs() < f32::EPSILON
                {
                    let has_pixels = rgba.is_some();
                    self.apply_image(rgba);
                    if has_pixels {
                        return Self::repaint_task();
                    }
                }
            }

            Message::PageRendered {
                path,
                page,
                zoom,
                rgba,
            } => {
                // Ignore renders that raced with a zoom change.
                if self.current_target == Some(CurrentTarget::Page { path, page })
                    && (self.zoom - zoom).abs() < f32::EPSILON
                {
                    let has_pixels = rgba.is_some();
                    self.apply_image(rgba);
                    if has_pixels {
                        return Self::repaint_task();
                    }
                }
            }

            Message::RepaintTick => {
                // The frame triggered by this message picks up the texture
                // uploaded by iced's image worker. State is already current.
            }

            Message::ZoomIn => {
                self.zoom = (self.zoom * ZOOM_STEP).min(8.0);
                return self.render_current();
            }

            Message::ZoomOut => {
                self.zoom = (self.zoom / ZOOM_STEP).max(0.05);
                return self.render_current();
            }

            Message::Zoom100 => {
                self.zoom = 1.0;
                return self.render_current();
            }

            Message::ToggleNavPanel => {
                self.show_nav_panel = !self.show_nav_panel;
            }

            Message::ToggleAbout => {
                self.core.window.show_context = !self.core.window.show_context;
            }

            Message::LaunchUrl(url) => match open::that_detached(&url) {
                Ok(()) => {}
                Err(err) => {
                    tracing::error!("failed to open {url:?}: {err}");
                }
            },

            Message::Quit => {
                self.save_session();
                std::process::exit(0);
            }
        }
        iced::Task::none()
    }
}
