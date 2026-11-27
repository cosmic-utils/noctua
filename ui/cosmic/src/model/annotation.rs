// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/model/annotation.rs
//
// Volatile view state of an annotation tab: page list, thumbnails and the
// currently shown page. Pure data; no tasks, no widgets.

use cosmic::iced::Point;
use cosmic::widget::image::Handle;

use noctua_core::pdfium_ops::FormField;
use noctua_core::pdfium_ops::model::AnnotationColor;

use super::{CurrentImage, ViewState};

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
    pub(crate) thumbs: Vec<Option<Handle>>,
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
    /// Interactive form fields on the selected page.
    pub(crate) form_fields: Vec<FormField>,
    /// Focused form field: an index into `form_fields`.
    pub(crate) form_focus: Option<usize>,
    /// Text being edited in the focused text or choice field.
    pub(crate) form_draft: String,
    /// Caret position in `form_draft`, in characters.
    pub(crate) form_caret: usize,
    /// Value the focused field had when focus started, restored on Escape.
    pub(crate) form_original: String,
    /// Anchor of the open option list of a choice field, in viewer coordinates
    /// (origin top left). `None` while the list is closed.
    pub(crate) form_menu: Option<Point>,
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
            form_fields: Vec::new(),
            form_focus: None,
            form_draft: String::new(),
            form_caret: 0,
            form_original: String::new(),
            form_menu: None,
        }
    }
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
