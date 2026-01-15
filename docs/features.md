# Noctua Features

This document describes the implemented and planned features of Noctua, a modern image viewer for the COSMIC desktop environment.

## Current Features

### Document Support

#### Raster Images (Implemented)
- **Formats**: PNG, JPEG, GIF, BMP, TIFF, WebP, and all formats supported by `image-rs`
- **Capabilities**:
  - Full pixel-perfect rendering at 100% zoom
  - Lossless transformations (rotate, flip)
  - Real-time transformation preview
  - EXIF metadata extraction

#### Vector Graphics (Planned)
- **Formats**: SVG
- **Status**: Code structure prepared, but rendering not yet implemented
- **Planned capabilities**:
  - High-quality rendering via `resvg`
  - Scalable display without quality loss
  - Metadata extraction
  - Transformations (rotate, flip)

#### Portable Documents (Planned)
- **Formats**: PDF
- **Status**: Code structure prepared, but rendering not yet implemented
- **Planned capabilities**:
  - First page rendering
  - Multi-page navigation
  - Basic transformations on rendered pages

### Navigation

#### Folder Navigation (Implemented)
- **Automatic folder scanning**: When opening an image, all supported images in the same folder are indexed
- **Quick navigation**:
  - Arrow keys (Left/Right) to navigate between images
  - Footer displays current position (e.g., "3 / 42")
  - Seamless transitions between images

#### File Opening (Implemented)
- **Command-line arguments**: Open images directly from terminal
- **Default directory**: Configurable starting location (defaults to XDG Pictures)
- File dialog not yet implemented

### View Controls

#### Zoom (Implemented)
- **Mouse wheel**: Zoom in/out centered on cursor position
- **Keyboard shortcuts**:
  - `+` or `=` - Zoom in
  - `-` - Zoom out
  - `1` - Reset to 100% (Actual Size)
  - `f` - Fit to window
- **View modes**:
  - **Fit**: Automatically scales image to fit window while preserving aspect ratio
  - **Actual Size**: Displays image at 100% (1:1 pixel mapping)
  - **Custom**: Any zoom level from 10% to 2000%
- **Footer display**: Real-time zoom percentage or "Fit" indicator

#### Pan (Implemented)
- **Mouse drag**: Click and drag to pan around zoomed images
- **Keyboard shortcuts**: `Ctrl + Arrow Keys` for precise panning
- **Smart boundaries**: Pan is automatically limited to image boundaries
- **Auto-center**: Images smaller than viewport are automatically centered

#### Bidirectional State Sync (Implemented)
- Mouse interactions update keyboard/button controls
- Keyboard/button controls update mouse interaction state
- No conflicts between input methods

### Transformations

#### Image Manipulation (Implemented)
- **Rotate**:
  - `r` - Rotate 90° clockwise
  - `Shift + r` - Rotate 90° counter-clockwise
  - Toolbar buttons available
- **Flip**:
  - `h` - Flip horizontally (mirror)
  - `v` - Flip vertically
  - Toolbar buttons available
- **Lossless operations**: All transformations preserve original image quality
- **Real-time preview**: Changes are immediately visible

### User Interface

#### COSMIC Integration (Implemented)
- **Native COSMIC design**: Follows COSMIC desktop design language
- **Theme support**: Automatically adapts to system light/dark theme
- **Header toolbar**:
  - Navigation controls (Previous/Next)
  - Transformation buttons (Rotate, Flip)
  - Information panel toggle
- **Footer bar**:
  - Zoom controls with buttons
  - Current zoom level display
  - Image dimensions
  - Navigation position counter

#### Panels (Implemented)
- **Properties panel**:
  - Image metadata display
  - File information
  - Action buttons:
    - Set as Wallpaper (works with COSMIC, GNOME, KDE, XFCE, and tiling WMs)
    - Open With… (planned)
    - Show in Folder (planned)
  - Toggle with `i` key or toolbar button
- **Navigation panel** (Left sidebar):
  - Toggle with `n` key or toolbar button
  - Content not yet implemented

#### Keyboard Shortcuts (Implemented)
Full keyboard-driven workflow:
- Navigation: `←` `→`
- Zoom: `+` `-` `1` `f`
- Pan: `Ctrl + ←` `Ctrl + →` `Ctrl + ↑` `Ctrl + ↓`
- Transform: `r` `Shift+r` `h` `v`
- Panels: `i` `n`
- Actions: `w` (Set as Wallpaper)

### Desktop Integration

#### Wallpaper Support (Implemented)
- **Set as Wallpaper**: One-click wallpaper setting with cross-desktop compatibility
- **Supported desktop environments**:
  - COSMIC Desktop (direct config file integration)
  - GNOME (via gsettings)
  - KDE Plasma (via wallpaper crate)
  - XFCE (via wallpaper crate)
  - Tiling window managers (via feh)
- **Multiple access methods**:
  - Keyboard shortcut: `w`
  - Icon button in Properties panel
  - Tooltip support for discoverability
- **Automatic fallback**: Tries multiple methods until one succeeds

### Configuration

#### Persistent Settings (Implemented)
- **Panel states**: Remembers which panels were open
- **Default directory**: Customizable starting location
- **Settings location**: `~/.config/noctua/config.toml`

### Technical Features

#### Architecture (Implemented)
- **Clean separation**: View layer agnostic to document format
- **Polymorphic documents**: Single `DocumentContent` interface for all formats
- **Efficient rendering**: Leverages COSMIC's iced renderer
- **Type-safe transformations**: Compile-time guarantees for image operations

#### Performance (Implemented)
- **Lazy loading**: Images loaded on-demand
- **Efficient folder scanning**: Fast directory traversal
- **Minimal memory footprint**: Only active document kept in memory
- **Smooth zooming**: Hardware-accelerated rendering

## Planned Features

### High Priority

#### SVG Support
- Implement proper SVG parsing and rendering
- Integration with `resvg` library
- Scalable vector display
- Transformation support

#### PDF Support
- Implement PDF rendering backend
- First page display
- Multi-page navigation
- Page thumbnails

#### File Operations
- File dialog integration (OpenPath message prepared)
- Save transformed images
- Copy/Move/Delete operations
- Drag-and-drop support

#### Error Handling
- User-friendly error messages (ShowError/ClearError prepared)
- Graceful handling of corrupted files
- Recovery suggestions

### Medium Priority

#### Multi-format TIFF Support
- Multi-page TIFF navigation
- Page thumbnails

#### Enhanced Navigation
- Thumbnail strip
- Grid view for folder contents
- Quick jump to file

#### Slideshow Mode
- Auto-advance timer
- Configurable intervals
- Fullscreen support

### Low Priority

#### Advanced Editing
- Crop tool (message prepared)
- Scale/Resize tool (message prepared)
- Basic color adjustments

#### Metadata Editing
- EXIF data modification
- Comment annotations
- Tag management

## Feature Status Legend

- **Implemented**: Fully functional and tested
- **Planned**: Design complete, implementation pending
- **Partial**: Basic functionality exists, enhancements needed
- **In Progress**: Currently being developed
