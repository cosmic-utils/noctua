// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/model/annotation.rs
//
// Volatile view state of an annotation tab: page list, thumbnails and the
// currently shown page. Pure data; no tasks, no widgets. The document itself
// lives in the worker, keyed by the tab's `DocumentId`.

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

/// Volatile view state of an annotation tab. Every mutation goes through a
/// named method, so page, tool and form transitions stay explicit and the
/// invariants (e.g. "selecting a page clears the form focus") live in one
/// place.
pub(crate) struct AnnotationUiState {
    /// Native page sizes in points (≈ px at 72 dpi).
    page_sizes: Vec<(f32, f32)>,
    /// 1-based index of the selected page.
    selected: u32,
    /// Per-page thumbnail handles; `None` until rendered.
    thumbs: Vec<Option<Handle>>,
    /// Full-resolution pixels of the selected page, for the content view.
    current: Option<CurrentImage>,
    /// View transform (zoom/pan/rotate/flip), like single images.
    view: ViewState,
    /// Currently active annotation tool; `None` means browse/pan.
    active_tool: Option<AnnotationTool>,
    /// Selected free-text colour.
    annotation_color: AnnotationColor,
    /// Selected free-text font size in points.
    font_size_pt: f32,
    /// An in-progress free-text placement; `None` when no text is being entered.
    pending_text: Option<PendingText>,
    /// Whether the insertion marker is currently shown; toggled by the blink
    /// timer while a text edit is active.
    caret_visible: bool,
    /// Interactive form fields on the selected page.
    form_fields: Vec<FormField>,
    /// Focused form field: an index into `form_fields`.
    form_focus: Option<usize>,
    /// Text being edited in the focused text or choice field.
    form_draft: String,
    /// Caret position in `form_draft`, in characters.
    form_caret: usize,
    /// Value the focused field had when focus started, restored on Escape.
    form_original: String,
    /// Anchor of the open option list of a choice field, in viewer coordinates
    /// (origin top left). `None` while the list is closed.
    form_menu: Option<Point>,
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

impl AnnotationUiState {
    /// The 1-based index of the selected page.
    pub(crate) fn selected(&self) -> u32 {
        self.selected
    }

    /// Native page sizes in points, in page order.
    pub(crate) fn page_sizes(&self) -> &[(f32, f32)] {
        &self.page_sizes
    }

    /// Per-page thumbnail handles, in page order.
    pub(crate) fn thumbs(&self) -> &[Option<Handle>] {
        &self.thumbs
    }

    /// Full-resolution pixels of the selected page, if rendered.
    pub(crate) fn current(&self) -> Option<&CurrentImage> {
        self.current.as_ref()
    }

    /// The view transform of the annotation tab.
    pub(crate) fn view(&self) -> ViewState {
        self.view
    }

    /// Mutable access to the view transform.
    pub(crate) fn view_mut(&mut self) -> &mut ViewState {
        &mut self.view
    }

    /// The rendered page image and its view transform together, for view
    /// operations that read both (rotate/flip while the image is shown).
    pub(crate) fn image_view_mut(&mut self) -> (Option<&mut CurrentImage>, &mut ViewState) {
        (self.current.as_mut(), &mut self.view)
    }

    /// The currently active annotation tool.
    pub(crate) fn active_tool(&self) -> Option<AnnotationTool> {
        self.active_tool
    }

    /// The selected free-text colour.
    pub(crate) fn annotation_color(&self) -> AnnotationColor {
        self.annotation_color
    }

    /// The selected free-text font size in points.
    pub(crate) fn font_size_pt(&self) -> f32 {
        self.font_size_pt
    }

    /// The in-progress free-text placement, if any.
    pub(crate) fn pending_text(&self) -> Option<&PendingText> {
        self.pending_text.as_ref()
    }

    /// Mutable access to the in-progress free-text placement, if any.
    pub(crate) fn pending_text_mut(&mut self) -> Option<&mut PendingText> {
        self.pending_text.as_mut()
    }

    /// Whether the insertion marker is currently shown.
    pub(crate) fn caret_visible(&self) -> bool {
        self.caret_visible
    }

    /// The interactive form fields of the selected page.
    pub(crate) fn form_fields(&self) -> &[FormField] {
        &self.form_fields
    }

    /// The focused form field index, if any.
    pub(crate) fn form_focus(&self) -> Option<usize> {
        self.form_focus
    }

    /// The text being edited in the focused field.
    pub(crate) fn form_draft(&self) -> &str {
        &self.form_draft
    }

    /// The caret position inside `form_draft`, in characters.
    pub(crate) fn form_caret(&self) -> usize {
        self.form_caret
    }

    /// The anchor of the open option list, if any.
    pub(crate) fn form_menu(&self) -> Option<Point> {
        self.form_menu
    }

    /// Replace the page sizes (a re-render or page mutation) and reset the
    /// dependent per-page state: thumbnails, the current image and the clamped
    /// selection.
    pub(crate) fn set_page_sizes(&mut self, sizes: Vec<(f32, f32)>) {
        let page_count = sizes.len();
        self.page_sizes = sizes;
        self.thumbs = vec![None; page_count];
        self.selected = self.selected.clamp(1, page_count.max(1) as u32);
        self.current = None;
    }

    /// Select a page and drop the state that belongs to the previous page.
    pub(crate) fn select_page(&mut self, page: u32) {
        self.selected = page;
        self.current = None;
        self.form_fields.clear();
        self.form_focus = None;
        self.form_menu = None;
    }

    /// Store the rendered thumbnail of one page (1-based).
    pub(crate) fn set_thumb(&mut self, page: u32, handle: Option<Handle>) {
        if let Some(slot) = self.thumbs.get_mut(page as usize - 1) {
            *slot = handle;
        }
    }

    /// Replace the current page image, if the render is still for the selected
    /// page. Returns `true` when the render was applied.
    pub(crate) fn apply_page_render(&mut self, page: u32, image: Option<CurrentImage>) -> bool {
        if self.selected != page {
            return false;
        }
        self.current = image;
        true
    }

    /// Store the view transform reported by the viewer and close any option
    /// list anchored to the old position.
    pub(crate) fn set_view_state(&mut self, scale: f32, offset_x: f32, offset_y: f32) {
        self.view.fit = false;
        self.view.scale = scale.clamp(super::MIN_SCALE, super::MAX_SCALE);
        self.view.offset_x = offset_x;
        self.view.offset_y = offset_y;
        self.form_menu = None;
    }

    /// Toggle `tool` on or off.
    pub(crate) fn toggle_tool(&mut self, tool: AnnotationTool) {
        self.active_tool = if self.active_tool == Some(tool) {
            None
        } else {
            Some(tool)
        };
    }

    /// Set the free-text font size in points.
    pub(crate) fn set_font_size(&mut self, size: f32) {
        self.font_size_pt = size;
    }

    /// Set the free-text colour.
    pub(crate) fn set_color(&mut self, color: AnnotationColor) {
        self.annotation_color = color;
    }

    /// Start editing a free-text annotation.
    pub(crate) fn begin_text_edit(&mut self, pending: PendingText) {
        self.pending_text = Some(pending);
        self.caret_visible = true;
    }

    /// Take the pending text edit, ending it.
    pub(crate) fn take_pending_text(&mut self) -> Option<PendingText> {
        self.pending_text.take()
    }

    /// Show or hide the insertion marker.
    pub(crate) fn set_caret_visible(&mut self, visible: bool) {
        self.caret_visible = visible;
    }

    /// Toggle the insertion marker while a text edit is active.
    pub(crate) fn toggle_caret(&mut self) {
        if self.pending_text.is_some() {
            self.caret_visible = !self.caret_visible;
        }
    }

    /// Replace the form fields of the selected page and drop any focus.
    pub(crate) fn set_form_fields(&mut self, fields: Vec<FormField>) {
        self.form_fields = fields;
        self.form_focus = None;
        self.form_menu = None;
    }

    /// Begin editing the form field at `index` with the given current value.
    /// `menu` opens the option list of a choice field, if any.
    pub(crate) fn begin_form_edit(&mut self, index: usize, value: String, menu: Option<Point>) {
        self.form_focus = Some(index);
        self.form_draft = value.clone();
        self.form_original = value;
        self.form_caret = self.form_draft.chars().count();
        self.caret_visible = true;
        self.form_menu = menu;
    }

    /// Drop the form focus and any open option list.
    pub(crate) fn clear_form_focus(&mut self) {
        self.form_focus = None;
        self.form_menu = None;
    }

    /// Close the option list of a choice field.
    pub(crate) fn close_form_menu(&mut self) {
        self.form_menu = None;
    }

    /// Mutable access to the focused field's draft text and caret, for the
    /// shared text-editing helpers.
    pub(crate) fn form_draft_mut(&mut self) -> (&mut String, &mut usize) {
        (&mut self.form_draft, &mut self.form_caret)
    }

    /// Restore the focused field's value to what it held when focus began.
    pub(crate) fn restore_form_draft(&mut self) {
        self.form_draft.clone_from(&self.form_original);
        self.form_menu = None;
    }

    /// Move the form caret by `delta` characters, clamped to the draft.
    pub(crate) fn move_form_caret(&mut self, delta: i32) {
        let len = self.form_draft.chars().count() as i32;
        self.form_caret = (self.form_caret as i32 + delta).clamp(0, len) as usize;
    }

    /// Set the form caret to an absolute character offset.
    pub(crate) fn set_form_caret(&mut self, caret: usize) {
        self.form_caret = caret;
    }

    /// Set the form caret to the end of the draft.
    pub(crate) fn set_form_caret_to_end(&mut self) {
        self.form_caret = self.form_draft.chars().count();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn image() -> CurrentImage {
        CurrentImage {
            handle: Handle::from_rgba(1, 1, vec![0, 0, 0, 255]),
            rgba: (1, 1, vec![0, 0, 0, 255]),
        }
    }

    fn pending() -> PendingText {
        PendingText {
            index: 0,
            original_text: String::new(),
            is_new: true,
            text: String::new(),
            cursor: 0,
            origin: (0.0, 0.0),
            max_width: 100.0,
            caret: (0.0, 0.0, 0.0, 0.0),
        }
    }

    #[test]
    fn select_page_clears_dependent_state() {
        let mut state = AnnotationUiState::default();
        state.set_page_sizes(vec![(100.0, 100.0), (200.0, 200.0)]);
        state.begin_form_edit(0, "draft".to_string(), Some(Point::new(1.0, 2.0)));

        state.select_page(2);

        assert_eq!(state.selected(), 2);
        assert!(state.current().is_none());
        assert!(state.form_fields().is_empty());
        assert!(state.form_focus().is_none());
        assert!(state.form_menu().is_none());
    }

    #[test]
    fn set_page_sizes_clamps_selection_and_resets_thumbs() {
        let mut state = AnnotationUiState::default();
        state.select_page(5);

        state.set_page_sizes(vec![(10.0, 10.0)]);

        assert_eq!(state.selected(), 1);
        assert_eq!(state.thumbs().len(), 1);
        assert!(state.current().is_none());
    }

    #[test]
    fn restore_form_draft_reverts_to_original() {
        let mut state = AnnotationUiState::default();
        state.begin_form_edit(0, "original".to_string(), None);
        let (draft, _) = state.form_draft_mut();
        *draft = "changed".to_string();

        state.restore_form_draft();

        assert_eq!(state.form_draft(), "original");
        assert!(state.form_menu().is_none());
    }

    #[test]
    fn move_form_caret_clamps_to_draft() {
        let mut state = AnnotationUiState::default();
        state.begin_form_edit(0, "abc".to_string(), None);
        assert_eq!(state.form_caret(), 3);

        state.move_form_caret(-10);
        assert_eq!(state.form_caret(), 0);

        state.move_form_caret(10);
        assert_eq!(state.form_caret(), 3);
    }

    #[test]
    fn toggle_tool_switches_on_and_off() {
        let mut state = AnnotationUiState::default();
        assert!(state.active_tool().is_none());

        state.toggle_tool(AnnotationTool::Text);
        assert_eq!(state.active_tool(), Some(AnnotationTool::Text));

        state.toggle_tool(AnnotationTool::Text);
        assert!(state.active_tool().is_none());
    }

    #[test]
    fn text_edit_begin_and_take() {
        let mut state = AnnotationUiState::default();
        state.begin_text_edit(pending());
        assert!(state.pending_text().is_some());
        assert!(state.caret_visible());

        assert!(state.take_pending_text().is_some());
        assert!(state.pending_text().is_none());
    }

    #[test]
    fn apply_page_render_drops_stale_renders() {
        let mut state = AnnotationUiState::default();
        state.select_page(2);

        // A render for a page that is no longer selected is dropped.
        assert!(!state.apply_page_render(1, Some(image())));
        assert!(state.current().is_none());

        // A render for the selected page is applied.
        assert!(state.apply_page_render(2, Some(image())));
        assert!(state.current().is_some());
    }
}
