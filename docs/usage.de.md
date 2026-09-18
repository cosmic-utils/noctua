# Noctua – Bedienungsanleitung

Noctua ist ein Bild- und Dokumenten-Viewer für das COSMIC-Desktop-Environment. Der Fokus liegt auf der Durchsicht und Annotation von Dokumenten.

> **Hinweis:** Diese Anleitung beschreibt den aktuellen Implementierungsstand. Noch nicht umgesetzte Funktionen sind unter [Geplante Funktionen](#geplante-funktionen) zusammengefasst und dort als solche gekennzeichnet.

## Überblick

Noctua kennt zwei Tab-Arten in einem gemeinsamen Tab-Streifen:

- **Browser-Tab** (read-only): zeigt einen Ordner wie ein Bildbetrachter – Raster, Vektor und PDFs Seite für Seite. Hier wird nie verändert.
- **Annotation-Tab** (geplant): bearbeitet genau ein PDF.

Der Anwendungszustand (offene Tabs, Auswahl) wird als **Session** gespeichert und beim nächsten Start wiederhergestellt.

## Erste Schritte

### Start über die Kommandozeile

```bash
noctua [PFAD]
```

- Mit einem **Bild oder Ordner** als Argument öffnet Noctua nur diesen Inhalt (keine Session-Wiederherstellung).
- Ohne Argument wird die **letzte Session** automatisch geöffnet.

### Ordner öffnen

`Ctrl+O` öffnet einen Ordner-Dialog und legt einen neuen Browser-Tab an. Noctua scannt den Ordner und listet alle unterstützten Dokumente.

## Browser-Tab

- Die linke **Navigationsleiste** zeigt Thumbnails der Ordner-Einträge (freedesktop-Thumbnail-Cache).
- Ein **Klick** wählt einen Eintrag und zeigt ihn rechts an:
  - Einseitige Dokumente (PNG, JPEG, SVG, …) werden direkt angezeigt.
  - **Mehrseitige PDFs** zeigen alle Seiten fortlaufend (vertikal scrollbar).
- Ein **Doppelklick** auf ein PDF klappt dessen Seiten direkt in der Navigationsleiste auf (eingerückte, kleinere Seiten-Thumbnails mit Seitenzahl). Ein weiterer Doppelklick klappt sie wieder zu. Es ist höchstens ein PDF gleichzeitig aufgeklappt.

## Navigation

| Taste | Aktion | Beschreibung |
|:------|:-------|:-------------|
| `←` | Vorheriger Eintrag | Zum vorherigen Eintrag im Ordner |
| `→` | Nächster Eintrag | Zum nächsten Eintrag im Ordner |
| `↑` | Vorherige Seite | Eine Seite zurück (in der PDF-Vorschau) |
| `↓` | Nächste Seite | Eine Seite weiter (in der PDF-Vorschau) |

## Zoom und Ansicht

Ein Zoom-Schritt entspricht Faktor **1,25** (≈ +25 % beim Vergrößern, −20 % beim Verkleinern).

| Taste | Aktion |
|:------|:-------|
| `Ctrl+=` (oder `Ctrl+Shift++`) | Vergrößern |
| `Ctrl+-` | Verkleinern |
| `Ctrl+1` | Originalgröße (100 % = 1:1) |
| `Ctrl+0` | Einpassen (an das Fenster anpassen) |

Zusätzlich:

- **`Ctrl` + Mausrad** zoomt, zentriert auf den Mauszeiger.
- **Mausrad allein** scrollt bzw. verschiebt das Bild.
- **Ziehen mit der Maus** verschiebt ein vergrößertes Bild (Pan).

In der Statusleiste befinden sich außerdem Zoom-Schaltflächen: Verkleinern (−), ein Prozent-Menü (Einpassen, 50 %, 100 %, 200 %, 400 %) und Vergrößern (+).

## Menüleiste

### Datei

| Eintrag | Tastenkürzel |
|:--------|:-------------|
| Ordner öffnen | `Ctrl+O` |
| Tab schließen | `Ctrl+W` |
| Beenden | `Ctrl+Q` |

### Ansicht

| Eintrag | Tastenkürzel |
|:--------|:-------------|
| Vergrößern | `Ctrl+=` |
| Verkleinern | `Ctrl+-` |
| Einpassen | `Ctrl+0` |
| 100 % | `Ctrl+1` |
| Vollbild | `F11` |
| Navigationsleiste anzeigen | `Ctrl+B` |
| Über Noctua | – |

## Statusleiste

Die Statusleiste zeigt:

- `[<] [>]` – Navigation (Eintrag/Seite) im aktiven Tab
- `Seite N/M` – aktuelle Seite (bei einseitigen Dateien 1/1)
- Zoom (z. B. `150 %` oder `Einpassen`)
- Dateiname
- Dateigröße

Im (geplanten) Annotation-Tab zusätzlich: `Geändert` bei ungespeicherten Änderungen.

## Sessions

- Gespeichert unter `XDG_DATA_HOME/noctua/sessions/<name>.ron`.
- Die letzte Session wird beim Beenden automatisch gespeichert und beim nächsten Start wiederhergestellt.
- Ungespeicherte Annotation-Änderungen gehen beim Schließen verloren (explizites Speichern ist Anwender-Sache).

## Unterstützte Formate

- **Raster:** PNG, JPEG, GIF, BMP, TIFF, WebP
- **Vektor:** SVG
- **PDF:** mehrseitig, mit Seiten-Navigation und Thumbnails

## Geplante Funktionen

Folgendes ist noch nicht implementiert (oder nur vorbereitet):

- **Annotation-Tab:** genau ein PDF bearbeiten – Seiten einfügen/entfernen/verschieben/drehen, annotieren (nativ in `/Annots`); explizites Speichern (`Ctrl+S`) mit Dirty-Indikator, „Speichern unter".
- **Drehen** (Ansicht → Drehen).
- **Spiegeln** (horizontal/vertikal).
- **An Breite / An Seite anpassen.**
- **Tastatur-Pan** (`Ctrl` + Pfeiltasten).
- **Eigenschaften-Panel** mit Metadaten (EXIF, DPI, GPS, …).
- **Als Hintergrund setzen.**
- **Zuschneiden / Skalieren.**
- **Session-Menü** („Session speichern" / „Session speichern unter").

Hinweis: JavaScript in PDFs wird erkannt (`has_javascript`), aber nie ausgeführt.
