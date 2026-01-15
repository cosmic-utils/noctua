# Usage Guide

Noctua is a modern image viewer for the COSMIC desktop environment with full keyboard support.

## Opening Images

### Command Line
Open an image directly from the terminal:
```bash
noctua /path/to/image.png
```

When you open an image, Noctua automatically scans the folder and indexes all supported images for quick navigation.

### Supported Formats
- **Raster Images**: PNG, JPEG, GIF, BMP, TIFF, WebP, and all formats supported by `image-rs`
- **Vector Graphics**: SVG (planned, not yet implemented)
- **Portable Documents**: PDF (planned, not yet implemented)

## Keyboard Shortcuts

All keyboard shortcuts are case-insensitive unless otherwise noted.

### Navigation

| Key   | Action            | Description                                    |
|:------|:------------------|:-----------------------------------------------|
| `←`   | Previous image    | Navigate to the previous image in the folder   |
| `→`   | Next image        | Navigate to the next image in the folder       |

The footer shows your current position (e.g., "3 / 42").

### Zoom and View

| Key       | Action                     | Description                                           |
|:----------|:---------------------------|:------------------------------------------------------|
| `+` / `=` | Zoom in                    | Increase zoom by 10%                                  |
| `-`       | Zoom out                   | Decrease zoom by ~9%                                  |
| `1`       | Actual size (100%)         | Display image at pixel-perfect 1:1 scale              |
| `f`       | Fit to window              | Scale image to fit the window while preserving ratio  |

You can also zoom with the **mouse wheel** - the zoom centers on your cursor position.

The current zoom level is displayed in the footer (e.g., "150%" or "Fit").

### Pan

Pan controls allow you to move around zoomed images:

| Key        | Action             | Description                              |
|:-----------|:-------------------|:-----------------------------------------|
| `Ctrl + ←` | Pan left           | Move view to the left                    |
| `Ctrl + →` | Pan right          | Move view to the right                   |
| `Ctrl + ↑` | Pan up             | Move view upward                         |
| `Ctrl + ↓` | Pan down           | Move view downward                       |
| `0`        | Reset pan          | Center the image                         |

You can also **click and drag** with the mouse to pan around zoomed images.

### Transformations

| Key         | Action                         | Description                               |
|:------------|:-------------------------------|:------------------------------------------|
| `h`         | Flip horizontal                | Mirror the image horizontally             |
| `v`         | Flip vertical                  | Flip the image upside down                |
| `r`         | Rotate clockwise               | Rotate 90° clockwise                      |
| `Shift + r` | Rotate counter-clockwise       | Rotate 90° counter-clockwise              |

All transformations are lossless and show in real-time.

### Panels and UI

| Key | Action                 | Description                              |
|:----|:-----------------------|:-----------------------------------------|
| `i` | Toggle properties      | Show/hide the properties panel (metadata)|
| `n` | Toggle navigation      | Show/hide the navigation sidebar         |

## Mouse Controls

### Zoom
- **Mouse wheel up/down**: Zoom in/out centered on cursor
- **Footer buttons**: Click zoom in/out buttons for step-by-step control

### Pan
- **Click and drag**: Pan around zoomed images
- Hold and drag anywhere on the image to move the view

### Navigation
- **Footer navigation**: Use Previous/Next buttons to browse images

## Toolbar

The header toolbar provides quick access to common operations:

### Left Side
- **Navigation toggle**: Show/hide the sidebar
- **Previous/Next buttons**: Navigate between images in the folder
- **Rotate buttons**: Rotate clockwise or counter-clockwise
- **Flip buttons**: Flip horizontally or vertically

### Right Side
- **Properties toggle**: Show/hide the metadata panel

## Footer Information

The footer displays useful information:
- **Zoom controls**: Zoom out, current zoom level, zoom in, fit buttons
- **Image dimensions**: Width × Height in pixels
- **Navigation position**: Current image / Total images in folder

## Tips and Tricks

### Keyboard-Driven Workflow
Noctua is designed for efficient keyboard use:
1. Open an image from terminal
2. Use `←` `→` to browse through the folder
3. Press `r` to rotate, `h` or `v` to flip
4. Use `+` `-` to zoom, `Ctrl + arrows` to pan
5. Press `i` to check metadata

### Zoom and Pan Together
- Zoom with mouse wheel while hovering over the area you want to examine
- The zoom centers on your cursor, making it easy to focus on details
- Once zoomed, drag with mouse or use `Ctrl + arrows` to navigate

### Bidirectional Controls
Mouse and keyboard work together seamlessly:
- Zoom with keyboard, pan with mouse
- Zoom with mouse wheel, the footer updates automatically
- Pan with keyboard, continue with mouse drag

## Configuration

Settings are stored in `~/.config/noctua/config.toml`.

### Configurable Options
- **Default directory**: Set your preferred starting location
- **Panel states**: Your panel preferences are remembered between sessions

## Planned Features

The following features are prepared in code but not yet implemented:

### File Operations
- File open dialog
- Save transformed images
- Copy/Move/Delete operations

### Document Support
- SVG rendering with `resvg`
- PDF rendering with multi-page support

### Advanced Editing
- Crop mode (`c` key prepared)
- Scale/Resize mode (`s` key prepared)

See [features.md](features.md) for a complete list of planned features.
