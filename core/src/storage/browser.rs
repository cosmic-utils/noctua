// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/storage/browser.rs
//
// Browser mode helpers: list the supported documents in a folder.

use std::path::{Path, PathBuf};

use super::StorageError;
use super::document::Format;

/// A document found in a browsed folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserEntry {
    /// Full path to the document file.
    pub path: PathBuf,
    /// File name for display purposes.
    pub name: String,
    /// Whether the file is a PDF, detected once from magic bytes.
    pub is_pdf: bool,
}

/// List the supported documents directly inside a folder.
///
/// Non-recursive, sorted case-insensitively by file name. Unsupported
/// files and hidden files are skipped. Format detection uses magic bytes
/// with an extension fallback, so mislabeled files are not listed.
pub fn list_documents(dir: &Path) -> Result<Vec<BrowserEntry>, StorageError> {
    let mut entries = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Only regular files are candidates.
        if !entry.file_type()?.is_file() {
            continue;
        }

        // Skip hidden files (dotfiles).
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }

        let format = super::document::format(&path)?;
        if format == Format::Unknown {
            continue;
        }
        entries.push(BrowserEntry {
            is_pdf: format == Format::Pdf,
            path,
            name: name_str.into_owned(),
        });
    }

    entries.sort_by_key(|e| e.name.to_lowercase());
    Ok(entries)
}

/// Display name for a path: its last component. Used for tab titles and
/// nav labels; UIs may override it with a custom name.
pub fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}
