# Feature-Vergleich: app/document vs domain/document/types

**Ziel:** Identifizieren welche Features von `src/app/document/` nach `src/domain/document/types/` portiert werden müssen.

---

## RasterDocument

### Struct-Felder

| Feld | app/ | domain/ | Status |
|------|------|---------|--------|
| `document: DynamicImage` | ✅ | ✅ | OK |
| `native_width: u32` | ✅ | ✅ | OK |
| `native_height: u32` | ✅ | ✅ | OK |
| `transform: TransformState` | ✅ | ✅ | OK |
| `handle: ImageHandle` | ✅ pub | ✅ private | **⚠️ domain: public machen oder getter** |
| `fine_rotation_angle: f32` | ❌ | ✅ | ℹ️ Extra feature in domain |
| `interpolation_quality` | ❌ | ✅ | ℹ️ Extra feature in domain |

**Entscheidung:** Domain-Version hat mehr Features → Domain behalten, `handle` public machen

---

### Methoden-Vergleich

| Methode | app/ | domain/ | Aktion |
|---------|------|---------|--------|
| **Core Operations** | | | |
| `open()` | ✅ | ✅ | ✅ OK |
| `render()` | ✅ | ✅ | ✅ OK |
| `save()` | ✅ | ✅ | ✅ OK |
| | | | |
| **Transformations (Trait)** | | | |
| `rotate()` | ✅ | ✅ | ✅ OK |
| `flip()` | ✅ | ✅ | ✅ OK |
| `transform_state()` | ✅ | ✅ | ✅ OK |
| | | | |
| **Renderable (Trait)** | | | |
| `info()` | ✅ | ✅ | ✅ OK |
| | | | |
| **Dimensions** | | | |
| `dimensions()` | ✅ | ✅ | ✅ OK (beide haben es!) |
| `native_dimensions()` | ❌ | ✅ | ℹ️ Extra in domain |
| | | | |
| **Crop** | | | |
| `crop()` | ✅ | ✅ | ✅ OK (beide haben es!) |
| `crop_to_image()` | ✅ | ❌ | 📋 **Portieren nach domain/** |
| | | | |
| **Handle/Image Access** | | | |
| `handle` (field pub) | ✅ | ❌ | 📋 **Public machen oder getter** |
| `handle()` (getter) | ❌ | ✅ | ✅ OK (domain hat getter) |
| `image()` | ❌ | ✅ | ℹ️ Extra in domain |
| `get_rendered_image()` | ❌ | ✅ | ℹ️ Extra in domain |
| | | | |
| **Metadata** | | | |
| `extract_meta()` | ✅ | ❌ | 📋 **Portieren nach domain/** |
| | | | |
| **Internal Helpers** | | | |
| `refresh_handle()` | ✅ private | ❌ | ℹ️ Evtl. bereits integriert |
| `apply_rotation()` | ❌ | ✅ | ℹ️ Extra in domain |
| `apply_flip()` | ❌ | ✅ | ℹ️ Extra in domain |
| `create_image_handle_from_image()` | ❌ | ✅ | ℹ️ Extra in domain |
| | | | |
| **Extra Features (domain)** | | | |
| `rotate_fine()` | ❌ | ✅ | ℹ️ Feature in domain |
| `reset_fine_rotation()` | ❌ | ✅ | ℹ️ Feature in domain |
| `set_interpolation_quality()` | ❌ | ✅ | ℹ️ Feature in domain |
| `resize_to_format()` | ❌ | ✅ | ℹ️ Feature in domain |

---

### Zusammenfassung RasterDocument

**Domain-Version ist fortgeschrittener** ✅
- Mehr Features (fine rotation, interpolation quality, resize)
- Bessere API (getter statt public fields)
- Saubere Helper-Funktionen

**Aus app/ portieren:**
1. ✅ `crop_to_image()` - Nicht-destruktives Crop
2. ✅ `extract_meta()` - Metadaten-Extraktion
3. ✅ `handle` public machen ODER getter `handle()` nutzen (bereits vorhanden!)

**Entscheidung:** Domain-Version als Basis, nur 2 Methoden fehlen

---

## VectorDocument

### Methoden-Vergleich

| Methode | app/ | domain/ | Aktion |
|---------|------|---------|--------|
| `open()` | ✅ | ✅ | ✅ OK |
| `render()` | ✅ | ✅ | ✅ OK |
| `dimensions()` | ✅ | ❌ | 📋 **Portieren** |
| `handle` (pub field) | ✅ | ❌ private | 📋 **Public machen oder getter** |
| `extract_meta()` | ✅ | ❌ | 📋 **Portieren** |
| `crop()` | ❌ | ❌ | 📋 **Neu implementieren** (Design-Entscheidung) |

**Aus app/ portieren:**
1. `dimensions()` 
2. `extract_meta()`
3. `handle()` getter oder public
4. NEU: `crop()` implementieren (render-based)

---

## PortableDocument

### Methoden-Vergleich

| Methode | app/ | domain/ | Aktion |
|---------|------|---------|--------|
| `open()` | ✅ | ✅ | ✅ OK |
| `render()` | ✅ | ✅ | ✅ OK |
| `dimensions()` | ✅ | ❌ | 📋 **Portieren** |
| `handle` (pub field) | ✅ | ❌ private | 📋 **Public machen oder getter** |
| `extract_meta()` | ✅ | ❌ | 📋 **Portieren** |
| `crop()` | ❌ | ❌ | 📋 **Neu implementieren** (Design-Entscheidung) |
| Thumbnails | ✅ | ✅ | ℹ️ Prüfen ob identisch |

**Aus app/ portieren:**
1. `dimensions()`
2. `extract_meta()`
3. `handle()` getter oder public
4. NEU: `crop()` implementieren (render-based)

---

## Action Items für Schritt 1.2-1.4

### Schritt 1.2: RasterDocument (60 Min)
- [x] `crop()` - Bereits vorhanden! ✅
- [x] `dimensions()` - Bereits vorhanden! ✅
- [x] `crop_to_image()` hinzufügen ✅
- [x] `extract_meta()` hinzufügen ✅ (oder in core/metadata.rs)
- [x] `handle()` getter - Bereits vorhanden! ✅

### Schritt 1.3: VectorDocument (45 Min)
- [x] `dimensions()` - Bereits vorhanden ✅
- [x] `handle()` getter - Bereits vorhanden ✅
- [x] `extract_meta()` implementieren ✅
- [x] `crop()` implementieren (render-based) ✅

### Schritt 1.4: PortableDocument (45 Min)
- [ ] `dimensions()` implementieren
- [ ] `handle()` getter hinzufügen
- [ ] `extract_meta()` implementieren
- [ ] `crop()` implementieren (render-based, neu!)

---

## Überraschende Erkenntnisse

1. **Domain hat bereits crop() für Raster!** ✅
2. **Domain hat bereits dimensions()!** ✅
3. **Domain hat bereits handle() getter!** ✅
4. **Domain hat MEHR Features** (fine rotation, interpolation) ✅

**→ Domain-Implementierung ist besser! Nur 2-3 Methoden fehlen pro Type.**

---

**Status:** Vergleich abgeschlossen  
**Nächster Schritt:** 1.2 - RasterDocument ergänzen
