// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/message.rs
//
// The language of the UI: messages emitted by widgets, grouped by the part of
// the application they drive.

use std::path::PathBuf;

use cosmic::iced::Point;
use cosmic::iced::keyboard::Key;
use cosmic::iced::keyboard::Modifiers;
use cosmic::iced::mouse;
use cosmic::widget::menu;
use cosmic::widget::segmented_button::Entity;

use noctua_core::pdfium_ops::FormField;
use noctua_core::pdfium_ops::model::{AnnotationColor, FreeTextAnnotation};
use noctua_core::render::worker::DocumentId;
use noctua_core::storage::browser::BrowserEntry;

use crate::model::{AnnotationTool, Rgba, Target};

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    App(AppMessage),
    Browser(BrowserMessage),
    Annotation(AnnotationMessage),
    View(ViewMessage),
}

/// Whole-application messages that do not belong to one feature area.
#[derive(Debug, Clone)]
pub enum AppMessage {
    ModifiersChanged(Modifiers),
    Noop,
    Surface(cosmic::surface::Action<Message>),
    Quit,
}

/// Folder and browser messages: tabs, strip and the document preview.
#[derive(Debug, Clone)]
pub enum BrowserMessage {
    TabActivated(Entity),
    TabCloseRequested(Entity),
    CloseTab,
    StripActivated(usize),
    StripDoubleClicked(usize),
    PrevEntry,
    NextEntry,
    PrevPage,
    NextPage,
    OpenFolder,
    FolderChosen(Option<PathBuf>),
    FolderListed {
        dir: PathBuf,
        result: Result<Vec<BrowserEntry>, String>,
    },
    ContextOpened {
        path: PathBuf,
    },
    PagesKnown {
        path: PathBuf,
        pages: Option<u32>,
    },
    StripThumbsReady {
        tab: Entity,
        thumbs: Vec<(usize, Option<Rgba>)>,
    },
    PreviewSizesKnown {
        path: PathBuf,
        sizes: Option<Vec<(f32, f32)>>,
    },
    PreviewPagesRendered {
        path: PathBuf,
        zoom: f32,
        pages: Vec<(u32, u32, u32, Vec<u8>)>,
    },
    PreviewScrolled {
        path: PathBuf,
        offset_y: f32,
        viewport_height: f32,
        viewport_width: f32,
    },
    PreviewWheel {
        path: PathBuf,
        delta: mouse::ScrollDelta,
    },
    StripScrolled {
        tab: Entity,
        offset_y: f32,
        viewport_height: f32,
    },
    FileRendered {
        path: PathBuf,
        rgba: Option<(u32, u32, Vec<u8>)>,
    },
    PageRendered {
        path: PathBuf,
        page: u32,
        rgba: Option<(u32, u32, Vec<u8>)>,
    },
    ViewerStateChanged {
        target: Target,
        scale: f32,
        offset_x: f32,
        offset_y: f32,
    },
}

/// Annotation-tab messages: pages, editing, forms and saving.
#[derive(Debug, Clone)]
pub enum AnnotationMessage {
    NewAnnotation,
    OpenAnnotationFile,
    AnnotationFileChosen(Option<PathBuf>),
    OpenForEditing,
    AddToAnnotation {
        document: DocumentId,
    },
    AnnotationPagesAdded {
        document: DocumentId,
        ok: bool,
    },
    AnnotationOpened {
        document: Option<DocumentId>,
        name: String,
        save_target: Option<PathBuf>,
        dirty: bool,
    },
    AnnotationSizesKnown {
        tab: Entity,
        sizes: Option<Vec<(f32, f32)>>,
    },
    AnnotationThumbsReady {
        tab: Entity,
        thumbs: Vec<(u32, Option<Rgba>)>,
    },
    AnnotationPageRendered {
        tab: Entity,
        page: u32,
        rgba: Option<Rgba>,
    },
    AnnotationPageSelected {
        tab: Entity,
        page: u32,
    },
    AnnotationRotatePage {
        tab: Entity,
        page: u32,
        clockwise: bool,
    },
    AnnotationDeletePage {
        tab: Entity,
        page: u32,
    },
    AnnotationMovePage {
        tab: Entity,
        page: u32,
        up: bool,
    },
    AnnotationPageMutated {
        tab: Entity,
        ok: bool,
    },
    AnnotationToolToggled(AnnotationTool),
    AnnotationFontSizeChanged(f32),
    AnnotationColorChanged(AnnotationColor),
    AnnotationPointerClicked {
        tab: Entity,
        x: f32,
        y: f32,
        anchor: Point,
    },
    AnnotationTargetsKnown {
        tab: Entity,
        annotations: Vec<FreeTextAnnotation>,
        fields: Vec<FormField>,
        x: f32,
        y: f32,
        anchor: Point,
    },
    AnnotationTextKey(Key),
    AnnotationTextTyped(String),
    CaretTicked,
    AnnotationTextCreated {
        tab: Entity,
        index: Option<usize>,
        origin: (f32, f32),
        max_width: f32,
        caret: (f32, f32, f32, f32),
    },
    AnnotationTextMutated {
        tab: Entity,
        ok: bool,
        caret: Option<(f32, f32, f32, f32)>,
    },
    AnnotationViewerStateChanged {
        tab: Entity,
        scale: f32,
        offset_x: f32,
        offset_y: f32,
    },
    SaveAnnotation,
    SaveAnnotationAs,
    SaveTargetChosen(Option<PathBuf>),
    AnnotationSaved {
        tab: Entity,
        path: PathBuf,
        ok: bool,
    },
    RenameAnnotation,
    RenameAnnotationChanged(String),
    RenameAnnotationConfirmed,
    RenameAnnotationCancelled,
    AnnotationRenamed {
        tab: Entity,
        stem: String,
        result: Result<PathBuf, String>,
    },
    AnnotationFormMutated {
        tab: Entity,
        ok: bool,
    },
    AnnotationFormOptionPicked {
        tab: Entity,
        field: usize,
        option: usize,
    },
    AnnotationFormMenuClosed,
}

/// View messages: zoom, rotation and the navigation chrome.
#[derive(Debug, Clone)]
pub enum ViewMessage {
    ZoomIn,
    ZoomOut,
    Zoom100,
    ZoomToFit,
    ZoomBy(f32),
    RotateClockwise,
    RotateCounterClockwise,
    FlipHorizontal,
    FlipVertical,
    ToggleFullscreen,
    ToggleNavPanel,
    ToggleAbout,
    LaunchUrl(String),
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
    RenameAnnotation,
    CloseTab,
    ZoomIn,
    ZoomOut,
    Zoom100,
    ZoomToFit,
    RotateClockwise,
    RotateCounterClockwise,
    RotatePageClockwise { tab: Entity, page: u32 },
    RotatePageCounterClockwise { tab: Entity, page: u32 },
    DeletePage { tab: Entity, page: u32 },
    MovePageUp { tab: Entity, page: u32 },
    MovePageDown { tab: Entity, page: u32 },
    FlipHorizontal,
    FlipVertical,
    Fullscreen,
    About,
    Quit,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::OpenFolder => Message::Browser(BrowserMessage::OpenFolder),
            MenuAction::NewAnnotation => Message::Annotation(AnnotationMessage::NewAnnotation),
            MenuAction::OpenAnnotationFile => {
                Message::Annotation(AnnotationMessage::OpenAnnotationFile)
            }
            MenuAction::OpenForEditing => Message::Annotation(AnnotationMessage::OpenForEditing),
            MenuAction::AddToAnnotation(document) => {
                Message::Annotation(AnnotationMessage::AddToAnnotation {
                    document: *document,
                })
            }
            MenuAction::SavePdf => Message::Annotation(AnnotationMessage::SaveAnnotation),
            MenuAction::SavePdfAs => Message::Annotation(AnnotationMessage::SaveAnnotationAs),
            MenuAction::RenameAnnotation => {
                Message::Annotation(AnnotationMessage::RenameAnnotation)
            }
            MenuAction::CloseTab => Message::Browser(BrowserMessage::CloseTab),
            MenuAction::ZoomIn => Message::View(ViewMessage::ZoomIn),
            MenuAction::ZoomOut => Message::View(ViewMessage::ZoomOut),
            MenuAction::Zoom100 => Message::View(ViewMessage::Zoom100),
            MenuAction::ZoomToFit => Message::View(ViewMessage::ZoomToFit),
            MenuAction::RotateClockwise => Message::View(ViewMessage::RotateClockwise),
            MenuAction::RotateCounterClockwise => {
                Message::View(ViewMessage::RotateCounterClockwise)
            }
            MenuAction::RotatePageClockwise { tab, page } => {
                Message::Annotation(AnnotationMessage::AnnotationRotatePage {
                    tab: *tab,
                    page: *page,
                    clockwise: true,
                })
            }
            MenuAction::RotatePageCounterClockwise { tab, page } => {
                Message::Annotation(AnnotationMessage::AnnotationRotatePage {
                    tab: *tab,
                    page: *page,
                    clockwise: false,
                })
            }
            MenuAction::DeletePage { tab, page } => {
                Message::Annotation(AnnotationMessage::AnnotationDeletePage {
                    tab: *tab,
                    page: *page,
                })
            }
            MenuAction::MovePageUp { tab, page } => {
                Message::Annotation(AnnotationMessage::AnnotationMovePage {
                    tab: *tab,
                    page: *page,
                    up: true,
                })
            }
            MenuAction::MovePageDown { tab, page } => {
                Message::Annotation(AnnotationMessage::AnnotationMovePage {
                    tab: *tab,
                    page: *page,
                    up: false,
                })
            }
            MenuAction::FlipHorizontal => Message::View(ViewMessage::FlipHorizontal),
            MenuAction::FlipVertical => Message::View(ViewMessage::FlipVertical),
            MenuAction::Fullscreen => Message::View(ViewMessage::ToggleFullscreen),
            MenuAction::About => Message::View(ViewMessage::ToggleAbout),
            MenuAction::Quit => Message::App(AppMessage::Quit),
        }
    }
}
