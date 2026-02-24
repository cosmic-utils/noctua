# SPDX-License-Identifier: GPL-3.0-or-later
# i18n/en/noctua.ftl
#
# Localization strings for Noctua (English).
# Usage: fl!("message-id", arg1, arg2, ...)
#
# Positional arguments ($1, $2, ...) are used for variable content.


## Application
noctua-app-name = Noctua
noctua-app-description = Prohlížeč dokumentů a obrázků pro prostředí COSMIC


## Main window
window-title = { $filename ->
    [none] Noctua
   *[some] { $filename } — Noctua
}


## Menu entries
menu-file-open = Otevřít…
menu-file-quit = Ukončit
menu-view-zoom-in = Přiblížit
menu-view-zoom-out = Oddálit
menu-view-zoom-reset = Obnovit přiblížení
menu-view-zoom-fit = Přizpůsobit oknu
menu-view-flip-horizontal = Překlopit vodorovně
menu-view-flip-vertical = Překlopit svisle
menu-view-rotate-cw = Otočit po směru hodinových ručiček
menu-view-rotate-ccw = Otočit proti směru hodinových ručiček


## Tooltips (for buttons and icons)
tooltip-nav-previous = Předchozí dokument
tooltip-nav-next = Další dokument
tooltip-nav-toggle = Přepnout navigační panel
tooltip-zoom-in = Přiblížit
tooltip-zoom-out = Oddálit
tooltip-zoom-fit = Přizpůsobit oknu
tooltip-rotate-ccw = Otočit proti směru hodinových ručiček
tooltip-rotate-cw = Otočit po směru hodinových ručiček
tooltip-flip-horizontal = Překlopit vodorovně
tooltip-flip-vertical = Překlopit svisle
tooltip-info-panel = Přepnout informační panel


## Footer / Status bar
status-zoom-fit = Přizpůsobit
status-zoom-percent = { $percent }%
status-doc-dimensions = { $width } × { $height }
status-nav-position = { $current } / { $total }
status-separator =  | 


## Placeholders / Empty states
no-document = Není načten žádný dokument


## Labels
label-zoom = Přiblížení
label-tools = Nástroje
label-crop = Oříznout
label-scale = Měřítko
label-page = Stránka
label-pages = Stránky


## Loading states
loading-metadata = Načítání metadat…
loading-thumbnails = Načítání { $current } / { $total }…


## Error messages
error-failed-to-open = Nepodařilo se otevřít „{ $path }“
error-unsupported-format = Nepodporovaný formát souboru
error-no-image-loaded = Není načten žádný obrázek


## Properties panel
panel-properties = Vlastnosti
panel-actions = Akce

meta-section-file = Informace o souboru
meta-section-exif = Informace o kameře
meta-section-image = Informace o obrázku

## File metadata
meta-filename = Název
meta-format = Formát
meta-dimensions = Rozměry
meta-filesize = Velikost
meta-colortype = Typ barev
meta-path = Cesta
meta-pages = Stránky
meta-current-page = Aktuální stránka

## Image metadata
meta-width = Šířka
meta-height = Výška
meta-depth = Bitová hloubka

## EXIF metadata
meta-camera = Kamera
meta-datetime = Datum pořízení
meta-exposure = Expozice
meta-aperture = Clona
meta-iso = ISO { $iso }
meta-focal = Ohnisková vzdálenost
meta-gps = Poloha GPS

## Action buttons
action-set-wallpaper = Nastavit jako tapetu
action-open-with = Otevřít pomocí…
action-show-in-folder = Zobrazit ve složce


## Navigation panel (thumbnails)
nav-panel-title = Stránky
nav-panel-loading = Načítání { $current } / { $total }…
