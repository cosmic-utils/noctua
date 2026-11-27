// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/model/browser.rs
//
// Per-tab browser state: the thumbnail strip, selection and the continuous
// PDF preview. Pure data; no tasks, no widgets.

use std::path::PathBuf;

use cosmic::widget::image::Handle;

use super::{Target, ViewState};

/// One entry of the thumbnail strip of a tab.
pub(crate) struct StripEntry {
    pub(crate) target: Target,
    pub(crate) name: String,
    pub(crate) thumb: Option<Handle>,
    /// Whether the entry is a PDF that can be expanded in place.
    pub(crate) expandable: bool,
    /// Per-entry view state (zoom/pan/rotate/flip). Owned by the entry, not
    /// copied into the tab or a side map.
    pub(crate) view: ViewState,
}

/// One page of the continuous PDF preview.
pub(crate) enum PageSlot {
    /// Sizing placeholder; no pixels yet.
    Empty,
    /// Low-resolution placeholder from the thumbnail pass.
    Thumb { handle: Handle },
    /// Full-resolution render.
    Full { handle: Handle },
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
/// Survives tab switches but is not persisted in the session. Mutations go
/// through named methods so selection, preview and expand state stay explicit.
pub(crate) struct TabUiState {
    entries: Vec<StripEntry>,
    selected: Option<usize>,
    preview: Option<DocumentPreview>,
    /// Path of the PDF currently expanded in the strip, if any.
    expanded: Option<PathBuf>,
}

impl TabUiState {
    /// A fresh tab state selecting the first entry, if any.
    pub(crate) fn new(entries: Vec<StripEntry>) -> Self {
        let selected = (!entries.is_empty()).then_some(0);
        Self {
            entries,
            selected,
            preview: None,
            expanded: None,
        }
    }

    /// Rebuild the state from its parts (a folder rescan keeps the view state).
    pub(crate) fn from_parts(
        entries: Vec<StripEntry>,
        selected: Option<usize>,
        preview: Option<DocumentPreview>,
        expanded: Option<PathBuf>,
    ) -> Self {
        Self {
            entries,
            selected,
            preview,
            expanded,
        }
    }

    /// The strip entries, in order.
    pub(crate) fn entries(&self) -> &[StripEntry] {
        &self.entries
    }

    /// Mutable access to the strip entries (push, splice, drain).
    pub(crate) fn entries_mut(&mut self) -> &mut Vec<StripEntry> {
        &mut self.entries
    }

    /// The selected strip index.
    pub(crate) fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Set the selected strip index.
    pub(crate) fn set_selected(&mut self, selected: Option<usize>) {
        self.selected = selected;
    }

    /// The continuous PDF preview, if one is shown.
    pub(crate) fn preview(&self) -> Option<&DocumentPreview> {
        self.preview.as_ref()
    }

    /// Mutable access to the continuous PDF preview.
    pub(crate) fn preview_mut(&mut self) -> Option<&mut DocumentPreview> {
        self.preview.as_mut()
    }

    /// Replace the continuous PDF preview.
    pub(crate) fn set_preview(&mut self, preview: Option<DocumentPreview>) {
        self.preview = preview;
    }

    /// Take the preview out of the state, consuming it.
    pub(crate) fn into_preview(self) -> Option<DocumentPreview> {
        self.preview
    }

    /// The path of the PDF expanded in the strip, if any.
    pub(crate) fn expanded(&self) -> Option<&PathBuf> {
        self.expanded.as_ref()
    }

    /// Set the path of the PDF expanded in the strip.
    pub(crate) fn set_expanded(&mut self, expanded: Option<PathBuf>) {
        self.expanded = expanded;
    }
}
