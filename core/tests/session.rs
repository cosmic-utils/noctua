// SPDX-License-Identifier: GPL-3.0-or-later
// core/tests/session.rs
//
// Integration tests for session storage (uses an explicit data root).

use noctua_core_test_common as common;

use noctua_core::session::Session;
use noctua_core::storage::session as storage;
use std::path::PathBuf;

fn session() -> Session {
    Session {
        browser_tabs: vec![PathBuf::from("/home/user/pictures")],
        annotation_tabs: vec![PathBuf::from("/home/user/notes.pdf")],
        active_tab: 1,
    }
}

#[test]
fn save_load_roundtrip() {
    let dir = common::temp_dir("session-roundtrip");
    let sessions = storage::sessions_dir_at(&dir);

    let path = storage::save_at(&sessions, "work", &session()).unwrap();
    assert!(path.ends_with("work.ron"));

    let loaded = storage::load_at(&sessions, "work").unwrap();
    assert_eq!(loaded.browser_tabs, session().browser_tabs);
    assert_eq!(loaded.annotation_tabs, session().annotation_tabs);
    assert_eq!(loaded.active_tab, 1);
    common::remove_dir(&dir);
}

#[test]
fn list_sessions_is_sorted() {
    let dir = common::temp_dir("session-list");
    let sessions = storage::sessions_dir_at(&dir);

    storage::save_at(&sessions, "beta", &session()).unwrap();
    storage::save_at(&sessions, "alpha", &session()).unwrap();

    let names = storage::list_at(&sessions).unwrap();
    assert_eq!(names, vec!["alpha".to_string(), "beta".to_string()]);
    common::remove_dir(&dir);
}

#[test]
fn last_session_roundtrip() {
    let dir = common::temp_dir("session-last");
    let sessions = storage::sessions_dir_at(&dir);

    // Empty directory: no last session.
    assert_eq!(storage::last_at(&sessions).unwrap(), None);

    storage::set_last_at(&sessions, "work").unwrap();
    assert_eq!(
        storage::last_at(&sessions).unwrap(),
        Some("work".to_string())
    );
    common::remove_dir(&dir);
}

#[test]
fn list_on_missing_directory_is_empty() {
    let dir = common::temp_dir("session-missing");
    let sessions = storage::sessions_dir_at(&dir);
    // The directory was never created; listing must not fail.
    assert_eq!(storage::list_at(&sessions).unwrap(), Vec::<String>::new());
    common::remove_dir(&dir);
}
