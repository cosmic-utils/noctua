// SPDX-License-Identifier: GPL-3.0-or-later
// src/storage/mod.rs
//
// Central storage module for file I/O operations and error handling.

pub mod document;
pub mod error;
pub mod portable;
pub mod raster;
pub mod thumbcache;
pub mod vector;
pub mod workspace;

pub use error::StorageError;
