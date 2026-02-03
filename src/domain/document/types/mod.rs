// SPDX-License-Identifier: GPL-3.0-or-later
// src/domain/document/types/mod.rs
//
// Concrete document type implementations.

pub mod raster;
#[cfg(feature = "vector")]
pub mod vector;
#[cfg(feature = "portable")]
pub mod portable;
