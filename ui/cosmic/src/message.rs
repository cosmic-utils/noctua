// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/message.rs
//
// The language of the UI: messages emitted by the application and its widgets.

use std::path::PathBuf;

use cosmic::iced::keyboard::Modifiers;
use cosmic::iced::mouse;
use cosmic::widget::menu;
use cosmic::widget::segmented_button::Entity;

use noctua_core::render::worker::DocumentId;
use noctua_core::storage::browser::BrowserEntry;

use crate::model::{CurrentTarget, Rgba};

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    /// A tab in the tab strip was activated.
    TabActivated(Entity),
    /// The close button of a tab was pressed.
    TabCloseRequested(Entity),
    /// Close the active tab.
    CloseTab,
    /// An entry of the thumbnail strip was activated (0-based index).
    StripActivated(usize),
    /// An entry of the thumbnail strip was double-clicked: toggle the
    /// in-strip page expansion of a multi-page PDF.
    StripDoubleClicked(usize),
    /// Move to the previous strip entry.
    PrevEntry,
    /// Move to the next strip entry.
    NextEntry,
    /// Scroll the active preview one page up.
    PrevPage,
    /// Scroll the active preview one page down.
    NextPage,
    /// The Open Folder menu entry was activated.
    OpenFolder,
    /// The folder dialog returned a result.
    FolderChosen(Option<PathBuf>),
    /// Create a new, empty annotation tab.
    NewAnnotation,
    /// Open the annotation-file chooser.
    OpenAnnotationFile,
    /// The annotation-file chooser returned a result.
    AnnotationFileChosen(Option<PathBuf>),
    /// Open the currently shown document for editing: create an annotation tab.
    OpenForEditing,
    /// Insert the currently shown document into an existing annotation tab.
    AddToAnnotation {
        document: DocumentId,
    },
    /// Adding a source to an annotation finished.
    AnnotationPagesAdded {
        document: DocumentId,
        ok: bool,
    },
    /// A new annotation tab was opened in the worker. `document` is `None`
    /// when opening failed; no tab is created in that case.
    AnnotationOpened {
        document: Option<DocumentId>,
        name: String,
        save_target: Option<PathBuf>,
        dirty: bool,
    },
    /// Page sizes of an annotation document arrived.
    AnnotationSizesKnown {
        tab: Entity,
        sizes: Option<Vec<(f32, f32)>>,
    },
    /// Thumbnails of an annotation document arrived.
    AnnotationThumbsReady {
        tab: Entity,
        thumbs: Vec<(u32, Option<Rgba>)>,
    },
    /// A full-resolution page of an annotation document was rendered.
    AnnotationPageRendered {
        tab: Entity,
        page: u32,
        rgba: Option<Rgba>,
    },
    /// The user selected a page of an annotation tab (1-based).
    AnnotationPageSelected {
        tab: Entity,
        page: u32,
    },
    /// The annotation viewer reported a zoom/pan state change.
    AnnotationViewerStateChanged {
        tab: Entity,
        scale: f32,
        offset_x: f32,
        offset_y: f32,
    },
    /// Save the active annotation; choose a target first if it has none yet.
    SaveAnnotation,
    /// Save the active annotation to a new target.
    SaveAnnotationAs,
    /// The save-target chooser returned a result.
    SaveTargetChosen(Option<PathBuf>),
    /// Saving an annotation finished; carries the tab, target and outcome.
    AnnotationSaved {
        tab: Entity,
        path: PathBuf,
        ok: bool,
    },
    /// The folder content was listed.
    FolderListed {
        dir: PathBuf,
        result: Result<Vec<BrowserEntry>, String>,
    },
    /// The worker counted the pages of a PDF selected for expansion.
    PagesKnown {
        path: PathBuf,
        pages: Option<u32>,
    },
    /// Thumbnails for strip entries arrived. Guarded by the tab entity.
    StripThumbsReady {
        tab: Entity,
        thumbs: Vec<(usize, Option<Rgba>)>,
    },
    /// Page sizes of a preview candidate arrived; one page means the PDF
    /// is displayed as a single image, more pages build the preview.
    PreviewSizesKnown {
        path: PathBuf,
        sizes: Option<Vec<(f32, f32)>>,
    },
    /// Rendered preview pages arrived as (page, width, height, rgba).
    /// `zoom` separates full renders from thumbnail placeholders.
    PreviewPagesRendered {
        path: PathBuf,
        zoom: f32,
        pages: Vec<(u32, u32, u32, Vec<u8>)>,
    },
    /// The preview was scrolled; carries absolute content offset and the
    /// viewport size to compute the visible page window.
    PreviewScrolled {
        path: PathBuf,
        offset_y: f32,
        viewport_height: f32,
        viewport_width: f32,
    },
    /// The mouse wheel scrolled over a PDF preview; Ctrl turns it into a zoom.
    PreviewWheel {
        path: PathBuf,
        delta: mouse::ScrollDelta,
    },
    /// The thumbnail strip was scrolled; carries the absolute offset and
    /// the viewport height to compute the visible tile range.
    StripScrolled {
        tab: Entity,
        offset_y: f32,
        viewport_height: f32,
    },
    /// A raster or SVG file was rendered for the content area.
    FileRendered {
        path: PathBuf,
        rgba: Option<(u32, u32, Vec<u8>)>,
    },
    /// A PDF page was rendered for the content area.
    PageRendered {
        path: PathBuf,
        page: u32,
        rgba: Option<(u32, u32, Vec<u8>)>,
    },
    /// The image viewer reported a zoom/pan state change.
    ViewerStateChanged {
        target: CurrentTarget,
        scale: f32,
        offset_x: f32,
        offset_y: f32,
    },
    ZoomIn,
    ZoomOut,
    Zoom100,
    ZoomToFit,
    /// Zoom by the given number of steps (positive in, negative out), produced
    /// by a wheel scroll over the zoom label.
    ZoomBy(f32),
    /// Rotate the current single image 90° clockwise.
    RotateClockwise,
    /// Rotate the current single image 90° counter-clockwise.
    RotateCounterClockwise,
    /// Mirror the current single image horizontally.
    FlipHorizontal,
    /// Mirror the current single image vertically.
    FlipVertical,
    /// The keyboard modifiers changed; tracked to keep widgets in sync.
    ModifiersChanged(Modifiers),
    ToggleFullscreen,
    ToggleNavPanel,
    ToggleAbout,
    LaunchUrl(String),
    /// Internal no-op: lets a fire-and-forget task return a message.
    Noop,
    Quit,
}

/// Actions offered by the menu bar.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MenuAction {
    OpenFolder,
    NewAnnotation,
    OpenAnnotationFile,
    OpenForEditing,
    AddToAnnotation(DocumentId),
    SavePdf,
    SavePdfAs,
    CloseTab,
    ZoomIn,
    ZoomOut,
    Zoom100,
    ZoomToFit,
    RotateClockwise,
    RotateCounterClockwise,
    FlipHorizontal,
    FlipVertical,
    Fullscreen,
    ToggleNavPanel,
    About,
    Quit,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::OpenFolder => Message::OpenFolder,
            MenuAction::NewAnnotation => Message::NewAnnotation,
            MenuAction::OpenAnnotationFile => Message::OpenAnnotationFile,
            MenuAction::OpenForEditing => Message::OpenForEditing,
            MenuAction::AddToAnnotation(document) => Message::AddToAnnotation {
                document: *document,
            },
            MenuAction::SavePdf => Message::SaveAnnotation,
            MenuAction::SavePdfAs => Message::SaveAnnotationAs,
            MenuAction::CloseTab => Message::CloseTab,
            MenuAction::ZoomIn => Message::ZoomIn,
            MenuAction::ZoomOut => Message::ZoomOut,
            MenuAction::Zoom100 => Message::Zoom100,
            MenuAction::ZoomToFit => Message::ZoomToFit,
            MenuAction::RotateClockwise => Message::RotateClockwise,
            MenuAction::RotateCounterClockwise => Message::RotateCounterClockwise,
            MenuAction::FlipHorizontal => Message::FlipHorizontal,
            MenuAction::FlipVertical => Message::FlipVertical,
            MenuAction::Fullscreen => Message::ToggleFullscreen,
            MenuAction::ToggleNavPanel => Message::ToggleNavPanel,
            MenuAction::About => Message::ToggleAbout,
            MenuAction::Quit => Message::Quit,
        }
    }
}
