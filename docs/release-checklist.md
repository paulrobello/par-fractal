# Release Checklist

Pre-release verification. The automated gates (`make checkall`, `make visual-test`,
`make smoke-resize`) run in CI / locally without a human; the items below are the
**lifecycle and cross-platform cases that need a person at a real display/device**
(QA-027 carry-over: the winit 0.30 `ApplicationHandler` lifecycle is runtime-core,
and most of it can't be exercised headlessly).

## Automated (run first)

- `make checkall` — format, clippy, tests (the hard gate).
- `make visual-test` — GPU golden-image regression (deep-zoom rendering, ENH-007).
- `make smoke-resize` — winit resize lifecycle: 256×256 → resize → 512×512
  screenshot (QA-027; the one lifecycle case that *is* automatable).

## Manual lifecycle sweep (native)

Run `make r` and verify each survives and re-renders correctly:

- [ ] **Resize** — drag the window border smaller and larger; the fractal
      re-renders at the new size with no stretch/crash. (Also covered by
      `make smoke-resize`.)
- [ ] **Minimize → restore** — the window restores to a correct frame (no black,
      no frozen input — this was the ARC-006 death-spiral class).
- [ ] **Multi-monitor move** — drag to a second monitor; the surface
      reconfigures (different scale/format) and renders correctly.
- [ ] **HiDPI / scale change** — move between a Retina and a non-Retina display
      (or change the display scale in System Settings); physical-pixel rendering
      stays crisp, no half-resolution artifacts.
- [ ] **Focus loss / regain** — click away and back; input (keyboard zoom, mouse)
      resumes without a stale first click.
- [ ] **Long idle** — leave the window still for >30s; ARC-006 render-on-demand
      parks the loop (CPU/GPU ~0) and the next input immediately redraws.

## Manual capture sanity

- [ ] **Screenshot** (hotkey) — saves a PNG at the right size; auto-open works.
- [ ] **Hi-res render** — a custom-resolution render completes and opens.
- [ ] **Video record** (native) — start/stop produces a non-empty file.
- [ ] **Web screenshot + hi-res** (`make web-serve`) — both download, and colors
      are correct (red/blue not swapped). *Note: `capture_screenshot_web` does
      not apply the BGRA→RGBA swap that native + hi-res paths do — if web
      screenshots look color-swapped, that's the gap to fix in
      `src/app/capture_web.rs`.*

## Manual web sweep (`make web-serve` + browser)

- [ ] **Initial load** — the canvas renders on first paint (no 0×0 stall).
- [ ] **Resize / orientation change** — rotating a phone or resizing the browser
      reconfigures the canvas; the image stays correct.
- [ ] **bfcache restore** — tab away and back (or suspend/resume on mobile);
      `resumed()` is a no-op rather than re-creating the window/renderer.
- [ ] **Settings persistence** — change a setting, reload; it persists
      (ARC-014: web now saves to localStorage, not just loads).

## Cross-backend

- [ ] **Metal** (macOS native) — the above native sweep.
- [ ] **WebGPU** (browser) — the above web sweep.
- (Linux Vulkan / Windows DX12 — run the native sweep if targeting those.)
