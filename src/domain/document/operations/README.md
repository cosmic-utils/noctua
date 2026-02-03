# Document Operations

This module provides transformation, rendering, and export operations for documents.

## Architecture: Two-Level Operations

The operations module is designed with **two distinct levels** of abstraction:

### 1. Low-Level Operations (Internal/Private)

**Purpose:** Direct manipulation of pixel data for raster images.

**Visibility:** `pub(crate)` - Internal to the crate only.

**Location:** `transform.rs` (internal helpers)

**Functions:**
- `apply_rotation(img, rotation)` - Rotate raster pixels
- `apply_flip(img, direction)` - Flip raster pixels
- `crop_to_image(img, x, y, w, h)` - Crop raster to image

**When to use:**
- ONLY in document type implementations (RasterDocument, VectorDocument, PortableDocument)
- NOT accessible outside the crate
- NOT for application or UI code

**Example:**
```rust
// INTERNAL USE ONLY - in document type implementations
impl Transformable for RasterDocument {
    fn rotate(&mut self, rotation: Rotation) {
        // Low-level operation used internally
        self.image = apply_rotation(self.image, rotation);
    }
}
```

### 2. High-Level Operations (Type-Agnostic)

**Purpose:** Document transformations that work across **all** document types (Raster, Vector, Portable).

**Location:** `transform.rs` (high-level section)

**Functions:**
- `rotate_document_cw(document)` - Rotate any document 90° CW
- `rotate_document_ccw(document)` - Rotate any document 90° CCW
- `flip_document_horizontal(document)` - Flip any document horizontally
- `flip_document_vertical(document)` - Flip any document vertically
- `rotate_document_to(document, rotation)` - Rotate to specific angle
- `reset_document_transforms(document)` - Reset all transformations

**When to use:**
- In application commands (`TransformDocumentCommand`)
- In UI message handlers
- Anywhere you work with `DocumentContent` (type-erased document)

**Example:**
```rust
use crate::domain::document::operations::transform;

// RECOMMENDED: Use high-level operations
let mut document = DocumentContent::Raster(raster_doc);
transform::rotate_document_cw(&mut document)?;
transform::flip_document_horizontal(&mut document)?;

// Works with Vector and Portable too!
let mut svg = DocumentContent::Vector(vector_doc);
transform::rotate_document_cw(&mut svg)?; // Lossless viewport transform

// Works with PDF!
let mut pdf = DocumentContent::Portable(portable_doc);
transform::rotate_document_cw(&mut pdf)?; // Backend handles rendering
```

## Why This Separation?

### Why Low-Level Operations Are Internal

**Problem:** Exposing low-level operations creates confusion:
- Developers don't know whether to use `apply_rotation()` or `rotate_document_cw()`
- Low-level operations only work on `DynamicImage`, not `DocumentContent`
- Creates two ways to do the same thing (violates DRY)

**Solution:** Make them `pub(crate)`:
```rust
// NOT POSSIBLE - apply_rotation is internal
transform::apply_rotation(img, Rotation::Cw90);  // Compile error!

// USE THIS - high-level operation
transform::rotate_document_cw(&mut document)?;  // Works!
```

### Why High-Level Operations Exist

**Problem without them:**
```rust
// Coupled to implementation details
match document {
    DocumentContent::Raster(ref mut doc) => doc.rotate(Rotation::Cw90),
    DocumentContent::Vector(ref mut doc) => doc.rotate(Rotation::Cw90),
    DocumentContent::Portable(ref mut doc) => doc.rotate(Rotation::Cw90),
}
```

**Solution:**
```rust
// Single API for all types
transform::rotate_document_cw(&mut document)?;
```

### Benefits

1. **Single Source of Truth**
   - Rotation logic (handling RotationMode::Fine, etc.) is in ONE place
   - No duplication across UI handlers, commands, and tests

2. **Type Safety**
   - Works through `DocumentContent` abstraction
   - Compiler ensures all document types implement required traits

3. **Future-Proof**
   - Adding new document types (DJVU, EPUB) doesn't require updating call sites
   - Operations automatically work with new types

4. **Testable**
   - High-level operations can be tested independently
   - No UI dependencies

## Implementation Details

### How It Works

High-level operations use the `Transformable` trait:

```rust
pub fn rotate_document_cw(document: &mut DocumentContent) -> DocResult<()> {
    let new_rotation_mode = document.transform_state().rotation.rotate_cw();
    
    match new_rotation_mode {
        RotationMode::Standard(rot) => document.rotate(rot),
        RotationMode::Fine(deg) => {
            // Convert fine rotation to nearest 90° standard rotation
            // ...
        }
    }
    
    Ok(())
}
```

This delegates to the document type's implementation:

- **Raster:** Actual pixel rotation via `imageops::rotate90()`
- **Vector:** Viewport matrix transformation (lossless!)
- **Portable:** View rotation, rendered by backend (Poppler)

### Each Type Transforms Differently

```rust
// Raster: Pixel manipulation (lossy for fine rotations)
impl Transformable for RasterDocument {
    fn rotate(&mut self, rotation: Rotation) {
        self.image = apply_rotation(self.image, rotation);
    }
}

// Vector: Viewport transform (always lossless!)
impl Transformable for VectorDocument {
    fn rotate(&mut self, rotation: Rotation) {
        self.transform_matrix = self.transform_matrix.rotate(rotation.to_degrees());
        // No rasterization needed
    }
}

// Portable: View rotation (backend handles rendering)
impl Transformable for PortableDocument {
    fn rotate(&mut self, rotation: Rotation) {
        self.view_rotation = (self.view_rotation + rotation.to_degrees()) % 360;
    }
}
```

## Usage Guidelines

### Prefer High-Level Operations

```rust
// In application commands
pub fn execute(&self, manager: &mut DocumentManager) -> DocResult<()> {
    let document = manager.current_document_mut()?;
    transform::rotate_document_cw(document)?;
    Ok(())
}

// In UI message handlers
AppMessage::RotateCW => {
    if let Some(doc) = &mut self.model.document {
        transform::rotate_document_cw(doc)?;
    }
}
```

### Don't Use Low-Level Operations in Application/UI Code

```rust
// COMPILE ERROR - Low-level operations are pub(crate)
let pixels = transform::apply_rotation(img, Rotation::Cw90);  // Won't compile!

// CORRECT - Use high-level operations
transform::rotate_document_cw(&mut document)?;
```

### ℹ️ Low-Level Operations in Document Implementations

Low-level operations are only accessible within document type implementations:

```rust
// INTERNAL ONLY - in domain/document/types/raster.rs
impl Transformable for RasterDocument {
    fn rotate(&mut self, rotation: Rotation) {
        // This works because we're inside the crate
        self.image = apply_rotation(self.image, rotation);
    }
}
```

## Module Structure

```
operations/
├── mod.rs              # Public API exports
├── transform.rs        # Low-level + High-level transforms
├── render.rs           # Rendering utilities (scale, fit, etc.)
├── export.rs           # Export to various formats
└── README.md           # This file
```

## Adding New Operations

When adding a new operation:

1. **Add low-level function** (if pixel manipulation is needed) - mark as `pub(crate)`
2. **Add high-level function** that works on `DocumentContent` - mark as `pub`
3. **Export high-level function only** from `mod.rs`
4. **Update domain exports** in `domain/document/mod.rs`
5. **Create command** in `application/commands/`

Example:

```rust
// 1. Low-level (internal only) - in transform.rs
pub(crate) fn apply_grayscale(img: DynamicImage) -> DynamicImage { ... }

// 2. High-level (public API) - in transform.rs
pub fn grayscale_document(document: &mut DocumentContent) -> DocResult<()> {
    // Delegates to Transformable trait or uses low-level helper
    ...
}

// 3. Export high-level only - in operations/mod.rs
pub use transform::{grayscale_document};  // NOT apply_grayscale!

// 4. Export from domain - in document/mod.rs
pub use operations::{grayscale_document};

// 5. Command - in application/commands/
pub struct GrayscaleCommand;
impl GrayscaleCommand {
    pub fn execute(&self, manager: &mut DocumentManager) -> DocResult<()> {
        let doc = manager.current_document_mut()?;
        transform::grayscale_document(doc)  // High-level operation
    }
}
```

## Related Concepts

- **Traits:** `Renderable`, `Transformable`, `MultiPage` (in `domain/document/core/document.rs`)
- **Type Erasure:** `DocumentContent` enum (in `domain/document/core/content.rs`)
- **Commands:** Application layer operations (in `application/commands/`)
- **Domain Layer:** Pure business logic, no UI dependencies
