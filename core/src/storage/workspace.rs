// SPDX-License-Identifier: GPL-3.0-or-later
// src/storage/workspace.rs
//
// Handles serialization and deserialization of the workspace state.

use super::StorageError;
use serde::{Serialize, de::DeserializeOwned};
use std::fs;
use std::path::Path;

/// Serialize and save data to the specified path.
pub fn save<T: Serialize>(data: &T, path: &Path) -> Result<(), StorageError> {
    let ron_string = ron::ser::to_string_pretty(data, ron::ser::PrettyConfig::default())?;
    fs::write(path, ron_string)?;
    Ok(())
}

/// Load and deserialize data from the specified path.
pub fn load<T: DeserializeOwned>(path: &Path) -> Result<T, StorageError> {
    let content = fs::read_to_string(path)?;
    let data: T = ron::from_str(&content)?;
    Ok(data)
}
