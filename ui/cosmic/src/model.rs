// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/model.rs
//
// UI state: pure data of the application model. No tasks, no widgets.

use std::collections::HashMap;
use std::path::PathBuf;

use cosmic::widget;
use cosmic::widget::about::About;
use cosmic::widget::menu;
use cosmic::widget::nav_bar;
use cosmic::widget::segmented_button::{Entity, SingleSelectModel};

use noctua_core::render::worker::SharedWorker;
use noctua_core::storage::browser::BrowserEntry;

use crate::message::MenuAction;

/// Session file used for automatic save and restore.
pub(crate) const SESSION_NAME: &str = "default";

/// How many nav entries receive thumbnails up front; the rest stay
/// text-only until the nav bar becomes viewport-aware. Kept small:
/// thumbnails decode full images, which is expensive even in the
/// background.
pub(crate) const INITIAL_THUMBS: usize = 8;

/// Zoom factor applied per zoom step.
pub(crate) const ZOOM_STEP: f32 = 1.25;

/// Render scale for page thumbnails in the document tab nav bar.
pub(crate) const PAGE_THUMB_ZOOM: f32 = 0.2;

/// RGBA pixels: (width, height, data).
pub(crate) type Rgba = (u32, u32, Vec<u8>);

/// A thumbnail result for one nav entry.
pub(crate) type ThumbResult = (nav_bar::Id, Option<Rgba>);

/// Content of one browser tab.
pub(crate) enum TabContent {
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
    pub(crate) fn path(&self) -> &PathBuf {
        match self {
            TabContent::Folder { path, .. } => path,
            TabContent::Document { path, .. } => path,
        }
    }
}

/// What a nav bar entry points to.
#[derive(Debug, Clone)]
pub(crate) enum NavEntry {
    /// A file in a folder tab. PDFs dive into a new tab when clicked.
    File { path: PathBuf },
    /// A page of the document tab (1-based).
    Page { path: PathBuf, page: u32 },
}

/// What kind of thumbnail a nav entry needs.
pub(crate) enum ThumbRequest {
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
pub(crate) enum CurrentTarget {
    File { path: PathBuf },
    Page { path: PathBuf, page: u32 },
}

/// A rendered image shown in the content area. The handle is created
/// once per image so the renderer reuses the uploaded texture instead
/// of re-uploading it on every frame.
pub(crate) struct CurrentImage {
    pub(crate) handle: widget::image::Handle,
}

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    pub(crate) core: cosmic::Core,
    /// The about page for this app.
    pub(crate) about: About,
    /// Key bindings for the application's menu bar.
    pub(crate) key_binds: HashMap<menu::KeyBind, MenuAction>,
    /// Tab strip model; tab contents are stored per entity.
    pub(crate) tab_model: SingleSelectModel,
    /// Contents of each open tab.
    pub(crate) tabs: HashMap<Entity, TabContent>,
    /// Nav bar entries of the active tab.
    pub(crate) nav_model: nav_bar::Model,
    /// Entry currently shown in the content area.
    pub(crate) current_target: Option<CurrentTarget>,
    /// Rendered content for the current target.
    pub(crate) current_image: Option<CurrentImage>,
    /// Current zoom factor (1.0 = 100%).
    pub(crate) zoom: f32,
    /// Whether the nav panel is visible.
    pub(crate) show_nav_panel: bool,
    /// Single pdfium worker; all pdfium jobs run on its thread.
    pub(crate) worker: SharedWorker,
    /// File size of the current target, for the status bar.
    pub(crate) current_size: Option<u64>,
    /// 1-based position of the active nav entry (status bar).
    pub(crate) current_position: Option<usize>,
    /// When a start argument is a file, its nav entry is selected
    /// after the parent folder has been listed.
    pub(crate) pending_select: Option<PathBuf>,
    /// Path of a start argument that could not be opened; shown in the
    /// empty view instead of the folder hint.
    pub(crate) start_error: Option<String>,
}
