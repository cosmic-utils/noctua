// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/model.rs
//
// UI state: pure data of the application model. No tasks, no widgets.

use std::cell::Cell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use cosmic::iced::Size;
use cosmic::iced::keyboard::Modifiers;
use cosmic::widget;
use cosmic::widget::about::About;
use cosmic::widget::menu;
use cosmic::widget::segmented_button::{Entity, SingleSelectModel};

use noctua_core::pdfium_ops::model::AnnotationColor;
use noctua_core::render::worker::{DocumentId, SharedWorker};
use noctua_core::storage::browser::BrowserEntry;

use crate::config::NoctuaConfig;
use crate::message::MenuAction;

/// Session file used for automatic save and restore.
pub(crate) const SESSION_NAME: &str = "default";

/// Zoom factor applied per zoom step.
pub(crate) const ZOOM_STEP: f32 = 1.25;

/// Minimum zoom factor for single images (0.25 = 25 % of native pixels).
pub(crate) const MIN_SCALE: f32 = 0.25;

/// Maximum zoom factor for single images (8.0 = 800 % of native pixels).
pub(crate) const MAX_SCALE: f32 = 8.0;

/// Minimum zoom factor for the PDF preview (0.05 = 5 % of native size).
pub(crate) const PREVIEW_MIN_SCALE: f32 = 0.05;

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
    /// An annotation tab: an independent PDF edit state held by the worker.
    Annotation {
        document: DocumentId,
        /// Display name (the file name); changes when saved to a new target.
        name: String,
        /// Target path once saved; `None` until the first save.
        save_target: Option<PathBuf>,
        /// Whether the edit state has unsaved changes.
        dirty: bool,
    },
}

/// What a strip entry points to, and what is shown in the content area. The
/// file format is detected once when a folder is listed and carried as
/// `StripEntry.expandable` (not here), so the target stays a pure path/page.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Target {
    /// A file in a folder tab. PDFs show an inline preview.
    File { path: PathBuf },
    /// A page of a document (1-based).
    Page { path: PathBuf, page: u32 },
}

/// One entry of the thumbnail strip of a tab.
pub(crate) struct StripEntry {
    pub(crate) target: Target,
    pub(crate) name: String,
    pub(crate) thumb: Option<widget::image::Handle>,
    /// Whether the entry is a PDF that can be expanded in place.
    pub(crate) expandable: bool,
    /// Per-entry view state (zoom/pan/rotate/flip). Owned by the entry, not
    /// copied into the tab or a side map.
    pub(crate) view: ViewState,
}

/// Per-image view state (zoom, pan, rotate, flip) remembered across selections.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ViewState {
    /// Whether the image is shown fitted to the viewport.
    pub(crate) fit: bool,
    /// Zoom factor (1.0 = 100 % = native pixels); ignored while `fit`.
    pub(crate) scale: f32,
    /// Pan offset in logical pixels.
    pub(crate) offset_x: f32,
    pub(crate) offset_y: f32,
    /// Cumulative view rotation in degrees (0/90/180/270).
    pub(crate) rotation: u16,
    /// Cumulative horizontal flip; vertical flip folds into `rotation == 180`
    /// plus `flip_h`, so the pair covers all eight dihedral symmetries.
    pub(crate) flip_h: bool,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            fit: true,
            scale: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
            rotation: 0,
            flip_h: false,
        }
    }
}

/// A rendered image shown in the content area. The handle is created
/// once per image so the renderer reuses the uploaded texture instead
/// of re-uploading it on every frame. The raw pixels are kept so the
/// view can rotate/flip without re-rendering from disk.
pub(crate) struct CurrentImage {
    pub(crate) handle: widget::image::Handle,
    pub(crate) rgba: Rgba,
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
    /// Cumulative view rotation in degrees (0/90/180/270).
    pub(crate) rotation: u16,
    /// Cumulative horizontal flip. Vertical flip is `rotation == 180` with
    /// `flip_h` set, so this pair represents all eight dihedral symmetries.
    pub(crate) flip_h: bool,
    /// Content scroll offset in logical pixels, restored on tab switch.
    pub(crate) scroll: f32,
    /// Height of the visible viewport, for the visible-window calculation.
    pub(crate) viewport_height: f32,
    /// Width of the visible viewport, for fit-to-width.
    pub(crate) viewport_width: f32,
    /// Insertion order of full pages, oldest first, for the LRU cap.
    pub(crate) full_order: Vec<u32>,
    /// Last requested full-render window; scroll events re-request only
    /// when the visible window actually changes.
    pub(crate) requested: Option<(u32, u32)>,
}

impl DocumentPreview {
    /// Displayed page size in pixels, accounting for the cumulative view
    /// rotation: 90°/270° swap width and height.
    pub(crate) fn effective_page_size(&self, index: usize) -> (f32, f32) {
        let (w, h) = self
            .page_sizes
            .get(index)
            .copied()
            .unwrap_or((600.0, 800.0));
        if self.rotation.is_multiple_of(180) {
            (w, h)
        } else {
            (h, w)
        }
    }
}

/// Volatile per-tab UI state: strip entries, selection and the preview.
/// Survives tab switches but is not persisted in the session.
pub(crate) struct TabUiState {
    pub(crate) entries: Vec<StripEntry>,
    pub(crate) selected: Option<usize>,
    pub(crate) preview: Option<DocumentPreview>,
    /// Path of the PDF currently expanded in the strip, if any.
    pub(crate) expanded: Option<PathBuf>,
}

impl TabUiState {
    pub(crate) fn new(entries: Vec<StripEntry>) -> Self {
        let selected = (!entries.is_empty()).then_some(0);
        Self {
            entries,
            selected,
            preview: None,
            expanded: None,
        }
    }
}

/// Annotation tools offered by the annotation mode. Text is implemented
/// first; the remaining tools follow in later steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationTool {
    Text,
}

/// Volatile view state of an annotation tab: page list, thumbnails and the
/// currently shown page. The document itself lives in the worker, keyed by
/// the tab's `DocumentId`.
pub(crate) struct AnnotationUiState {
    /// Native page sizes in points (≈ px at 72 dpi).
    pub(crate) page_sizes: Vec<(f32, f32)>,
    /// 1-based index of the selected page.
    pub(crate) selected: u32,
    /// Per-page thumbnail handles; `None` until rendered.
    pub(crate) thumbs: Vec<Option<widget::image::Handle>>,
    /// Full-resolution pixels of the selected page, for the content view.
    pub(crate) current: Option<CurrentImage>,
    /// View transform (zoom/pan/rotate/flip), like single images.
    pub(crate) view: ViewState,
    /// Currently active annotation tool; `None` means browse/pan.
    pub(crate) active_tool: Option<AnnotationTool>,
    /// Selected free-text colour.
    pub(crate) annotation_color: AnnotationColor,
    /// Selected free-text font size in points.
    pub(crate) font_size_pt: f32,
    /// An in-progress free-text placement; `None` when no text is being entered.
    pub(crate) pending_text: Option<PendingText>,
    /// Whether the insertion marker is currently shown; toggled by the blink
    /// timer while a text edit is active.
    pub(crate) caret_visible: bool,
}

/// An in-progress free-text edit. The annotation itself lives in the document
/// and is rewritten on every keystroke, so the page render is the preview.
#[derive(Debug, Clone)]
pub(crate) struct PendingText {
    /// Index of the annotation in the page's annotation list.
    pub(crate) index: usize,
    /// Text when the edit started; restored when the edit is cancelled.
    pub(crate) original_text: String,
    /// Whether this edit created the annotation, so cancelling removes it.
    pub(crate) is_new: bool,
    /// Current text.
    pub(crate) text: String,
    /// Caret position as a character offset into `text`.
    pub(crate) cursor: usize,
    /// Top-left corner of the box in page points (origin bottom-left).
    pub(crate) origin: (f32, f32),
    /// Width the text is wrapped at, in points.
    pub(crate) max_width: f32,
    /// Insertion marker rectangle in page points (origin bottom-left), as
    /// reported by the core after the last write.
    pub(crate) caret: (f32, f32, f32, f32),
}

impl Default for AnnotationUiState {
    fn default() -> Self {
        Self {
            page_sizes: Vec::new(),
            selected: 1,
            thumbs: Vec::new(),
            current: None,
            view: ViewState::default(),
            active_tool: None,
            annotation_color: AnnotationColor {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 255,
            },
            font_size_pt: 24.0,
            pending_text: None,
            caret_visible: true,
        }
    }
}

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    pub(crate) core: cosmic::Core,
    /// Application settings loaded from cosmic-config.
    pub(crate) config: NoctuaConfig,
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
    /// View state of annotation tabs: page thumbnails and the current page.
    pub(crate) annotation_ui: HashMap<Entity, AnnotationUiState>,
    /// Entry currently shown in the content area (active tab).
    pub(crate) displayed_target: Option<Target>,
    /// Rendered single-page content for the displayed target.
    pub(crate) displayed_image: Option<CurrentImage>,
    /// Viewport of the currently drawn viewer, written by the `Viewer` widget
    /// each frame. Lets the toolbar show the true percentage and lets
    /// zoom-from-fit continue from the displayed factor instead of jumping to
    /// 1:1. The viewport is stable across image changes, so reads never lag.
    pub(crate) viewer_bounds: Rc<Cell<Size>>,
    /// Current keyboard modifiers, tracked so widgets know the state at
    /// creation time (e.g. Ctrl already held).
    pub(crate) keyboard_modifiers: Modifiers,
    /// Whether the nav panel is visible.
    pub(crate) show_nav_panel: bool,
    /// Single pdfium worker; all pdfium jobs run on its thread.
    pub(crate) worker: SharedWorker,
    /// File size of the displayed target, for the status bar.
    pub(crate) displayed_size: Option<u64>,
    /// Path of the file the right-click context menu was opened on.
    pub(crate) context_target: Option<PathBuf>,
    /// When a start argument is a file, its strip entry is selected
    /// after the parent folder has been listed.
    pub(crate) pending_select: Option<PathBuf>,
    /// Path of a start argument that could not be opened; shown in the
    /// empty view instead of the folder hint.
    pub(crate) start_error: Option<String>,
    /// Open rename dialog of an annotation tab.
    pub(crate) rename: Option<RenameDraft>,
}

/// Text being edited in the rename dialog of an annotation tab.
pub(crate) struct RenameDraft {
    /// The annotation tab whose target file is being renamed.
    pub(crate) tab: Entity,
    /// New file name without extension.
    pub(crate) text: String,
}
