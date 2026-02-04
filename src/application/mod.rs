// SPDX-License-Identifier: GPL-3.0-or-later
// src/application/mod.rs
//
// Application layer: use cases, commands, queries, and services.

pub mod commands;
pub mod document_manager;
pub mod services;

// Re-export document manager
pub use document_manager::DocumentManager;
