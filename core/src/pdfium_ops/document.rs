// SPDX-License-Identifier: GPL-3.0-only
// core/src/pdfium_ops/document.rs
//
// Owned PDF document and pages, created through the raw pdfium bindings.
//
// Owning the document handle is what makes pdfium's form API usable: listing,
// filling and drawing interactive fields all need the per-document form-fill
// environment, whose handle pdfium-render keeps private. Annotations stay on
// the plain `FPDFAnnot_*` API; the environment only governs widget fields.
//
// All calls happen on the pdfium worker thread only.

use std::ffi::c_void;
use std::os::raw::{c_int, c_ulong};
use std::pin::Pin;

use pdfium_render::prelude::{
    FPDF_ANNOTATION, FPDF_DOCUMENT, FPDF_FILEWRITE, FPDF_FORMFILLINFO, FPDF_FORMHANDLE, FPDF_PAGE,
    FPDF_PAGEOBJECT, FS_RECTF, PdfiumLibraryBindings,
};

use super::bindings::pdfium_or_err;
use super::error::PdfOpsError;

/// Fill the bitmap white; pdfium does not paint the background itself.
const WHITE: u64 = 0xFFFF_FFFF;

/// `FPDFBitmap_BGRA`, pdfium's native 32-bit layout.
const BITMAP_BGRA: i32 = 4;

/// `FPDF_ANNOT`, draw annotation appearances.
const RENDER_ANNOTATIONS: c_int = 0x01;

/// `FPDF_ANNOT_WIDGET`.
const WIDGET_SUBTYPE: c_int = 20;

/// `FPDF_ANNOT_APPEARANCEMODE_NORMAL`.
#[derive(Clone, Copy)]
enum AppearanceMode {
    Normal = 0,
}

/// An open PDF document owned by us, with its form-fill environment.
pub(crate) struct Document {
    bindings: &'static dyn PdfiumLibraryBindings,
    handle: FPDF_DOCUMENT,
    form: FPDF_FORMHANDLE,
    /// pdfium keeps this pointer for the environment's lifetime.
    _form_info: Pin<Box<FPDF_FORMFILLINFO>>,
    /// pdfium keeps pointers into the loaded bytes for the document's lifetime.
    _bytes: Vec<u8>,
}

impl Document {
    /// Load a document from memory and attach a form-fill environment.
    pub(crate) fn open(bytes: Vec<u8>) -> Result<Self, PdfOpsError> {
        let bindings = pdfium_or_err()?.bindings();
        let handle = bindings.FPDF_LoadMemDocument64(&bytes, None);
        if handle.is_null() {
            return Err(PdfOpsError::Pdfium(format!(
                "failed to load PDF (pdfium error {})",
                bindings.FPDF_GetLastError()
            )));
        }
        Self::attach(bindings, handle, bytes)
    }

    /// Create an empty document and attach a form-fill environment.
    pub(crate) fn create() -> Result<Self, PdfOpsError> {
        let bindings = pdfium_or_err()?.bindings();
        let handle = bindings.FPDF_CreateNewDocument();
        if handle.is_null() {
            return Err(PdfOpsError::Pdfium(
                "failed to create a PDF document".to_string(),
            ));
        }
        Self::attach(bindings, handle, Vec::new())
    }

    fn attach(
        bindings: &'static dyn PdfiumLibraryBindings,
        handle: FPDF_DOCUMENT,
        bytes: Vec<u8>,
    ) -> Result<Self, PdfOpsError> {
        // pdfium requires version 1 or 2 and keeps the struct's address, so it
        // is boxed and pinned. No callbacks are needed: we do not expose
        // pdfium's own timers, menus or clipboard.
        let mut form_info = Box::pin(unsafe { std::mem::zeroed::<FPDF_FORMFILLINFO>() });
        form_info.version = 2;
        let form = bindings.FPDFDOC_InitFormFillEnvironment(handle, &mut *form_info);
        if form.is_null() {
            bindings.FPDF_CloseDocument(handle);
            return Err(PdfOpsError::Pdfium(
                "failed to initialize the pdfium form environment".to_string(),
            ));
        }

        Ok(Self {
            bindings,
            handle,
            form,
            _form_info: form_info,
            _bytes: bytes,
        })
    }

    pub(crate) fn bindings(&self) -> &'static dyn PdfiumLibraryBindings {
        self.bindings
    }

    pub(crate) fn handle(&self) -> FPDF_DOCUMENT {
        self.handle
    }

    /// The form-fill environment; the entry point of pdfium's field API.
    pub(crate) fn form(&self) -> FPDF_FORMHANDLE {
        self.form
    }

    pub(crate) fn page_count(&self) -> u32 {
        self.bindings.FPDF_GetPageCount(self.handle).max(0) as u32
    }

    /// Load a 0-based page. The page is closed when it is dropped.
    pub(crate) fn page(&self, index: u32) -> Result<Page<'_>, PdfOpsError> {
        let handle = self.bindings.FPDF_LoadPage(self.handle, index as c_int);
        if handle.is_null() {
            return Err(PdfOpsError::PageOutOfRange(index + 1));
        }
        // pdfium's form environment tracks pages it was told about; widget
        // annotations are only interactive after this call.
        self.bindings.FORM_OnAfterLoadPage(handle, self.form);
        Ok(Page {
            bindings: self.bindings,
            _document: self,
            handle,
        })
    }

    /// Set a string-valued form field (`/Tx`, `/Ch`) and drop its appearance, so
    /// pdfium redraws it from the new `/V`.
    pub(crate) fn set_field_text(
        &self,
        page_index: u32,
        annot_index: usize,
        value: &str,
    ) -> Result<(), PdfOpsError> {
        let page = self.page(page_index)?;
        let annot = self.widget(page.annotation(annot_index))?;
        self.bindings
            .FPDFAnnot_SetStringValue_str(annot.handle, "V", value);
        self.bindings.FPDFAnnot_SetAP(
            annot.handle,
            AppearanceMode::Normal as c_int,
            std::ptr::null(),
        );
        Ok(())
    }

    /// Click the centre of a widget, which toggles checkboxes and radio buttons
    /// and also focuses text fields.
    pub(crate) fn toggle_field(
        &self,
        page_index: u32,
        annot_index: usize,
    ) -> Result<(), PdfOpsError> {
        let page = self.page(page_index)?;
        let annot = self.widget(page.annotation(annot_index))?;
        let rect = annot_rect(self.bindings, annot.handle).ok_or_else(|| {
            PdfOpsError::Pdfium("failed to read the annotation rectangle".to_string())
        })?;
        let x = f64::from((rect.left + rect.right) / 2.0);
        let y = f64::from((rect.bottom + rect.top) / 2.0);
        let down = self
            .bindings
            .FORM_OnLButtonDown(self.form, page.handle, 0, x, y);
        let up = self
            .bindings
            .FORM_OnLButtonUp(self.form, page.handle, 0, x, y);
        if down == 0 && up == 0 {
            return Err(PdfOpsError::Pdfium(
                "failed to toggle the form field".to_string(),
            ));
        }
        Ok(())
    }

    /// Give a widget the form environment's input focus.
    pub(crate) fn focus_field(
        &self,
        page_index: u32,
        annot_index: usize,
    ) -> Result<(), PdfOpsError> {
        let page = self.page(page_index)?;
        let annot = self.widget(page.annotation(annot_index))?;
        if self.bindings.FORM_SetFocusedAnnot(self.form, annot.handle) == 0 {
            return Err(PdfOpsError::Pdfium(
                "failed to focus the form field".to_string(),
            ));
        }
        Ok(())
    }

    /// Select one option of a choice field, then commit it the way a viewer
    /// does when the field loses focus.
    ///
    /// pdfium neither exposes option export values nor accepts a display label
    /// as the choice value, so the selection is made in the form filler's own
    /// window state and committed by dropping the focus: pdfium then writes
    /// `/V` (the export value) and `/I` (list boxes) itself and regenerates the
    /// appearance. A field that still has the focus is committed on the way,
    /// because focusing always drops the previous focus first.
    pub(crate) fn select_field_option(
        &self,
        page_index: u32,
        annot_index: usize,
        option: usize,
    ) -> Result<(), PdfOpsError> {
        let page = self.page(page_index)?;
        let annot = self.widget(page.annotation(annot_index))?;
        if self.bindings.FORM_SetFocusedAnnot(self.form, annot.handle) == 0 {
            return Err(PdfOpsError::Pdfium(
                "failed to focus the form field".to_string(),
            ));
        }
        let option = c_int::try_from(option).map_err(|_| {
            PdfOpsError::Pdfium("option index does not fit into an int".to_string())
        })?;
        if self
            .bindings
            .FORM_SetIndexSelected(self.form, page.handle, option, 1)
            == 0
        {
            self.bindings.FORM_ForceToKillFocus(self.form);
            return Err(PdfOpsError::Pdfium(
                "failed to select the form field option".to_string(),
            ));
        }
        // Committing happens on losing focus; the return value is false when the
        // selection equals the current value, which is not a failure.
        self.bindings.FORM_ForceToKillFocus(self.form);
        Ok(())
    }

    /// Drop the input focus.
    pub(crate) fn blur_field(&self) {
        self.bindings.FORM_ForceToKillFocus(self.form);
    }

    /// Borrow an annotation and require it to be a widget.
    fn widget(&self, handle: FPDF_ANNOTATION) -> Result<AnnotBorrow<'_>, PdfOpsError> {
        let annot = AnnotBorrow::new(self.bindings, handle)
            .ok_or_else(|| PdfOpsError::Pdfium("failed to access the annotation".to_string()))?;
        if self.bindings.FPDFAnnot_GetSubtype(annot.handle) != WIDGET_SUBTYPE {
            return Err(PdfOpsError::Pdfium(
                "annotation is not a form field".to_string(),
            ));
        }
        Ok(annot)
    }

    /// Width and height of every page in points, in page order.
    pub(crate) fn page_sizes(&self) -> Result<Vec<(f32, f32)>, PdfOpsError> {
        let count = self.page_count();
        let mut sizes = Vec::with_capacity(count as usize);
        for index in 0..count {
            let page = self.page(index)?;
            sizes.push((page.width(), page.height()));
        }
        Ok(sizes)
    }

    /// Serialize the document to bytes.
    pub(crate) fn save(&self) -> Result<Vec<u8>, PdfOpsError> {
        let mut sink = WriteSink {
            base: unsafe { std::mem::zeroed() },
            data: Vec::new(),
        };
        sink.base.version = 1;
        sink.base.WriteBlock = Some(write_block);

        let ok =
            self.bindings
                .FPDF_SaveAsCopy(self.handle, &mut sink.base as *mut FPDF_FILEWRITE, 0);
        if ok == 0 {
            return Err(PdfOpsError::Pdfium(
                "failed to serialize the PDF document".to_string(),
            ));
        }
        Ok(sink.data)
    }

    /// Append every page of `source`.
    pub(crate) fn import_all(&mut self, source: &Document) -> Result<(), PdfOpsError> {
        let at = self.page_count() as c_int;
        self.import_range(source, "", at)
    }

    /// Import the pages named by `range` (1-based, e.g. "1,3-5") at 0-based `at`.
    /// An empty range imports every page.
    pub(crate) fn import_range(
        &mut self,
        source: &Document,
        range: &str,
        at: c_int,
    ) -> Result<(), PdfOpsError> {
        let ok = self
            .bindings
            .FPDF_ImportPages(self.handle, source.handle, range, at);
        if ok == 0 {
            return Err(PdfOpsError::Pdfium("failed to import pages".to_string()));
        }
        Ok(())
    }

    /// Insert `pages` of `source` (0-based) at 0-based `at`.
    pub(crate) fn import_indices(
        &mut self,
        source: &Document,
        pages: &[i32],
        at: i32,
    ) -> Result<(), PdfOpsError> {
        let ok = self.bindings.FPDF_ImportPagesByIndex(
            self.handle,
            source.handle,
            pages.as_ptr(),
            pages.len() as c_ulong,
            at,
        );
        if ok == 0 {
            return Err(PdfOpsError::Pdfium("failed to import pages".to_string()));
        }
        Ok(())
    }

    /// Delete the 0-based page `index`.
    pub(crate) fn delete_page(&mut self, index: i32) {
        self.bindings.FPDFPage_Delete(self.handle, index);
    }

    /// Render the 0-based page at `scale` pixels per point, painting form
    /// fields on top of the page so interactive forms are visible.
    pub(crate) fn render(
        &self,
        index: u32,
        scale: f32,
    ) -> Result<(u32, u32, Vec<u8>), PdfOpsError> {
        let page = self.page(index)?;
        let (display_width, display_height) = page.display_size();
        let width = ((display_width * scale).round() as i32).max(1);
        let height = ((display_height * scale).round() as i32).max(1);
        let rotation = page.rotation_quarters();

        let stride = width * 4;
        let mut pixels = vec![0u8; stride as usize * height as usize];
        let bitmap = self.bindings.FPDFBitmap_CreateEx(
            width,
            height,
            BITMAP_BGRA,
            pixels.as_mut_ptr() as *mut c_void,
            stride,
        );
        if bitmap.is_null() {
            return Err(PdfOpsError::Pdfium(
                "failed to allocate a render bitmap".to_string(),
            ));
        }

        self.bindings
            .FPDFBitmap_FillRect(bitmap, 0, 0, width, height, WHITE);
        self.bindings.FPDF_RenderPageBitmap(
            bitmap,
            page.handle,
            0,
            0,
            width,
            height,
            rotation,
            RENDER_ANNOTATIONS,
        );
        // Widget fields are drawn by the form environment, not by the page.
        self.bindings.FPDF_FFLDraw(
            self.form,
            bitmap,
            page.handle,
            0,
            0,
            width,
            height,
            rotation,
            RENDER_ANNOTATIONS,
        );
        self.bindings.FPDFBitmap_Destroy(bitmap);

        // pdfium writes BGRA; the UI expects RGBA.
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }
        Ok((width as u32, height as u32, pixels))
    }

    /// Create a page of `width` × `height` points at the end of the document.
    pub(crate) fn create_page(&mut self, width: f32, height: f32) -> Result<Page<'_>, PdfOpsError> {
        let index = self.page_count() as c_int;
        let handle = self
            .bindings
            .FPDFPage_New(self.handle, index, width as f64, height as f64);
        if handle.is_null() {
            return Err(PdfOpsError::Pdfium("failed to create a page".to_string()));
        }
        Ok(Page {
            bindings: self.bindings,
            _document: self,
            handle,
        })
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        if !self.form.is_null() {
            self.bindings.FPDFDOC_ExitFormFillEnvironment(self.form);
        }
        if !self.handle.is_null() {
            self.bindings.FPDF_CloseDocument(self.handle);
        }
    }
}

/// A loaded page, closed on drop.
pub(crate) struct Page<'a> {
    bindings: &'static dyn PdfiumLibraryBindings,
    _document: &'a Document,
    handle: FPDF_PAGE,
}

impl Page<'_> {
    pub(crate) fn handle(&self) -> FPDF_PAGE {
        self.handle
    }

    pub(crate) fn width(&self) -> f32 {
        self.bindings.FPDF_GetPageWidthF(self.handle)
    }

    pub(crate) fn height(&self) -> f32 {
        self.bindings.FPDF_GetPageHeightF(self.handle)
    }

    /// Width and height as displayed, i.e. with `/Rotate` applied.
    pub(crate) fn display_size(&self) -> (f32, f32) {
        let (width, height) = (self.width(), self.height());
        if matches!(self.rotation_quarters(), 1 | 3) {
            (height, width)
        } else {
            (width, height)
        }
    }

    /// `/Rotate` in quarter turns (0, 1, 2, 3).
    pub(crate) fn rotation_quarters(&self) -> c_int {
        self.bindings.FPDFPage_GetRotation(self.handle)
    }

    pub(crate) fn set_rotation_quarters(&self, quarters: c_int) {
        self.bindings.FPDFPage_SetRotation(self.handle, quarters);
    }

    /// Number of entries in the page's `/Annots` array.
    pub(crate) fn annotation_count(&self) -> usize {
        self.bindings.FPDFPage_GetAnnotCount(self.handle).max(0) as usize
    }

    /// Borrow the annotation at `index`; the caller closes it.
    pub(crate) fn annotation(&self, index: usize) -> FPDF_ANNOTATION {
        self.bindings.FPDFPage_GetAnnot(self.handle, index as c_int)
    }

    /// Insert an image object into the page, scaled to `width` × `height`
    /// points.
    ///
    /// Image objects draw in the unit square, so the transform spreads the
    /// bitmap over the page.
    pub(crate) fn insert_image(
        &mut self,
        obj: FPDF_PAGEOBJECT,
        width: f32,
        height: f32,
    ) -> Result<(), PdfOpsError> {
        self.bindings.FPDFPageObj_Transform(
            obj,
            f64::from(width),
            0.0,
            0.0,
            f64::from(height),
            0.0,
            0.0,
        );
        self.bindings.FPDFPage_InsertObject(self.handle, obj);
        if self.bindings.FPDFPage_GenerateContent(self.handle) == 0 {
            return Err(PdfOpsError::Pdfium(
                "failed to generate page content".to_string(),
            ));
        }
        Ok(())
    }
}

impl Drop for Page<'_> {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            self.bindings
                .FORM_OnBeforeClosePage(self.handle, self._document.form());
            self.bindings.FPDF_ClosePage(self.handle);
        }
    }
}

/// An annotation handle owned by the caller, closed on drop.
struct AnnotBorrow<'a> {
    bindings: &'a dyn PdfiumLibraryBindings,
    handle: FPDF_ANNOTATION,
}

impl<'a> AnnotBorrow<'a> {
    fn new(bindings: &'a dyn PdfiumLibraryBindings, handle: FPDF_ANNOTATION) -> Option<Self> {
        (!handle.is_null()).then_some(Self { bindings, handle })
    }
}

impl Drop for AnnotBorrow<'_> {
    fn drop(&mut self) {
        self.bindings.FPDFPage_CloseAnnot(self.handle);
    }
}

/// Read an annotation's rectangle.
fn annot_rect(bindings: &dyn PdfiumLibraryBindings, annot: FPDF_ANNOTATION) -> Option<FS_RECTF> {
    let mut rect = FS_RECTF {
        left: 0.0,
        bottom: 0.0,
        right: 0.0,
        top: 0.0,
    };
    (bindings.FPDFAnnot_GetRect(annot, &mut rect) != 0).then_some(rect)
}

/// `FPDF_FILEWRITE` implementation that appends into a `Vec<u8>`.
#[repr(C)]
struct WriteSink {
    base: FPDF_FILEWRITE,
    data: Vec<u8>,
}

unsafe extern "C" fn write_block(
    this: *mut FPDF_FILEWRITE,
    data: *const c_void,
    size: c_ulong,
) -> c_int {
    if this.is_null() || data.is_null() {
        return 0;
    }
    // SAFETY: pdfium passes back the pointer we handed to FPDF_SaveAsCopy,
    // whose first field is the FPDF_FILEWRITE it knows.
    let sink = unsafe { &mut *(this as *mut WriteSink) };
    // SAFETY: pdfium owns `data` for `size` bytes and calls us synchronously.
    let bytes = unsafe { std::slice::from_raw_parts(data as *const u8, size as usize) };
    sink.data.extend_from_slice(bytes);
    1
}
