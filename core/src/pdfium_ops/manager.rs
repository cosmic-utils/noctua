// SPDX-License-Identifier: GPL-3.0-only
// core/src/pdfium_ops/manager.rs
//
// PDF operations manager. Holds one open document, executes commands.
// All pdfium calls happen here. Must only be used from a single thread
// (the pdfium worker) — pdfium is not thread-safe.

use std::path::{Path, PathBuf};

use pdfium_render::prelude::*;

use super::bindings::pdfium_or_err;
use super::command::{Command, CommandResult};
use super::document::Document;
use super::error::PdfOpsError;
use super::model::{
    AnnotationColor, BindSource, FormField, FormFieldKind, FreeTextAnnotation, MarkupKind,
    PdfMetadata,
};

use crate::storage::document::Format;

fn pdfium_err(e: PdfiumError) -> PdfOpsError {
    PdfOpsError::Pdfium(format!("{e:?}"))
}

/// `FPDF_ANNOT_FLAG_PRINT` from `fpdf_annot.h`. The pdfium-render prelude does
/// not re-export the flag constants.
const ANNOT_FLAG_PRINT: i32 = 1 << 2;

/// Rectangle in page points: `(x, y, width, height)`, origin bottom-left.
type CaretRect = (f32, f32, f32, f32);

/// Owns an annotation handle returned by `FPDFPage_CreateAnnot` or
/// `FPDFPage_GetAnnot` and closes it on drop. pdfium requires
/// `FPDFPage_CloseAnnot` for both.
struct AnnotGuard<'a> {
    bindings: &'a dyn PdfiumLibraryBindings,
    handle: FPDF_ANNOTATION,
}

impl<'a> AnnotGuard<'a> {
    fn new(bindings: &'a dyn PdfiumLibraryBindings, handle: FPDF_ANNOTATION) -> Option<Self> {
        (!handle.is_null()).then_some(Self { bindings, handle })
    }
}

impl Drop for AnnotGuard<'_> {
    fn drop(&mut self) {
        self.bindings.FPDFPage_CloseAnnot(self.handle);
    }
}

/// Build a free-text `/DA` appearance string: RGB colour (0–1) plus the Helvetica
/// font size in points.
fn free_text_da(color: AnnotationColor, font_size: f32) -> String {
    let r = color.red as f32 / 255.0;
    let g = color.green as f32 / 255.0;
    let b = color.blue as f32 / 255.0;
    format!("{r:.3} {g:.3} {b:.3} rg /Helv {font_size} Tf")
}

/// Parse the colour and font size out of a free-text `/DA` string. Falls back to
/// black and 12 pt, matching the PDF defaults.
fn parse_free_text_da(da: &str) -> (AnnotationColor, f32) {
    let tokens: Vec<&str> = da.split_whitespace().collect();
    let number = |index: usize| {
        tokens
            .get(index)
            .and_then(|token| token.parse::<f32>().ok())
    };
    let mut rgb = [0.0f32; 3];
    let mut size = 12.0f32;
    for (index, token) in tokens.iter().enumerate() {
        match *token {
            "rg" if index >= 3 => {
                if let (Some(r), Some(g), Some(b)) =
                    (number(index - 3), number(index - 2), number(index - 1))
                {
                    rgb = [r, g, b];
                }
            }
            "g" if index >= 1 => {
                if let Some(grey) = number(index - 1) {
                    rgb = [grey; 3];
                }
            }
            "Tf" if index >= 1 => {
                if let Some(value) = number(index - 1).filter(|value| *value > 0.0) {
                    size = value;
                }
            }
            _ => {}
        }
    }
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    (
        AnnotationColor {
            red: channel(rgb[0]),
            green: channel(rgb[1]),
            blue: channel(rgb[2]),
            alpha: 255,
        },
        size.clamp(1.0, 400.0),
    )
}

/// Read a string value from a raw annotation handle. pdfium returns UTF-16LE.
fn read_annot_string(
    bindings: &dyn PdfiumLibraryBindings,
    annot: FPDF_ANNOTATION,
    key: &str,
) -> Option<String> {
    let length = bindings.FPDFAnnot_GetStringValue(annot, key, std::ptr::null_mut(), 0);
    if length <= 2 {
        return None;
    }
    let mut buffer = vec![0u16; length as usize / 2];
    bindings.FPDFAnnot_GetStringValue(annot, key, buffer.as_mut_ptr(), length);
    let text = String::from_utf16_lossy(&buffer);
    Some(text.trim_end_matches('\0').to_string())
}

/// Helvetica line advance as a multiple of the font size.
const FREE_TEXT_LINE_HEIGHT: f32 = 1.2;

/// Inset between the free-text box and its text, in points.
const FREE_TEXT_PADDING: f32 = 2.0;

/// Helvetica ascent and descent in em units (Adobe AFM metrics).
const HELVETICA_ASCENT: f32 = 0.718;
const HELVETICA_DESCENT: f32 = 0.207;

/// Width of the insertion marker, in points.
const CARET_WIDTH: f32 = 0.8;

/// Helvetica advance widths for printable ASCII (32..=126), in 1/1000 em,
/// taken from Adobe's Helvetica AFM.
const HELVETICA_ASCII_WIDTHS: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, // 32..=47
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, // 48..=63
    1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722,
    778, // 64..=79
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556, // 80..=95
    333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556,
    556, // 96..=111
    556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584, // 112..=126
];

/// Advance width of one character in 1/1000 em. Characters outside the table
/// use a typical lowercase width; the full text stays in `/Contents`.
fn helvetica_width_units(ch: char) -> u16 {
    let code = ch as u32;
    if (32..=126).contains(&code) {
        return HELVETICA_ASCII_WIDTHS[(code - 32) as usize];
    }
    match ch {
        'Ä' => 667,
        'Ö' => 778,
        'Ü' => 722,
        'ç' => 500,
        '°' => 400,
        _ => 556,
    }
}

/// Advance width of one character at `font_size` points.
fn char_width(ch: char, font_size: f32) -> f32 {
    f32::from(helvetica_width_units(ch)) * font_size / 1000.0
}

/// Rendered width of `text` at `font_size` points.
fn text_width(text: &str, font_size: f32) -> f32 {
    text.chars().map(|ch| char_width(ch, font_size)).sum()
}

/// Rendered width of `text` in Helvetica at `font_size` points.
///
/// Public because the UI places the caret inside a form field with the same
/// metrics the appearance uses; the width table lives in this module.
pub fn helvetica_text_width(text: &str, font_size: f32) -> f32 {
    text_width(text, font_size)
}

/// Greedy wrap: break at spaces, split words that are wider than a line. Each
/// line carries the character offset it starts at in the source text, so a
/// caret index can be mapped back to a position.
fn wrap_text(text: &str, font_size: f32, max_width: f32) -> Vec<(String, usize)> {
    // Even the narrowest usable box must fit one character per line.
    let max_width = max_width.max(char_width('m', font_size));
    let mut lines = Vec::new();
    let mut paragraph_start = 0;

    for paragraph in text.split('\n') {
        let characters: Vec<char> = paragraph.chars().collect();
        let mut line = String::new();
        let mut line_start = 0;
        let mut line_width = 0.0_f32;

        for (index, ch) in characters.iter().copied().enumerate() {
            let advance = char_width(ch, font_size);
            if line_width + advance > max_width && !line.is_empty() {
                match line.rfind(' ') {
                    // Keep the word that no longer fits for the next line.
                    Some(space) => {
                        let carry = line.split_off(space + 1);
                        let carry_start = line_start + line[..=space].chars().count();
                        line_width = text_width(&carry, font_size);
                        lines.push((
                            std::mem::take(&mut line).trim_end().to_string(),
                            paragraph_start + line_start,
                        ));
                        line = carry;
                        line_start = carry_start;
                    }
                    None => {
                        lines.push((std::mem::take(&mut line), paragraph_start + line_start));
                        line_start = index;
                        line_width = 0.0;
                    }
                }
            }
            line.push(ch);
            line_width += advance;
        }

        lines.push((line, paragraph_start + line_start));
        paragraph_start += characters.len() + 1;
    }

    lines
}

/// Laid-out free-text: the wrapped lines plus the box that holds them.
struct FreeTextLayout {
    lines: Vec<String>,
    /// Character offset of each line's first character in the source text.
    starts: Vec<usize>,
    width: f32,
    height: f32,
}

fn layout_free_text(text: &str, font_size: f32, max_width: f32) -> FreeTextLayout {
    let wrapped = wrap_text(text, font_size, max_width);
    let widest = wrapped
        .iter()
        .map(|(line, _)| text_width(line, font_size))
        .fold(0.0_f32, f32::max)
        // An empty box still needs a clickable width.
        .max(char_width('n', font_size));
    let line_count = wrapped.len().max(1) as f32;
    FreeTextLayout {
        width: widest + 2.0 * FREE_TEXT_PADDING,
        height: line_count * font_size * FREE_TEXT_LINE_HEIGHT + 2.0 * FREE_TEXT_PADDING,
        starts: wrapped.iter().map(|(_, start)| *start).collect(),
        lines: wrapped.into_iter().map(|(line, _)| line).collect(),
    }
}

/// Escape `text` as a PDF literal string. pdfium re-encodes the whole stream
/// through PDFDocEncoding, so characters outside Latin-1 would corrupt the
/// operators; those are replaced (the real text stays in `/Contents`).
fn pdf_text_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('(');
    for ch in text.chars() {
        match ch {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            ch if (ch as u32) < 0x20 || (0x7F..=0x9F).contains(&(ch as u32)) => out.push('?'),
            ch if (ch as u32) > 0xFF => out.push('?'),
            ch => out.push(ch),
        }
    }
    out.push(')');
    out
}

/// Baseline of the first line of a box whose top edge is `top`.
fn first_baseline(top: f32, font_size: f32) -> f32 {
    top - FREE_TEXT_PADDING - font_size * HELVETICA_ASCENT
}

/// Build the normal appearance stream of a free-text annotation. Writing the
/// stream ourselves keeps the frame off (pdfium's own generator always draws
/// one) and gives us line breaks, which its generator does not produce.
fn free_text_appearance(
    layout: &FreeTextLayout,
    font_size: f32,
    color: AnnotationColor,
    top_left: (f32, f32),
) -> String {
    let (left, top) = top_left;
    let r = color.red as f32 / 255.0;
    let g = color.green as f32 / 255.0;
    let b = color.blue as f32 / 255.0;
    let x = left + FREE_TEXT_PADDING;
    let line_step = font_size * FREE_TEXT_LINE_HEIGHT;
    let mut baseline = first_baseline(top, font_size);

    let mut content = format!("q\n{r:.3} {g:.3} {b:.3} rg\nBT\n/Helv {font_size:.3} Tf\n");
    for line in &layout.lines {
        content.push_str(&format!("1 0 0 1 {x:.3} {baseline:.3} Tm\n"));
        content.push_str(&format!("{} Tj\n", pdf_text_string(line)));
        baseline -= line_step;
    }
    content.push_str("ET\nQ");
    content
}

/// Rectangle of the insertion marker at the end of the laid-out text, in page
/// points with the origin at the bottom-left.
fn free_text_caret(
    layout: &FreeTextLayout,
    font_size: f32,
    top_left: (f32, f32),
    cursor: usize,
) -> CaretRect {
    let (left, top) = top_left;

    // The cursor sits on the last line that starts at or before it.
    let mut line = 0;
    for (index, start) in layout.starts.iter().enumerate() {
        if *start <= cursor {
            line = index;
        }
    }

    let prefix: String = layout
        .lines
        .get(line)
        .map(|text| {
            text.chars()
                .take(cursor.saturating_sub(layout.starts[line]))
                .collect()
        })
        .unwrap_or_default();
    let baseline = first_baseline(top, font_size) - line as f32 * font_size * FREE_TEXT_LINE_HEIGHT;

    (
        left + FREE_TEXT_PADDING + text_width(&prefix, font_size),
        baseline - font_size * HELVETICA_DESCENT,
        CARET_WIDTH,
        font_size * (HELVETICA_ASCENT + HELVETICA_DESCENT),
    )
}

/// `FPDF_FORMFIELD_*` from `fpdf_formfill.h`: the field type pdfium reports.
const FORM_FIELD_PUSH_BUTTON: i32 = 1;
const FORM_FIELD_CHECKBOX: i32 = 2;
const FORM_FIELD_RADIO: i32 = 3;
const FORM_FIELD_COMBO: i32 = 4;
const FORM_FIELD_LISTBOX: i32 = 5;
const FORM_FIELD_TEXT: i32 = 6;
const FORM_FIELD_SIGNATURE: i32 = 7;

/// `FPDFANNOT_COLORTYPE_COLOR`, the annotation's stroke colour.
const ANNOT_COLOR_STROKE: u32 = 0;

/// `FPDFBitmap_BGRA`, pdfium's native 32-bit layout.
const RENDER_BITMAP_BGRA: i32 = 4;

/// Ink stroke width in points, as drawn before the raw port.
const INK_WIDTH: f32 = 2.0;

/// Rectangle of an annotation in page points, origin bottom-left.
fn annot_box(x: f32, y: f32, width: f32, height: f32) -> FS_RECTF {
    FS_RECTF {
        left: x,
        bottom: y,
        right: x + width,
        top: y + height,
    }
}

/// Read a UTF-16 string a pdfium form getter writes in two calls.
fn form_text(
    fetch: impl Fn(*mut u16, std::os::raw::c_ulong) -> std::os::raw::c_ulong,
) -> Option<String> {
    let length = fetch(std::ptr::null_mut(), 0);
    if length <= 2 {
        return None;
    }
    let mut buffer = vec![0u16; length as usize / 2];
    fetch(buffer.as_mut_ptr(), length);
    let text = String::from_utf16_lossy(&buffer);
    Some(text.trim_end_matches('\0').to_string())
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

/// Quad points of a markup rectangle, counter-clockwise from the upper left.
/// Text-markup annotations are visible only through these attachment points.
fn annot_quad(x: f32, y: f32, width: f32, height: f32) -> FS_QUADPOINTSF {
    FS_QUADPOINTSF {
        x1: x,
        y1: y + height,
        x2: x + width,
        y2: y + height,
        x3: x,
        y3: y,
        x4: x + width,
        y4: y,
    }
}

/// Manager for PDF editing operations. Owns the currently open document.
pub struct PdfOpsManager {
    document: Option<Document>,
    path: Option<PathBuf>,
    dirty: bool,
}

impl Default for PdfOpsManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfOpsManager {
    /// Create a manager without an open document.
    pub fn new() -> Self {
        Self {
            document: None,
            path: None,
            dirty: false,
        }
    }

    /// Whether the open document has unsaved changes.
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Path of the open document, if any.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Whether a document is currently open.
    pub fn has_document(&self) -> bool {
        self.document.is_some()
    }

    /// Execute a command on the open document.
    pub fn execute(&mut self, command: Command) -> CommandResult {
        match command {
            Command::Open { data, path } => self.open(data, &path).into(),
            Command::New => self.new_document().into(),
            Command::Close => {
                self.document = None;
                self.path = None;
                self.dirty = false;
                CommandResult::Ok
            }
            Command::Bind { sources, target } => self.bind(sources, &target).into(),
            Command::InsertPages { source, at } => self.insert_pages(&source, at).into(),
            Command::DeletePages { pages } => self.delete_pages(&pages).into(),
            Command::MovePage { from, to } => self.move_page(from, to).into(),
            Command::RotatePage { page, degrees } => self.rotate_page(page, degrees).into(),
            Command::RotatePageRelative { page, clockwise } => {
                self.rotate_page_relative(page, clockwise).into()
            }
            Command::AddFreeTextAnnotation {
                page,
                text,
                font_size,
                color,
                position,
                max_width,
            } => match self
                .add_free_text_annotation(page, &text, font_size, color, position, max_width)
            {
                Ok((index, caret)) => CommandResult::FreeTextWritten {
                    index: Some(index),
                    caret,
                },
                Err(error) => CommandResult::Error(error),
            },
            Command::UpdateFreeText {
                page,
                index,
                text,
                font_size,
                color,
                position,
                max_width,
                cursor,
            } => match self.update_free_text(
                page, index, &text, font_size, color, position, max_width, cursor,
            ) {
                Ok(caret) => CommandResult::FreeTextWritten { index: None, caret },
                Err(error) => CommandResult::Error(error),
            },
            Command::RemoveAnnotation { page, index } => self.remove_annotation(page, index).into(),
            Command::AddInkAnnotation {
                page,
                color,
                points,
                x,
                y,
                width,
                height,
            } => self
                .add_ink_annotation(page, color, &points, (x, y, width, height))
                .into(),
            Command::AddMarkupAnnotation {
                page,
                kind,
                color,
                x,
                y,
                width,
                height,
            } => self
                .add_markup_annotation(page, kind, color, (x, y, width, height))
                .into(),
            Command::ListFreeTextAnnotations { page } => {
                match self.list_free_text_annotations(page) {
                    Ok(list) => CommandResult::FreeTextAnnotations(list),
                    Err(e) => CommandResult::Error(e),
                }
            }
            Command::ListFormFields { page } => match self.list_form_fields(page) {
                Ok(list) => CommandResult::FormFields(list),
                Err(e) => CommandResult::Error(e),
            },
            Command::PageTargets { page } => match self.page_targets(page) {
                Ok((annotations, fields)) => CommandResult::PageTargets {
                    annotations,
                    fields,
                },
                Err(e) => CommandResult::Error(e),
            },
            Command::SetFormFieldValue { page, index, value } => {
                self.set_form_field_value(page, index, &value).into()
            }
            Command::ToggleFormField { page, index } => self.toggle_form_field(page, index).into(),
            Command::SelectFormFieldOption {
                page,
                index,
                option,
            } => self.select_form_field_option(page, index, option).into(),
            Command::FocusFormField { page, index } => self.focus_form_field(page, index).into(),
            Command::BlurFormField => {
                if let Some(document) = self.document.as_ref() {
                    document.blur_field();
                }
                CommandResult::Ok
            }
            Command::SetFreeTextStyle {
                page,
                index,
                color,
                font_size,
            } => self
                .set_free_text_style(page, index, color, font_size)
                .into(),
            Command::Save => self.save().into(),
            Command::SaveAs { path } => self.save_as(&path).into(),
            Command::PageCount => match self.page_count() {
                Ok(n) => CommandResult::PageCount(n),
                Err(e) => CommandResult::Error(e),
            },
            Command::PageSizes => match self.page_sizes() {
                Ok(sizes) => CommandResult::PageSizes(sizes),
                Err(e) => CommandResult::Error(e),
            },
            Command::RenderPage { page, zoom } => match self.render_page(page, zoom) {
                Ok((width, height, rgba_data)) => CommandResult::Rendered {
                    width,
                    height,
                    rgba_data,
                },
                Err(e) => CommandResult::Error(e),
            },
            Command::RenderThumbnail { page, max_px } => {
                match self.render_thumbnail(page, max_px) {
                    Ok((width, height, rgba_data)) => CommandResult::Rendered {
                        width,
                        height,
                        rgba_data,
                    },
                    Err(e) => CommandResult::Error(e),
                }
            }
        }
    }

    // ── Internal operations ──

    fn require_document(&self) -> Result<&Document, PdfOpsError> {
        self.document.as_ref().ok_or(PdfOpsError::NoDocumentOpen)
    }

    fn open(&mut self, data: Vec<u8>, path: &Path) -> Result<(), PdfOpsError> {
        // `Document::open` keeps the bytes alive for pdfium's lifetime.
        self.document = Some(Document::open(data)?);
        self.path = Some(path.to_path_buf());
        self.dirty = false;
        Ok(())
    }

    fn new_document(&mut self) -> Result<(), PdfOpsError> {
        self.document = Some(Document::create()?);
        self.path = None;
        self.dirty = true;
        Ok(())
    }

    fn page_count(&self) -> Result<u32, PdfOpsError> {
        Ok(self.require_document()?.page_count())
    }

    fn page_sizes(&self) -> Result<Vec<(f32, f32)>, PdfOpsError> {
        self.require_document()?.page_sizes()
    }

    fn save(&mut self) -> Result<Vec<u8>, PdfOpsError> {
        if self.path.is_none() {
            return Err(PdfOpsError::NoDocumentOpen);
        }
        let data = self.require_document()?.save()?;
        self.dirty = false;
        Ok(data)
    }

    fn save_as(&mut self, path: &Path) -> Result<Vec<u8>, PdfOpsError> {
        let data = self.require_document()?.save()?;
        self.path = Some(path.to_path_buf());
        self.dirty = false;
        Ok(data)
    }

    /// Bind sources into a fresh PDF, then open it. Returns the serialized
    /// bytes for the caller to write to `target`.
    fn bind(&mut self, sources: Vec<BindSource>, target: &Path) -> Result<Vec<u8>, PdfOpsError> {
        // Reject unknown formats up front, before creating a document.
        for source in &sources {
            if matches!(source.format, Format::Unknown) {
                return Err(PdfOpsError::UnsupportedSource(source.name.clone()));
            }
        }

        let mut document = Document::create()?;

        for source in &sources {
            match source.format {
                Format::Pdf => {
                    let src_doc = Document::open(source.data.clone())?;
                    match &source.pages {
                        None => document.import_all(&src_doc)?,
                        Some(range) => {
                            let at = document.page_count() as i32;
                            document.import_range(&src_doc, range, at)?;
                        }
                    }
                }
                Format::Raster => {
                    embed_raster_page(&mut document, &source.data)?;
                }
                Format::Svg => {
                    embed_vector_page(&mut document, &source.data)?;
                }
                Format::Unknown => {
                    // Validated above; kept defensive.
                    return Err(PdfOpsError::UnsupportedSource(source.name.clone()));
                }
            }
        }

        let data = document.save()?;
        self.document = Some(document);
        self.path = Some(target.to_path_buf());
        self.dirty = false;
        Ok(data)
    }

    fn insert_pages(&mut self, source: &BindSource, at: u32) -> Result<(), PdfOpsError> {
        if at == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        let dest_index = (at - 1) as PdfPageIndex;

        match source.format {
            Format::Pdf => {
                let src_doc = Document::open(source.data.clone())?;
                let document = self.document.as_mut().ok_or(PdfOpsError::NoDocumentOpen)?;
                match &source.pages {
                    None => document.import_all(&src_doc)?,
                    Some(range) => document.import_range(&src_doc, range, dest_index as i32)?,
                }
            }
            Format::Raster => {
                // Raster pages are inserted by binding into a scratch document
                // and importing the resulting page.
                let mut scratch = Document::create()?;
                embed_raster_page(&mut scratch, &source.data)?;
                let document = self.document.as_mut().ok_or(PdfOpsError::NoDocumentOpen)?;
                document.import_indices(&scratch, &[0], dest_index as i32)?;
            }
            Format::Svg => {
                let mut scratch = Document::create()?;
                embed_vector_page(&mut scratch, &source.data)?;
                let document = self.document.as_mut().ok_or(PdfOpsError::NoDocumentOpen)?;
                document.import_indices(&scratch, &[0], dest_index as i32)?;
            }
            Format::Unknown => {
                return Err(PdfOpsError::UnsupportedSource(source.name.clone()));
            }
        }

        self.dirty = true;
        Ok(())
    }

    fn delete_pages(&mut self, pages: &[u32]) -> Result<(), PdfOpsError> {
        let mut indices: Vec<i32> = Vec::with_capacity(pages.len());
        for page in pages {
            if *page == 0 {
                return Err(PdfOpsError::PageOutOfRange(0));
            }
            indices.push((*page - 1) as i32);
        }
        // Delete in descending order so indices stay valid.
        indices.sort_unstable_by(|a, b| b.cmp(a));
        let document = self.document.as_mut().ok_or(PdfOpsError::NoDocumentOpen)?;
        for index in indices {
            if index as u32 >= document.page_count() {
                return Err(PdfOpsError::PageOutOfRange(index as u32 + 1));
            }
            document.delete_page(index);
        }
        self.dirty = true;
        Ok(())
    }

    /// Move a page by copying it into a scratch document, deleting the
    /// original, and re-inserting the copy at the target position.
    fn move_page(&mut self, from: u32, to: u32) -> Result<(), PdfOpsError> {
        if from == 0 || to == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        let from_idx = (from - 1) as i32;
        let to_idx = (to - 1) as i32;

        // 1. Copy the source page into a scratch document.
        let mut scratch = Document::create()?;
        {
            let document = self.require_document()?;
            if from_idx as u32 >= document.page_count() {
                return Err(PdfOpsError::PageOutOfRange(from));
            }
            scratch.import_indices(document, &[from_idx], 0)?;
        }

        // 2. Delete the original page.
        self.document
            .as_mut()
            .ok_or(PdfOpsError::NoDocumentOpen)?
            .delete_page(from_idx);

        // 3. Insert the copy at the target position. pdfium inserts AT the
        // destination index, so the 1-based target `to` maps directly to the
        // 0-based `to_idx` — the deleted page already shifted everything up.
        self.document
            .as_mut()
            .ok_or(PdfOpsError::NoDocumentOpen)?
            .import_indices(&scratch, &[0], to_idx)?;

        self.dirty = true;
        Ok(())
    }

    fn rotate_page(&mut self, page: u32, degrees: u16) -> Result<(), PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        if !matches!(degrees, 0 | 90 | 180 | 270) {
            return Err(PdfOpsError::Pdfium(format!(
                "unsupported rotation: {degrees} degrees"
            )));
        }
        let document = self.document.as_mut().ok_or(PdfOpsError::NoDocumentOpen)?;
        let pdf_page = document.page(page - 1)?;
        pdf_page.set_rotation_quarters(i32::from(degrees / 90) as std::os::raw::c_int);
        self.dirty = true;
        Ok(())
    }

    /// Rotate a page 90° clockwise or counter-clockwise relative to its
    /// current rotation. Reads the current rotation from pdfium, computes the
    /// new absolute value and delegates to [`Self::rotate_page`].
    fn rotate_page_relative(&mut self, page: u32, clockwise: bool) -> Result<(), PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        let current = {
            let document = self.require_document()?;
            let pdf_page = document.page(page - 1)?;
            f32::from(pdf_page.rotation_quarters() as u16) * 90.0
        };
        // Counter-clockwise is expressed as three clockwise quarter turns.
        let delta = if clockwise { 90.0 } else { 270.0 };
        let new_degrees = ((current + delta) % 360.0) as u16;
        self.rotate_page(page, new_degrees)
    }

    fn add_free_text_annotation(
        &mut self,
        page: u32,
        text: &str,
        font_size: f32,
        color: AnnotationColor,
        position: (f32, f32),
        max_width: f32,
    ) -> Result<(usize, CaretRect), PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }

        let result = {
            let document = self.require_document()?;
            let pdf_page = document.page(page - 1)?;
            let bindings = document.bindings();
            let page_handle = pdf_page.handle();

            let annot = bindings.FPDFPage_CreateAnnot(
                page_handle,
                PdfPageAnnotationType::FreeText as FPDF_ANNOTATION_SUBTYPE,
            );
            let annot = AnnotGuard::new(bindings, annot).ok_or_else(|| {
                PdfOpsError::Pdfium("failed to create free-text annotation".to_string())
            })?;

            // Annotations are only printed when the Print flag is set.
            bindings.FPDFAnnot_SetFlags(annot.handle, ANNOT_FLAG_PRINT);

            let caret = self.write_free_text(
                bindings,
                annot.handle,
                text,
                font_size,
                color,
                position,
                max_width,
                text.chars().count(),
            )?;

            let index = (bindings.FPDFPage_GetAnnotCount(page_handle) as usize).saturating_sub(1);
            (index, caret)
        };

        self.dirty = true;
        Ok(result)
    }

    /// Write text, style and geometry into an open free-text annotation handle.
    /// Returns the insertion marker rectangle in page points.
    #[allow(clippy::too_many_arguments)]
    fn write_free_text(
        &self,
        bindings: &dyn PdfiumLibraryBindings,
        annot: FPDF_ANNOTATION,
        text: &str,
        font_size: f32,
        color: AnnotationColor,
        position: (f32, f32),
        max_width: f32,
        cursor: usize,
    ) -> Result<CaretRect, PdfOpsError> {
        let layout = layout_free_text(text, font_size, max_width);
        let (left, top) = position;
        let rect = PdfRect::new(
            PdfPoints::new(top - layout.height),
            PdfPoints::new(left),
            PdfPoints::new(top),
            PdfPoints::new(left + layout.width),
        );
        let fs_rect = bindings.get_fs_rect_from_rect(&rect);
        bindings.FPDFAnnot_SetRect(annot, &fs_rect);

        // `/Contents` and `/DA` keep the annotation readable for other viewers;
        // the appearance below is what pdfium renders.
        bindings.FPDFAnnot_SetStringValue_str(annot, "Contents", text);
        bindings.FPDFAnnot_SetStringValue_str(annot, "DA", &free_text_da(color, font_size));

        // pdfium re-encodes the stream as a PDFDocEncoding string, so it has to
        // be passed as UTF-16 and terminated.
        let mut wide: Vec<u16> = free_text_appearance(&layout, font_size, color, position)
            .encode_utf16()
            .collect();
        wide.push(0);
        if bindings.FPDFAnnot_SetAP(annot, PdfAppearanceMode::Normal as i32, wide.as_ptr()) == 0 {
            return Err(PdfOpsError::Pdfium(
                "failed to set the annotation appearance".to_string(),
            ));
        }

        Ok(free_text_caret(&layout, font_size, position, cursor))
    }

    /// List the free-text annotations on a page, in page points. `index` is the
    /// annotation's position in the page's annotation list.
    fn list_free_text_annotations(
        &self,
        page: u32,
    ) -> Result<Vec<FreeTextAnnotation>, PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        let document = self.require_document()?;
        let pdf_page = document.page(page - 1)?;
        let bindings = document.bindings();

        let mut annotations = Vec::new();
        for index in 0..pdf_page.annotation_count() {
            let Some(handle) = AnnotGuard::new(bindings, pdf_page.annotation(index)) else {
                continue;
            };
            if bindings.FPDFAnnot_GetSubtype(handle.handle)
                != PdfPageAnnotationType::FreeText as FPDF_ANNOTATION_SUBTYPE
            {
                continue;
            }
            let mut rect = FS_RECTF {
                left: 0.0,
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
            };
            if bindings.FPDFAnnot_GetRect(handle.handle, &mut rect) == 0 {
                continue;
            }
            // The colour and size live in `/DA`, which has no typed getter.
            let da = read_annot_string(bindings, handle.handle, "DA").unwrap_or_default();
            let (color, font_size) = parse_free_text_da(&da);
            annotations.push(FreeTextAnnotation {
                index,
                contents: read_annot_string(bindings, handle.handle, "Contents")
                    .unwrap_or_default(),
                color,
                font_size,
                x: rect.left,
                y: rect.bottom,
                width: rect.right - rect.left,
                height: rect.top - rect.bottom,
            });
        }
        Ok(annotations)
    }

    /// Free-text annotations and form fields of one page, for hit-testing a
    /// click with a single round trip.
    fn page_targets(
        &self,
        page: u32,
    ) -> Result<(Vec<FreeTextAnnotation>, Vec<FormField>), PdfOpsError> {
        Ok((
            self.list_free_text_annotations(page)?,
            self.list_form_fields(page)?,
        ))
    }

    /// List the interactive form fields on a page, in page points.
    fn list_form_fields(&self, page: u32) -> Result<Vec<FormField>, PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        let document = self.require_document()?;
        let pdf_page = document.page(page - 1)?;
        let bindings = document.bindings();
        let form = document.form();

        let mut fields = Vec::new();
        for index in 0..pdf_page.annotation_count() {
            let Some(annot) = AnnotGuard::new(bindings, pdf_page.annotation(index)) else {
                continue;
            };
            if bindings.FPDFAnnot_GetSubtype(annot.handle)
                != PdfPageAnnotationType::Widget as FPDF_ANNOTATION_SUBTYPE
            {
                continue;
            }
            let Some(rect) = annot_rect(bindings, annot.handle) else {
                continue;
            };
            let kind = match bindings.FPDFAnnot_GetFormFieldType(form, annot.handle) {
                FORM_FIELD_PUSH_BUTTON => FormFieldKind::PushButton,
                FORM_FIELD_CHECKBOX => FormFieldKind::Checkbox,
                FORM_FIELD_RADIO => FormFieldKind::Radio,
                FORM_FIELD_TEXT => FormFieldKind::Text,
                FORM_FIELD_LISTBOX => FormFieldKind::ListBox,
                FORM_FIELD_COMBO => FormFieldKind::Combo,
                FORM_FIELD_SIGNATURE => FormFieldKind::Signature,
                _ => FormFieldKind::Unknown,
            };

            let da = read_annot_string(bindings, annot.handle, "DA").unwrap_or_default();
            let (_, font_size) = parse_free_text_da(&da);

            let mut options = Vec::new();
            let mut labels_complete = true;
            let option_count = bindings.FPDFAnnot_GetOptionCount(form, annot.handle);
            for option in 0..option_count.max(0) {
                let label = form_text(|buffer, length| {
                    bindings.FPDFAnnot_GetOptionLabel(form, annot.handle, option, buffer, length)
                });
                match label {
                    Some(label) => options.push(label),
                    None => labels_complete = false,
                }
            }
            // Option positions only line up with pdfium's option indices when
            // every label could be read.
            let selected = if labels_complete {
                (0..option_count.max(0)).find(|option| {
                    bindings.FPDFAnnot_IsOptionSelected(form, annot.handle, *option) != 0
                })
            } else {
                None
            }
            .and_then(|option| usize::try_from(option).ok());

            fields.push(FormField {
                index,
                name: form_text(|buffer, length| {
                    bindings.FPDFAnnot_GetFormFieldName(form, annot.handle, buffer, length)
                })
                .unwrap_or_default(),
                kind,
                flags: bindings.FPDFAnnot_GetFormFieldFlags(form, annot.handle),
                value: form_text(|buffer, length| {
                    bindings.FPDFAnnot_GetFormFieldValue(form, annot.handle, buffer, length)
                })
                .unwrap_or_default(),
                options,
                selected,
                font_size,
                x: rect.left,
                y: rect.bottom,
                width: rect.right - rect.left,
                height: rect.top - rect.bottom,
            });
        }
        Ok(fields)
    }

    /// Write a new value into a text or choice field.
    fn set_form_field_value(
        &mut self,
        page: u32,
        index: usize,
        value: &str,
    ) -> Result<(), PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        self.require_document()?
            .set_field_text(page - 1, index, value)?;
        self.dirty = true;
        Ok(())
    }

    /// Toggle a checkbox or radio button.
    fn toggle_form_field(&mut self, page: u32, index: usize) -> Result<(), PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        self.require_document()?.toggle_field(page - 1, index)?;
        self.dirty = true;
        Ok(())
    }

    /// Select one option of a combo or list box.
    fn select_form_field_option(
        &mut self,
        page: u32,
        index: usize,
        option: usize,
    ) -> Result<(), PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        self.require_document()?
            .select_field_option(page - 1, index, option)?;
        self.dirty = true;
        Ok(())
    }

    /// Give a form field the input focus.
    fn focus_form_field(&self, page: u32, index: usize) -> Result<(), PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        self.require_document()?.focus_field(page - 1, index)
    }

    /// Rewrite text, style and geometry of an existing free-text annotation.
    /// `cursor` is the character offset the insertion marker is drawn at.
    /// Returns that marker's rectangle in page points.
    #[allow(clippy::too_many_arguments)]
    fn update_free_text(
        &mut self,
        page: u32,
        index: usize,
        text: &str,
        font_size: f32,
        color: AnnotationColor,
        position: (f32, f32),
        max_width: f32,
        cursor: usize,
    ) -> Result<CaretRect, PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        let caret = {
            let document = self.require_document()?;
            let pdf_page = document.page(page - 1)?;
            let bindings = document.bindings();
            let page_handle = pdf_page.handle();
            let annot = AnnotGuard::new(
                bindings,
                bindings.FPDFPage_GetAnnot(page_handle, index as i32),
            )
            .ok_or_else(|| PdfOpsError::Pdfium("failed to access annotation".to_string()))?;

            self.write_free_text(
                bindings,
                annot.handle,
                text,
                font_size,
                color,
                position,
                max_width,
                cursor,
            )?
        };
        self.dirty = true;
        Ok(caret)
    }

    /// Remove an annotation from a page's annotation list.
    fn remove_annotation(&mut self, page: u32, index: usize) -> Result<(), PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        {
            let document = self.require_document()?;
            let pdf_page = document.page(page - 1)?;
            let bindings = document.bindings();
            let page_handle = pdf_page.handle();
            if bindings.FPDFPage_RemoveAnnot(page_handle, index as i32) == 0 {
                return Err(PdfOpsError::Pdfium(
                    "failed to remove annotation".to_string(),
                ));
            }
        }
        self.dirty = true;
        Ok(())
    }

    /// Rewrite the appearance of a free-text annotation with a new text colour
    /// and font size, keeping its top-left corner and wrap width.
    fn set_free_text_style(
        &mut self,
        page: u32,
        index: usize,
        color: AnnotationColor,
        font_size: f32,
    ) -> Result<(), PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        let (text, position, max_width) = {
            let document = self.require_document()?;
            let pdf_page = document.page(page - 1)?;
            let bindings = document.bindings();
            let handle = AnnotGuard::new(bindings, pdf_page.annotation(index))
                .ok_or_else(|| PdfOpsError::Pdfium("failed to access annotation".to_string()))?;
            if bindings.FPDFAnnot_GetSubtype(handle.handle)
                != PdfPageAnnotationType::FreeText as FPDF_ANNOTATION_SUBTYPE
            {
                return Err(PdfOpsError::Pdfium(
                    "annotation is not a free-text annotation".to_string(),
                ));
            }
            let rect = annot_rect(bindings, handle.handle).ok_or_else(|| {
                PdfOpsError::Pdfium("failed to read the annotation rectangle".to_string())
            })?;
            let text = read_annot_string(bindings, handle.handle, "Contents").unwrap_or_default();
            (
                text,
                (rect.left, rect.top),
                (rect.right - rect.left).max(0.0),
            )
        };
        self.update_free_text(
            page,
            index,
            &text,
            font_size,
            color,
            position,
            max_width,
            text.chars().count(),
        )
        .map(|_| ())
    }

    fn add_ink_annotation(
        &mut self,
        page: u32,
        color: AnnotationColor,
        points: &[(f32, f32)],
        bounds: (f32, f32, f32, f32),
    ) -> Result<(), PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        let (x, y, width, height) = bounds;
        let document = self.document.as_mut().ok_or(PdfOpsError::NoDocumentOpen)?;
        let pdf_page = document.page(page - 1)?;
        let bindings = document.bindings();

        let handle = AnnotGuard::new(
            bindings,
            bindings.FPDFPage_CreateAnnot(
                pdf_page.handle(),
                PdfPageAnnotationType::Ink as FPDF_ANNOTATION_SUBTYPE,
            ),
        )
        .ok_or_else(|| PdfOpsError::Pdfium("failed to create ink annotation".to_string()))?;
        bindings.FPDFAnnot_SetFlags(handle.handle, ANNOT_FLAG_PRINT);
        bindings.FPDFAnnot_SetRect(handle.handle, &annot_box(x, y, width, height));
        bindings.FPDFAnnot_SetColor(
            handle.handle,
            ANNOT_COLOR_STROKE,
            u32::from(color.red),
            u32::from(color.green),
            u32::from(color.blue),
            u32::from(color.alpha),
        );
        bindings.FPDFAnnot_SetBorder(handle.handle, 0.0, 0.0, INK_WIDTH);

        // One stroke per segment keeps the original drawing: each pair of
        // points is its own line inside the annotation.
        for pair in points.windows(2) {
            let stroke = [
                FS_POINTF {
                    x: pair[0].0,
                    y: pair[0].1,
                },
                FS_POINTF {
                    x: pair[1].0,
                    y: pair[1].1,
                },
            ];
            bindings.FPDFAnnot_AddInkStroke(handle.handle, stroke.as_ptr(), stroke.len());
        }

        self.dirty = true;
        Ok(())
    }

    fn add_markup_annotation(
        &mut self,
        page: u32,
        kind: MarkupKind,
        color: AnnotationColor,
        bounds: (f32, f32, f32, f32),
    ) -> Result<(), PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        let (x, y, width, height) = bounds;
        let subtype = match kind {
            MarkupKind::Highlight => PdfPageAnnotationType::Highlight,
            MarkupKind::Underline => PdfPageAnnotationType::Underline,
            MarkupKind::Strikeout => PdfPageAnnotationType::Strikeout,
            MarkupKind::Squiggly => PdfPageAnnotationType::Squiggly,
        };

        let document = self.document.as_mut().ok_or(PdfOpsError::NoDocumentOpen)?;
        let pdf_page = document.page(page - 1)?;
        let bindings = document.bindings();

        let handle = AnnotGuard::new(
            bindings,
            bindings.FPDFPage_CreateAnnot(pdf_page.handle(), subtype as FPDF_ANNOTATION_SUBTYPE),
        )
        .ok_or_else(|| PdfOpsError::Pdfium("failed to create markup annotation".to_string()))?;
        bindings.FPDFAnnot_SetFlags(handle.handle, ANNOT_FLAG_PRINT);
        bindings.FPDFAnnot_SetRect(handle.handle, &annot_box(x, y, width, height));
        bindings.FPDFAnnot_SetColor(
            handle.handle,
            ANNOT_COLOR_STROKE,
            u32::from(color.red),
            u32::from(color.green),
            u32::from(color.blue),
            u32::from(color.alpha),
        );
        // Text markups are visible only through their attachment points.
        bindings.FPDFAnnot_AppendAttachmentPoints(handle.handle, &annot_quad(x, y, width, height));

        self.dirty = true;
        Ok(())
    }

    fn render_page(&self, page: u32, zoom: f32) -> Result<(u32, u32, Vec<u8>), PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        self.require_document()?.render(page - 1, zoom)
    }

    /// Render a page as a thumbnail that fits inside `max_px` × `max_px`.
    fn render_thumbnail(&self, page: u32, max_px: u32) -> Result<(u32, u32, Vec<u8>), PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        let document = self.require_document()?;
        let pdf_page = document.page(page - 1)?;
        let (width, height) = pdf_page.display_size();
        let longest = width.max(height);
        if longest <= 0.0 {
            return Ok((0, 0, Vec::new()));
        }
        let scale = max_px as f32 / longest;
        document.render(page - 1, scale)
    }
}

/// Read PDF metadata (version, encryption, text layer, page count) via
/// pdfium without keeping the document open. pdfium access belongs here,
/// not in the storage layer; the caller provides the raw file bytes.
pub fn read_pdf_metadata(data: &[u8]) -> Result<PdfMetadata, PdfOpsError> {
    let document = pdfium_or_err()?
        .load_pdf_from_byte_slice(data, None)
        .map_err(pdfium_err)?;

    let version = match document.version() {
        PdfDocumentVersion::Pdf1_0 => "1.0".to_string(),
        PdfDocumentVersion::Pdf1_1 => "1.1".to_string(),
        PdfDocumentVersion::Pdf1_2 => "1.2".to_string(),
        PdfDocumentVersion::Pdf1_3 => "1.3".to_string(),
        PdfDocumentVersion::Pdf1_4 => "1.4".to_string(),
        PdfDocumentVersion::Pdf1_5 => "1.5".to_string(),
        PdfDocumentVersion::Pdf1_6 => "1.6".to_string(),
        PdfDocumentVersion::Pdf1_7 => "1.7".to_string(),
        PdfDocumentVersion::Pdf2_0 => "2.0".to_string(),
        PdfDocumentVersion::Other(v) => format!("{}.{}", v / 10, v % 10),
        PdfDocumentVersion::Unset => "unknown".to_string(),
    };

    Ok(PdfMetadata {
        version,
        is_encrypted: document
            .permissions()
            .security_handler_revision()
            .map(|revision| revision != PdfSecurityHandlerRevision::Unprotected)
            .unwrap_or(false),
        // Probing only the first page is a cheap representative heuristic:
        // born-digital PDFs carry text from page one, scanned PDFs lack it
        // throughout.
        has_text_layer: document.pages().iter().next().is_some_and(|page| {
            page.text()
                .map(|text| !text.all().is_empty())
                .unwrap_or(false)
        }),
        page_count: document.pages().len() as u32,
    })
}

/// Embed a raster image as a full-page image in the given document.
fn embed_raster_page(document: &mut Document, data: &[u8]) -> Result<(), PdfOpsError> {
    let img = image::load_from_memory(data)
        .map_err(|e| PdfOpsError::UnsupportedSource(format!("{e:?}")))?;
    embed_image(document, &img)
}

/// Embed an image as a full-page image in the given document.
fn embed_image(document: &mut Document, img: &image::DynamicImage) -> Result<(), PdfOpsError> {
    use image::GenericImageView;

    let (px_w, px_h) = img.dimensions();
    let width = px_w as i32;
    let height = px_h as i32;

    // pdfium expects premultiplied BGRA; our images are RGBA.
    let mut pixels = img.to_rgba8().into_raw();
    for pixel in pixels.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }

    let bindings = document.bindings();
    let stride = width * 4;
    let bitmap = bindings.FPDFBitmap_CreateEx(
        width,
        height,
        RENDER_BITMAP_BGRA,
        pixels.as_mut_ptr() as *mut std::ffi::c_void,
        stride,
    );
    if bitmap.is_null() {
        return Err(PdfOpsError::Pdfium(
            "failed to allocate an image bitmap".to_string(),
        ));
    }

    // 1 px = 1 pt keeps the source resolution; acceptable for bind.
    let obj = bindings.FPDFPageObj_NewImageObj(document.handle());
    let mut page = document.create_page(px_w as f32, px_h as f32)?;
    let mut page_handle = page.handle();
    let ok = bindings.FPDFImageObj_SetBitmap(&mut page_handle, 1, obj, bitmap);
    bindings.FPDFBitmap_Destroy(bitmap);
    if ok == 0 {
        return Err(PdfOpsError::Pdfium(
            "failed to attach an image to a page".to_string(),
        ));
    }
    page.insert_image(obj, px_w as f32, px_h as f32)
}

/// Rasterize an SVG via resvg and embed it as a full-page image.
#[cfg(feature = "resvg")]
fn embed_vector_page(document: &mut Document, data: &[u8]) -> Result<(), PdfOpsError> {
    let page = crate::render::render_svg_bytes(data, 1.0)
        .map_err(|e| PdfOpsError::UnsupportedSource(format!("{e:?}")))?;
    let img = image::RgbaImage::from_raw(page.width, page.height, page.rgba_data)
        .ok_or_else(|| PdfOpsError::UnsupportedSource("invalid svg render".to_string()))?;
    embed_image(document, &image::DynamicImage::ImageRgba8(img))
}

/// Fallback when the resvg feature is disabled.
#[cfg(not(feature = "resvg"))]
fn embed_vector_page(document: &mut Document, _data: &[u8]) -> Result<(), PdfOpsError> {
    let _ = document;
    Err(PdfOpsError::UnsupportedSource("svg".to_string()))
}
