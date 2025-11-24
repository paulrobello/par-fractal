# Par Fractal Web/WASM Build Plan

## Overview

Add web/WASM support to Par Fractal with full feature parity between desktop and web versions. Use trait abstractions for platform-specific code and IndexedDB for complete persistence.

**Target:** WebGPU only (Chrome 113+, Edge 113+, Firefox 141+, Safari 26+)
**Build Tool:** Trunk
**Deployment:** GitHub Pages via GitHub Actions
**URL:** https://par-fractal.pardev.net

---

## Architecture: Platform Abstraction Layer

### New Module Structure

```
src/
├── platform/
│   ├── mod.rs              # Trait definitions + PlatformContext
│   ├── native/
│   │   ├── mod.rs          # Native implementations
│   │   ├── storage.rs      # File-based storage (directories crate)
│   │   ├── file_dialog.rs  # rfd file dialogs
│   │   └── capture.rs      # Native screenshot (image crate)
│   └── web/
│       ├── mod.rs          # Web implementations
│       ├── storage.rs      # IndexedDB storage
│       ├── file_dialog.rs  # HTML5 file input/download
│       └── capture.rs      # Canvas blob download
```

### Core Traits

```rust
// src/platform/mod.rs

pub trait Storage: Send + Sync {
    fn save(&self, category: &str, key: &str, data: &[u8]) -> Result<(), PlatformError>;
    fn load(&self, category: &str, key: &str) -> Result<Option<Vec<u8>>, PlatformError>;
    fn delete(&self, category: &str, key: &str) -> Result<(), PlatformError>;
    fn list_keys(&self, category: &str) -> Result<Vec<String>, PlatformError>;
    fn clear_category(&self, category: &str) -> Result<(), PlatformError>;
}

pub trait FileDialog: Send + Sync {
    fn save_file(&self, name: &str, data: &[u8], mime: &str) -> Result<(), PlatformError>;
    fn open_file(&self, filters: &[&str]) -> Result<Option<(String, Vec<u8>)>, PlatformError>;
}

pub trait Capture: Send + Sync {
    fn save_screenshot(&self, width: u32, height: u32, data: &[u8], prefix: &str) -> Result<String, PlatformError>;
    fn supports_auto_open(&self) -> bool;
    fn open_file(&self, path: &str) -> Result<(), PlatformError>;
}

pub struct PlatformContext {
    pub storage: Box<dyn Storage>,
    pub file_dialog: Box<dyn FileDialog>,
    pub capture: Box<dyn Capture>,
}
```

---

## Implementation Phases

### Phase 1: Project Setup & Dependencies

- [ ] Update `Cargo.toml` with features (native/web), conditional dependencies
- [ ] Add lib target with `crate-type = ["cdylib", "rlib"]`
- [ ] Create `index.html` - HTML wrapper with canvas
- [ ] Create `Trunk.toml` - Trunk build configuration
- [ ] Create `src/lib.rs` - Library entry point

### Phase 2: Platform Abstraction Layer

- [ ] Create `src/platform/mod.rs` - Trait definitions
- [ ] Create `src/platform/native/mod.rs` - Native platform context
- [ ] Create `src/platform/native/storage.rs` - File-based storage using `directories` crate
- [ ] Create `src/platform/native/file_dialog.rs` - `rfd` wrapper
- [ ] Create `src/platform/native/capture.rs` - Screenshot via `image` crate
- [ ] Refactor `src/app/mod.rs` - Add `PlatformContext` field to `App`
- [ ] Refactor `src/app/persistence.rs` - Use `Storage` trait
- [ ] Refactor `src/app/capture.rs` - Use `Capture` trait
- [ ] Refactor `src/fractal/mod.rs` - Use `Storage` for save/load
- [ ] Refactor `src/fractal/presets.rs` - Use `Storage` for presets/bookmarks
- [ ] Refactor `src/fractal/palettes.rs` - Use `Storage` for custom palettes

### Phase 3: Web Entry Point & Event Loop

- [ ] Create `src/web_main.rs` - WASM entry point with `#[wasm_bindgen(start)]`
- [ ] Modify `src/main.rs` - Add `#![cfg(not(target_arch = "wasm32"))]`
- [ ] Update `App::new()` to accept `PlatformContext`

### Phase 4: Web Platform Implementation

- [ ] Create `src/platform/web/mod.rs` - Web platform context
- [ ] Create `src/platform/web/storage.rs` - IndexedDB implementation
- [ ] Create `src/platform/web/file_dialog.rs` - HTML5 file API
- [ ] Create `src/platform/web/capture.rs` - Blob download

### Phase 5: Disable Video Recording for Web

- [ ] Add `#![cfg(not(target_arch = "wasm32"))]` to `src/video_recorder.rs`
- [ ] Add web stub `VideoRecorder` in `src/app/mod.rs`
- [ ] Wrap video UI elements with `#[cfg(not(target_arch = "wasm32"))]` in `src/ui/mod.rs`

### Phase 6: Build & Deploy Configuration

- [ ] Create `.github/workflows/deploy-web.yml` - GitHub Actions workflow
- [ ] Update `Makefile` - Add web build targets
- [ ] Configure GitHub Pages with custom domain

### Phase 7: Testing

- [ ] Test web build locally with `trunk serve`
- [ ] Test in Chrome, Firefox, Safari, Edge
- [ ] Verify settings persistence in IndexedDB
- [ ] Verify screenshot download

---

## Cargo.toml Changes

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[features]
default = ["native"]
native = ["directories", "rfd", "open", "crossbeam-channel", "chrono", "env_logger", "pollster"]
web = ["wasm-bindgen", "wasm-bindgen-futures", "web-sys", "js-sys", "console_log", "console_error_panic_hook", "indexed_db_futures", "gloo-timers"]

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
pollster = "0.4"
directories = "6.0"
rfd = "0.15"
open = "5"
crossbeam-channel = "0.5"
chrono = "0.4"
env_logger = "0.11"

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = [
    "Window", "Document", "Element", "HtmlCanvasElement",
    "HtmlAnchorElement", "HtmlInputElement", "Storage",
    "Blob", "BlobPropertyBag", "Url", "File", "FileList", "FileReader",
    "IdbFactory", "IdbDatabase", "IdbObjectStore", "IdbRequest", "IdbTransaction",
] }
js-sys = "0.3"
console_log = "1.0"
console_error_panic_hook = "0.1"
indexed_db_futures = "0.4"
gloo-timers = "0.3"

[[bin]]
name = "par-fractal"
path = "src/main.rs"
required-features = ["native"]
```

---

## Files to Create (14 files)

| File | Purpose |
|------|---------|
| `src/lib.rs` | Library entry point with module exports |
| `src/web_main.rs` | WASM entry point |
| `src/platform/mod.rs` | Trait definitions |
| `src/platform/native/mod.rs` | Native platform context |
| `src/platform/native/storage.rs` | File-based storage |
| `src/platform/native/file_dialog.rs` | rfd wrapper |
| `src/platform/native/capture.rs` | Native screenshot |
| `src/platform/web/mod.rs` | Web platform context |
| `src/platform/web/storage.rs` | IndexedDB storage |
| `src/platform/web/file_dialog.rs` | HTML5 file API |
| `src/platform/web/capture.rs` | Blob download |
| `index.html` | HTML wrapper |
| `Trunk.toml` | Trunk config |
| `.github/workflows/deploy-web.yml` | CI/CD |

## Files to Modify (12 files)

| File | Changes |
|------|---------|
| `Cargo.toml` | Add features, web deps, lib target |
| `src/main.rs` | Add native-only guard, use PlatformContext |
| `src/app/mod.rs` | Add PlatformContext field, refactor I/O |
| `src/app/persistence.rs` | Use Storage trait |
| `src/app/capture.rs` | Use Capture trait |
| `src/fractal/mod.rs` | Use Storage for settings |
| `src/fractal/presets.rs` | Use Storage for presets/bookmarks |
| `src/fractal/palettes.rs` | Use Storage for palettes |
| `src/video_recorder.rs` | Add native-only guard |
| `src/ui/mod.rs` | Hide video UI on web |
| `src/renderer/initialization.rs` | Handle web canvas surface |
| `Makefile` | Add web build targets |

---

## Feature Comparison

| Feature | Desktop | Web |
|---------|---------|-----|
| All fractal types | Yes | Yes |
| Post-processing | Yes | Yes |
| Camera controls | Yes | Yes |
| LOD system | Yes | Yes |
| Settings persistence | YAML files | IndexedDB |
| User presets | YAML files | IndexedDB |
| Camera bookmarks | YAML files | IndexedDB |
| Custom palettes | YAML files | IndexedDB |
| Screenshots | Save to disk | Blob download |
| Video recording | ffmpeg | Disabled |
| File import/export | rfd dialogs | HTML5 file API |
| GPU selection | Manual | Browser-managed |

---

## GitHub Actions Workflow

```yaml
name: Deploy to GitHub Pages

on:
  workflow_dispatch:
  push:
    branches: [main]
    paths: ['src/**', 'Cargo.toml', 'index.html', 'Trunk.toml']

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: "pages"
  cancel-in-progress: true

jobs:
  build:
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - run: cargo install trunk
      - run: trunk build --release --public-url /
      - name: Add CNAME for custom domain
        run: echo "par-fractal.pardev.net" > dist/CNAME
      - uses: actions/upload-pages-artifact@v3
        with:
          path: dist

  deploy:
    needs: build
    runs-on: ubuntu-latest
    timeout-minutes: 5
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - uses: actions/deploy-pages@v4
        id: deployment
```

---

## Makefile Additions

```makefile
# Web/WASM Build
web-install:
	cargo install trunk
	rustup target add wasm32-unknown-unknown

web-build:
	trunk build --release

web-serve:
	trunk serve

web-clean:
	rm -rf dist/
```

---

## Deployment Setup

1. Enable GitHub Pages (Settings > Pages > Source: GitHub Actions)
2. Add custom domain `par-fractal.pardev.net` in GitHub Pages settings
3. Ensure DNS CNAME record exists: `par-fractal.pardev.net` → `paulrobello.github.io`
