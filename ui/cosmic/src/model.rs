// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/model.rs
//
// UI state: pure data of the application model. No tasks, no widgets.

use std::collections::HashMap;
use std::path::PathBuf;

use cosmic::iced::keyboard::Modifiers;
use cosmic::widget;
use cosmic::widget::about::About;
use cosmic::widget::menu;
use cosmic::widget::segmented_button::{Entity, SingleSelectModel};

use noctua_core::render::worker::SharedWorker;
use noctua_core::storage::browser::BrowserEntry;

use crate::message::MenuAction;

/// Session file used for automatic save and restore.
pub(crate) const SESSION_NAME: &str = "default";

/// Zoom factor applied per zoom step.
pub(crate) const ZOOM_STEP: f32 = 1.25;

/// Minimum zoom factor for single images (0.25 = 25 % of native pixels).
pub(crate) const MIN_SCALE: f32 = 0.25;

/// Maximum zoom factor for single images (8.0 = 800 % of native pixels).
pub(crate) const MAX_SCALE: f32 = 8.0;

/// Render scale for page thumbnails (strip and preview placeholders).
pub(crate) const THUMB_ZOOM: f32 = 0.2;

/// Strip entries that receive thumbnails when a tab becomes active; the
/// rest are rendered lazily while scrolling.
pub(crate) const STRIP_INITIAL_THUMBS: usize = 8;

/// How many full-resolution preview pages are kept at once; older pages
/// fall back to their thumbnails.
pub(crate) const PREVIEW_FULL_CACHE: usize = 12;

/// Stable widget ids; restoring scroll offsets needs them.
pub(crate) const PREVIEW_SCROLL_ID: &str = "preview-scroll";

/// RGBA pixels: (width, height, data).
pub(crate) type Rgba = (u32, u32, Vec<u8>);

/// Content of one browser tab.
pub(crate) enum TabContent {
    /// A folder listing.
    Folder {
        path: PathBuf,
        entries: Vec<BrowserEntry>,
    },
}

impl TabContent {
    pub(crate) fn path(&self) -> &PathBuf {
        match self {
            TabContent::Folder { path, .. } => path,
        }
    }
}

/// What a strip entry points to.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NavEntry {
    /// A file in a folder tab. PDFs show an inline preview. The format is
    /// detected once when the folder is listed and carried here so the UI
    /// never re-reads file bytes to decide the render path.
    File { path: PathBuf, is_pdf: bool },
    /// A page of the document tab (1-based).
    Page { path: PathBuf, page: u32 },
}

/// One entry of the thumbnail strip of a tab.
pub(crate) struct StripEntry {
    pub(crate) target: NavEntry,
    pub(crate) name: String,
    pub(crate) thumb: Option<widget::image::Handle>,
    /// Whether the entry is a multi-page PDF that can be expanded in place.
    pub(crate) expandable: bool,
}

/// The entry currently shown in the content area.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CurrentTarget {
    File { path: PathBuf },
    Page { path: PathBuf, page: u32 },
}

/// Per-image zoom/pan state remembered across selections.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ZoomState {
    /// Whether the image is shown fitted to the viewport.
    pub(crate) fit: bool,
    /// Zoom factor (1.0 = 100 % = native pixels); ignored while `fit`.
    pub(crate) scale: f32,
    /// Pan offset in logical pixels.
    pub(crate) offset_x: f32,
    pub(crate) offset_y: f32,
}

impl Default for ZoomState {
    fn default() -> Self {
        Self {
            fit: true,
            scale: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

/// A rendered image shown in the content area. The handle is created
/// once per image so the renderer reuses the uploaded texture instead
/// of re-uploading it on every frame.
pub(crate) struct CurrentImage {
    pub(crate) handle: widget::image::Handle,
}

/// One page of the continuous PDF preview.
pub(crate) enum PageSlot {
    /// Sizing placeholder; no pixels yet.
    Empty,
    /// Low-resolution placeholder from the thumbnail pass.
    Thumb { handle: widget::image::Handle },
    /// Full-resolution render.
    Full { handle: widget::image::Handle },
}

/// Continuous preview of a multi-page PDF inside a folder tab.
pub(crate) struct DocumentPreview {
    pub(crate) path: PathBuf,
    /// Native page sizes in pixels at zoom 1.0 (points ≈ px at 72 dpi).
    pub(crate) page_sizes: Vec<(f32, f32)>,
    pub(crate) pages: Vec<PageSlot>,
    /// Zoom the full pages were rendered at.
    pub(crate) zoom: f32,
    /// Content scroll offset in logical pixels, restored on tab switch.
    pub(crate) scroll: f32,
    /// Height of the visible viewport, for the visible-window calculation.
    pub(crate) viewport_height: f32,
    /// Insertion order of full pages, oldest first, for the LRU cap.
    pub(crate) full_order: Vec<u32>,
    /// Last requested full-render window; scroll events re-request only
    /// when the visible window actually changes.
    pub(crate) requested: Option<(u32, u32)>,
}

/// Volatile per-tab UI state: strip entries, selection and the preview.
/// Survives tab switches but is not persisted in the session.
pub(crate) struct TabUiState {
    pub(crate) strip: Vec<StripEntry>,
    pub(crate) selected: Option<usize>,
    pub(crate) preview: Option<DocumentPreview>,
    /// Path of the PDF currently expanded in the strip, if any.
    pub(crate) expanded: Option<PathBuf>,
}

impl TabUiState {
    pub(crate) fn new(strip: Vec<StripEntry>) -> Self {
        let selected = (!strip.is_empty()).then_some(0);
        Self {
            strip,
            selected,
            preview: None,
            expanded: None,
        }
    }
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
    /// UI state per tab: thumbnail strip, selection and preview.
    pub(crate) tab_ui: HashMap<Entity, TabUiState>,
    /// Entry currently shown in the content area (active tab).
    pub(crate) current_target: Option<CurrentTarget>,
    /// Rendered single-page content for the current target.
    pub(crate) current_image: Option<CurrentImage>,
    /// Zoom/pan state per image, keyed by the current target.
    pub(crate) zoom_states: HashMap<CurrentTarget, ZoomState>,
    /// Current keyboard modifiers, tracked so widgets know the state at
    /// creation time (e.g. Ctrl already held).
    pub(crate) keyboard_modifiers: Modifiers,
    /// Whether the nav panel is visible.
    pub(crate) show_nav_panel: bool,
    /// Single pdfium worker; all pdfium jobs run on its thread.
    pub(crate) worker: SharedWorker,
    /// File size of the current target, for the status bar.
    pub(crate) current_size: Option<u64>,
    /// When a start argument is a file, its nav entry is selected
    /// after the parent folder has been listed.
    pub(crate) pending_select: Option<PathBuf>,
    /// Path of a start argument that could not be opened; shown in the
    /// empty view instead of the folder hint.
    pub(crate) start_error: Option<String>,
    /// Set after an async render updates the image; requests a single
    /// timer-driven redraw because async results are not presented on their
    /// own (libcosmic/iced bug).
    pub(crate) pending_redraw: bool,
}
