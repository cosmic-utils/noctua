# SPDX-License-Identifier: GPL-3.0-or-later
# i18n/en/noctua.ftl
#
# Localization strings for Noctua (English).
# Usage: fl!("message-id", arg1, arg2, ...)
#
# Positional arguments ($1, $2, ...) are used for variable content.


## Application
noctua-app-name = Noctua
noctua-app-description = A document and image viewer for the COSMIC desktop


## Main window
window-title = { $filename ->
    [none] Noctua
   *[some] { $filename } — Noctua
}


## Menu entries
menu-main = Menu
menu-file-open = Open…
menu-file-quit = Quit
menu-view-zoom-in = Zoom In
menu-view-zoom-out = Zoom Out
menu-view-zoom-reset = Reset Zoom
menu-view-zoom-fit = Fit to Window
menu-view-flip-horizontal = Flip Horizontally
menu-view-flip-vertical = Flip Vertically
menu-view-rotate-cw = Rotate Clockwise
menu-view-rotate-ccw = Rotate Counter-Clockwise


## Tooltips (for buttons and icons)
tooltip-nav-previous = Previous document
tooltip-nav-next = Next document
tooltip-nav-toggle = Toggle navigation panel
tooltip-zoom-in = Zoom in
tooltip-zoom-out = Zoom out
tooltip-zoom-fit = Fit to window
tooltip-rotate-ccw = Rotate counter-clockwise
tooltip-rotate-cw = Rotate clockwise
tooltip-flip-horizontal = Flip horizontally
tooltip-flip-vertical = Flip vertically
tooltip-info-panel = Toggle info panel


## Footer / Status bar
status-zoom-fit = Fit
status-zoom-percent = { $percent }%
status-doc-dimensions = { $width } × { $height }
status-nav-position = { $current } / { $total }
status-separator =  |


## Placeholders / Empty states
no-document = No document loaded


## Labels
label-zoom = Zoom
label-tools = Tools
label-crop = Crop
label-scale = Scale
label-page = Page
label-pages = Pages


## Loading states
loading-metadata = Loading metadata…
loading-thumbnails = Loading { $current } / { $total }…


## Error messages
error-failed-to-open = Failed to open "{ $path }"
error-unsupported-format = Unsupported file format
error-no-image-loaded = No image loaded


## Properties panel
panel-properties = Properties
panel-actions = Actions

meta-section-file = File Information
meta-section-exif = Camera Information
meta-section-image = Image Information

## File metadata
meta-filename = Name
meta-format = Format
meta-dimensions = Dimensions
meta-filesize = Size
meta-colortype = Color Type
meta-path = Path
meta-pages = Pages
meta-current-page = Current Page

## Image metadata
meta-width = Width
meta-height = Height
meta-depth = Bit Depth

## EXIF metadata
meta-camera = Camera
meta-datetime = Date Taken
meta-exposure = Exposure
meta-aperture = Aperture
meta-iso = ISO { $iso }
meta-focal = Focal Length
meta-gps = GPS Location

## Action buttons
action-set-wallpaper = Set as Wallpaper
action-open-with = Open With…
action-show-in-folder = Show in Folder


## Navigation panel (thumbnails)
nav-panel-title = Pages
nav-panel-loading = Loading { $current } / { $total }…


## Format panel
format-section-title = Paper Format
format-section-subtitle = Select paper size for export
orientation-section-title = Orientation
