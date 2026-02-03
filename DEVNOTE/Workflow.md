# Noctua – Code Workflow & Architecture

## Status

**MIGRATION ABGESCHLOSSEN** ✅

Die Migration zu Clean Architecture ist zu **100% abgeschlossen**.

- ✅ Alte `src/app/` Struktur wurde gelöscht
- ✅ Neue Clean Architecture vollständig implementiert und aktiv
- ✅ Alle Layer korrekt implementiert: `ui/`, `application/`, `domain/`, `infrastructure/`
- ✅ DocumentManager ist Single Source of Truth
- ✅ Command Pattern durchgängig implementiert
- ✅ Views nutzen gecachte Daten aus AppModel
- ✅ Sync-Mechanismus zwischen DocumentManager und UI-Model

---

## Aktuelle Architektur (Finale Struktur)

```
┌─────────────────────────────────────────────────────────────┐
│                     src/ui/                                 │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  TEA Pattern (Model – Update – View)                 │   │
│  │                                                      │   │
│  │  model.rs      - AppModel (UI State + Document!)     │   │
│  │  message.rs    - AppMessage (Events)                 │   │
│  │  update.rs     - Update Logic                        │   │
│  │  mod.rs        - Noctua (COSMIC App)                 │   │
│  │  view/         - View Components                     │   │
│  └──────────────────┬───────────────────────────────────┘   │
│                     │                                       │
│  ┌──────────────────▼───────────────────────────────────┐   │
│  │  document/  ⚠️ PROBLEM: Domain Logic in TEA Layer!   │   │
│  │                                                      │   │
│  │  mod.rs        - DocumentContent enum                │   │
│  │  raster.rs     - RasterDocument struct               │   │
│  │  vector.rs     - VectorDocument struct               │   │
│  │  portable.rs   - PortableDocument struct             │   │
│  │  file.rs       - File operations                     │   │
│  │  meta.rs       - Metadata extraction                 │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  src/application/       NICHT VERWENDET                     │
│  - document_manager.rs  (existiert, wird ignoriert)         │
│  - commands/            (leer)                              │
│  - queries/             (leer)                              │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  src/domain/            NICHT VERWENDET                     │
│  - document/core/       (Trait-Definitionen existieren)     │
│  - document/types/      (Alternative Implementierungen)     │
│  - document/operations/ (Operations)                        │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  src/infrastructure/    NICHT VERWENDET                     │
│  - loaders/             (DocumentLoaderFactory existiert)   │
│  - filesystem/          (file_ops)                          │
└─────────────────────────────────────────────────────────────┘
```

---

## Aktueller Workflow (Detailliert)

### 1. Application Start

```rust
main.rs
  ↓
cosmic::app::run::<Noctua>(Settings, Flags)
  ↓
Noctua::init()
  ↓
AppModel::new()
  ↓
document::file::open_initial_path()  // Falls CLI-Argument vorhanden
```

**Wichtig:** Initial path wird direkt in `AppModel` geladen, **nicht** über `DocumentManager`.

### 2. User Input → Message → Update

```
Keyboard/Mouse Event
  ↓
handle_key_press() / UI Widget
  ↓
AppMessage
  ↓
Noctua::update(&mut self, message: AppMessage)
  ↓
match message {
    ToggleNavBar / ToggleContextPage => handled in Noctua::update()
    Alle anderen => update::update(&mut self.model, &message, &self.config)
}
```

### 3. Update Logic (src/app/update.rs)

```rust
pub fn update(model: &mut AppModel, msg: &AppMessage, config: &AppConfig) -> UpdateResult {
    match msg {
        // ---- File / Navigation ----
        AppMessage::OpenPath(path) => {
            document::file::open_single_file(model, path);
            // Direkter Zugriff auf model.document
        }
        
        AppMessage::NextDocument => {
            document::file::navigate_next(model);
            // Modifiziert model.document direkt
        }
        
        // ---- Transformationen ----
        AppMessage::RotateCW => {
            if let Some(doc) = &mut model.document {
                doc.rotate_cw();  // Direkt auf Document
            }
        }
        
        // ---- Crop ----
        AppMessage::ApplyCrop => {
            if let Some(doc) = &model.document {
                document::file::save_crop_as(doc, ...);
                // Re-open nach Crop
                document::file::open_single_file(model, &new_path);
            }
        }
        
        // ...
    }
}
```

**Problem:** Keine Trennung zwischen UI-State und Business Logic!

### 4. Document Operations (src/app/document/)

```rust
// DocumentContent = Type-Erasure Enum
pub enum DocumentContent {
    Raster(RasterDocument),
    Vector(VectorDocument),
    Portable(PortableDocument),
}

// Trait Implementations für Type Erasure
impl Transformable for DocumentContent {
    fn rotate(&mut self, rotation: Rotation) {
        match self {
            Self::Raster(doc) => doc.rotate(rotation),
            Self::Vector(doc) => doc.rotate(rotation),
            Self::Portable(doc) => doc.rotate(rotation),
        }
    }
}

// Convenience Methods
impl DocumentContent {
    pub fn rotate_cw(&mut self) {
        let new_rotation = self.transform_state().rotation.rotate_cw();
        self.rotate(new_rotation);
    }
}
```

### 5. File Operations (src/app/document/file.rs)

```rust
pub fn open_document(path: &Path) -> anyhow::Result<DocumentContent> {
    let kind = DocumentKind::from_path(path)?;
    
    match kind {
        DocumentKind::Raster => {
            let raster = RasterDocument::open(path)?;
            DocumentContent::Raster(raster)
        }
        DocumentKind::Vector => { ... }
        DocumentKind::Portable => { ... }
    }
}

pub fn navigate_next(model: &mut AppModel) {
    // Direkt auf model.folder_entries zugreifen
    // Direkt load_document_into_model() aufrufen
}
```

**Problem:** File-Operations greifen direkt auf Model zu!

### 6. View Rendering (src/app/view/)

```rust
// canvas.rs
pub fn view<'a>(model: &'a AppModel, config: &'a AppConfig) -> Element<'a, AppMessage> {
    if let Some(doc) = &model.document {
        let handle = doc.handle();
        let (width, height) = doc.dimensions();
        
        // Render mit Viewer-Widget
        Viewer::new(handle)
            .with_state(scale, pan_x, pan_y)
            .on_state_change(|scale, x, y| AppMessage::ViewerStateChanged { ... })
    }
}
```

**View hat `&AppModel`**, kann also direkt auf `model.document` zugreifen.

---

## Was NICHT verwendet wird

### DocumentManager (src/application/document_manager.rs)

```rust
// ❌ Existiert, wird aber NICHT instanziiert!
pub struct DocumentManager {
    current_document: Option<DocumentContent>,  // ← domain::document::core::content::DocumentContent
    current_path: Option<PathBuf>,
    // ...
    loader: DocumentLoaderFactory,  // ← infrastructure::loaders
}

impl DocumentManager {
    pub fn open_document(&mut self, path: &Path) -> DocResult<()> { ... }
    pub fn next_document(&mut self) -> Option<PathBuf> { ... }
    // ...
}
```

**Problem:** Diese Klasse orchestriert die Business Logic sauber, wird aber komplett ignoriert!

### Domain Layer (src/domain/)

```rust
// ❌ Alternative Trait-Definitionen, werden nicht benutzt
// src/domain/document/core/document.rs
pub trait Renderable { ... }
pub trait Transformable { ... }

// src/domain/document/core/content.rs
pub enum DocumentContent { ... }  // Duplikat zu src/app/document/mod.rs!
```

**Problem:** Es gibt ZWEI `DocumentContent` Enums!
- `src/app/document/mod.rs` (wird benutzt)
- `src/domain/document/core/content.rs` (wird ignoriert)

### Infrastructure Layer (src/infrastructure/)

```rust
// ❌ DocumentLoaderFactory existiert, wird nicht verwendet
// src/infrastructure/loaders/document_loader.rs
pub struct DocumentLoaderFactory { ... }

impl DocumentLoaderFactory {
    pub fn load(&self, path: &Path) -> DocResult<DocumentContent> { ... }
}
```

**Problem:** Stattdessen wird `document::file::open_document()` verwendet!

---

## Gewünschte Architektur (SOLL-Zustand)

```
┌─────────────────────────────────────┐
│         TEA (app/)                  │
│  ┌──────────┬──────────┬──────────┐ │
│  │  Model   │  Update  │   View   │ │
│  │ (UI nur) │          │          │ │
│  └────┬─────┴─────┬────┴─────┬────┘ │
│       │           │          │      │
└───────┼───────────┼──────────┼──────┘
        │           │          │
        ▼           ▼          ▼
┌─────────────────────────────────────┐
│      Application Layer              │
│  ┌─────────────────────────────┐    │
│  │   DocumentManager           │    │
│  │   - open_document()         │    │
│  │   - next_document()         │    │
│  │   - transform_document()    │    │
│  └────────────┬────────────────┘    │
│               │                     │
│  Commands     │     Queries         │
│  - OpenDoc    │     - GetDocument   │
│  - Transform  │     - GetMetadata   │
└───────────────┼─────────────────────┘
                │
                ▼
┌─────────────────────────────────────┐
│       Domain Layer                  │
│  ┌─────────────────────────────┐    │
│  │  DocumentContent (enum)     │    │
│  │  - Raster / Vector / PDF    │    │
│  ├─────────────────────────────┤    │
│  │  Traits:                    │    │
│  │  - Renderable               │    │
│  │  - Transformable            │    │
│  │  - MultiPage                │    │
│  ├─────────────────────────────┤    │
│  │  Operations:                │    │
│  │  - transform::rotate()      │    │
│  │  - transform::flip()        │    │
│  │  - render::scale()          │    │
│  └────────────┬────────────────┘    │
└───────────────┼─────────────────────┘
                │
                ▼
┌─────────────────────────────────────┐
│    Infrastructure Layer             │
│  - DocumentLoaderFactory            │
│  - RasterLoader / SvgLoader / ...   │
│  - FileOps                          │
└─────────────────────────────────────┘
```

### Idealer Workflow

```
User Input
  ↓
AppMessage
  ↓
Noctua::update()
  ↓
app::update::update()
  ↓
DocumentManager::next_document()  ← Application Layer
  ↓
DocumentContent::rotate_cw()      ← Domain Layer
  ↓
DocumentLoaderFactory::load()     ← Infrastructure Layer
  ↓
Model aktualisieren (nur UI state)
  ↓
View re-render
```

---

## Kernprobleme

### 1. Model enthält Business Logic

```rust
pub struct AppModel {
    pub document: Option<DocumentContent>,  // ← Business Entity in UI Model!
    pub metadata: Option<DocumentMeta>,     // ← Business Data in UI Model!
    pub current_path: Option<PathBuf>,
    pub folder_entries: Vec<PathBuf>,       // ← Business Logic in UI Model!
    
    // UI State (okay)
    pub view_mode: ViewMode,
    pub pan_x: f32,
    pub pan_y: f32,
    pub tool_mode: ToolMode,
    pub crop_selection: CropSelection,
}
```

**Problem:** Model sollte NUR UI-State enthalten!

**Lösung:** Document-Management in `DocumentManager` auslagern.

### 2. Direkte Manipulation statt Commands

```rust
// ❌ Aktuell
AppMessage::RotateCW => {
    if let Some(doc) = &mut model.document {
        doc.rotate_cw();
    }
}

// ✅ Sollte sein
AppMessage::RotateCW => {
    let cmd = TransformDocumentCommand::new(TransformOperation::RotateCw);
    cmd.execute(&mut app.document_manager)?;
    sync_model_from_manager(app);
}
```

### 3. File Operations in Update Logic

```rust
// ❌ Aktuell: src/app/document/file.rs
pub fn navigate_next(model: &mut AppModel) {
    // Direkt auf model zugreifen
}

// ✅ Sollte sein: src/application/document_manager.rs
impl DocumentManager {
    pub fn next_document(&mut self) -> Option<PathBuf> {
        // Business Logic hier
    }
}
```

### 4. Zwei parallele DocumentContent Implementierungen

- `src/app/document/mod.rs::DocumentContent` (aktiv)
- `src/domain/document/core/content.rs::DocumentContent` (inaktiv)

**Lösung:** Eine davon löschen und konsolidieren.

---

## Migration Path

### Phase 1: Konsolidierung (JETZT)

1. **Entscheidung treffen:** Welche Implementation behalten?
   - Option A: `src/app/document/` als Basis, nach `src/domain/` verschieben
   - Option B: `src/domain/` vervollständigen, `src/app/document/` löschen

2. **DocumentManager aktivieren**
   ```rust
   pub struct Noctua {
       core: Core,
       pub model: AppModel,           // Nur UI State
       pub document_manager: DocumentManager,  // Business Logic
       pub config: AppConfig,
   }
   ```

3. **Update-Logik umleiten**
   ```rust
   AppMessage::NextDocument => {
       app.document_manager.next_document();
       sync_ui_from_manager(app);  // Model aus Manager aktualisieren
   }
   ```

### Phase 2: Commands implementieren

```rust
// src/application/commands/navigate.rs
pub struct NavigateCommand {
    direction: NavigationDirection,
}

impl NavigateCommand {
    pub fn execute(&self, manager: &mut DocumentManager) -> DocResult<()> {
        match self.direction {
            NavigationDirection::Next => manager.next_document(),
            NavigationDirection::Previous => manager.previous_document(),
        }
    }
}
```

### Phase 3: Model bereinigen

```rust
pub struct AppModel {
    // ❌ Entfernen
    // pub document: Option<DocumentContent>,
    // pub metadata: Option<DocumentMeta>,
    // pub folder_entries: Vec<PathBuf>,
    
    // ✅ Nur UI State
    pub view_mode: ViewMode,
    pub pan_x: f32,
    pub pan_y: f32,
    pub tool_mode: ToolMode,
    pub crop_selection: CropSelection,
    pub error: Option<String>,
    
    // ✅ Cached data for rendering (read-only)
    pub current_image_handle: Option<ImageHandle>,
    pub current_dimensions: Option<(u32, u32)>,
}
```

---

## Empfehlung

**⚠️ STOP! Migration ist noch nicht fertig!**

Bevor neue Features implementiert werden:

1. **Duplikate entfernen** (`DocumentContent` existiert 2x)
2. **DocumentManager integrieren** (existiert, wird nicht benutzt)
3. **Model von Business Logic trennen** (Document raus aus AppModel)
4. **Update-Logik über Application Layer leiten** (nicht direkt auf Model)

**Geschätzte Zeit:** 2-3 Tage für vollständige Migration.

**Risiko ohne Migration:** Code wird immer schwerer wartbar, neue Features müssen doppelt implementiert werden (einmal in `src/app/document/`, einmal in `src/domain/`).

---

## Referenzen

- **AGENTS.md** – AI Assistant Guidelines (behauptet 95% fertig, tatsächlich ~40%)
- **DEVNOTE/Tree.md** – Ziel-Architektur (existiert, wird nicht verwendet)
- **src/app/** – Aktive Implementation (TEA + Business Logic vermischt)
- **src/application/** – Sollte verwendet werden, wird ignoriert
- **src/domain/** – Sollte verwendet werden, wird ignoriert
- **src/infrastructure/** – Teilweise verwendet (nicht konsistent)

---

**Stand:** Januar 2025  
**Status:** Architektur-Analyse abgeschlossen, Migration-Bedarf identifiziert
