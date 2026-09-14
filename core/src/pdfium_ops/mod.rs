// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/pdfium_ops/mod.rs
//
// PDF operations via pdfium: bind, insert, delete, rotate, annotate.
// This is the only place in the core that touches pdfium directly.

mod bindings;
mod command;
mod error;
mod manager;
pub mod model;

pub use command::{Command, CommandResult};
pub use error::PdfOpsError;
pub use manager::{PdfOpsManager, read_pdf_metadata};
pub use model::{AnnotationColor, BindSource, PageRef, PdfMetadata};

pub use bindings::try_pdfium;
