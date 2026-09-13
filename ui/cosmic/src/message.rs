// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/message.rs
//
// The language of the UI: messages emitted by the application and its widgets.

use std::path::PathBuf;

use cosmic::widget::menu;
use cosmic::widget::nav_bar;
use cosmic::widget::segmented_button::Entity;
use noctua_core::storage::browser::BrowserEntry;

use crate::model::ThumbResult;

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
    /// Scheduled shortly after an image was applied: iced's wgpu backend
    /// uploads textures larger than 2 MB on a background thread and skips
    /// them in the current frame, so the follow-up frame shows the image.
    RepaintTick,
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
