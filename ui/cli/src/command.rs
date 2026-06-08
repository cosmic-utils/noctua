// SPDX-License-Identifier: GPL-3.0-or-later
// ui/cli/src/command.rs
//
// Command definitions for the Noctua CLI.

use std::path::PathBuf;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub enum Selector {
    /// Select by zero-based index
    Index(usize),
    /// Select by exact name match
    Name(String),
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub enum CliCommand {
    /// Raw workspace command — forwarded directly to WorkspaceManager.
    Workspace(noctua_core::workspace::Command),

    /// Raw document command — forwarded to the internal DocumentManager.
    Document(noctua_core::document::Command),

    /// Select a collection by index or name (no UUID needed).
    SelectCollection(Selector),

    /// Select a document within the active collection by index or name (no UUID needed).
    SelectDocument(Selector),

    /// Import all files from a directory as a new Browser collection.
    ImportDirectory(PathBuf),

    /// Print the current workspace state to stdout.
    ListWorkspace,
}
