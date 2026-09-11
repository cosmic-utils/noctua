// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/storage/session.rs
//
// Loads and saves sessions. The default location follows freedesktop
// (XDG_DATA_HOME/noctua/sessions/); the `at` variants allow tests and
// alternative data roots.

use super::StorageError;
use crate::session::Session;
use std::fs;
use std::path::{Path, PathBuf};

/// Directory holding all session files under the default data root.
pub fn sessions_dir() -> Result<PathBuf, StorageError> {
    let data_root = dirs::data_dir()
        .ok_or_else(|| StorageError::Workspace("data directory not found".to_string()))?;
    Ok(sessions_dir_at(&data_root))
}

/// Directory holding session files under an explicit data root.
pub fn sessions_dir_at(data_root: &Path) -> PathBuf {
    data_root.join("noctua").join("sessions")
}

/// Full path for a session file with the given name.
pub fn path_for(name: &str) -> Result<PathBuf, StorageError> {
    Ok(sessions_dir()?.join(format!("{name}.ron")))
}

/// Save a session under the given name.
pub fn save(name: &str, session: &Session) -> Result<PathBuf, StorageError> {
    save_at(&sessions_dir()?, name, session)
}

/// Save a session into an explicit sessions directory.
pub fn save_at(dir: &Path, name: &str, session: &Session) -> Result<PathBuf, StorageError> {
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("{name}.ron"));
    let ron_string = ron::ser::to_string_pretty(session, ron::ser::PrettyConfig::default())?;
    fs::write(&path, ron_string)?;
    Ok(path)
}

/// Load a session by name.
pub fn load(name: &str) -> Result<Session, StorageError> {
    load_at(&sessions_dir()?, name)
}

/// Load a session from an explicit sessions directory.
pub fn load_at(dir: &Path, name: &str) -> Result<Session, StorageError> {
    let content = fs::read_to_string(dir.join(format!("{name}.ron")))?;
    let session: Session = ron::from_str(&content)?;
    Ok(session)
}

/// List all saved session names (file stem, sorted).
pub fn list() -> Result<Vec<String>, StorageError> {
    list_at(&sessions_dir()?)
}

/// List all session names in an explicit sessions directory.
pub fn list_at(dir: &Path) -> Result<Vec<String>, StorageError> {
    let mut names = Vec::new();
    if !dir.exists() {
        return Ok(names);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "ron")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            names.push(stem.to_string());
        }
    }
    names.sort();
    Ok(names)
}

/// Remember which session was opened last.
pub fn set_last(name: &str) -> Result<(), StorageError> {
    set_last_at(&sessions_dir()?, name)
}

/// Remember the last session in an explicit sessions directory.
pub fn set_last_at(dir: &Path, name: &str) -> Result<(), StorageError> {
    fs::create_dir_all(dir)?;
    fs::write(dir.join("_last"), name)?;
    Ok(())
}

/// Return the name of the last opened session, if any.
pub fn last() -> Result<Option<String>, StorageError> {
    last_at(&sessions_dir()?)
}

/// Return the name of the last opened session in an explicit directory.
pub fn last_at(dir: &Path) -> Result<Option<String>, StorageError> {
    let path = dir.join("_last");
    match fs::read_to_string(&path) {
        Ok(name) => {
            let name = name.trim().to_string();
            if name.is_empty() {
                Ok(None)
            } else {
                Ok(Some(name))
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Check whether a session file exists for the given name.
pub fn exists(name: &str) -> Result<bool, StorageError> {
    let path = path_for(name)?;
    Ok(Path::exists(&path))
}
