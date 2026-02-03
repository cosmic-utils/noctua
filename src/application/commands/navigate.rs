// SPDX-License-Identifier: GPL-3.0-or-later
// src/application/commands/navigate.rs
//
// Navigation command: next/previous document.
// Reserved for future CQRS pattern - currently using direct DocumentManager methods.

#![allow(dead_code)]

use std::path::PathBuf;

use crate::application::document_manager::DocumentManager;
use crate::domain::document::core::document::DocResult;

/// Navigation direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationDirection {
    /// Navigate to next document.
    Next,
    /// Navigate to previous document.
    Previous,
}

/// Navigate command.
pub struct NavigateCommand {
    direction: NavigationDirection,
}

impl NavigateCommand {
    /// Create a new navigate command.
    #[must_use]
    pub fn new(direction: NavigationDirection) -> Self {
        Self { direction }
    }

    /// Execute the navigate command.
    pub fn execute(&self, manager: &mut DocumentManager) -> DocResult<Option<PathBuf>> {
        let path = match self.direction {
            NavigationDirection::Next => manager.next_document(),
            NavigationDirection::Previous => manager.previous_document(),
        };

        Ok(path)
    }

    /// Check if navigation is possible.
    #[must_use]
    pub fn can_execute(&self, manager: &DocumentManager) -> bool {
        match self.direction {
            NavigationDirection::Next => manager.has_next(),
            NavigationDirection::Previous => manager.has_previous(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigate_command_creation() {
        let cmd = NavigateCommand::new(NavigationDirection::Next);
        assert_eq!(cmd.direction, NavigationDirection::Next);

        let cmd = NavigateCommand::new(NavigationDirection::Previous);
        assert_eq!(cmd.direction, NavigationDirection::Previous);
    }
}
