# Usage Guide

Noctua is an image and document viewer for the COSMIC desktop environment, focused on browsing and annotating documents.

> **Note:** This guide describes the current implementation. Features that are not yet implemented are listed under [Planned Features](#planned-features).

## Overview

Noctua has two tab types in a shared tab strip:

- **Browser tab** (read-only): shows a folder like an image viewer — raster, vector, and PDFs page by page. Nothing is ever modified here.
- **Annotation tab** (planned): edits exactly one PDF.

The application state (open tabs, selection) is stored as a **session** and restored on the next start.

## Getting Started

### Command Line

```bash
noctua [PATH]
```

- With an **image or folder** argument, Noctua opens only that content (no session restore).
- Without an argument, the **last session** is opened automatically.

### Opening a Folder

`Ctrl+O` opens a folder dialog and creates a new browser tab. Noctua scans the folder and lists all supported documents.

## Browser Tab

- The left **navigation panel** shows thumbnails of the folder entries (freedesktop thumbnail cache).
- A **click** selects an entry and shows it on the right:
  - Single-page documents (PNG, JPEG, SVG, …) are shown directly.
  - **Multi-page PDFs** show all pages in a continuous, vertically scrollable preview.
- A **double-click** on a PDF expands its pages directly in the navigation panel (indented, smaller page thumbnails with page numbers). A second double-click collapses them again. At most one PDF is expanded at a time.

## Navigation

| Key | Action | Description |
|:----|:-------|:------------|
| `←` | Previous entry | Move to the previous entry in the folder |
| `→` | Next entry | Move to the next entry in the folder |
| `↑` | Previous page | One page back (in the PDF preview) |
| `↓` | Next page | One page forward (in the PDF preview) |

## Zoom and View

One zoom step is a factor of **1.25** (≈ +25 % when zooming in, −20 % when zooming out).

| Key | Action |
|:----|:-------|
| `Ctrl+=` (or `Ctrl+Shift++`) | Zoom in |
| `Ctrl+-` | Zoom out |
| `Ctrl+1` | Actual size (100 % = 1:1) |
| `Ctrl+0` | Fit to window |

Additionally:

- **`Ctrl` + mouse wheel** zooms, centered on the cursor.
- **Mouse wheel alone** scrolls or pans the image.
- **Drag with the mouse** pans a zoomed image.

The status bar also has zoom controls: zoom out (−), a percentage menu (Fit, 50 %, 100 %, 200 %, 400 %), and zoom in (+).

## Menu Bar

### File

| Item | Shortcut |
|:-----|:---------|
| Open Folder | `Ctrl+O` |
| Close Tab | `Ctrl+W` |
| Quit | `Ctrl+Q` |

### View

| Item | Shortcut |
|:-----|:---------|
| Zoom In | `Ctrl+=` |
| Zoom Out | `Ctrl+-` |
| Fit to Window | `Ctrl+0` |
| 100 % | `Ctrl+1` |
| Fullscreen | `F11` |
| Show Nav Panel | `Ctrl+B` |
| About Noctua | – |

## Status Bar

The status bar shows:

- `[<] [>]` – navigation (entry/page) in the active tab
- `Page N/M` – current page (1/1 for single-page files)
- Zoom (e.g. `150 %` or `Fit`)
- File name
- File size

In the (planned) annotation tab, additionally: `Modified` for unsaved changes.

## Sessions

- Stored under `XDG_DATA_HOME/noctua/sessions/<name>.ron`.
- The last session is saved automatically on exit and restored on the next start.
- Unsaved annotation changes are lost on close (explicit saving is the user's responsibility).

## Supported Formats

- **Raster:** PNG, JPEG, GIF, BMP, TIFF, WebP
- **Vector:** SVG
- **PDF:** multi-page, with page navigation and thumbnails

## Planned Features

The following is not yet implemented (or only prepared):

- **Annotation tab:** edit exactly one PDF — insert/remove/move/rotate pages, annotate (natively in `/Annots`); explicit save (`Ctrl+S`) with a dirty indicator, "Save As".
- **Rotate** (View → Rotate).
- **Flip** (horizontal/vertical).
- **Fit to Width / Fit to Page.**
- **Keyboard pan** (`Ctrl` + arrow keys).
- **Properties panel** with metadata (EXIF, DPI, GPS, …).
- **Set as wallpaper.**
- **Crop / Resize.**
- **Session menu** ("Save Session" / "Save Session As").

Note: JavaScript in PDFs is detected (`has_javascript`) but never executed.
