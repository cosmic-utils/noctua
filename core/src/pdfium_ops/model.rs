// SPDX-License-Identifier: GPL-3.0-only
// core/src/pdfium_ops/model.rs
//
// Data types for PDF operations. Pure data, no logic.

use crate::storage::document::Format;

/// A source document to bind or insert. The caller (storage layer) reads the
/// file and detects its format; the manager only processes the bytes. Pages
/// are referenced relative to the source: `pages` None means all pages,
/// `Some(range)` selects a 1-based inclusive page range string like "1,3-5".
#[derive(Debug, Clone)]
pub struct BindSource {
    /// Display name (e.g. file name) for error messages; not used for I/O.
    pub name: String,
    /// Raw file bytes, read by the caller.
    pub data: Vec<u8>,
    /// Detected document format.
    pub format: Format,
    /// Optional page selection, 1-based inclusive range string ("1,3-5").
    pub pages: Option<String>,
}

/// Reference to a single page of a source document. 1-based page number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRef {
    pub page: u32,
}

/// RGBA color for annotations and markup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnnotationColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

/// Kinds of text markup: how a selected text region is marked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkupKind {
    /// Transparent background behind the text.
    Highlight,
    /// A line under the text.
    Underline,
    /// A line through the text.
    Strikeout,
    /// A wavy line under the text.
    Squiggly,
}

impl AnnotationColor {
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 255,
        }
    }
}

/// Base font family of a free-text annotation. Mapped onto the standard 14
/// PDF fonts, so no font has to be embedded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontFamily {
    #[default]
    Helvetica,
    Times,
    Courier,
}

/// Horizontal alignment of the lines of a free-text annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

/// The complete style of a free-text annotation. Font family, size and colour
/// live in the annotation's `/DA`; the remaining attributes are drawn in the
/// appearance stream and stored in the annotation's `NoctuaStyle` entry so they
/// survive a round trip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FreeTextStyle {
    pub family: FontFamily,
    /// Font size in points.
    pub font_size: f32,
    pub color: AnnotationColor,
    /// Background fill; `None` is transparent.
    pub background: Option<AnnotationColor>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub align: TextAlign,
}

impl Default for FreeTextStyle {
    fn default() -> Self {
        Self {
            family: FontFamily::Helvetica,
            font_size: 24.0,
            color: AnnotationColor::rgb(0, 0, 0),
            background: None,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            align: TextAlign::Left,
        }
    }
}

/// PDF properties read from a file via pdfium.
#[derive(Debug, Clone)]
pub struct PdfMetadata {
    /// PDF specification version as a display string ("1.7", "2.0", …).
    pub version: String,
    /// Whether the document uses a security handler (i.e. is encrypted).
    pub is_encrypted: bool,
    /// Whether at least one page exposes a text layer.
    pub has_text_layer: bool,
    /// Number of pages in the document.
    pub page_count: u32,
}

/// A free-text annotation read from a page. `x`/`y` is the lower-left corner
/// in page points (origin bottom-left); `width`/`height` extend up and right.
#[derive(Debug, Clone)]
pub struct FreeTextAnnotation {
    /// Index in the page's annotation collection, for later updates.
    pub index: usize,
    /// The annotation's text (`/Contents`).
    pub contents: String,
    /// The annotation's style, recovered from `/DA` and `NoctuaStyle`.
    pub style: FreeTextStyle,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Kind of an interactive form field, as pdfium's form environment reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormFieldKind {
    PushButton,
    Checkbox,
    Radio,
    Text,
    ListBox,
    Combo,
    Signature,
    Unknown,
}

/// One interactive form field on a page.
#[derive(Debug, Clone, PartialEq)]
pub struct FormField {
    /// Position in the page's annotation list.
    pub index: usize,
    /// Fully qualified field name.
    pub name: String,
    pub kind: FormFieldKind,
    /// `/Ff` field flags, as reported by pdfium.
    pub flags: i32,
    /// Current value, empty when unset.
    pub value: String,
    /// Choice options; empty for other kinds.
    pub options: Vec<String>,
    /// Position of the selected option in `options`, if one is selected.
    pub selected: Option<usize>,
    /// Font size from `/DA`, in points.
    pub font_size: f32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Common annotation colors used by the annotation mode.
pub mod palette {
    use super::AnnotationColor;

    pub const YELLOW: AnnotationColor = AnnotationColor::rgb(255, 235, 59);
    pub const GREEN: AnnotationColor = AnnotationColor::rgb(76, 175, 80);
    pub const RED: AnnotationColor = AnnotationColor::rgb(244, 67, 54);
    pub const BLUE: AnnotationColor = AnnotationColor::rgb(33, 150, 243);
    pub const ORANGE: AnnotationColor = AnnotationColor::rgb(255, 152, 0);
}
