# SPDX-License-Identifier: GPL-3.0-or-later
# i18n/sv/noctua.ftl
#
# Localization strings for Noctua (Swedish).
# Usage: fl!("message-id", arg1, arg2, ...)
#
# Positional arguments ($1, $2, ...) are used for variable content.


## Program
noctua-app-name = Noctua
noctua-app-description = En dokument och bildvisare för COSMIC skrivbordet

## Main window
window-title = { $filename ->
    [none] Noctua
   *[some] { $filename } — Noctua
}


## Menyposter
menu-main = Meny
menu-file-open = Öppna…
menu-file-quit = Avsluta
menu-view-zoom-in = Zooma in
menu-view-zoom-out = Zooma ut
menu-view-zoom-reset = Återställ zoom
menu-view-zoom-fit = Anpassa till fönster
menu-view-flip-horizontal = Vänd horisontellt
menu-view-flip-vertical = Vänd vertikalt
menu-view-rotate-cw = Rotera medurs
menu-view-rotate-ccw = Rotera moturs


## Verktygstips (för knappar och ikoner)
tooltip-nav-previous = Föregående dokument
tooltip-nav-next = Nästa dokument
tooltip-nav-toggle = Växla navigeringspanel
tooltip-zoom-in = Zooma in
tooltip-zoom-out = Zooma ut
tooltip-zoom-fit = Anpassa till fönster
tooltip-rotate-ccw = Rotera moturs
tooltip-rotate-cw = Rotera medurs
tooltip-flip-horizontal = Vänd horisontellt
tooltip-flip-vertical = Vänd vertikalt
tooltip-info-panel = Växla informationspanel


## Sidfot / Statusfält
status-zoom-fit = Passa
status-zoom-percent = { $percent }%
status-doc-dimensions = { $width } × { $height }
status-nav-position = { $current } / { $total }
status-separator =  |


## Platshållare / Tomma tillstånd
no-document = Inget dokument laddat


## Etiketter
label-zoom = Zoom
label-tools = Verktyg
label-crop = Beskära
label-scale = Skala
label-page = Sida
label-pages = Sidor


## Laddningstillstånd
loading-metadata = Laddar metadata…
loading-thumbnails = Laddar { $current } / { $total }…


## Felmeddelanden
error-failed-to-open = Misslyckades att öppna "{ $path }"
error-unsupported-format = Filformat som inte stöds
error-no-image-loaded = Ingen bild laddad


## Egenskapspanel
panel-properties = Egenskaper
panel-actions = Åtgärder

meta-section-file = Fil information
meta-section-exif = Kamera information
meta-section-image = Bild information

## Fil metadata
meta-filename = Namn
meta-format = Format
meta-dimensions = Dimensioner
meta-filesize = Storlek
meta-colortype = Färg-typ
meta-path = Sökväg
meta-pages = Sidor
meta-current-page = Nuvarande sida

## Bildmetadata
meta-width = Bredd
meta-height = Höjd
meta-depth = Bitdjup

## EXIF metadata
meta-camera = Kamera
meta-datetime = Datum taget
meta-exposure = Exponering
meta-aperture = Bländare
meta-iso = ISO { $iso }
meta-focal = Brännvidd
meta-gps = GPS plats

## Åtgärdsknappar
action-set-wallpaper = Använd som bakgrundsbild
action-open-with = Öppna med…
action-show-in-folder = Visa i mapp


## Navigeringspanel (tumnaglar)
nav-panel-title = Sidor
nav-panel-loading = Laddar { $current } / { $total }…


## Formatpanel
format-section-title = Pappersformat
format-section-subtitle = Välj pappersstorlek för export
orientation-section-title = Orientering
