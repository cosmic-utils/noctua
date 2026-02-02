# Metadata Konsolidierung - Vergleich

## Status: ✅ Strukturen sind identisch

### Strukturen

| Struktur | app/meta.rs | domain/core/metadata.rs | Status |
|----------|-------------|-------------------------|--------|
| `BasicMeta` | ✅ 7 fields | ✅ 7 fields | ✅ Identisch |
| `ExifMeta` | ✅ 9 fields | ✅ 9 fields | ✅ Identisch |
| `DocumentMeta` | ✅ | ✅ | ✅ Identisch |

### Methoden

| Methode | app/ | domain/ | Status |
|---------|------|---------|--------|
| `BasicMeta::file_size_display()` | ✅ | ✅ | ✅ Identisch |
| `BasicMeta::resolution_display()` | ✅ | ✅ | ✅ Identisch |
| `ExifMeta::camera_display()` | ✅ | ✅ | ✅ Identisch |
| `ExifMeta::gps_display()` | ✅ | ✅ | ✅ Identisch |
| `ExifMeta::from_bytes()` | ❌ private fn | ✅ pub fn | ✅ Domain besser |

### Helper-Funktionen

**App hat:**
- `build_raster_meta()` - Wird nicht außerhalb app/ verwendet
- `build_vector_meta()` - Wird nicht außerhalb app/ verwendet
- `build_portable_meta()` - Wird nicht außerhalb app/ verwendet
- `extract_exif_from_bytes()` - Private Funktion

**Domain hat:**
- `ExifMeta::from_bytes()` - Public Methode (sauberer)
- Document-Typen haben eigene `extract_meta()` Methoden

### Unterschiede

**Organisation:**
- **App:** Helper-Funktionen `build_*_meta()` außerhalb der Structs
- **Domain:** `extract_meta()` Methoden direkt in Document-Typen (RasterDocument, VectorDocument, PortableDocument)

**Vorteile Domain:**
- ✅ Sauberer: `doc.extract_meta(path)` statt `build_raster_meta(path, doc, ...)`
- ✅ Type-safe: Compiler weiß welcher Typ
- ✅ Erweiterbar: Jeder Document-Typ kontrolliert eigene Metadaten

## Entscheidung

✅ **Keine Änderungen nötig!**

- Domain-Version ist vollständig und sogar besser organisiert
- Strukturen sind identisch
- `ExifMeta::from_bytes()` ist bereits in Domain als public Methode
- `extract_meta()` Methoden in Document-Typen sind bereits implementiert

## Nächster Schritt

Schritt 1.7: Traits & Enums konsolidieren
