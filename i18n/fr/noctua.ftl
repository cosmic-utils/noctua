# SPDX-License-Identifier: GPL-3.0-or-later
# i18n/en/noctua.ftl
# #
# Chaînes de localisation pour Noctua (français).
# Usage: fl!("message-id", arg1, arg2, ...)
# #
# Les arguments positionnels ($1, $2, ...) sont utilisés pour le contenu variable.


## Application
noctua-app-name = Noctua
noctua-app-description = Visionneuse de documents et d'images pour le bureau COSMIC


## Fenêtre principale
window-title = { $filename ->
    [none] Noctua
   *[some] { $filename } — Noctua
} }


## Entrées du menu
menu-main = Menu
menu-file-open = Ouvrir…
menu-file-quit = Quitter
menu-view-zoom-in = Zoom avant
menu-view-zoom-out = Zoom arrière
menu-view-zoom-reset = Réinitialiser le zoom
menu-view-zoom-fit = Ajuster à la fenêtre
menu-view-flip-horizontal = Inverser horizontalement
menu-view-flip-vertical = Inverser verticalement
menu-view-rotate-cw = Rotation horaire
menu-view-rotate-ccw = Rotation antihoraire


## Infobulles (pour les boutons et les icônes)
tooltip-nav-previous = Document précédent
tooltip-nav-next = Document suivant
tooltip-nav-toggle = Afficher/masquer le panneau de navigation
tooltip-zoom-in = Zoom avant
tooltip-zoom-out = Zoom arrière
tooltip-zoom-fit = Ajuster à la fenêtre
tooltip-rotate-ccw = Rotation antihoraire
tooltip-rotate-cw = Rotation horaire
tooltip-flip-horizontal = Inverser horizontalement
tooltip-flip-vertical = Inverser verticalement
tooltip-info-panel = Afficher/masquer le panneau d'informations


## Pied de page / Barre d'état
status-zoom-fit = Fit
status-zoom-percent = { $percent }%
status-doc-dimensions = { $width } × { $height }
status-nav-position = { $current } / { $total }
status-separator =  |


## Espaces réservés / États vides
no-document = Aucun document chargé


## Étiquettes
label-zoom = Zoom
label-tools = Outils
label-crop = Recadrer
label-scale = Échelle
label-page = Page
label-pages = Pages


## État du chargement
loading-metadata = Chargement des métadonnées…
loading-thumbnails = Chargement de { $current } / { $total }…


## Messages d'erreur
error-failed-to-open = Impossible d'ouvrir « { $path } »
error-unsupported-format = Format de fichier non pris en charge
error-no-image-loaded = Aucune image chargée


## Panneau Propriétés
panel-properties = Propriétés
panel-actions = Actions

meta-section-file = Informations sur le fichier
meta-section-exif = Informations sur l'appareil photo
meta-section-image = Informations sur l'image

## Métadonnées du fichier
meta-filename = Nom
meta-format = Format
meta-dimensions = Dimensions
meta-filesize = Taille
meta-colortype = Type de couleur
meta-path = Chemin d'accès
meta-pages = Pages
meta-current-page = Page actuelle

## Métadonnées de l'image
meta-width = Largeur
meta-height = Hauteur
meta-depth = Profondeur de bits

## Métadonnées EXIF
meta-camera = Appareil photo
meta-datetime = Date de prise de vue
meta-exposure = Exposition
meta-aperture = Ouverture
meta-iso = ISO { $iso }
meta-focal = Distance focale
meta-gps = Coordonnées GPS

## Boutons d'action
action-set-wallpaper = Définir comme fond d'écran
action-open-with = Ouvrir avec…
action-show-in-folder = Afficher dans le dossier


## Panneau de navigation (vignettes)
nav-panel-title = Pages
nav-panel-loading = Chargement { $current } / { $total }…


## Panneau Format
format-section-title = Format de papier
format-section-subtitle = Sélectionner le format de papier pour l'exportation
orientation-section-title = Orientation
