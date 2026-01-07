// SPDX-License-Identifier: MPL-2.0 OR Apache-2.0
// src/app/document/file.rs
//
// Opening files and dispatching to the correct concrete document type.

use std::path::PathBuf;

use anyhow::anyhow;

use super::portable::PortableDocument;
use super::raster::RasterDocument;
use super::vector::VectorDocument;
use super::{DocumentContent, DocumentKind};

/// Open a document from a file path and dispatch to the correct type.
///
/// Raster formats are delegated to the `image` crate, which decides
/// based on enabled codecs (e.g. default-formats).
pub fn open_document(path: PathBuf) -> anyhow::Result<DocumentContent> {
    let kind = DocumentKind::from_path(&path)
        .ok_or_else(|| anyhow!("Unsupported document type: {:?}", path))?;

    let content = match kind {
        DocumentKind::Raster => {
            let raster = RasterDocument::open(path)?;
            DocumentContent::Raster(raster)
        }
        DocumentKind::Vector => {
            let vector = VectorDocument::open(path)?;
            DocumentContent::Vector(vector)
        }
        DocumentKind::Portable => {
            let portable = PortableDocument::open(path)?;
            DocumentContent::Portable(portable)
        }
    };

    Ok(content)
}
