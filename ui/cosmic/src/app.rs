// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/app.rs
//
// Browser mode UI: tab strip, nav bar with thumbnails, content view.
// The UI is a remote control for the core: all pdfium work runs on the
// render worker, all file I/O goes through noctua-core.

use std::collections::HashMap;
use std::path::PathBuf;

use cosmic::app::context_drawer;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::keyboard::Key;
use cosmic::iced::{Alignment, ContentFit, Length};
use cosmic::widget::about::About;
use cosmic::widget::menu::key_bind::Modifier;
use cosmic::widget::segmented_button::{Entity, SingleSelectModel};
use cosmic::widget::{self, icon, menu, nav_bar, tab_bar};
use cosmic::{iced, prelude::*};
use noctua_core::render::worker::{Job, JobResult, Priority, SharedWorker};
use noctua_core::session::Session;
use noctua_core::storage;
use noctua_core::storage::browser::BrowserEntry;
use noctua_core::storage::thumbcache::ThumbSize;

use crate::fl;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

/// Session file used for automatic save and restore.
const SESSION_NAME: &str = "default";

/// How many nav entries receive thumbnails up front; the rest stay
/// text-only until the nav bar becomes viewport-aware. Kept small:
/// thumbnails decode full images, which is expensive even in the
/// background.
const INITIAL_THUMBS: usize = 8;

/// Zoom factor applied per zoom step.
const ZOOM_STEP: f32 = 1.25;

/// Render scale for page thumbnails in the document tab nav bar.
const PAGE_THUMB_ZOOM: f32 = 0.2;

/// RGBA pixels: (width, height, data).
type Rgba = (u32, u32, Vec<u8>);

/// A thumbnail result for one nav entry.
type ThumbResult = (nav_bar::Id, Option<Rgba>);

/// Content of one browser tab.
enum TabContent {
    /// A folder listing.
    Folder {
        path: PathBuf,
        entries: Vec<BrowserEntry>,
    },
    /// A multi-page document (PDF) the user dove into. `pages` is 0
    /// until the worker has counted them.
    Document { path: PathBuf, pages: u32 },
}

impl TabContent {
    fn path(&self) -> &PathBuf {
        match self {
            TabContent::Folder { path, .. } => path,
            TabContent::Document { path, .. } => path,
        }
    }
}

/// What a nav bar entry points to.
#[derive(Debug, Clone)]
enum NavEntry {
    /// A file in a folder tab. PDFs dive into a new tab when clicked.
    File { path: PathBuf },
    /// A page of the document tab (1-based).
    Page { path: PathBuf, page: u32 },
}

/// What kind of thumbnail a nav entry needs.
enum ThumbRequest {
    /// A file thumbnail (freedesktop cache; worker for PDFs).
    File { entity: nav_bar::Id, path: PathBuf },
    /// A page thumbnail of a document tab.
    Page {
        entity: nav_bar::Id,
        path: PathBuf,
        page: u32,
    },
}

/// The entry currently shown in the content area.
#[derive(Debug, Clone, PartialEq)]
enum CurrentTarget {
    File { path: PathBuf },
    Page { path: PathBuf, page: u32 },
}

/// A rendered image shown in the content area. The handle is created
/// once per image so the renderer reuses the uploaded texture instead
/// of re-uploading it on every frame.
struct CurrentImage {
    handle: widget::image::Handle,
}

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// The about page for this app.
    about: About,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    /// Tab strip model; tab contents are stored per entity.
    tab_model: SingleSelectModel,
    /// Contents of each open tab.
    tabs: HashMap<Entity, TabContent>,
    /// Nav bar entries of the active tab.
    nav_model: nav_bar::Model,
    /// Entry currently shown in the content area.
    current_target: Option<CurrentTarget>,
    /// Rendered content for the current target.
    current_image: Option<CurrentImage>,
    /// Current zoom factor (1.0 = 100%).
    zoom: f32,
    /// Whether the nav panel is visible.
    show_nav_panel: bool,
    /// Single pdfium worker; all pdfium jobs run on its thread.
    worker: SharedWorker,
    /// File size of the current target, for the status bar.
    current_size: Option<u64>,
    /// 1-based position of the active nav entry (status bar).
    current_position: Option<usize>,
    /// When a start argument is a file, its nav entry is selected
    /// after the parent folder has been listed.
    pending_select: Option<PathBuf>,
    /// Path of a start argument that could not be opened; shown in the
    /// empty view instead of the folder hint.
    start_error: Option<String>,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    /// A tab in the tab strip was activated.
    TabActivated(Entity),
    /// The close button of a tab was pressed.
    TabCloseRequested(Entity),
    /// Close the active tab.
    CloseTab,
    /// An entry in the nav bar was activated.
    NavActivated(nav_bar::Id),
    /// Move to the previous nav entry.
    PrevEntry,
    /// Move to the next nav entry.
    NextEntry,
    /// The Open Folder menu entry was activated.
    OpenFolder,
    /// The folder dialog returned a result.
    FolderChosen(Option<PathBuf>),
    /// The folder content was listed.
    FolderListed {
        dir: PathBuf,
        result: Result<Vec<BrowserEntry>, String>,
    },
    /// The worker counted the pages of a document tab.
    PagesKnown {
        tab: Entity,
        pages: Option<u32>,
    },
    /// Thumbnails for nav entries were generated.
    ThumbsReady(Vec<ThumbResult>),
    /// A raster or SVG file was rendered for the content area.
    FileRendered {
        path: PathBuf,
        zoom: f32,
        rgba: Option<(u32, u32, Vec<u8>)>,
    },
    /// A PDF page was rendered for the content area.
    PageRendered {
        path: PathBuf,
        page: u32,
        zoom: f32,
        rgba: Option<(u32, u32, Vec<u8>)>,
    },
    ZoomIn,
    ZoomOut,
    Zoom100,
    ToggleNavPanel,
    ToggleAbout,
    LaunchUrl(String),
    Quit,
}

/// Actions offered by the menu bar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    OpenFolder,
    CloseTab,
    ZoomIn,
    ZoomOut,
    Zoom100,
    ToggleNavPanel,
    About,
    Quit,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::OpenFolder => Message::OpenFolder,
            MenuAction::CloseTab => Message::CloseTab,
            MenuAction::ZoomIn => Message::ZoomIn,
            MenuAction::ZoomOut => Message::ZoomOut,
            MenuAction::Zoom100 => Message::Zoom100,
            MenuAction::ToggleNavPanel => Message::ToggleNavPanel,
            MenuAction::About => Message::ToggleAbout,
            MenuAction::Quit => Message::Quit,
        }
    }
}

impl AppModel {
    /// Register the menu key bindings.
    fn key_binds() -> HashMap<menu::KeyBind, MenuAction> {
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
    fn active_tab(&self) -> Option<Entity> {
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
                noctua_core::render::render_path(&render_path, zoom)
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
    fn open_start_path(&mut self, path: PathBuf) -> iced::Task<cosmic::Action<Message>> {
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
    fn restore_session(&mut self, session: Session) -> iced::Task<cosmic::Action<Message>> {
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
                eprintln!("restore: skipping non-document path {path:?}");
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
    fn save_session(&mut self) {
        let session = self.build_session();
        if let Err(e) = storage::session::save(SESSION_NAME, &session) {
            eprintln!("failed to save session: {e}");
        }
        if let Err(e) = storage::session::set_last(SESSION_NAME) {
            eprintln!("failed to mark last session: {e}");
        }
    }

    /// Update the window title from the active tab.
    fn update_title(&mut self) -> iced::Task<cosmic::Action<Message>> {
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

    /// Status bar text describing the current nav position.
    fn position_label(&self) -> String {
        let (Some(position), Some(active)) = (self.current_position, self.active_tab()) else {
            return String::new();
        };
        let total = self.nav_model.len();

        match self.tabs.get(&active) {
            Some(TabContent::Document { .. }) => {
                format!("{} / {total}", fl!("page-num", num = position))
            }
            _ => format!("{position} / {total}"),
        }
    }

    /// The content area view.
    fn content_view(&self) -> Element<'_, Message> {
        let space = cosmic::theme::spacing();

        if self.tabs.is_empty() {
            return widget::container(
                widget::text::body(match &self.start_error {
                    Some(path) => fl!("start-error", path = path),
                    None => fl!("open-folder-hint"),
                })
                .size(18)
                .apply(widget::container)
                .width(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        }

        match &self.current_image {
            Some(image) => {
                // Known upstream issue: the iced image atlas splits images
                // larger than its 2048 px layers into fragments and its
                // upload bounds check drops the bottom-right fragment, so
                // such images render with a missing block. Deliberately
                // not worked around — fix belongs in iced/libcosmic.
                if (self.zoom - 1.0).abs() < f32::EPSILON {
                    // Fit view: scale down images larger than the viewport,
                    // keep smaller ones at their native size, never crop.
                    widget::container(
                        widget::Image::new(image.handle.clone()).content_fit(ContentFit::ScaleDown),
                    )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(space.space_m)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
                    .into()
                } else {
                    // Zoomed view: show native pixels and scroll.
                    widget::scrollable(
                        widget::container(widget::Image::new(image.handle.clone()))
                            .padding(space.space_m),
                    )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
                }
            }
            None => widget::container(
                widget::text::body(fl!("select-hint"))
                    .apply(widget::container)
                    .width(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        }
    }

    /// The status bar view.
    fn status_bar(&self) -> Element<'_, Message> {
        let space = cosmic::theme::spacing();

        let mut row = widget::row::with_capacity(8)
            .spacing(space.space_m)
            .align_y(Alignment::Center);

        row = row
            .push(
                widget::button::icon(icon::from_name("go-previous-symbolic"))
                    .on_press(Message::PrevEntry),
            )
            .push(
                widget::button::icon(icon::from_name("go-next-symbolic"))
                    .on_press(Message::NextEntry),
            );

        let position = self.position_label();
        if !position.is_empty() {
            row = row.push(widget::text::body(position));
        }

        row = row.push(widget::text::body(format!("{:.0}%", self.zoom * 100.0)));

        if let Some(target) = &self.current_target {
            let path = match target {
                CurrentTarget::File { path } => path,
                CurrentTarget::Page { path, .. } => path,
            };
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                row = row.push(widget::text::body(name.to_string()));
            }
        }

        if let Some(size) = self.current_size {
            row = row.push(widget::text::body(storage::document::format_size(size)));
        }

        widget::container(row)
            .width(Length::Fill)
            .padding([
                space.space_xxs,
                space.space_s,
                space.space_xxs,
                space.space_s,
            ])
            .into()
    }

    fn apply_image(&mut self, rgba: Option<(u32, u32, Vec<u8>)>) {
        self.current_image = rgba.map(|(width, height, rgba)| CurrentImage {
            handle: widget::image::Handle::from_rgba(width, height, rgba),
        });
    }
}

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
            nav_model: nav_bar::Model::default(),
            current_target: None,
            current_image: None,
            zoom: 1.0,
            show_nav_panel: true,
            worker: SharedWorker::spawn(),
            current_size: None,
            current_position: None,
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
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<'_, Self::Message> {
        let space = cosmic::theme::spacing();

        let tab_strip = tab_bar::horizontal(&self.tab_model)
            .on_activate(Message::TabActivated)
            .on_close(Message::TabCloseRequested);

        let mut column = widget::column::with_capacity(3).spacing(space.space_xxs);
        column = column.push(tab_strip);

        if self.tabs.is_empty() {
            column = column.push(self.content_view());
        } else {
            let mut row = widget::row::with_capacity(2).spacing(space.space_xxs);
            if self.show_nav_panel {
                row = row.push(
                    nav_bar::nav_bar(&self.nav_model, Message::NavActivated)
                        .into_container()
                        .width(Length::Fixed(220.0))
                        .height(Length::Fill),
                );
            }
            row = row.push(self.content_view());
            column = column.push(row.height(Length::Fill));
        }

        column = column.push(self.status_bar());

        widget::container(column)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> iced::Task<cosmic::Action<Self::Message>> {
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
                        eprintln!("failed to list folder: {e}");
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
                    eprintln!("failed to count pages of document tab");
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
                    self.apply_image(rgba);
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
                    self.apply_image(rgba);
                }
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
                    eprintln!("failed to open {url:?}: {err}");
                }
            },

            Message::Quit => {
                self.save_session();
                std::process::exit(0);
            }
        }
        iced::Task::none()
    }

    /// Save the session when the app exits.
    fn on_app_exit(&mut self) -> Option<Self::Message> {
        self.save_session();
        None
    }
}
