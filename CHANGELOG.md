# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **The 3D attractors rendered many times oversized, with the camera buried
  inside them.** Lorenz was the worst — ~10x — and its saved settings offered no
  explanation, because nothing in the settings was wrong. An egui slider clamps
  the value it is bound to as a side effect of being drawn, and the "Scale"
  slider is ungated, so it renders for every 3D type; its `0.5` lower bound
  silently rewrote every per-type default below it the first frame the 3D
  Parameters panel was open (`switch_fractal` set Lorenz to `0.05`, the panel
  drew, and `0.5` reached the GPU). The range is now `0.05..=5.0` — every
  shipped default is representable — and the slider is logarithmic now that it
  spans two decades. The bound lives in `FRACTAL_SCALE_RANGE` beside
  `RenderSettings` instead of as a literal inside a widget, and a regression
  test pins every per-type default against it.
- **The default camera sat inside the larger 3D fractals.** Framing was a fixed
  `(0, 0, 5)` for every 3D type, but their world-space extents differ by more
  than 5x, so Octahedron IFS, Icosahedron IFS, and Apollonian Gasket rendered as
  a wall of clipped geometry rather than a fractal. Framing distance is now a
  property of the type (`FractalType::default_camera_distance`), measured per
  type: 9.0 for both IFS types, 8.0 for Apollonian Gasket, 3.0 for Pickover, and
  the previous 5.0 for everything else. Re-framing is checked centrally once per
  frame, so it applies to every route that changes type (panel, command palette,
  function keys, `--switch-after`), and is gated on the framing *distance*
  changing so navigating between similarly scaled fractals keeps the view.
  Presets and `--camera-pos` mark the camera as deliberately framed and are not
  overwritten.
- **Chip, Quadruptwo, and Rossler rendered as specks.** Chip and Quadruptwo
  framed a view ~3x wider than their attractors actually span, so each showed a
  handful of pixels in an otherwise black frame; both now use a centre and zoom
  derived from the attractors' simulated bounds. Rossler's distance estimator
  integrated too briefly to ever reach its attractor — after the 500-step
  transient at `dt = 0.01` the trajectory was still at ~(0.1, -0.2, 0.0),
  sampling a 1.4-unit window of a 26-unit attractor — so it drew a tiny spiral.
  It now integrates at `dt = 0.02` for 2000 transient steps, which reaches the
  true extent; the scene pass costs ~0.24 ms more (0.88 ms to 1.12 ms).
- **Sierpinski Gasket rendered as empty space.** It had no arm in
  `switch_fractal` at all, so it inherited `max_iterations` — and it is an IFS
  whose every step multiplies the point by ~2.5, so the escape-time default of 80
  diverges to a distance estimator that never converges. It now seeds 10
  iterations (alongside fold, min radius, and scale) and renders.
- **A fractal's appearance depended on which fractal you viewed before it.**
  A sweep of all 35 types found several shape parameters that a distance
  estimator reads but `switch_fractal` never seeded, so they silently carried
  over: `power` for Hybrid Bulb-Julia, and `fractal_scale` for Mandelbulb,
  Sierpinski Pyramid, Julia 3D, both IFS types, and Apollonian Gasket. Each 3D
  type now seeds every uniform its estimator reads, and a regression test pins
  the whole class by switching into each type from two adversarial predecessors
  and asserting the results match.
- **Mandelbox rendered as a featureless blob.** `mandelbox_de` derives its
  internal fold scale as `-(power / 4.0)`, so the classic -2.0 needs
  `power == 8.0` — but nothing set it. `switch_fractal`'s Mandelbox arm left
  `power` untouched and the "Power" slider is shown for Mandelbulb only, so the
  value silently carried over from the previously selected fractal. From the
  2.0 default the internal scale was -0.5, and |scale| < 1 contracts the
  iteration into a smooth blob with no Mandelbox structure. Mandelbox only ever
  looked right if you happened to arrive from Mandelbulb. The `Mandelbox Cubic`
  preset was affected too — it loads via `from_settings`, bypassing
  `switch_fractal`, and inherited `power` from the shared preset defaults — so
  it now carries `power` explicitly.
- **`--clear-settings` / `make run-reset` cleared nothing.** Settings are saved
  through the platform storage abstraction to `<config>/settings/settings.yaml`
  (ARC-014), but `clear_settings()` deleted the pre-ARC-014 legacy path
  `<config>/settings.yaml`. A reset printed "No settings to clear" and the app
  kept loading the stale file, so bad saved parameters could not be recovered
  from without hand-deleting the file. Clearing now removes every location the
  loader reads (platform storage *and* the legacy fallback) and prints each path
  it removed.
- **Switching to a 3D Julia-family fractal inherited an out-of-range
  `julia_c`.** `JuliaSet3D`, `HybridMandelbulbJulia3D`, and `QuaternionCubic3D`
  all feed `julia_c` into their distance estimators but had no reset in
  `switch_fractal`, so the constant carried over from the previous fractal — the
  2D attractors leave values far outside the escape radius (Threeply2D leaves
  `[-55.0, -1.0]`, which the Julia slider clamps to `[-2.0, -1.0]`). Every point
  escaped immediately, rendering an empty set: only the floor and its shadow
  were visible. Each type now re-seeds `julia_c` to its preset value on switch.

### Performance
- **Pickover 3D cost 76 ms/frame against 2.7 ms for Mandelbulb** — 28x the next
  worst 3D type, and the reason the 3D attractors are kept out of the fractal
  panel. Its estimator re-simulates the entire 1500-iteration trajectory, five
  transcendentals per iteration, on every ray-marching step of every pixel, yet
  the trajectory does not depend on the sample position at all. Empty-space
  steps are now rejected against a bounding box that is exact and
  parameter-independent (`z = sin(x)` forces `|z| <= 1`, hence `|x|, |y| <= 2`).
  The bound is handed only to callers that use the estimate for visibility: soft
  shadows and ambient occlusion read its magnitude as a proximity signal and
  would otherwise shade the bounding box instead of the attractor. The shadow
  loop takes the bound first and pays for the exact estimate only when the bound
  cannot settle the sample, which is exact rather than an approximation.
  **76.2 ms to 19.1 ms (4x)** at 600x430 with the image unchanged (RMSE 0.00098,
  from ray-march step sequencing). Other fractals are untouched.

### Added
- `--camera-pos <x,y,z>` places the 3D camera looking at the origin. It is the
  agent-operability hook the per-type framing distances above were measured
  with, and what a future re-measure would use.

## [0.10.0] - 2026-07-20

### Added
- **`--switch-after <FractalType> <secs>` CLI flag** — switches the fractal type
  after a delay, for scripted type-transition testing (mirrors `--resize-after`;
  the variant name parses via serde). A test seam for fractal-type transitions.
- **Perturbation deep-zoom (ENH-001), Phase A + breadth.** Arbitrary-precision
  reference orbit (`dashu-float`, `src/deep_zoom/`) computed off-thread and uploaded
  as a storage buffer; per-pixel f32 delta recurrences for Mandelbrot, Julia, Burning
  Ship, and Tricorn in `shaders/fractal.wgsl`. Engages past zoom ~1e4
  (`PERTURBATION_LOG2_GATE = 13.3`, lowered from 24 / ~1.6e7 to cover the full
  df-degraded band) and fixes the >1e7 HP coordinate-precision collapse.
  Recurrence math CPU-verified vs direct f64 in `tests/perturb_math.rs`.
- **Perturbation deep-zoom (ENH-001), Phase C — precise center + zoom readout.**
  The 2D panel "Zoom" label shows a compact `≈ 1.23×10⁴⁵` readout at/above 1e4
  (commit `a05cbef`). A decimal-string precise center (`Settings.center_2d_precise`,
  `#[serde(default)]` so old files load) frees the reference orbit from f64:
  `parse_center_decimal` (`src/deep_zoom/orbit.rs`) parses a pasted coordinate
  straight to `FBig`, and the perturbation worker uses
  `compute_reference_orbit_best_precise` so zoom past ~1e15 stays correct. The 2D
  panel exposes a "Go to" field (`re, im`, optionally `@ zoom`), a "Copy" button,
  and a 🔒 indicator; pan / zoom-at-cursor clears the override, pure zoom-at-center
  keeps it. Unit-tested (`parse_center_decimal` + `parse_location_input` + driver
  invalidation).
- **1e8 deep-zoom golden** (`mandel-seahorse-1e8`) — pins the collapse-fix in the
  visual-regression harness: structured output (39 distinct gray values vs the 3 of the
  pre-fix collapse) and pixel-identical across runs (MAE 0.0) via the deterministic
  `perturbation_max_iterations` budget.
- **Deep-zoom visual-regression harness (ENH-007).** A CPU f64 reference renderer
  (`src/reference.rs`) plus a byte-by-byte Rust mirror of the shader's double-float
  math, with CI-safe precision teeth in `tests/reference_math.rs` (DF-vs-f64 on
  deep-zoom tiles, error-free-transform checks on `two_prod`/`two_sum`, known-point
  and drift-table pins). A local GPU golden-image layer (`scripts/visual_test.sh`,
  `make visual-test` / `make visual-bless`) drives the real binary per manifest row
  and compares PNGs via the new `imgdiff` bin; skips cleanly on headless boxes.
- CLI flags `--screenshot-path <path>` and `--window-size <WxH>` for deterministic,
  scriptable captures.
- **Dynamic render-scale (ENH-003).** `QualityLevel.render_scale` is now applied:
  during LOD motion the fractal pass renders into a sub-rect of `scene_texture`
  (`set_viewport`) and the post chain upsamples via a `scene_uv_scale` uniform
  (`scene_sample_uv` in `shaders/postprocess.wgsl`), cutting fragment cost ~4× at
  half resolution — imperceptible while moving, full resolution returns at idle.
  A bit-for-bit no-op at scale 1.0 (idle / LOD off / golden frames untouched);
  forced full-res for accumulation display and high-res capture. The slider is
  re-enabled.
- **GPU frame profiler (ENH-006).** A `GpuProfiler` (`src/renderer/profiler.rs`) wraps every
  render/compute pass (scene, bloom×3, composite, FXAA, egui, compute accumulation, Buddhabrot
  copy) in wgpu timestamp queries, reads them back through a 3-deep staging ring (2-frame latency,
  no pipeline stalls), and reports an EMA-smoothed per-scope ms table. `Features::TIMESTAMP_QUERY`
  is requested only when the adapter supports it — on wasm or unsupported drivers the profiler is a
  clean no-op and the HUD shows "timestamp queries unavailable". A new debug overlay
  (`render_gpu_profile_overlay`) shows per-scope ms, a proportional bar, total GPU ms, and CPU
  frame ms; toggle it with `Shift+G` or the Settings panel checkbox (the plain-`G` floor toggle is
  unchanged), and `show_gpu_profile` persists across restarts. A `--profile-dump <path>` CLI flag
  writes the EMA timings to YAML once after a 120-frame warmup, exposed as `make profile`; the
  prior Linux `perf`-based `make profile` target is now `make profile-cpu`.

### Performance
- **Half-resolution bloom pipeline (ENH-005).** Bloom extract and both blur
  passes now run at half the surface resolution; composite upsamples bilinearly.
  The blur shader derives its texel offset from the bound texture's own
  `textureDimensions`, so no texel-size uniform needed updating — the entire
  change is the three bloom textures (`bright`/`blur_temp`/`bloom`) created via
  the new `Renderer::bloom_size` helper in `src/renderer/{update,initialization}.rs`.
  Cuts bloom bandwidth/fragment work ~4× and reclaims ~¾ of the bloom-chain
  memory at 4K. Scoped to the interactive renderer textures only — the high-res
  capture path (`src/app/capture.rs`) keeps full-res bloom, so screenshots and
  goldens are byte-identical. A/B on an actively-blooming 3D preset (Mandelbox
  Cubic) is pixel-identical (corr 1.0, MAE 0.0); the bloom-OFF path is
  structurally untouched (ARC-005 gating). No threshold tweak needed.
- **Per-fractal pipeline specialization (ENH-004), Stage 2.** Completes the
  2D/3D pipeline split with per-type specialization of the 3D path: a WGSL
  pipeline-overridable constant `override SPECIALIZED_TYPE: u32 = 0xFFFFu;`
  drives the `scene_de_with_material` distance-estimator dispatch via
  `select(uniforms.fractal_type, SPECIALIZED_TYPE, SPECIALIZED_TYPE != 0xFFFFu)`
  (`shaders/fractal.wgsl`). `Renderer::ensure_specialized_3d_pipeline`
  (`src/renderer/initialization.rs`) lazily compiles one pipeline per 3D fractal
  type with `PipelineCompilationOptions.constants = [("SPECIALIZED_TYPE", ty)]`,
  cached in `Renderer::pipeline_3d_cache`; the generic `pipeline_3d` (0xFFFF
  default) stays as the runtime-switch fallback. Each specialized pipeline
  constant-folds the DE switch and dead-strips the other ~14 distance estimators
  from its compiled form. Both scene-pass sites in `src/app/render.rs` (main +
  tile-refine) select the specialized pipeline for 3D types. **Verification:** A/B
  specialized-vs-generic Mandelbulb is pixel-identical (MAE 0.0, corr 1.0,
  100% exact match) — specialization is a provable semantic no-op; 5/5 2D goldens
  unchanged; Mandelbulb (type 13) and Mandelbox (type 17) specialized smokes
  render cleanly. Perf measured via ENH-006: a ~6% best-case scene-pass delta
  within the machine-load noise floor on Apple Silicon (large register files make
  it the worst case for an occupancy win); the DCE is compiler-guaranteed and is
  expected to pay more on occupancy-bound integrated/mobile GPUs.

### Changed
- **Uniform layout automation via `encase` (ENH-008).** All seven GPU uniform
  struct families (`Uniforms`, `BloomUniforms`, `BlurUniforms`,
  `PostProcessUniforms`, `AccumulationDisplayUniforms`, `AttractorComputeUniforms`,
  `BuddhabrotComputeUniforms`) migrated from `#[repr(C)] + bytemuck + ~14 hand-
  maintained _padding_* fields` to `#[derive(encase::ShaderType)]` with glam
  vec/mat types (`glam` `encase` feature). encase and WGSL now independently
  derive the same compact layout from the field order, so the `_padding_*`
  fields are gone from both Rust and WGSL — eliminates the project's #1 silent-
  corruption bug class (add/forget-a-padding-field). `Uniforms` shrank 896→768B;
  uploads use `write_uniform_bytes`; buffer sizes use `ShaderType::min_size()`.
  Deduped three local `BlurUniforms` copies in the capture/render paths.
  Byte-pattern layout tests pin the encase-computed offsets; golden harness
  pixel-identical (incl. 1e8 perturbation deep-zoom).
- `capture_screenshot` honors `--screenshot-path` (no timestamp / toast / auto-open
  in harness mode); interactive behavior is unchanged.

### Fixed
- **Buddhabrot → attractor switch lockup.** Switching from Buddhabrot2D to a
  strange-attractor type (e.g. Hopalong2D) dispatched the Attractor compute
  pipeline with a stale Buddhabrot buffer-layout bind group, failing GPU
  validation every frame (the app appeared to hang). The Buddhabrot path stores
  a placeholder `AccumulationTexture` whose `compute_bind_group` is built
  against the buffer layout; switching to an attractor never rebuilt it. Added
  an `AccumulationBindGroupKind` tag so the attractor path detects the
  placeholder and rebuilds the texture with the correct StorageTexture layout.

### Notes
- The GPU's double-float HP path was correct through ~1e7 zoom but collapsed above ~3e7
  (root-caused via the harness: a fast ~1.2 ms frame with no device-loss, but per-pixel
  coordinate precision collapsed to one shared orbit → a near-uniform image; naga's Metal
  `two_prod` EFT was intact, so the collapse was downstream Metal flush-to-zero / sub-ULP
  lo-word loss). **Now fixed by ENH-001 perturbation** (engages past ~1e6e7) and pinned by
  the 1e8 golden; deep-zoom *math* correctness is also guarded by the CI-safe CPU teeth in
  `tests/reference_math.rs`.
- The reference orbit is cheap — ~1.5 ms single / ~7.5 ms for the 9× probe at 1e8 (91 bits),
  ~1.9 ms at 1e30 (measured via `cargo test orbit_timing -- --ignored`). An earlier "~15–20 s"
  estimate was the pre-fix LOD-churn (the orbit recomputed every frame because LOD's
  `iteration_scale` varied), resolved by the deterministic budget — not the per-orbit cost.
  Phase B step 8 (BLA series-approximation tables) is therefore deferred: GPU per-pixel cost
  is sub-frame at every testable zoom, and BLA's benefit only appears at 1e50+ (unverifiable
  today, since the f64 center carries ~15 significant digits).
- The harness's optional CPU cross-check (`CROSSCHECK=1`, `imgdiff render-ref`) now
  passes for every manifest row and catches gross failures. Three fixes, each rooted
  out by measurement:
  (1) `reference::pixel_to_c` mapped `py = 0` to clip-space `uv.y = -1`, but WebGPU
  clip space is y-up, so render-ref rendered a vertically flipped image (the 1e5
  golden matched only after a vertical flip); fixed to `uv.y = +1`.
  (2) render-ref wrote linear `t·255` while the GPU's `Bgra8UnormSrgb` surface
  sRGB-encodes its linear `vec3(t)` — the dominant error (t=0.094 → 87, exactly the
  sRGB OECF); render-ref now applies the same OECF.
  (3) render-ref iterated the manifest's raw `max_iterations`, but the GPU runs
  `max_iterations + zoom_iteration_bonus`; render-ref now uses the effective budget.
  Even faithful, the per-pixel MAE+frac gate is unpassable: the GPU iterates
  f32 / double-float / perturbation while the reference is f64-exact, so fractal-
  boundary pixels differ on ~half of all pixels (`bad_pixel_fraction` 0.2–0.95) on a
  *correct* render. The cross-check now gates on **Pearson correlation over luma**
  (`imgdiff --min-corr`, 0.7–0.87 for correct renders, ~0 for black frames / wrong
  fractals / collapsed deep zoom). It remains non-gating; the CI-safe CPU teeth in
  `tests/reference_math.rs` guard deep-zoom math correctness.

### Audit carry-overs (2026-07-16 audit, closed 2026-07-18)
- **DOC-015** — rustdoc'd the 5 submodules whose pub types rendered on docs.rs
  without descriptions (`fractal/{types,palettes,settings,ui_state,presets}.rs`):
  5 module docs + 11 type docs.
- **ARC-014 (web persistence)** — `save_all_settings` migrated from `std::fs`
  to the platform `Storage::save` abstraction, so the web target now persists
  settings to localStorage (it loaded but never saved before). Non-destructive:
  the load path reads the new platform location first, falls back to the legacy
  `<config_dir>/settings.yaml`.
- **QA-027 (winit 0.30 lifecycle)** — automated the resize case via a
  `--resize-after <s WxH>` agent-operability hook + `make smoke-resize`; the
  rest (minimize/restore, multi-monitor, HiDPI, mobile/bfcache, web) is a
  repeatable manual sweep in `docs/release-checklist.md`.
- **SEC-002 / SEC-003** — verified: SHA-pinned CI actions are in place; the
  `quick-xml` 0.39 advisories remain upstream-blocked by the
  `winit→smithay→wayland-scanner` pin and `cargo audit` passes with documented
  ignores.
- **ARC-014 (capture dedup).** Extracted the shared GPU-readback setup (staging
  buffer + texture→buffer copy + submit) and RGBA post-processing (strip
  256-byte row padding + BGRA→RGBA swap) into `app/capture_common.rs`, used by
  both `capture.rs` (native, sync `device.poll(Wait)`) and `capture_web.rs`
  (web, async `map_async`). The wait and save path stay per-target. Verified:
  native `make visual-test` pixel-identical (5/5) + web `cargo check` compiles.
  Surfaced a latent gap — `capture_screenshot_web` skips the BGRA swap the other
  paths do — preserved as-is (behavior-neutral) and flagged in the release
  checklist.

## [0.9.0] - 2026-07-17

Full remediation of the 2026-07-16 audit (81 issues across security, architecture, code quality, and documentation). See `git log` for per-commit detail.

### Security
- Clamp untrusted preset/settings/imported-JSON resource values at the trust boundary (rejects NaN/Inf) — closes a persistent GPU-DoS via hostile presets
- Replace all `unsafe transmute` render-pass lifetime extensions with `RenderPass::forget_lifetime()` (`app/render.rs` is now `unsafe`-free)
- Harden capture readback channels (device-lost no longer panics the render loop)
- Pin third-party CI actions (`Ilshidur/action-discord`, `rustsec/audit-check`) to commit SHAs
- Add a `cargo-audit` CI gate (`.cargo/audit.toml`); sanitize gallery filenames (CWE-22); clamp internal hi-res render resolution (CWE-789); surface a toast when imported presets are clamped

### Fixed
- **UI no longer freezes when clicking egui panels** — render-on-demand (ARC-006) parked the event loop based on egui's repaint flag, but egui only updates its pointer state during a render, so after one unresolved click it could never request the repaint it needed to resume (death spiral). Every interactive input event now forces a redraw; idle-sleep is preserved.
- **Deep-zoom correctness bundle**: high-precision (double-float) now auto-enables at zoom > 1e4 (was 1e6 — two decades late), `tricorn_hp` is reachable (gate `<=4`→`<=5`), `burning_ship_hp` DF `abs` negates both words correctly, and `two_prod` uses a Dekker split so DF no longer collapses to f32 on backends without fused FMA
- Clamp the `acos` input in LOD motion tracking — fixes permanent NaN-poisoning of the motion EMA mid-session
- Guard `max_iterations == 0` underflow in the shader; fix the inside-set sentinel (`0.0`→`-1.0`) so a legitimate first-iteration value isn't colored as interior
- GPU-index out-of-range now logs and falls back instead of unwrap-panicking on startup
- Make the Buddhabrot/attractor counter wrap explicit and documented

### Performance
- Render-on-demand: a `scene_dirty` flag + `ControlFlow::Wait` lets static scenes sleep instead of re-rendering at 60 Hz
- Gate the three full-screen bloom passes on `bloom_enabled` (off by default — was always-on)
- 2D LOD: `iteration_scale` quality lever + 2D zoom/pan motion detection (LOD previously couldn't reduce 2D cost)
- Split `fs_main` into `fs_main_2d`/`fs_main_3d` so each pipeline drops the other's register/occupancy footprint (per-entry-point DCE)
- Reusable `clear_texture`/`clear_buffer` accumulation clear (no per-frame multi-MB staging allocation during zoom/pan)

### Changed
- Migrate to winit 0.30 `ApplicationHandler` (drop deprecated `event_loop.run`)
- Store `zoom_2d` as `f64` (was `f32`); extract a single `FractalParams::zoom_at()` seam
- Stop LOD from mutating `FractalParams` — effective values are computed at uniform-build time (user slider edits are no longer clobbered; degraded values no longer persist)
- Decompose the 900-line `App::render` into `dispatch_accumulation` / `run_post_chain` / `render_ui` / `handle_ui_actions`
- Split the `FractalParams` God object into `RenderSettings` / `LodRuntime` / `AccumulationState` (undo no longer clones FPS/accumulation state); YAML schema unchanged (guarded by roundtrip tests)
- Deduplicate the native/web `App` constructors via a shared `init_common`
- Replace `UI::render`'s 11-element tuple with a named `UiActions` struct; split `ui/mod.rs` into per-panel submodules (3,349 → 604 lines)
- `#[repr(u32)]` discriminants on GPU-crossing enums (collapses the 30-arm match + triplicated channel matches); standardize logging on `log::` (keep CLI stdout)
- Move GPU enumeration off the render path (background thread + "Scanning…" UX); `VecDeque` undo history; Makefile bundle version derived from `Cargo.toml`; `typecheck` target added

### Added
- Numeric-core test suite (DF split, zoom→iteration bonus, enum discriminants, YAML/preset roundtrip, LOD NaN regression) — 54 → 114 tests
- Uniform-layout `offset_of!` tests cross-checked against the WGSL struct
- `CONTRIBUTING.md`; rustdoc for the crate and public API

### Documentation
- Correct README keybindings (F12 / `/`·Ctrl-K / E-Q), MSRV (Rust 1.85+, Edition 2024), and counts (35 fractals, 48 palettes, 864-byte uniform buffer)
- Sync ARCHITECTURE/FRACTALS3D/FEATURES (LOD 325/250/175/100, compute integrated, epsilon 0.00035), document deep-zoom thresholds, refresh the docs index, remove the unimplemented wheel-speed claim, fix stale CLAUDE.md facts

## [0.8.3] - 2026-07-07

### Changed
- Upgraded egui ecosystem to 0.35 (**egui** / **egui-wgpu** / **egui-winit** 0.34 → 0.35)
- Migrated Rust edition from 2021 to 2024
- Updated all dependencies to latest compatible versions
  - **log** → 0.4.33, **rand** → 0.10.2, **serde_json** → 1.0.150
  - **chrono** → 0.4.45, **env_logger** → 0.11.11, **open** → 5.3.6, **crossbeam-channel** → 0.5.16
  - Web: **wasm-bindgen** → 0.2.126, **web-sys** / **js-sys** → 0.3.103, **wasm-bindgen-futures** → 0.4.76, **getrandom** → 0.4.3
- **wgpu** held at 29.0.4 — wgpu 30 not adopted (no released egui-wgpu supports it yet)
- Added gitleaks secret-scanning pre-commit hook
- Bumped CI actions/checkout from v4 to v5

## [0.8.1] - 2025-12-26

### Fixed
- **Quality level CLI/URL parameters now apply** - `--quality`/`-q` and `?quality=` URL parameters were parsed but not actually wired into the initial quality level; they now take effect on startup
- Added missing `web-sys` `Location` feature required for URL parameter parsing on the web build

## [0.8.2] - 2026-04-11

### Changed
- Upgraded major dependencies to latest versions
  - **wgpu** 27 → 29.0.1 (migrated through API breaking changes)
  - **egui** / **egui-wgpu** / **egui-winit** 0.33 → 0.34.1
  - **glam** 0.31 → 0.32.1
  - **rand** 0.9 → 0.10
  - **image** → 0.25.10, **chrono** → 0.4.44, **env_logger** → 0.11.10
  - Web: **wasm-bindgen** 0.2.118, **web-sys** / **js-sys** 0.3.95, **wasm-bindgen-futures** 0.4.68, **gloo-timers** 0.4, **getrandom** 0.4.2

### Fixed
- Migrated to wgpu 29 `CurrentSurfaceTexture` enum-based surface acquisition
- Updated `InstanceDescriptor` construction for wgpu 29 (no more `Default` impl)
- Wrapped bind group layout entries in `Option` per wgpu 29 API
- Renamed `push_constant_ranges` → `immediate_size` per wgpu 28
- Added `multiview_mask` field to all render pipeline and render pass descriptors per wgpu 29
- Switched `MipmapFilterMode` for mipmap filtering per wgpu 29
- Updated `rand` API to use `RngExt` trait for `random_range`/`random_bool`
- Fixed egui 0.34 deprecations: `egui_wants_pointer_input`, `is_pointer_over_egui`, `egui_is_using_pointer`, `Context::run_ui`

## [0.8.0] - 2025-12-25

### Added

#### Quality Level CLI and URL Parameters
- **`--quality` / `-q` CLI parameter** - Set rendering quality level from command line
  - Supports: `low`, `medium`, `high`, `ultra` (and abbreviations: `l`, `m`, `h`, `u`)
  - Example: `par-fractal --quality low` or `par-fractal -q high`
- **URL parameters for web version** - Set quality and preset via URL
  - Query string: `?quality=high` or `?q=medium&preset=Mandelbulb`
  - Hash format: `#quality=low&p=Mandelbulb`
  - Preset parameter also supported: `?preset=name` or `?p=name`

### Changed
- Updated all dependencies to latest versions
  - egui ecosystem updated to 0.33.3
  - winit updated to 0.30.12
  - Various other dependency updates

## [0.7.2] - 2025-12-06

### Changed
- Updated dependencies to latest versions
- Added macOS quarantine troubleshooting documentation

## [0.7.1] - 2025-12-04

### Fixed
- **Buddhabrot screenshot capture** - Fixed high-resolution screenshots incorrectly using the standard fractal pipeline instead of the accumulation display pipeline for Buddhabrot
  - Changed `is_2d_attractor()` check to `uses_accumulation()` to properly include Buddhabrot in accumulation mode rendering

## [0.7.0] - 2025-12-03

### Added

#### New Fractal Type - Buddhabrot
- **Buddhabrot 2D** - Density visualization of Mandelbrot escape trajectories
  - Discovered by Melinda Green in 1993, resembles a seated Buddha figure
  - Uses compute shaders with atomic storage buffers for accumulation
  - Renders escape paths that leave the Mandelbrot set
  - Higher iteration counts reveal more detail in the "Buddha" shape
  - Preset: "Buddhabrot Classic" with optimized settings

### Changed
- Total fractal count increased from 34 to 35 (20 2D + 15 3D)
- Gallery updated with Buddhabrot and Julia 3D screenshots

## [0.6.1] - 2025-11-29

### Added
- **macOS App Bundle Support** - `make bundle` and `make run-bundle` targets for creating and running a proper macOS .app bundle with icon.

## [0.6.0] - 2025-11-26

### Added

#### Variable Power for 2D Fractals
- **Power slider for 6 escape-time 2D fractals** - Mandelbrot, Julia, Burning Ship, Tricorn, Phoenix, and Celtic fractals now support variable power (z^n + c)
  - Power range: -32 to 32 with 0.1 step increments
  - Classic fractals use power=2 by default
  - Power=3, 4, 5... creates multi-fold symmetry patterns (Multibrot, Multicorn, Multi-ship)
  - Negative powers create inverse fractal patterns
  - Smooth coloring adjusted for variable power with dynamic escape radius

### Fixed

#### Palette Animation
- **Fixed color jump when changing animation speed** - Palette animation now uses delta-time accumulation instead of elapsed-time multiplication
  - Changing speed no longer causes colors to jump to a different position
  - Animation continues smoothly from current position when speed is adjusted
  - Properly handles reverse animation direction with `rem_euclid`

## [0.5.0] - 2025-11-26

### Fixed

#### Web/Mobile Support
- **iOS Safari viewport fix** - Application now properly fills the entire viewport on iPhone devices
  - Added `viewport-fit=cover` and `maximum-scale=1.0` to viewport meta tag for proper handling of devices with notches
  - Fixed canvas sizing to use device pixel ratio for crisp rendering on high-DPI displays
  - Added `touch-action: none` and other iOS-specific CSS properties for proper touch event handling
  - Fixed position: fixed layout to prevent iOS Safari layout issues
  - Added `overscroll-behavior: none` to prevent pull-to-refresh and overscroll bounce
- **Browser window resize support** - Application now properly resizes when browser window is resized or device orientation changes
  - Added event listeners for `resize` and `orientationchange` events
  - Automatically updates canvas dimensions and notifies app of size changes
  - Properly handles device pixel ratio changes during resize
- **Touch panning and camera control** - Touch gestures now work correctly on mobile devices
  - Added explicit `WindowEvent::Touch` handling for 2D panning in `handle_2d_input()`
  - Added touch event support to 3D camera controller for rotation/looking around
  - Touch events properly map to pan/drag behavior (TouchPhase::Started/Moved/Ended)
  - Single-finger drag now works for both 2D fractal panning and 3D camera rotation
  - Disabled text selection and tap highlighting during touch interactions
- **Pinch-to-zoom gesture support** - Two-finger pinch gestures now control zoom on mobile
  - Multi-touch tracking with HashMap to manage multiple simultaneous touch points
  - Pinch-in/out gestures smoothly zoom 2D fractals in/out
  - Zoom center calculated between two fingers for intuitive zooming
  - Automatic transition between pan (1 finger) and zoom (2 fingers) modes
  - Smooth zoom factor calculation with 50% sensitivity for responsive control
  - Fixed touch state management - gestures now work reliably after UI interactions
  - Settings panel defaults to hidden on web builds for immediate gesture testing
  - Removed phantom touch detection - no longer needed with hidden settings panel
  - Natural simultaneous and delayed pinch gestures fully supported (no artificial timing constraints)

## [0.4.0] - 2025-11-25

### Added

#### Procedural Palettes
- **12 mathematically-generated color palettes** using cosine-based formulas:
  - **Fire Storm** - Classic Fractint `firestrm` palette with RGB phase-shifted cosines
  - **Rainbow** - Full spectrum HSV hue rotation
  - **Electric** - Cyan to blue to purple gradient
  - **Sunset** - Warm oranges to purples
  - **Forest** - Greens and earth tones
  - **Ocean** - Deep blues to cyan
  - **Grayscale** - Simple black to white
  - **Hot** - Black to red to yellow to white
  - **Cool** - Cyan to magenta gradient
  - **Plasma** - Purple to orange (scientific visualization)
  - **Viridis** - Perceptually uniform (scientific visualization)
  - **Custom** - User-defined cosine palette with adjustable brightness, contrast, frequency, and phase parameters

#### Command Palette Enhancements
- Added "Next Procedural Palette" command (`Shift+P`) to cycle through procedural palettes
- Renamed "Next Palette" to "Next Static Palette" for clarity

#### Keyboard Shortcuts
- `Shift+P` - Cycle procedural palette (new)
- `P` - Cycle static palette (unchanged)

### Changed
- Procedural palettes use GPU-computed colors for smooth, continuous gradients
- Palette animation works with both static and procedural palettes
- UI shows procedural palette preview and custom parameter controls

### Fixed
- Fixed command palette fractal selection not properly switching fractal type (was not calling `switch_fractal()`)
- Fixed Rainbow procedural palette being identical to Fire Storm (now uses proper HSV hue rotation)

## [0.3.0] - 2025-11-25

### Added

#### New Fractal Types - Strange Attractors
- **2D Strange Attractors (6 new):**
  - Hopalong - Barry Martin's hopalong attractor
  - Martin - Barry Martin's original attractor
  - Gingerbreadman - Chaotic 2D map
  - Chip - Chip attractor variant
  - Quadruptwo - Quadruptwo strange attractor
  - Threeply - Threeply strange attractor

- **3D Strange Attractors (3 new):**
  - Pickover - Clifford Pickover's chaotic attractor
  - Lorenz - Classic Lorenz butterfly attractor
  - Rossler - Rossler system attractor

#### Command Palette Enhancements
- Added all new strange attractor fractals to command palette
- Added shading model commands (Blinn-Phong, PBR)
- Added fog mode commands (Linear, Exponential, Quadratic)
- Added per-channel color source commands for PerChannel color mode:
  - Red/Green/Blue channel sources: Iterations, Distance, Position X/Y/Z, Normal, AO, Constant
- Toast notifications for palette changes to prevent notification stacking

### Changed
- Total fractal count increased from 26 to 34 (19 2D + 15 3D)

### Removed
- TgladFormula3D fractal type (consolidated into other IFS fractals)

### Fixed
- Fixed toast notification stacking when rapidly changing palettes
- Reduced custom palette preview squares from 30x30 to 20x20 for better UI fit

## [0.2.0] - 2025-11-24

### Added

#### Web/WASM Support
- WebGPU/WASM build support via Trunk
- Web deployment to [par-fractal.pardev.net](https://par-fractal.pardev.net)
- Platform abstraction layer for native and web builds
- Web-specific implementations for storage (localStorage), file dialogs (Blob downloads), and screenshots
- GitHub Actions workflow for automatic web deployment

#### New Fractal Types
- Sierpinski Triangle (2D) - Classic self-similar triangle fractal
- Sierpinski Gasket (3D) - 3D version of the Sierpinski fractal

#### Command Palette Enhancements
- Organized commands into categories: Fractal, Preset, Effect, Color, Camera, Recording, LOD, UI, Settings, Debug
- Fuzzy search matching for quick command filtering
- Keyboard shortcuts displayed for common commands
- Aliases for flexible command matching (e.g., "mb" for Mandelbrot)
- 10+ new commands including:
  - LOD profile switching (Balanced, Quality First, Performance First, Distance Only, Motion Only)
  - Color mode switching (17 modes including debug visualizations)
  - Effect toggles (AO, shadows, DoF, fog, bloom, vignette, FXAA, SSR, floor)
  - Recording commands (MP4, WebM, GIF)
  - Settings management (save/load presets, import/export)

### Changed
- Reorganized platform-specific code into `src/platform/` module structure
- Video recording disabled on web platform (requires ffmpeg)
- GPU selection handled by browser on web platform

### Fixed
- Improved conditional compilation guards for native-only features

## [0.1.0] - 2025-11-24

### Added
- Initial release
- GPU-accelerated rendering via WebGPU (wgpu-rs)
- 12 2D fractal types: Mandelbrot, Julia, Sierpinski Carpet, Burning Ship, Tricorn, Phoenix, Celtic, Newton, Lyapunov, Nova, Magnet, Collatz
- 12 3D fractal types: Mandelbulb, Menger Sponge, Sierpinski Pyramid, Julia Set 3D, Mandelbox, Tglad Formula, Octahedral IFS, Icosahedral IFS, Apollonian Gasket, Kleinian, Hybrid Mandelbulb-Julia, Quaternion Cubic
- Advanced rendering features:
  - PBR and Blinn-Phong shading models
  - Ambient occlusion
  - Soft shadows
  - Depth of field
  - Fog (linear, exponential, quadratic)
  - Bloom
  - FXAA anti-aliasing
  - Screen-space reflections
  - Ground plane with reflections
- Camera system with smooth animations and bookmarks
- LOD (Level of Detail) system with multiple profiles
- 6 built-in color palettes plus custom palette support
- Command palette (Ctrl/Cmd+P) for quick access to features
- Preset system for saving and loading configurations
- Undo/redo history
- Screenshot capture (PNG)
- Video recording (MP4, WebM, GIF via ffmpeg)
- Settings persistence (YAML)
- Cross-platform support: Windows (DX12/Vulkan), macOS (Metal), Linux (Vulkan)

<!-- v0.8.2 has no git tag; compare anchors use v0.8.1 as the base. -->
[0.10.0]: https://github.com/paulrobello/par-fractal/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/paulrobello/par-fractal/compare/v0.8.3...v0.9.0
[0.8.3]: https://github.com/paulrobello/par-fractal/compare/v0.8.1...v0.8.3
[0.8.2]: https://github.com/paulrobello/par-fractal/compare/v0.8.1...v0.8.2
[0.8.1]: https://github.com/paulrobello/par-fractal/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/paulrobello/par-fractal/compare/v0.7.2...v0.8.0
[0.7.2]: https://github.com/paulrobello/par-fractal/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/paulrobello/par-fractal/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/paulrobello/par-fractal/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/paulrobello/par-fractal/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/paulrobello/par-fractal/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/paulrobello/par-fractal/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/paulrobello/par-fractal/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/paulrobello/par-fractal/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/paulrobello/par-fractal/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/paulrobello/par-fractal/releases/tag/v0.1.0
