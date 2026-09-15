// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/message.rs
//
// The language of the UI: messages emitted by the application and its widgets.

use std::path::PathBuf;

use cosmic::widget::menu;
use cosmic::widget::segmented_button::Entity;

use noctua_core::storage::browser::BrowserEntry;

use crate::model::Rgba;

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
    /// An entry of the thumbnail strip was double-clicked: open it.
    /// PDFs dive into a document tab.
    StripDoubleClicked(usize),
    /// Move to the previous strip entry.
    PrevEntry,
    /// Move to the next strip entry.
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
    /// viewport height to compute the visible page window.
    PreviewScrolled {
        path: PathBuf,
        offset_y: f32,
        viewport_height: f32,
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
