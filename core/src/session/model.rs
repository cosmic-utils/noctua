// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/session/model.rs
//
// Data model for a session. A session remembers the application state:
// open browser tabs (folders) and annotation tabs (PDF files), plus the
// active tab. No logic, no UI, no filesystem.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Session {
    /// Folder paths of open browser tabs, in tab order.
    pub browser_tabs: Vec<PathBuf>,
    /// PDF file paths of open annotation tabs, in tab order.
    pub annotation_tabs: Vec<PathBuf>,
    /// Index of the focused tab. Flat index into
    /// `browser_tabs ++ annotation_tabs`.
    pub active_tab: usize,
}
