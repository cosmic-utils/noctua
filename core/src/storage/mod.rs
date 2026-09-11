// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/storage/mod.rs
//
// Central storage module for file I/O operations and error handling.

pub mod document;
pub mod error;
pub mod portable;
pub mod raster;
pub mod session;
pub mod thumbcache;
pub mod vector;

pub use error::StorageError;
