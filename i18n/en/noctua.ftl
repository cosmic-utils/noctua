# SPDX-License-Identifier: GPL-3.0-or-later
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

## Placeholders / empty states
no-document = No document loaded

## Labels
zoom = Zoom
tools = Tools
crop = Crop
scale = Scale

## Error messages
error-failed-to-open = Failed to open "{ $path }".
error-unsupported-format = Unsupported file format.

## Properties panel
panel-properties = Properties
meta-dimensions = Dimensions
meta-format = Format
meta-pages = Pages
meta-current-page = Current Page

## Metadata panel (extended)
metadata = Metadata
file-name = File
format = Format
resolution = Resolution
file-size = Size
color-type = Color

## EXIF data
exif-data = EXIF Data
camera = Camera
date-taken = Date
exposure = Exposure
aperture = Aperture
iso = ISO
focal-length = Focal
gps = GPS

## States
loading-metadata = Loading...
