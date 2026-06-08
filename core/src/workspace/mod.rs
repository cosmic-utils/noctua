// SPDX-License-Identifier: GPL-3.0-or-later
// src/workspace/mod.rs
//
// Workspace management and read-only API for the view.

mod command;
mod error;
pub mod manager;
mod model;
pub(crate) mod update;

pub use command::{Command, CommandResult};
pub use error::WorkspaceError;
pub use model::{Collection, CollectionType, DocumentEntry, Workspace};
