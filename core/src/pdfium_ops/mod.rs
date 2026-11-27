// SPDX-License-Identifier: GPL-3.0-only
// core/src/pdfium_ops/mod.rs
//
// PDF operations via pdfium: bind, insert, delete, rotate, annotate.
// This is the only place in the core that touches pdfium directly.

mod bindings;
mod command;
mod document;
mod error;
mod manager;
pub mod model;
pub mod text;

pub use command::{Command, CommandResult};
pub use error::PdfOpsError;
pub use manager::{PdfOpsManager, read_pdf_metadata};
pub use model::{
    AnnotationColor, BindSource, FontFamily, FormField, FormFieldKind, FreeTextAnnotation,
    FreeTextStyle, PageRef, PdfMetadata, TextAlign,
};
pub use text::helvetica_text_width;

pub use bindings::try_pdfium;
