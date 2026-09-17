// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/pdfium_ops/manager.rs
//
// PDF operations manager. Holds one open document, executes commands.
// All pdfium calls happen here. Must only be used from a single thread
// (the pdfium worker) — pdfium is not thread-safe.

use std::path::{Path, PathBuf};

use pdfium_render::prelude::*;

use super::bindings::pdfium_or_err;
use super::command::{Command, CommandResult};
use super::error::PdfOpsError;
use super::model::{AnnotationColor, BindSource, PdfMetadata};

use crate::storage::document::Format;

fn pdfium_err(e: PdfiumError) -> PdfOpsError {
    PdfOpsError::Pdfium(format!("{e:?}"))
}

fn to_color(c: AnnotationColor) -> PdfColor {
    PdfColor::new(c.red, c.green, c.blue, c.alpha)
}

/// Detect a bind source's format from magic bytes, mapping storage errors to
/// PDF ops errors. Cheaper than loading full document metadata.
fn load_source_format(source: &BindSource) -> Result<Format, PdfOpsError> {
    crate::storage::document::format(&source.path)
        .map_err(|e| PdfOpsError::UnsupportedSource(format!("{e:?}")))
}

/// Manager for PDF editing operations. Owns the currently open document.
pub struct PdfOpsManager {
    document: Option<PdfDocument<'static>>,
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

    /// Execute a command on the open document.
    pub fn execute(&mut self, command: Command) -> CommandResult {
        match command {
            Command::Open { path } => self.open(&path).into(),
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
            Command::AddTextAnnotation {
                page,
                text,
                x,
                y,
                width,
                height,
            } => self
                .add_text_annotation(page, &text, x, y, width, height)
                .into(),
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
            Command::AddHighlightAnnotation {
                page,
                color,
                x,
                y,
                width,
                height,
            } => self
                .add_highlight_annotation(page, color, x, y, width, height)
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

    fn require_document(&self) -> Result<&PdfDocument<'static>, PdfOpsError> {
        self.document.as_ref().ok_or(PdfOpsError::NoDocumentOpen)
    }

    fn open(&mut self, path: &Path) -> Result<(), PdfOpsError> {
        let document = pdfium_or_err()?
            .load_pdf_from_file(path, None)
            .map_err(pdfium_err)?;
        self.document = Some(document);
        self.path = Some(path.to_path_buf());
        self.dirty = false;
        Ok(())
    }

    fn new_document(&mut self) -> Result<(), PdfOpsError> {
        let document = pdfium_or_err()?.create_new_pdf().map_err(pdfium_err)?;
        self.document = Some(document);
        self.path = None;
        self.dirty = true;
        Ok(())
    }

    fn page_count(&self) -> Result<u32, PdfOpsError> {
        Ok(self.require_document()?.pages().len() as u32)
    }

    fn page_sizes(&self) -> Result<Vec<(f32, f32)>, PdfOpsError> {
        self.require_document()?
            .pages()
            .iter()
            .map(|page| Ok((page.width().value, page.height().value)))
            .collect()
    }

    fn save(&mut self) -> Result<(), PdfOpsError> {
        let path = self.path.clone().ok_or(PdfOpsError::NoDocumentOpen)?;
        self.require_document()?
            .save_to_file(&path)
            .map_err(pdfium_err)?;
        self.dirty = false;
        Ok(())
    }

    fn save_as(&mut self, path: &Path) -> Result<(), PdfOpsError> {
        self.require_document()?
            .save_to_file(path)
            .map_err(pdfium_err)?;
        self.path = Some(path.to_path_buf());
        self.dirty = false;
        Ok(())
    }

    /// Bind sources into a fresh PDF at `target`, then open it.
    fn bind(&mut self, sources: Vec<BindSource>, target: &Path) -> Result<(), PdfOpsError> {
        // Detect every source's format up front, so unsupported formats fail
        // fast without creating a document or loading full metadata.
        let formats: Vec<Format> = sources
            .iter()
            .map(load_source_format)
            .collect::<Result<_, PdfOpsError>>()?;

        for (source, format) in sources.iter().zip(&formats) {
            if matches!(format, Format::Unknown) {
                return Err(PdfOpsError::UnsupportedSource(
                    source.path.display().to_string(),
                ));
            }
        }

        let mut document = pdfium_or_err()?.create_new_pdf().map_err(pdfium_err)?;

        for (source, format) in sources.iter().zip(&formats) {
            match format {
                Format::Pdf => {
                    let src_doc = pdfium_or_err()?
                        .load_pdf_from_file(&source.path, None)
                        .map_err(pdfium_err)?;
                    match &source.pages {
                        None => document.pages_mut().append(&src_doc).map_err(pdfium_err)?,
                        Some(range) => {
                            let dest_index = document.pages().len();
                            document
                                .pages_mut()
                                .copy_pages_from_document(&src_doc, range, dest_index)
                                .map_err(pdfium_err)?
                        }
                    }
                }
                Format::Raster => {
                    embed_raster_page(&mut document, &source.path)?;
                }
                Format::Svg => {
                    embed_vector_page(&mut document, &source.path)?;
                }
                Format::Unknown => {
                    // Validated above; kept defensive.
                    return Err(PdfOpsError::UnsupportedSource(
                        source.path.display().to_string(),
                    ));
                }
            }
        }

        document.save_to_file(target).map_err(pdfium_err)?;
        self.document = Some(document);
        self.path = Some(target.to_path_buf());
        self.dirty = false;
        Ok(())
    }

    fn insert_pages(&mut self, source: &BindSource, at: u32) -> Result<(), PdfOpsError> {
        if at == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        let dest_index = (at - 1) as PdfPageIndex;

        match load_source_format(source)? {
            Format::Pdf => {
                let src_doc = pdfium_or_err()?
                    .load_pdf_from_file(&source.path, None)
                    .map_err(pdfium_err)?;
                let document = self.document.as_mut().ok_or(PdfOpsError::NoDocumentOpen)?;
                match &source.pages {
                    None => document.pages_mut().append(&src_doc).map_err(pdfium_err)?,
                    Some(range) => document
                        .pages_mut()
                        .copy_pages_from_document(&src_doc, range, dest_index)
                        .map_err(pdfium_err)?,
                }
            }
            Format::Raster => {
                // Raster pages are inserted by binding into a scratch document
                // and importing the resulting page.
                let mut scratch = pdfium_or_err()?.create_new_pdf().map_err(pdfium_err)?;
                embed_raster_page(&mut scratch, &source.path)?;
                let document = self.document.as_mut().ok_or(PdfOpsError::NoDocumentOpen)?;
                document
                    .pages_mut()
                    .copy_page_from_document(&scratch, 0, dest_index)
                    .map_err(pdfium_err)?;
            }
            Format::Svg => {
                let mut scratch = pdfium_or_err()?.create_new_pdf().map_err(pdfium_err)?;
                embed_vector_page(&mut scratch, &source.path)?;
                let document = self.document.as_mut().ok_or(PdfOpsError::NoDocumentOpen)?;
                document
                    .pages_mut()
                    .copy_page_from_document(&scratch, 0, dest_index)
                    .map_err(pdfium_err)?;
            }
            Format::Unknown => {
                return Err(PdfOpsError::UnsupportedSource(
                    source.path.display().to_string(),
                ));
            }
        }

        self.dirty = true;
        Ok(())
    }

    fn delete_pages(&mut self, pages: &[u32]) -> Result<(), PdfOpsError> {
        let mut indices: Vec<PdfPageIndex> = Vec::with_capacity(pages.len());
        for page in pages {
            if *page == 0 {
                return Err(PdfOpsError::PageOutOfRange(0));
            }
            indices.push((*page - 1) as PdfPageIndex);
        }
        // Delete in descending order so indices stay valid.
        indices.sort_unstable_by(|a, b| b.cmp(a));
        for index in indices {
            let document = self.document.as_mut().ok_or(PdfOpsError::NoDocumentOpen)?;
            let page = document
                .pages()
                .get(index)
                .map_err(|_| PdfOpsError::PageOutOfRange(index as u32 + 1))?;
            page.delete().map_err(pdfium_err)?;
        }
        self.dirty = true;
        Ok(())
    }

    /// Move a page by copying it into a scratch document, deleting the
    /// original, and re-inserting the copy at the target position.
    /// pdfium-render exposes FPDF_MovePages only as a raw binding without
    /// access to the document handle, so this is the safe high-level path.
    fn move_page(&mut self, from: u32, to: u32) -> Result<(), PdfOpsError> {
        if from == 0 || to == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        let from_idx = (from - 1) as PdfPageIndex;
        let to_idx = (to - 1) as PdfPageIndex;

        // 1. Copy the source page into a scratch document.
        let mut scratch = pdfium_or_err()?.create_new_pdf().map_err(pdfium_err)?;
        {
            let document = self.require_document()?;
            if from_idx >= document.pages().len() {
                return Err(PdfOpsError::PageOutOfRange(from));
            }
            scratch
                .pages_mut()
                .copy_page_from_document(document, from_idx, 0)
                .map_err(pdfium_err)?;
        }

        // 2. Delete the original page.
        {
            let document = self.document.as_mut().ok_or(PdfOpsError::NoDocumentOpen)?;
            let page = document
                .pages()
                .get(from_idx)
                .map_err(|_| PdfOpsError::PageOutOfRange(from))?;
            page.delete().map_err(pdfium_err)?;
        }

        // 3. Insert the copy at the target position. pdfium inserts AT the
        // destination index, so the 1-based target `to` maps directly to the
        // 0-based `to_idx` — the deleted page already shifted everything up.
        let document = self.document.as_mut().ok_or(PdfOpsError::NoDocumentOpen)?;
        document
            .pages_mut()
            .copy_page_from_document(&scratch, 0, to_idx)
            .map_err(pdfium_err)?;

        self.dirty = true;
        Ok(())
    }

    fn rotate_page(&mut self, page: u32, degrees: u16) -> Result<(), PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        let rotation = match degrees {
            0 => PdfPageRenderRotation::None,
            90 => PdfPageRenderRotation::Degrees90,
            180 => PdfPageRenderRotation::Degrees180,
            270 => PdfPageRenderRotation::Degrees270,
            _ => {
                return Err(PdfOpsError::Pdfium(format!(
                    "unsupported rotation: {degrees} degrees"
                )));
            }
        };
        let document = self.document.as_mut().ok_or(PdfOpsError::NoDocumentOpen)?;
        let mut pdf_page = document
            .pages_mut()
            .get((page - 1) as PdfPageIndex)
            .map_err(|_| PdfOpsError::PageOutOfRange(page))?;
        pdf_page.set_rotation(rotation);
        self.dirty = true;
        Ok(())
    }

    fn add_text_annotation(
        &mut self,
        page: u32,
        text: &str,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Result<(), PdfOpsError> {
        let mut pdf_page = self.page_mut(page)?;
        let mut annotation = pdf_page
            .annotations_mut()
            .create_text_annotation(text)
            .map_err(pdfium_err)?;
        annotation
            .set_position(PdfPoints::new(x), PdfPoints::new(y))
            .map_err(pdfium_err)?;
        annotation
            .set_width(PdfPoints::new(width))
            .map_err(pdfium_err)?;
        annotation
            .set_height(PdfPoints::new(height))
            .map_err(pdfium_err)?;
        self.dirty = true;
        Ok(())
    }

    fn add_ink_annotation(
        &mut self,
        page: u32,
        color: AnnotationColor,
        points: &[(f32, f32)],
        bounds: (f32, f32, f32, f32),
    ) -> Result<(), PdfOpsError> {
        let (x, y, width, height) = bounds;
        let mut pdf_page = self.page_mut(page)?;
        let mut annotation = pdf_page
            .annotations_mut()
            .create_ink_annotation()
            .map_err(pdfium_err)?;
        annotation
            .set_position(PdfPoints::new(x), PdfPoints::new(y))
            .map_err(pdfium_err)?;
        annotation
            .set_width(PdfPoints::new(width))
            .map_err(pdfium_err)?;
        annotation
            .set_height(PdfPoints::new(height))
            .map_err(pdfium_err)?;

        // Ink strokes are path objects inside the annotation. Connect the
        // points with line segments.
        for pair in points.windows(2) {
            let (x1, y1) = pair[0];
            let (x2, y2) = pair[1];
            annotation
                .objects_mut()
                .create_path_object_line(
                    PdfPoints::new(x1),
                    PdfPoints::new(y1),
                    PdfPoints::new(x2),
                    PdfPoints::new(y2),
                    to_color(color),
                    PdfPoints::new(2.0),
                )
                .map_err(pdfium_err)?;
        }
        self.dirty = true;
        Ok(())
    }

    fn add_highlight_annotation(
        &mut self,
        page: u32,
        color: AnnotationColor,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Result<(), PdfOpsError> {
        let mut pdf_page = self.page_mut(page)?;
        let mut annotation = pdf_page
            .annotations_mut()
            .create_highlight_annotation()
            .map_err(pdfium_err)?;
        annotation
            .set_position(PdfPoints::new(x), PdfPoints::new(y))
            .map_err(pdfium_err)?;
        annotation
            .set_width(PdfPoints::new(width))
            .map_err(pdfium_err)?;
        annotation
            .set_height(PdfPoints::new(height))
            .map_err(pdfium_err)?;
        annotation
            .set_stroke_color(to_color(color))
            .map_err(pdfium_err)?;
        annotation
            .set_fill_color(to_color(color))
            .map_err(pdfium_err)?;

        // A highlight is visible only with attachment points (quad points).
        let rect = PdfRect::new(
            PdfPoints::new(y),
            PdfPoints::new(x),
            PdfPoints::new(y + height),
            PdfPoints::new(x + width),
        );
        let quad = PdfQuadPoints::from_rect(&rect);
        annotation
            .attachment_points_mut()
            .create_attachment_point_at_end(quad)
            .map_err(pdfium_err)?;

        self.dirty = true;
        Ok(())
    }

    fn page_mut(&mut self, page: u32) -> Result<PdfPage<'static>, PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        let document = self.document.as_mut().ok_or(PdfOpsError::NoDocumentOpen)?;
        document
            .pages_mut()
            .get((page - 1) as PdfPageIndex)
            .map_err(|_| PdfOpsError::PageOutOfRange(page))
    }

    fn render_page(&self, page: u32, zoom: f32) -> Result<(u32, u32, Vec<u8>), PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        let document = self.require_document()?;
        let pdf_page = document
            .pages()
            .get((page - 1) as PdfPageIndex)
            .map_err(|_| PdfOpsError::PageOutOfRange(page))?;

        // scale_page_by_factor expresses the zoom directly and lets
        // pdfium handle the points-to-pixels conversion.
        let config = PdfRenderConfig::new().scale_page_by_factor(zoom);

        let bitmap = pdf_page.render_with_config(&config).map_err(pdfium_err)?;
        let image = bitmap.as_image();
        let rgba = image.to_rgba8();
        Ok((rgba.width(), rgba.height(), rgba.into_raw()))
    }

    /// Render a page as a thumbnail that fits inside `max_px` × `max_px`.
    fn render_thumbnail(&self, page: u32, max_px: u32) -> Result<(u32, u32, Vec<u8>), PdfOpsError> {
        if page == 0 {
            return Err(PdfOpsError::PageOutOfRange(0));
        }
        let document = self.require_document()?;
        let pdf_page = document
            .pages()
            .get((page - 1) as PdfPageIndex)
            .map_err(|_| PdfOpsError::PageOutOfRange(page))?;

        let config = PdfRenderConfig::new().thumbnail(max_px as i32);
        let bitmap = pdf_page.render_with_config(&config).map_err(pdfium_err)?;
        let image = bitmap.as_image();
        let rgba = image.to_rgba8();
        Ok((rgba.width(), rgba.height(), rgba.into_raw()))
    }
}

/// Read PDF metadata (version, encryption, text layer, page count) via
/// pdfium without keeping the document open. pdfium access belongs here,
/// not in the storage layer.
pub fn read_pdf_metadata(path: &Path) -> Result<PdfMetadata, PdfOpsError> {
    let document = pdfium_or_err()?
        .load_pdf_from_file(path, None)
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
fn embed_raster_page(document: &mut PdfDocument<'static>, path: &Path) -> Result<(), PdfOpsError> {
    use image::GenericImageView;

    let img = image::open(path).map_err(|e| PdfOpsError::UnsupportedSource(format!("{e:?}")))?;
    let (px_w, px_h) = img.dimensions();

    // 1 px = 1 pt keeps the source resolution; acceptable for bind.
    let page_size =
        PdfPagePaperSize::new_custom(PdfPoints::new(px_w as f32), PdfPoints::new(px_h as f32));
    let mut page = document
        .pages_mut()
        .create_page_at_end(page_size)
        .map_err(pdfium_err)?;
    page.objects_mut()
        .create_image_object(
            PdfPoints::ZERO,
            PdfPoints::ZERO,
            &img,
            Some(PdfPoints::new(px_w as f32)),
            Some(PdfPoints::new(px_h as f32)),
        )
        .map_err(pdfium_err)?;
    Ok(())
}

/// Rasterize an SVG via resvg and embed it as a full-page image.
#[cfg(feature = "resvg")]
fn embed_vector_page(document: &mut PdfDocument<'static>, path: &Path) -> Result<(), PdfOpsError> {
    use resvg::tiny_skia;

    let data = std::fs::read(path).map_err(|e| PdfOpsError::UnsupportedSource(format!("{e:?}")))?;
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(&data, &options)
        .map_err(|e| PdfOpsError::UnsupportedSource(format!("{e:?}")))?;

    let svg_size = tree.size();
    let px_w = svg_size.width().ceil() as u32;
    let px_h = svg_size.height().ceil() as u32;

    let mut pixmap = tiny_skia::Pixmap::new(px_w, px_h)
        .ok_or_else(|| PdfOpsError::UnsupportedSource("svg too large".to_string()))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    // Convert premultiplied RGBA to straight RGBA.
    let rgba: Vec<u8> = pixmap
        .data()
        .chunks(4)
        .flat_map(|p| {
            let a = p[3];
            if a == 0 {
                return [0u8, 0, 0, 0];
            }
            let inv_a = 255.0 / a as f32;
            [
                (p[0] as f32 * inv_a).round().min(255.0) as u8,
                (p[1] as f32 * inv_a).round().min(255.0) as u8,
                (p[2] as f32 * inv_a).round().min(255.0) as u8,
                a,
            ]
        })
        .collect();

    let img = image::RgbaImage::from_raw(px_w, px_h, rgba)
        .ok_or_else(|| PdfOpsError::UnsupportedSource("invalid svg render".to_string()))?;
    let dynamic = image::DynamicImage::ImageRgba8(img);

    let page_size =
        PdfPagePaperSize::new_custom(PdfPoints::new(px_w as f32), PdfPoints::new(px_h as f32));
    let mut page = document
        .pages_mut()
        .create_page_at_end(page_size)
        .map_err(pdfium_err)?;
    page.objects_mut()
        .create_image_object(
            PdfPoints::ZERO,
            PdfPoints::ZERO,
            &dynamic,
            Some(PdfPoints::new(px_w as f32)),
            Some(PdfPoints::new(px_h as f32)),
        )
        .map_err(pdfium_err)?;
    Ok(())
}

/// Fallback when the resvg feature is disabled.
#[cfg(not(feature = "resvg"))]
fn embed_vector_page(_document: &mut PdfDocument<'static>, path: &Path) -> Result<(), PdfOpsError> {
    Err(PdfOpsError::UnsupportedSource(path.display().to_string()))
}
