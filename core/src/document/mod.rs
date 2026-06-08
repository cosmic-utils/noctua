// SPDX-License-Identifier: GPL-3.0-or-later
// src/document/mod.rs
//
// Document management and read-only API for the view.

mod command;
mod error;
pub mod format;

pub mod manager;
pub mod model;
pub(crate) mod update;

// Re-export command and manager types for workspace management.

pub use command::{Command, CommandResult};
pub use error::DocumentError;
pub use format::PaperSize;
pub use manager::DocumentManager;
pub use model::{DocumentInfo, Kind, LoadedContent, PageInfo};
