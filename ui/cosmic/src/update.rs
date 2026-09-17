// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cosmic/src/update.rs
//
// Update logic: message handling and all state changes of the UI model.
// Produces tasks; never builds widgets and never touches the filesystem
// directly (all I/O goes through noctua-core).

use std::collections::HashMap;
use std::path::PathBuf;

use cosmic::iced::keyboard::Key;
use cosmic::iced::keyboard::key::Named;
use cosmic::iced::mouse;
use cosmic::iced::widget::scrollable::{AbsoluteOffset, scroll_to};
use cosmic::widget::menu;
use cosmic::widget::menu::key_bind::Modifier;
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
    AppModel, CurrentImage, CurrentTarget, DocumentPreview, MAX_SCALE, MIN_SCALE, NavEntry,
    PREVIEW_FULL_CACHE, PREVIEW_SCROLL_ID, PageSlot, Rgba, SESSION_NAME, STRIP_INITIAL_THUMBS,
    StripEntry, THUMB_ZOOM, TabContent, TabUiState, ZOOM_STEP,
};

/// Estimated strip tile height including spacing, for lazy thumbnails.
const STRIP_TILE: f32 = 150.0;

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
                &[Modifier::Ctrl, Modifier::Shift],
                Key::Character("+".into()),
                MenuAction::ZoomIn,
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
                Key::Character("0".into()),
                MenuAction::ZoomToFit,
            ),
            bind(&[], Key::Named(Named::F11), MenuAction::Fullscreen),
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

    /// Rebuild the thumbnail strip of a tab. The selection survives when
    /// it still points into the new strip; the preview survives only when
    /// it still matches the selected entry.
    fn rebuild_strip(&mut self, tab: Entity) {
        let entries: Vec<(NavEntry, String, bool)> = match &self.tabs[&tab] {
            TabContent::Folder { entries, .. } => entries
                .iter()
                .map(|entry| {
                    (
                        NavEntry::File {
                            path: entry.path.clone(),
                        },
                        entry.name.clone(),
                        entry.is_pdf,
                    )
                })
                .collect(),
        };

        let previous = self.tab_ui.remove(&tab);
        let selected = previous
            .as_ref()
            .and_then(|state| state.selected)
            .filter(|index| *index < entries.len())
            .or((!entries.is_empty()).then_some(0));
        let preview = previous.and_then(|state| state.preview).filter(|preview| {
            matches!(
                selected.and_then(|index| entries.get(index)),
                Some((NavEntry::File { path }, _, _)) if path == &preview.path
            )
        });

        let strip = entries
            .into_iter()
            .map(|(target, name, expandable)| StripEntry {
                target,
                name,
                thumb: None,
                expandable,
            })
            .collect();
        self.tab_ui.insert(
            tab,
            TabUiState {
                strip,
                selected,
                preview,
                expanded: None,
            },
        );
    }

    /// Activate the current tab: restore its selection, render it and
    /// restore the preview scroll offset. New tabs select their first entry.
    fn activate_tab(&mut self) -> iced::Task<cosmic::Action<Message>> {
        self.current_target = None;
        self.current_image = None;
        self.current_size = None;

        let Some(tab) = self.active_tab() else {
            return iced::Task::none();
        };

        // Tabs that received their content while inactive build their
        // strip on first activation.
        let has_content = match self.tabs.get(&tab) {
            Some(TabContent::Folder { entries, .. }) => !entries.is_empty(),
            None => false,
        };
        if has_content
            && self
                .tab_ui
                .get(&tab)
                .is_none_or(|state| state.strip.is_empty())
        {
            self.rebuild_strip(tab);
        }

        let target = self.tab_ui.get(&tab).and_then(|state| {
            state
                .selected
                .and_then(|index| state.strip.get(index))
                .map(|entry| entry.target.clone())
        });
        let scroll = self
            .tab_ui
            .get(&tab)
            .and_then(|state| state.preview.as_ref())
            .map(|preview| preview.scroll);

        let mut tasks: Vec<iced::Task<cosmic::Action<Message>>> = Vec::new();
        if let Some(target) = target {
            tasks.push(self.activate_target(target));
        }
        if let Some(offset_y) = scroll {
            tasks.push(
                scroll_to::<Message>(
                    cosmic::widget::Id::new(PREVIEW_SCROLL_ID),
                    AbsoluteOffset {
                        x: None,
                        y: Some(offset_y),
                    },
                )
                .map(cosmic::Action::from),
            );
        }
        tasks.push(self.request_strip_thumbs(tab, 0..STRIP_INITIAL_THUMBS));

        cosmic::task::batch(tasks)
    }

    /// Show or render the given strip entry.
    fn activate_target(&mut self, entry: NavEntry) -> iced::Task<cosmic::Action<Message>> {
        match entry {
            NavEntry::File { path } => {
                self.current_image = None;
                self.current_size = storage::document::metadata(&path)
                    .ok()
                    .map(|meta| meta.size_bytes);
                if storage::document::is_pdf(&path).unwrap_or(false) {
                    self.start_preview(path)
                } else {
                    self.current_target = Some(CurrentTarget::File { path: path.clone() });
                    self.render_file_task(path)
                }
            }
            NavEntry::Page { path, page } => {
                self.current_image = None;
                self.current_size = storage::document::metadata(&path)
                    .ok()
                    .map(|meta| meta.size_bytes);
                self.current_target = Some(CurrentTarget::Page {
                    path: path.clone(),
                    page,
                });
                self.render_page_task(path, page)
            }
        }
    }

    /// Start the inline preview for a PDF: ask the worker for the page
    /// sizes; one page is shown as a single image, more pages build the
    /// continuous scroll preview.
    fn start_preview(&mut self, path: PathBuf) -> iced::Task<cosmic::Action<Message>> {
        self.current_target = Some(CurrentTarget::File { path: path.clone() });
        let worker = self.worker.clone();
        let worker_path = path.clone();
        cosmic::task::future(async move {
            let sizes = tokio::task::spawn_blocking(move || worker.page_sizes(&worker_path))
                .await
                .ok()
                .flatten();
            Message::PreviewSizesKnown { path, sizes }
        })
    }

    /// Build the preview once the page sizes are known and start the
    /// first render passes. A preview for the same path with the same
    /// page count is reused, so re-selecting an entry keeps its scroll
    /// position and rendered pages.
    fn build_preview(
        &mut self,
        path: PathBuf,
        sizes: Vec<(f32, f32)>,
    ) -> iced::Task<cosmic::Action<Message>> {
        let Some(tab) = self.active_tab() else {
            return iced::Task::none();
        };
        let count = sizes.len() as u32;
        let visible = {
            let Some(state) = self.tab_ui.get_mut(&tab) else {
                return iced::Task::none();
            };
            if let Some(existing) = state.preview.as_mut()
                && existing.path == path
                && existing.page_sizes.len() == sizes.len()
            {
                existing.page_sizes = sizes;
                Some(Self::preview_visible_range(existing))
            } else {
                let pages = (0..sizes.len()).map(|_| PageSlot::Empty).collect();
                state.preview = Some(DocumentPreview {
                    path: path.clone(),
                    page_sizes: sizes,
                    pages,
                    zoom: 1.0,
                    scroll: 0.0,
                    viewport_height: 800.0,
                    full_order: Vec::new(),
                    requested: None,
                });
                None
            }
        };

        match visible {
            Some(visible) => self.request_preview_window(&path, visible),
            None => cosmic::task::batch(vec![
                self.request_preview_pages(&path, 1..=count.min(2), 1.0, true),
                self.request_preview_pages(&path, 1..=count.min(12), THUMB_ZOOM, false),
                scroll_to::<Message>(
                    cosmic::widget::Id::new(PREVIEW_SCROLL_ID),
                    AbsoluteOffset {
                        x: None,
                        y: Some(0.0),
                    },
                )
                .map(cosmic::Action::from),
            ]),
        }
    }

    /// Request preview pages in the given 1-based range. `full` selects
    /// full resolution (visible window) over thumbnails (placeholder).
    /// Pages that already have a matching render are skipped.
    fn request_preview_pages(
        &self,
        path: &PathBuf,
        range: std::ops::RangeInclusive<u32>,
        zoom: f32,
        full: bool,
    ) -> iced::Task<cosmic::Action<Message>> {
        let mut wanted: Vec<u32> = Vec::new();
        if let Some(tab) = self.active_tab()
            && let Some(state) = self.tab_ui.get(&tab)
            && let Some(preview) = state.preview.as_ref()
            && preview.path == *path
        {
            for page in range {
                let index = page as usize - 1;
                if index >= preview.pages.len() {
                    continue;
                }
                let already = if full {
                    matches!(preview.pages[index], PageSlot::Full { .. })
                } else {
                    matches!(
                        preview.pages[index],
                        PageSlot::Thumb { .. } | PageSlot::Full { .. }
                    )
                };
                if !already {
                    wanted.push(page);
                }
            }
        }

        if wanted.is_empty() {
            return iced::Task::none();
        }

        let worker = self.worker.clone();
        let worker_path = path.clone();
        let result_path = (*path).clone();
        let priority = if full {
            Priority::VisiblePage
        } else {
            Priority::Low
        };
        cosmic::task::future(async move {
            let pages = tokio::task::spawn_blocking(move || {
                worker.render_pages(&worker_path, &wanted, zoom, priority)
            })
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
            Message::PreviewPagesRendered {
                path: result_path,
                zoom,
                pages,
            }
        })
    }

    /// The 1-based page range currently visible in the preview.
    fn preview_visible_range(preview: &DocumentPreview) -> Option<(u32, u32)> {
        let mut visible: Option<(u32, u32)> = None;
        let mut y = 0.0f32;
        for (index, (_, height)) in preview.page_sizes.iter().enumerate() {
            let bottom = y + height * preview.zoom;
            if bottom >= preview.scroll && y <= preview.scroll + preview.viewport_height {
                let page = index as u32 + 1;
                visible = Some(match visible {
                    Some((first, last)) => (first.min(page), last.max(page)),
                    None => (page, page),
                });
            }
            y = bottom;
        }
        visible
    }

    /// Re-render the preview window around the given visible range.
    fn request_preview_window(
        &self,
        path: &PathBuf,
        visible: Option<(u32, u32)>,
    ) -> iced::Task<cosmic::Action<Message>> {
        let Some((first, last)) = visible else {
            return iced::Task::none();
        };
        let Some(count) = self.preview_pages(path) else {
            return iced::Task::none();
        };
        let Some(zoom) = self.preview_zoom(path) else {
            return iced::Task::none();
        };
        let thumb_first = first.saturating_sub(2).max(1);
        let thumb_last = (last + 2).min(count);
        cosmic::task::batch(vec![
            self.request_preview_pages(path, first..=last, zoom, true),
            self.request_preview_pages(path, thumb_first..=thumb_last, THUMB_ZOOM, false),
        ])
    }

    fn preview_pages(&self, path: &PathBuf) -> Option<u32> {
        let tab = self.active_tab()?;
        let state = self.tab_ui.get(&tab)?;
        let preview = state.preview.as_ref()?;
        (preview.path == *path).then_some(preview.pages.len() as u32)
    }

    fn preview_zoom(&self, path: &PathBuf) -> Option<f32> {
        let tab = self.active_tab()?;
        let state = self.tab_ui.get(&tab)?;
        let preview = state.preview.as_ref()?;
        (preview.path == *path).then_some(preview.zoom)
    }

    /// Request thumbnails for strip entries in the given index range that
    /// do not have one yet. File thumbnails come from the freedesktop
    /// cache; page thumbnails are batch-rendered per PDF.
    fn request_strip_thumbs(
        &self,
        tab: Entity,
        range: std::ops::Range<usize>,
    ) -> iced::Task<cosmic::Action<Message>> {
        let mut wanted: Vec<(usize, NavEntry)> = Vec::new();
        if let Some(state) = self.tab_ui.get(&tab) {
            for index in range {
                if let Some(entry) = state.strip.get(index)
                    && entry.thumb.is_none()
                {
                    wanted.push((index, entry.target.clone()));
                }
            }
        }
        if wanted.is_empty() {
            return iced::Task::none();
        }

        let worker = self.worker.clone();
        cosmic::task::future(async move {
            let thumbs = tokio::task::spawn_blocking(move || {
                let mut results: Vec<(usize, Option<Rgba>)> = Vec::new();
                let mut page_requests: Vec<(usize, PathBuf, u32)> = Vec::new();

                for (index, target) in wanted {
                    match target {
                        NavEntry::File { path } => {
                            let thumb = worker.thumbnail(&path, ThumbSize::Normal);
                            if thumb.is_none() {
                                tracing::warn!("thumbnail generation failed for {path:?}");
                            }
                            results.push((index, thumb));
                        }
                        NavEntry::Page { path, page } => {
                            page_requests.push((index, path, page));
                        }
                    }
                }

                let mut by_path: Vec<(PathBuf, Vec<(usize, u32)>)> = Vec::new();
                for (index, path, page) in page_requests {
                    match by_path.iter_mut().find(|(p, _)| *p == path) {
                        Some((_, pages)) => pages.push((index, page)),
                        None => by_path.push((path, vec![(index, page)])),
                    }
                }
                for (path, pages) in by_path {
                    let page_numbers: Vec<u32> = pages.iter().map(|(_, page)| *page).collect();
                    match worker.page_thumbs(&path, &page_numbers, THUMB_ZOOM) {
                        Some(thumbs) => {
                            for (index, page) in pages {
                                let rgba = thumbs
                                    .iter()
                                    .find(|(p, ..)| *p == page)
                                    .map(|(_, w, h, data)| (*w, *h, data.clone()));
                                results.push((index, rgba));
                            }
                        }
                        None => {
                            for (index, _) in pages {
                                results.push((index, None));
                            }
                        }
                    }
                }

                results
            })
            .await
            .ok()
            .unwrap_or_default();
            Message::StripThumbsReady { tab, thumbs }
        })
    }

    /// Toggle the in-strip page expansion of the PDF at the given strip index.
    fn toggle_expand(&mut self, index: usize) -> iced::Task<cosmic::Action<Message>> {
        let Some(tab) = self.active_tab() else {
            return iced::Task::none();
        };
        let (path, already_expanded) = {
            let Some(state) = self.tab_ui.get(&tab) else {
                return iced::Task::none();
            };
            let Some(path) = state
                .strip
                .get(index)
                .and_then(|entry| match &entry.target {
                    NavEntry::File { path } => Some(path.clone()),
                    NavEntry::Page { .. } => None,
                })
            else {
                return iced::Task::none();
            };
            (path.clone(), state.expanded.as_ref() == Some(&path))
        };

        if !storage::document::is_pdf(&path).unwrap_or(false) {
            return iced::Task::none();
        }

        if already_expanded {
            self.collapse_pdf(tab, &path);
            return iced::Task::none();
        }

        if let Some(previous) = self
            .tab_ui
            .get(&tab)
            .and_then(|state| state.expanded.clone())
        {
            self.collapse_pdf(tab, &previous);
        }
        if let Some(state) = self.tab_ui.get_mut(&tab) {
            state.expanded = Some(path.clone());
        }

        let worker = self.worker.clone();
        let count_path = path.clone();
        cosmic::task::future(async move {
            let pages = tokio::task::spawn_blocking(move || {
                match worker.execute(
                    Priority::VisiblePage,
                    Job::FilePageCount { path: count_path },
                ) {
                    JobResult::PageCount(count) => Some(count),
                    _ => None,
                }
            })
            .await
            .ok()
            .flatten();
            Message::PagesKnown { path, pages }
        })
    }

    /// Remove the page entries of the expanded PDF at `path`.
    fn collapse_pdf(&mut self, tab: Entity, path: &PathBuf) {
        let Some(state) = self.tab_ui.get_mut(&tab) else {
            return;
        };
        if state.expanded.as_ref() == Some(path) {
            state.expanded = None;
        }
        let Some(start) = state
            .strip
            .iter()
            .position(|entry| matches!(&entry.target, NavEntry::File { path: p } if p == path))
        else {
            return;
        };
        let mut end = start + 1;
        while end < state.strip.len() && matches!(state.strip[end].target, NavEntry::Page { .. }) {
            end += 1;
        }
        state.strip.drain(start + 1..end);
    }

    /// Insert the page entries of an expanded PDF after its file entry.
    fn insert_pages(&mut self, tab: Entity, path: &PathBuf, pages: u32) {
        let Some(state) = self.tab_ui.get_mut(&tab) else {
            return;
        };
        let Some(start) = state
            .strip
            .iter()
            .position(|entry| matches!(&entry.target, NavEntry::File { path: p } if p == path))
        else {
            return;
        };
        let page_entries = (1..=pages).map(|page| StripEntry {
            target: NavEntry::Page {
                path: path.clone(),
                page,
            },
            name: fl!("page-num", num = page),
            thumb: None,
            expandable: false,
        });
        state.strip.splice(start + 1..start + 1, page_entries);
    }

    /// Render a raster or SVG file into the content area at native resolution.
    /// The image viewer scales it for zooming.
    fn render_file_task(&self, path: PathBuf) -> iced::Task<cosmic::Action<Message>> {
        let render_path = path.clone();
        cosmic::task::future(async move {
            let rgba = tokio::task::spawn_blocking(move || {
                render::render_path(&render_path, 1.0)
                    .ok()
                    .map(|rendered| (rendered.width, rendered.height, rendered.rgba_data))
            })
            .await
            .ok()
            .flatten();
            Message::FileRendered { path, rgba }
        })
    }

    /// Render a PDF page into the content area at native resolution. The
    /// image viewer scales it for zooming.
    fn render_page_task(&self, path: PathBuf, page: u32) -> iced::Task<cosmic::Action<Message>> {
        let worker = self.worker.clone();
        let render_path = path.clone();
        cosmic::task::future(async move {
            let rgba =
                tokio::task::spawn_blocking(move || worker.render_page(&render_path, page, 1.0))
                    .await
                    .ok()
                    .flatten();
            Message::PageRendered { path, page, rgba }
        })
    }

    /// Re-render the visible window of the PDF preview at `zoom`.
    fn render_preview_at(
        &mut self,
        path: &PathBuf,
        zoom: f32,
    ) -> iced::Task<cosmic::Action<Message>> {
        let Some(tab) = self.active_tab() else {
            return iced::Task::none();
        };
        let visible = {
            let Some(state) = self.tab_ui.get_mut(&tab) else {
                return iced::Task::none();
            };
            let Some(preview) = state.preview.as_mut() else {
                return iced::Task::none();
            };
            if preview.path != *path {
                return iced::Task::none();
            }
            preview.zoom = zoom;
            preview.full_order.clear();
            preview.requested = None;
            for slot in preview.pages.iter_mut() {
                if matches!(slot, PageSlot::Full { .. }) {
                    *slot = PageSlot::Empty;
                }
            }
            Self::preview_visible_range(preview)
        };
        self.request_preview_window(path, visible)
    }

    /// Change the zoom of a PDF preview by `steps` (signed) and re-render.
    fn zoom_preview(&mut self, path: &PathBuf, steps: f32) -> iced::Task<cosmic::Action<Message>> {
        let zoom = self.preview_zoom(path).unwrap_or(1.0);
        let zoom = (zoom * ZOOM_STEP.powf(steps)).clamp(0.05, 8.0);
        self.render_preview_at(path, zoom)
    }

    /// Scroll the active preview by one document page. `direction` is -1 (up)
    /// or +1 (down). No-op when the active tab has no continuous preview.
    fn scroll_preview_page(&mut self, direction: i32) -> iced::Task<cosmic::Action<Message>> {
        let Some(tab) = self.active_tab() else {
            return iced::Task::none();
        };
        let Some(state) = self.tab_ui.get(&tab) else {
            return iced::Task::none();
        };
        let Some(preview) = state.preview.as_ref() else {
            return iced::Task::none();
        };
        if preview.page_sizes.is_empty() {
            return iced::Task::none();
        }

        // The page whose top is currently at or above the viewport top.
        let mut current = 0usize;
        let mut top = 0.0;
        for (index, (_, height)) in preview.page_sizes.iter().enumerate() {
            let bottom = top + height * preview.zoom;
            if bottom > preview.scroll + 1.0 {
                current = index;
                break;
            }
            top = bottom;
        }

        let last = preview.page_sizes.len() - 1;
        let target = (current as i32 + direction).clamp(0, last as i32) as usize;
        let target_y = preview.page_sizes[..target]
            .iter()
            .map(|(_, height)| height * preview.zoom)
            .sum();

        scroll_to::<Message>(
            cosmic::widget::Id::new(PREVIEW_SCROLL_ID),
            AbsoluteOffset {
                x: None,
                y: Some(target_y),
            },
        )
        .map(cosmic::Action::from)
    }

    /// Change the zoom by `steps` (signed). Single images are scaled by the
    /// viewer; PDF previews re-render at the new zoom.
    fn zoom_by(&mut self, steps: f32) -> iced::Task<cosmic::Action<Message>> {
        let Some(target) = self.current_target.clone() else {
            return iced::Task::none();
        };
        match target {
            CurrentTarget::File { path } if storage::document::is_pdf(&path).unwrap_or(false) => {
                self.zoom_preview(&path, steps)
            }
            target => {
                let state = self.zoom_states.entry(target).or_default();
                if state.fit {
                    // Leave fit. Zoom in lands on native 100 %; zoom out
                    // starts one step below native.
                    state.fit = false;
                    state.scale = if steps > 0.0 { 1.0 } else { 1.0 / ZOOM_STEP };
                } else {
                    state.scale = (state.scale * ZOOM_STEP.powf(steps)).clamp(MIN_SCALE, MAX_SCALE);
                }
                iced::Task::none()
            }
        }
    }

    /// Set the zoom of the current target to `scale` (1.0 = 100 %).
    fn set_zoom(&mut self, scale: f32) -> iced::Task<cosmic::Action<Message>> {
        let Some(target) = self.current_target.clone() else {
            return iced::Task::none();
        };
        match target {
            CurrentTarget::File { path } if storage::document::is_pdf(&path).unwrap_or(false) => {
                self.render_preview_at(&path, scale)
            }
            target => {
                let state = self.zoom_states.entry(target).or_default();
                state.fit = false;
                state.scale = scale.clamp(MIN_SCALE, MAX_SCALE);
                state.offset_x = 0.0;
                state.offset_y = 0.0;
                iced::Task::none()
            }
        }
    }

    /// Fit the current target to the viewport (single images only).
    fn set_fit(&mut self) {
        let Some(target) = self.current_target.clone() else {
            return;
        };
        if let CurrentTarget::File { path } = &target
            && storage::document::is_pdf(path).unwrap_or(false)
        {
            return;
        }
        let state = self.zoom_states.entry(target).or_default();
        state.fit = true;
        state.scale = 1.0;
        state.offset_x = 0.0;
        state.offset_y = 0.0;
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

    /// Create a folder tab and return its entity plus the asynchronous
    /// folder-listing task.
    fn open_folder_tab(&mut self, dir: PathBuf) -> (Entity, iced::Task<cosmic::Action<Message>>) {
        let title = storage::browser::display_name(&dir);
        let tab = self.tab_model.insert().text(title).closable().id();
        self.tabs.insert(
            tab,
            TabContent::Folder {
                path: dir.clone(),
                entries: Vec::new(),
            },
        );
        self.tab_ui.insert(tab, TabUiState::new(Vec::new()));

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

        (tab, list)
    }

    /// Start listing a folder and open a tab for it.
    fn open_folder(&mut self, dir: PathBuf) -> iced::Task<cosmic::Action<Message>> {
        let (tab, list) = self.open_folder_tab(dir);
        self.tab_model.activate(tab);
        let activate = self.activate_tab();
        cosmic::task::batch(vec![activate, list])
    }

    /// Open a tab for a start path: folders list, files open their parent
    /// folder with the file pre-selected.
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
    pub(crate) fn restore_session(
        &mut self,
        session: Session,
    ) -> iced::Task<cosmic::Action<Message>> {
        let mut tasks: Vec<iced::Task<cosmic::Action<Message>>> = Vec::new();

        for (index, path) in session.browser_tabs.into_iter().enumerate() {
            if path.is_dir() {
                let (tab, list) = self.open_folder_tab(path);
                if index == session.active_tab {
                    self.tab_model.activate(tab);
                }
                tasks.push(list);
            } else {
                // Browser tabs now hold only folders; document paths are stale
                // because multi-page PDFs expand inside their folder tab.
                tracing::warn!("restore: skipping non-folder path {path:?}");
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

    /// Close a tab; activates a neighbor when the active tab was closed.
    fn close_tab(&mut self, tab: Entity) -> iced::Task<cosmic::Action<Message>> {
        let was_active = self.active_tab() == Some(tab);
        self.tab_model.remove(tab);
        self.tabs.remove(&tab);
        self.tab_ui.remove(&tab);

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

    /// Request a redraw so a freshly rendered image is uploaded and drawn.
    /// The image viewer has no handle cache, so updating `current_image` does
    /// not schedule a redraw on its own in the Wayland event loop.
    fn request_redraw(&self) -> iced::Task<cosmic::Action<Message>> {
        iced::runtime::task::effect(iced::runtime::Action::Window(
            iced::window::Action::RedrawAll,
        ))
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

            Message::StripActivated(index) => {
                let Some(tab) = self.active_tab() else {
                    return iced::Task::none();
                };
                let Some(target) = self
                    .tab_ui
                    .get(&tab)
                    .and_then(|state| state.strip.get(index))
                    .map(|entry| entry.target.clone())
                else {
                    return iced::Task::none();
                };
                if let Some(state) = self.tab_ui.get_mut(&tab) {
                    state.selected = Some(index);
                }
                return self.activate_target(target);
            }

            Message::PrevEntry => {
                let Some(tab) = self.active_tab() else {
                    return iced::Task::none();
                };

                let target = {
                    let Some(state) = self.tab_ui.get_mut(&tab) else {
                        return iced::Task::none();
                    };
                    let Some(previous) =
                        state.selected.and_then(|selected| selected.checked_sub(1))
                    else {
                        return iced::Task::none();
                    };
                    let Some(entry) = state.strip.get(previous) else {
                        return iced::Task::none();
                    };

                    state.selected = Some(previous);
                    entry.target.clone()
                };

                return self.activate_target(target);
            }

            Message::NextEntry => {
                let Some(tab) = self.active_tab() else {
                    return iced::Task::none();
                };

                let target = {
                    let Some(state) = self.tab_ui.get_mut(&tab) else {
                        return iced::Task::none();
                    };
                    let Some(next) = state.selected.and_then(|selected| selected.checked_add(1))
                    else {
                        return iced::Task::none();
                    };
                    let Some(entry) = state.strip.get(next) else {
                        return iced::Task::none();
                    };

                    state.selected = Some(next);
                    entry.target.clone()
                };

                return self.activate_target(target);
            }

            Message::PrevPage => {
                return self.scroll_preview_page(-1);
            }

            Message::NextPage => {
                return self.scroll_preview_page(1);
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
                            self.rebuild_strip(tab);
                            if let Some(path) = self.pending_select.take()
                                && let Some(state) = self.tab_ui.get_mut(&tab)
                            {
                                state.selected = state.strip.iter().position(|entry| {
                                    matches!(&entry.target, NavEntry::File { path: p } if p == &path)
                                });
                            }
                            return self.activate_tab();
                        }
                    }
                    (Some(_), Err(e)) => {
                        tracing::error!("failed to list folder: {e}");
                    }
                    (None, _) => {}
                }
            }

            Message::PagesKnown { path, pages } => {
                let Some(tab) = self.active_tab() else {
                    return iced::Task::none();
                };
                match pages {
                    Some(pages) => {
                        self.insert_pages(tab, &path, pages);
                        let range = {
                            let Some(state) = self.tab_ui.get(&tab) else {
                                return iced::Task::none();
                            };
                            let Some(start) = state.strip.iter().position(|entry| {
                                matches!(&entry.target, NavEntry::File { path: p } if p == &path)
                            }) else {
                                return iced::Task::none();
                            };
                            let end = (start + 1 + STRIP_INITIAL_THUMBS).min(state.strip.len());
                            start + 1..end
                        };
                        return self.request_strip_thumbs(tab, range);
                    }
                    None => {
                        tracing::error!("failed to count pages of {path:?}");
                        self.collapse_pdf(tab, &path);
                    }
                }
            }

            Message::StripThumbsReady { tab, thumbs } => {
                if let Some(state) = self.tab_ui.get_mut(&tab) {
                    for (index, rgba) in thumbs {
                        if let Some(entry) = state.strip.get_mut(index)
                            && let Some((width, height, rgba)) = rgba
                        {
                            entry.thumb = Some(cosmic::widget::image::Handle::from_rgba(
                                width, height, rgba,
                            ));
                        }
                    }
                }
            }

            Message::PreviewSizesKnown { path, sizes } => {
                // Only for the target currently shown.
                if self.current_target != Some(CurrentTarget::File { path: path.clone() }) {
                    return iced::Task::none();
                }
                match sizes {
                    Some(sizes) if sizes.len() <= 1 => {
                        // Single-page PDF: display as one image.
                        self.current_target = Some(CurrentTarget::Page {
                            path: path.clone(),
                            page: 1,
                        });
                        return self.render_page_task(path, 1);
                    }
                    Some(sizes) => {
                        return self.build_preview(path, sizes);
                    }
                    None => {
                        tracing::error!("failed to read page sizes of {path:?}");
                    }
                }
            }

            Message::PreviewPagesRendered { path, zoom, pages } => {
                let Some(tab) = self.active_tab() else {
                    return iced::Task::none();
                };
                let Some(state) = self.tab_ui.get_mut(&tab) else {
                    return iced::Task::none();
                };
                let Some(preview) = state.preview.as_mut() else {
                    return iced::Task::none();
                };
                if preview.path != path {
                    return iced::Task::none();
                }

                let is_thumb = (zoom - THUMB_ZOOM).abs() < f32::EPSILON;
                let is_full = !is_thumb && (zoom - preview.zoom).abs() < f32::EPSILON;
                if !is_thumb && !is_full {
                    // Renders that raced with a zoom change are dropped.
                    return iced::Task::none();
                }

                for (page, width, height, rgba) in pages {
                    let index = page as usize - 1;
                    if index >= preview.pages.len() {
                        continue;
                    }
                    let handle = cosmic::widget::image::Handle::from_rgba(width, height, rgba);
                    if is_thumb {
                        if matches!(preview.pages[index], PageSlot::Empty) {
                            preview.pages[index] = PageSlot::Thumb { handle };
                        }
                    } else {
                        preview.pages[index] = PageSlot::Full { handle };
                        if !preview.full_order.contains(&page) {
                            preview.full_order.push(page);
                        }
                    }
                }

                // LRU cap: drop the oldest full pages back to placeholders.
                while preview.full_order.len() > PREVIEW_FULL_CACHE {
                    let oldest = preview.full_order.remove(0);
                    if let Some(slot) = preview.pages.get_mut(oldest as usize - 1) {
                        *slot = PageSlot::Empty;
                    }
                }

                return self.request_redraw();
            }

            Message::PreviewScrolled {
                path,
                offset_y,
                viewport_height,
            } => {
                let Some(tab) = self.active_tab() else {
                    return iced::Task::none();
                };
                let visible = {
                    let Some(state) = self.tab_ui.get_mut(&tab) else {
                        return iced::Task::none();
                    };
                    let Some(preview) = state.preview.as_mut() else {
                        return iced::Task::none();
                    };
                    if preview.path != path {
                        return iced::Task::none();
                    }
                    preview.scroll = offset_y;
                    preview.viewport_height = viewport_height;
                    let visible = Self::preview_visible_range(preview);
                    // Re-request only when the visible window changed; plain
                    // scroll ticks within one window must stay quiet.
                    if preview.requested == visible {
                        return iced::Task::none();
                    }
                    preview.requested = visible;
                    visible
                };
                return self.request_preview_window(&path, visible);
            }

            Message::PreviewWheel { path, delta } => {
                let steps = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y,
                    mouse::ScrollDelta::Pixels { y, .. } => y / 50.0,
                };
                if steps == 0.0 {
                    return iced::Task::none();
                }
                return self.zoom_preview(&path, steps);
            }

            Message::StripScrolled {
                tab,
                offset_y,
                viewport_height,
            } => {
                let Some(state) = self.tab_ui.get(&tab) else {
                    return iced::Task::none();
                };
                if state.strip.is_empty() {
                    return iced::Task::none();
                }
                let first = (offset_y / STRIP_TILE).floor() as usize;
                let last = ((offset_y + viewport_height) / STRIP_TILE).ceil() as usize + 2;
                return self.request_strip_thumbs(tab, first..last.min(state.strip.len()));
            }

            Message::StripDoubleClicked(index) => {
                return self.toggle_expand(index);
            }

            Message::FileRendered { path, rgba } => {
                // Ignore renders that raced with a target change.
                if self.current_target == Some(CurrentTarget::File { path }) {
                    self.apply_image(rgba);
                    return self.request_redraw();
                }
            }

            Message::PageRendered { path, page, rgba } => {
                // Ignore renders that raced with a target change.
                if self.current_target == Some(CurrentTarget::Page { path, page }) {
                    self.apply_image(rgba);
                    return self.request_redraw();
                }
            }

            Message::ViewerStateChanged {
                target,
                scale,
                offset_x,
                offset_y,
            } => {
                let state = self.zoom_states.entry(target).or_default();
                state.fit = false;
                state.scale = scale.clamp(MIN_SCALE, MAX_SCALE);
                state.offset_x = offset_x;
                state.offset_y = offset_y;
            }

            Message::ZoomIn => return self.zoom_by(1.0),

            Message::ZoomOut => return self.zoom_by(-1.0),

            Message::Zoom100 => {
                return self.set_zoom(1.0);
            }

            Message::SetZoom(scale) => {
                return self.set_zoom(scale);
            }

            Message::ModifiersChanged(modifiers) => {
                self.keyboard_modifiers = modifiers;
            }

            Message::ZoomToFit => {
                self.set_fit();
            }

            Message::ToggleFullscreen => {
                if let Some(window_id) = self.core.main_window_id() {
                    return cosmic::command::toggle_maximize(window_id);
                }
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
                return iced::exit();
            }
        }
        iced::Task::none()
    }
}
