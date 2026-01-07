# SPDX-License-Identifier: MPL-2.0
# i18n/en/noctua.ftl
#
# Localization strings for Noctua's user interface (English).


## Application metadata
noctua-app-name = Noctua
noctua-app-description = A wise document and image viewer for the COSMIC™ desktop

## Main window
window-title = { $filename ->
    *[none] Noctua
    *[some] { $filename } — Noctua
}

## Menu entries
menu-file-open = Open…
menu-file-quit = Quit
menu-view-zoom-in = Zoom In
menu-view-zoom-out = Zoom Out
menu-view-zoom-reset = Reset Zoom
menu-view-flip-horizontal = Flip Horizontally
menu-view-flip-vertical = Flip Vertically
menu-view-rotate-cw = Rotate Clockwise
menu-view-rotate-ccw = Rotate Counter-Clockwise

## Note messages
no_document_loaded = No document loaded.

## Labels
zoom = Zoom

## Error messages
error-failed-to-open = Failed to open “{ $path }”.
error-unsupported-format = Unsupported file format.
