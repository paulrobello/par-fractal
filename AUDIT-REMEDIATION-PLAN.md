# Audit Remediation Playbook

> **Companion to**: `AUDIT.md` (2026-07-16, HEAD `8ee42cc`)
> **Consumer**: `/fix-audit` agents (any model). Each entry is self-contained: exact files,
> ordered steps, method notes with pitfalls, and verification commands. Do not re-derive the
> analysis — it is already done. If a file has drifted from the line numbers given, re-locate
> with the par-mem queries noted in the entry (repo_id: `par-fractal`) or by searching the
> quoted code strings, then apply the same logical change.
>
> **Global rules for every fixer agent:**
> 1. Read the current file state before editing — earlier phases may have changed it (see AUDIT.md File Conflict Map).
> 2. Uniform-struct edits MUST keep `src/renderer/uniforms.rs` and `src/shaders/fractal.wgsl` byte-identical in layout; the compile assert `size_of::<Uniforms>() == 864` (uniforms.rs:532-535) must be updated only when layout genuinely changes on BOTH sides.
> 3. Verify with `make checkall` (runs fmt + clippy autofix + tests) from `/Users/probello/Repos/par-fractal`. Shader changes additionally need a runtime check: `cargo run --release -- --screenshot-delay 3 --exit-delay 5` and inspect the produced screenshot.
> 4. Commit each issue (or noted bundle) atomically. Do NOT push.
> 5. Duplicate findings (≙) are fixed once at the primary entry; the twin entries just verify.

---

## Phase 1 — Security clamps + unsafe removal (sequential)

### [SEC-001] Clamp untrusted preset/settings resource values at the trust boundary
- **Files**: `src/fractal/mod.rs` (fn `from_settings`, starts ~line 361), `src/fractal/settings.rs`, `src/app/capture.rs:855` (`import_from_json`), `src/fractal/presets.rs:734-747` (`load_preset`)
- **Steps**:
  1. In `src/fractal/mod.rs`, add near the top of the file a private helper module or free functions:
     ```rust
     fn clamp_finite_f32(v: f32, min: f32, max: f32, default: f32) -> f32 {
         if v.is_finite() { v.clamp(min, max) } else { default }
     }
     fn clamp_finite_f64(v: f64, min: f64, max: f64, default: f64) -> f64 {
         if v.is_finite() { v.clamp(min, max) } else { default }
     }
     ```
  2. In `from_settings`, wrap every resource-driving field. Mirror the UI slider bounds — confirm the current maxima at `src/ui/mod.rs:1468, 1701, 1708` (iterations/steps sliders) before hardcoding; use these unless the sliders say otherwise:
     - `max_iterations: settings.max_iterations.clamp(1, 100_000)`
     - `max_steps: settings.max_steps.clamp(1, 2_000)`
     - `zoom_2d: clamp_finite_f32(settings.zoom_2d, 1e-6, 1e15, 1.0)` (note: if QA-013/ARC-001 already made zoom f64, clamp as f64)
     - `attractor_iterations_per_frame: settings.attractor_iterations_per_frame.clamp(1, 2_000_000)`
     - `attractor_max_iterations`: clamp to a sane cap (e.g. `u64::min(v, 10_000_000_000)`)
     - `shadow_samples: settings.shadow_samples.min(512)`, `dof_samples: settings.dof_samples.min(64)`
     - `min_distance: clamp_finite_f32(settings.min_distance, 1e-7, 1.0, 0.00035)`
     - All other f32/f64 fields that reach the GPU uniform: pass through `clamp_finite_*` with generous bounds (reject NaN/Inf above all).
  3. `from_settings` is the single choke point for native settings, presets (`load_preset` deserializes then calls into the same path — verify with par-mem `get_symbol_context` on `from_settings`), imported JSON, and web localStorage. Confirm each loader routes through it; if `import_from_json` (`src/app/capture.rs:855`) constructs params directly, route it through `from_settings` or apply the same clamps there.
  4. Keep the existing `palette_index` clamp (mod.rs:362) as-is.
- **Method**: This is a trust-boundary fix — the UI sliders are bounded but file/localStorage input bypasses them; a hostile preset persists via auto-save, so the DoS survives restart. Pitfalls for a smaller model: (a) do NOT clamp inside `to_uniforms`/`Uniforms::update` — that would mask the stored bad values and fight the UI; clamp at deserialization only. (b) Do not change field types. (c) f32 NaN comparisons are always false — `.clamp()` alone does NOT reject NaN (it propagates it); the `is_finite()` guard is mandatory. Multi-site enumeration: par-mem `get_symbol_context(from_settings)` and `find_code("deserialize settings yaml preset")`.
- **Verify**: `make checkall`. Then create a hostile preset: `printf 'fractal_params:\n  max_iterations: 4000000000\n' > /tmp/evil.yaml` — hand-craft a full settings file by copying `~/.config/par-fractal/settings.yaml` and editing `max_iterations: 4000000000`, `zoom_2d: .nan`; run `cargo run --release -- --exit-delay 3` after placing it and confirm the app clamps (add a temporary `log::warn!` or check the rendered frame isn't a GPU hang). Also `cargo test settings` for any roundtrip tests.

### [SEC-004] (≙ QA-007, ARC-011) Replace 8 `transmute` sites with `RenderPass::forget_lifetime()`
- **Files**: `src/app/render.rs:311-312, 350-351, 383-384, 413-414, 460-461, 492-493, 522-523, 890-891`
- **Steps**:
  1. At each site, the pattern is:
     ```rust
     let mut render_pass: wgpu::RenderPass<'static> = unsafe { std::mem::transmute(render_pass) };
     ```
     Replace with:
     ```rust
     let mut render_pass = render_pass.forget_lifetime();
     ```
  2. Delete the accompanying `// SAFETY:` comments (they explain the transmute, which no longer exists).
  3. Remove the now-unneeded `unsafe` blocks; if the file has no remaining `unsafe`, nothing else to do (there is no `#![allow(unsafe_code)]` to remove).
- **Method**: wgpu 29 (`Cargo.toml`) provides `forget_lifetime()` — same semantics (pass holds the encoder borrow internally), fully safe. Pitfall: `forget_lifetime()` consumes `self` and returns `RenderPass<'static>`; the binding pattern above handles it. Do not reorder any encoder usage. This is a mechanical swap — resist refactoring anything else in `render.rs` (that's ARC-004, later).
- **Verify**: `make checkall`; then `grep -c 'transmute' src/app/render.rs` must return 0; runtime smoke: `cargo run --release -- --screenshot-delay 3 --exit-delay 5` renders normally.

### [SEC-005] Graceful channel error handling in capture readback
- **Files**: `src/app/capture.rs:60, 72, 128, 188, 200` (and 650 if present — search `send(result).unwrap()` / `recv().unwrap()`)
- **Steps**:
  1. Every `sender.send(result).unwrap()` inside a `map_async` callback → `let _ = sender.send(result);`
  2. Every `receiver.recv().unwrap()` on the main thread → handle the Err path:
     ```rust
     let Ok(result) = receiver.recv() else {
         log::error!("GPU readback channel closed (device lost?)");
         return; // or return Err(...) matching the fn signature
     };
     ```
     Match each function's actual return type — some return `Result`, some `()`; propagate or log accordingly.
  3. Use `src/app/capture_web.rs` as the reference pattern — it already handles these gracefully with `if let Ok(...)`.
- **Method**: On device-lost (which SEC-001's attack triggers), the map callback may never fire or the receiver may be dropped; unwrap converts a recoverable GPU error into a process crash. Pitfall: don't swallow errors silently — log them; and don't change the happy-path behavior (screenshot/video output must still work).
- **Verify**: `make checkall`; `grep -n 'unwrap()' src/app/capture.rs` — remaining unwraps must be non-channel (justify each); screenshot smoke test as above, confirm the PNG file is produced.

---

## Phase 2 — Deep-zoom & structural foundations (sequential, in this order)

### [ARC-013] Extract the single `zoom_at()` seam (zoom-at-cursor triplication)
- **Files**: `src/app/update.rs:43-71` (continuous shift+drag zoom), `src/app/input.rs:490-510` (pinch), `src/app/input.rs:620-645` (scroll wheel), new code in `src/fractal/mod.rs`
- **Steps**:
  1. Add to `impl FractalParams` in `src/fractal/mod.rs`:
     ```rust
     /// Zoom by `factor` keeping the fractal point under `cursor_ndc` fixed.
     /// cursor_ndc: normalized device coords in [-1, 1], y-up, BEFORE aspect correction.
     pub fn zoom_at(&mut self, cursor_ndc: (f64, f64), factor: f64, aspect: f64) {
         let old_zoom = self.zoom_2d as f64;
         let fx = self.center_2d[0] + (cursor_ndc.0 * 2.0 / old_zoom) * aspect;
         let fy = self.center_2d[1] + cursor_ndc.1 * 2.0 / old_zoom;
         let new_zoom = old_zoom * factor;
         self.center_2d[0] = fx - (cursor_ndc.0 * 2.0 / new_zoom) * aspect;
         self.center_2d[1] = fy - cursor_ndc.1 * 2.0 / new_zoom;
         self.zoom_2d = new_zoom as f32;
     }
     ```
     **Before writing this**, read all three existing sites and reconcile their exact NDC conventions (sign of y, where aspect is applied) — the three copies have "slight variations"; the extracted function must reproduce the behavior of `src/app/update.rs:43-71`, which is the reference (verified correct f64 end-to-end).
  2. Replace all three call sites with `self.fractal_params.zoom_at(...)`, converting each site's cursor coordinates to the shared NDC convention.
  3. Do not change zoom's type in this step (that's ARC-001/QA-013).
- **Method**: This creates the one seam through which the f64/representation change flows. Pitfall: the three sites differ in how they compute `norm_x/norm_y` from window coords — reconcile carefully or zoom will "drift" toward a corner; test each input path (wheel, pinch on trackpad, shift+drag) interactively if possible, or at minimum compare a before/after screenshot at the same coords. par-mem check for missed callers: `find_code("zoom_2d multiply factor center")` and `get_symbol_context` on `zoom_at` after the change.
- **Verify**: `make checkall`; manual/scripted: `cargo run --release -- --screenshot-delay 3 --exit-delay 5` (default view unchanged). Grep `zoom_2d \*=` — should appear only inside `zoom_at` (plus any keyboard-zoom that goes through it).

### [ARC-001] Deep-zoom foundations: f64 zoom storage + derived hp threshold
- **Files**: `src/fractal/mod.rs:46-47`, `src/fractal/settings.rs` (zoom field), `src/renderer/uniforms.rs:280-301`, `src/app/update.rs`, `src/app/input.rs`, `src/ui/mod.rs` (zoom display/slider), `src/web_main.rs` (URL params if zoom is parsed)
- **Steps** (this entry = foundations only; the perturbation subsystem is `docs/fable/ENH-001-perturbation-deep-zoom.md`):
  1. Change `pub zoom_2d: f32` → `pub zoom_2d: f64` in `src/fractal/mod.rs:47`.
  2. Chase compile errors — this is the point of the seam from ARC-013. Expected sites: `settings.rs` serde field (change to f64; serde will still read old f32 YAML values fine), `uniforms.rs:280` (`self.zoom = params.zoom_2d as f32` — keep the uniform f32, cast at the boundary), `zoom_at` (drop the `as f64`/`as f32` round-trips), UI display code, presets defaults (`0.3`, `0.05`, etc. — just become f64 literals), `fractal/tests.rs` fixtures.
  3. In `Uniforms::update` (uniforms.rs:283-301): compute the hp gate from pixel spacing instead of the magic constant — see ARC-002 step 2 (do it there; these two entries land adjacent).
  4. Also split zoom for the shader the way center is split IF trivially possible — otherwise defer to ENH-001. Minimum for this step: the uniform keeps receiving `zoom as f32`; document with a comment that the shader-side f32 zoom is the remaining precision limiter (per AUDIT ARC-001).
  5. SEC-001's clamp on zoom becomes `clamp_finite_f64(v, 1e-6, 1e30, 1.0)`.
- **Method**: f64 zoom removes per-frame f32 rounding accumulation in `zoom_2d *= factor` and unblocks perturbation work; it does NOT by itself extend the on-GPU ceiling (~1e11 from DF center + f32 zoom uniform) — do not claim otherwise in commit messages. Pitfalls: (a) `egui` sliders take f64 ranges via `Slider::new(&mut value, range)` — but if the slider binds `&mut f32`, add a local f64↔f32 shim at the UI only; (b) YAML settings roundtrip: old files store f32-precision values, which parse into f64 losslessly — no migration needed; (c) do not change `center_2d` (already f64). Enumerate all uses first: par-mem `get_impact(symbol: "zoom_2d")` or `grep -rn 'zoom_2d' src/`— roughly 12 files per the initial survey (update.rs, input.rs, uniforms.rs, mod.rs, presets.rs, settings.rs, tests.rs, ui/mod.rs, ui/command.rs, app/mod.rs, render.rs, compute.rs).
- **Verify**: `make checkall` (tests include settings roundtrips); `cargo run --release -- --screenshot-delay 3 --exit-delay 5`; load an old settings.yaml (pre-change) and confirm it parses.

### [ARC-002 bundle] Fix the hp gate — MUST land as ONE commit with QA-001, QA-002, QA-004, QA-005
> Lowering the threshold without fixing the hp math widens exposure to broken hp paths.
> Recommended order inside the bundle: QA-002 → QA-005 → QA-001 → QA-004/threshold, then verify all.

#### [QA-002] Fix `burning_ship_hp` double-float abs
- **Files**: `src/shaders/fractal.wgsl:506`
- **Steps**: Replace
  ```wgsl
  let z_abs = df2(abs(z.hi), abs(z.lo) * sign(z.hi));
  ```
  with
  ```wgsl
  let neg = select(vec2<f32>(1.0), vec2<f32>(-1.0), z.hi < vec2<f32>(0.0));
  let z_abs = df2(abs(z.hi), z.lo * neg);
  ```
  (Adjust to the actual df2 component types at that site — if `z.hi`/`z.lo` are scalars there, use the scalar form `select(1.0, -1.0, z.hi < 0.0)`.)
- **Method**: DF `abs` must negate BOTH words when hi < 0, preserving lo's independent sign; `select` avoids `sign(0.0)==0.0` zeroing lo. Pitfall: read the surrounding code to see whether `z` is a `DF2` of vec2s (complex packed) or two scalars — apply component-wise accordingly.

#### [QA-005] Dekker-split `two_prod` fallback
- **Files**: `src/shaders/fractal.wgsl:355-392` (`two_sum`, `two_prod`, df helpers)
- **Steps**:
  1. Add a Dekker split product (works without fused FMA):
     ```wgsl
     // Dekker split: exact a*b = p + e without relying on hardware FMA.
     fn two_prod(a: f32, b: f32) -> vec2<f32> {
         let p = a * b;
         let SPLIT = 4097.0; // 2^12 + 1 for f32 (24-bit mantissa)
         let a_t = a * SPLIT; let a_hi = a_t - (a_t - a); let a_lo = a - a_hi;
         let b_t = b * SPLIT; let b_hi = b_t - (b_t - b); let b_lo = b - b_hi;
         let e = ((a_hi * b_hi - p) + a_hi * b_lo + a_lo * b_hi) + a_lo * b_lo;
         return vec2<f32>(p, e);
     }
     ```
     Replace the FMA-based body (keep the same signature so callers don't change). If both variants are desired, keep FMA behind a comment explaining why the split version is the default (portability > speed here).
  2. Check `two_sum`: its error term must not be optimizable away; if naga/backends apply fast-math, the standard guard is to keep it as-is (WGSL forbids reassociation by spec) but ADD a comment noting the dependence and the verification below.
- **Method**: WGSL's `fma()` may lower to `a*b+c` on some backends, making the error term always 0 and silently collapsing DF to f32. The Dekker split is branch-free and correct on all IEEE-754 f32 hardware. Pitfall: keep intermediate `let`s exactly — "simplifying" the algebra destroys the error-free transformation; do NOT let a formatter or reviewer merge the expressions.

#### [QA-001] Make `tricorn_hp` reachable
- **Files**: `src/shaders/fractal.wgsl:2946` (gate), `2946-2972` (dispatch)
- **Steps**:
  1. Change the gate `uniforms.high_precision == 1u && uniforms.fractal_type <= 4u` → `... <= 5u`.
  2. In the dispatch chain, add an explicit `else if uniforms.fractal_type == 5u { t = tricorn_hp(...); }` and make the final `else` a safe fallback to the standard path (never an hp function).
- **Method**: Types 0–4 have explicit branches; the old trailing `else` was intended for Tricorn (type 5) but the gate excluded it. Pitfall: confirm type 5 is Tricorn by checking the fractal-type mapping in `src/renderer/uniforms.rs:307-348` before editing — do not trust the ordinal from memory.

#### [QA-004 / ARC-002] Lower + derive the hp auto-enable threshold
- **Files**: `src/renderer/uniforms.rs:283-301`
- **Steps**:
  1. Replace `let use_high_precision = params.zoom_2d > 1_000_000.0;` with a derived criterion:
     ```rust
     /// Enable double-float when f32 per-pixel spacing nears the f32 ulp at the
     /// current center magnitude. spacing = (2/zoom)*(2/height); ulp(x) ~ x * 2^-23.
     const HP_SAFETY_FACTOR: f64 = 256.0; // engage well before visible quantization
     let center_mag = params.center_2d[0].abs().max(params.center_2d[1].abs()).max(1.0);
     let pixel_spacing = (2.0 / params.zoom_2d as f64) * (2.0 / window_height as f64);
     let ulp = center_mag * (f32::EPSILON as f64);
     let use_high_precision = pixel_spacing < ulp * HP_SAFETY_FACTOR;
     ```
     `window_height`: `Uniforms::update` already receives resolution (it sets `aspect_ratio`) — use the actual param name in scope. If plumbing height is disruptive, fall back to the simple fix: `params.zoom_2d > 10_000.0` with a named `const HP_ZOOM_THRESHOLD: f64 = 1e4;` and a comment. Either is acceptable; derived is better.
  2. Name whichever constant is used and document the rationale (AUDIT ARC-002/QA-004).
- **Method**: f32 spacing collapses at zoom ~2e4–6e4 at 1080p; 1e6 is two decades late. The derived form scales with window size and center magnitude. Pitfall: hp is ~10–20× slower — engaging it too early tanks perf; the safety factor 256 ≈ engages around zoom 3–4e3 at 1080p center |c|≈1, which is conservative-but-cheap; tune down to 64 if perf regresses at moderate zoom (document the tradeoff).

#### Bundle verification (run after all four edits)
- `make checkall`
- Runtime sweep: for Z in 1e3 1e5 1e7 1e9; render Mandelbrot at a detailed location, e.g.
  `cargo run --release -- --screenshot-delay 4 --exit-delay 6` after setting zoom via a preset file (create presets with center `(-0.7436438870, 0.1318259042)` at each zoom; the preset mechanism is `~/.config/par-fractal/presets/*.yaml` or `--preset name`).
  Confirm: no blocky quantization at 1e5 (pre-fix it was visible), smooth structure at 1e7–1e9.
- Tricorn: same at zoom 1e6 for fractal_type Tricorn2D — pre-fix pixelated, post-fix sharp.
- Burning Ship at 1e8: structurally sharp (QA-002).
- Keep the four screenshots as before/after evidence in the commit message or PR description.

### [ARC-008] (≙ QA-010) Stop LOD mutating `FractalParams` — effective values at uniform build
- **Files**: `src/fractal/mod.rs:820-1013` (`apply_lod_quality`, `update_lod`, `calculate_distance_lod`), `src/renderer/uniforms.rs` (`Uniforms::update`), `src/lod.rs`, call sites in `src/app/update.rs` / `src/app/render.rs`
- **Steps**:
  1. Read `apply_lod_quality` (mod.rs:1000-1013). It currently writes `max_steps`, `min_distance`, `shadow_samples`, `shadow_step_factor`, `ao_step_size`, `dof_samples` into `self` (FractalParams). Delete those writes.
  2. Introduce an "effective quality" product instead: add a method on `FractalParams` (or free fn) `pub fn effective_quality(&self) -> QualityLevel` returning the LOD-active `QualityLevel` (from `lod_state.get_active_quality(...)`) or a pass-through of the user's own values when LOD is disabled.
  3. In `Uniforms::update` (uniforms.rs:273+), where those six fields are read from `params`, take the min/merge with the effective quality: `let q = params.effective_quality(); self.max_steps = params.max_steps.min(q.max_steps); ...` (min for costs, max for `min_distance` — think "cheaper of the two", verify per field which direction is cheaper).
  4. Delete any "restore user values when LOD disabled" workaround if one exists — with no mutation, nothing needs restoring (this also fixes QA-010(b): stale degraded values after disabling LOD).
  5. Confirm `to_settings()` (mod.rs:296-305) now serializes only user-authored values by construction.
- **Method**: Classic source-vs-derived state separation; the bug class is user edits being clobbered and degraded values being persisted. Pitfalls: (a) the LOD debug overlay (`ui/overlays.rs:348`) displays active values — point it at `effective_quality()` so it still shows the truth; (b) `calculate_distance_lod`/FPS logic stays where it is — only the *application* of quality moves; (c) some UI sliders display `params.max_steps` — they must now show the authored value (that's the fix, not a regression). par-mem: `get_symbol_context(apply_lod_quality)` to enumerate all callers.
- **Verify**: `make checkall`. Runtime: enable LOD, move camera (quality drops), stop — sliders retain user values; disable LOD mid-motion — values snap back to authored, not stuck at Low. Check `settings.yaml` after a session with LOD active: `max_steps` equals the user's slider value, not 100.

### [ARC-003] Replace `UI::render`'s 11-tuple with a `UiActions` struct
- **Files**: `src/ui/mod.rs:373` (signature + all `return` sites), `src/app/render.rs:610-628` (destructuring call site)
- **Steps**:
  1. Define in `src/ui/mod.rs`:
     ```rust
     #[derive(Default)]
     pub struct UiActions {
         pub screenshot_requested: bool,
         pub reset_requested: bool,
         // ... one named field per current tuple element, in the same order —
         // derive the names from how app/render.rs:610-628 uses each position.
         pub preset_to_load: Option<Preset>,
         pub resolution_change: Option<(u32, u32)>,
         pub bookmark_to_apply: Option<CameraBookmark>,
     }
     ```
     **Read the call site first** (`app/render.rs:610-628`) — the destructuring names there are the authoritative meaning of each tuple position. Name fields after those, not after guesses.
  2. Change `UI::render` to build and return `UiActions` (initialize `UiActions::default()` at the top; replace each tuple-element assignment with a field write).
  3. Update the call site to use named fields; delete the `#[allow(clippy::type_complexity)]`.
- **Method**: Pure mechanical de-tupling; zero behavior change. Pitfall: two adjacent bools swapped is exactly the bug this prevents — while migrating, map positions one at a time, compiling between steps if unsure. Do NOT also split the method into panels here (that's QA-009, later, to keep this diff reviewable).
- **Verify**: `make checkall`; runtime smoke: click through screenshot button, preset load, resolution change in the UI (`cargo run --release`), or at minimum confirm compile + tests + a screenshot render.

### [ARC-010] (≙ QA-018) Uniform layout offset tests + CLAUDE.md byte-count fix
- **Files**: `src/renderer/uniforms.rs` (add test module), `CLAUDE.md` (line stating "784 bytes")
- **Steps**:
  1. Add to `uniforms.rs` a test (std `offset_of!` is stable on Rust ≥1.77):
     ```rust
     #[cfg(test)]
     mod layout_tests {
         use super::*;
         use std::mem::offset_of;
         #[test]
         fn wgsl_layout_contract() {
             assert_eq!(std::mem::size_of::<Uniforms>(), 864);
             // Sentinel offsets — MUST match the WGSL struct in src/shaders/fractal.wgsl.
             // Update BOTH files together when the layout changes.
             assert_eq!(offset_of!(Uniforms, palette), /* fill from actual layout */);
             assert_eq!(offset_of!(Uniforms, center_hi), /* ... */);
             assert_eq!(offset_of!(Uniforms, aspect_ratio), /* ... */);
             assert_eq!(offset_of!(Uniforms, max_iterations), /* ... */);
         }
     }
     ```
     To fill the expected offsets: write the test with placeholder `0`s, run `cargo test wgsl_layout_contract` — the failure output prints actual offsets; then verify each against the WGSL struct (`src/shaders/fractal.wgsl:60-147`) by summing WGSL alignment rules (vec4 16B aligned, vec3 16B aligned + 4B pad, etc.), and hard-code the verified numbers. The point is: a future field swap breaks the test.
  2. Pick 4–6 sentinel fields spread across the struct (early, middle, post-padding, last field).
  3. `CLAUDE.md`: change "Current size is 784 bytes" → "Current size is 864 bytes (compile-asserted in `src/renderer/uniforms.rs` and offset-tested in its `layout_tests`)".
  4. (Optional, larger) `encase` migration is deliberately NOT done here — it's scoped as ENH-008.
- **Method**: The existing size assert misses equal-size field swaps. Offsets are the actual GPU contract. Pitfall: do not "verify" offsets only by running Rust — the point is cross-checking against WGSL; a wrong shared assumption would pass. Manually confirm at least `center_hi` and `palette` against the WGSL declaration order.
- **Verify**: `cargo test wgsl_layout_contract` green; `make checkall`.

---

## Phase 3a — Security (remaining, parallelizable)

### [SEC-002] Pin `Ilshidur/action-discord` to a commit SHA
- **Files**: `.github/workflows/release.yml:194`, `.github/workflows/publish-crates.yml:80`
- **Steps**:
  1. Resolve the SHA: `gh api repos/Ilshidur/action-discord/commits/master --jq .sha` (or use the latest release tag's SHA: `gh api repos/Ilshidur/action-discord/tags --jq '.[0] | .name + " " + .commit.sha'`).
  2. Replace `uses: Ilshidur/action-discord@master` with `uses: Ilshidur/action-discord@<full-40-char-sha> # master as of 2026-07-16` in BOTH files.
  3. Confirm the ref resolves: `gh api repos/Ilshidur/action-discord/commits/<sha> --jq .sha`.
- **Method**: Supply-chain pin per the house git-ci guidance (pin to exact ref; verify it resolves before committing). Pitfall: use the FULL 40-char SHA — short SHAs are movable by collision and some runners reject them. This is a security-sensitive CI change: flag it in the commit message for the user's review; do not alter secrets/permissions beyond this unless asked.
- **Verify**: `grep -n 'action-discord' .github/workflows/*.yml` shows only SHA-pinned refs; YAML parses (`python3 -c "import yaml,glob; [yaml.safe_load(open(f)) for f in glob.glob('.github/workflows/*.yml')]"`).

### [SEC-003] Update `quick-xml`; add `cargo audit` CI gate
- **Files**: `Cargo.lock`; one CI workflow (e.g. `.github/workflows/` main CI file — inspect to pick)
- **Steps**:
  1. `cargo update -p quick-xml` — check the result: `cargo tree -i quick-xml` must show ≥0.41.0. If the winit/smithay graph pins it lower, note that in the commit and skip to step 2 (tracking upstream is the fallback the audit accepts).
  2. Add an audit job to CI (new job in the existing test workflow):
     ```yaml
     audit:
       runs-on: ubuntu-latest
       steps:
         - uses: actions/checkout@v5
         - uses: rustsec/audit-check@v2.0.0
           with:
             token: ${{ secrets.GITHUB_TOKEN }}
     ```
     Pin `rustsec/audit-check` to its current SHA (same procedure as SEC-002).
  3. Run `cargo audit` locally first; if pre-existing advisories (ttf-parser RUSTSEC-2026-0192 — SEC-008) would fail the gate, configure `ignore` entries in `audit.toml` with a comment, so the gate lands green.
- **Method**: The vulnerable crate is build-time/Wayland-only, so urgency is low, but the CI gate prevents future regressions. Pitfall: don't let the new gate break CI on day one — handle known-ignored advisories explicitly.
- **Verify**: `cargo audit` exits 0 locally; `cargo tree -i quick-xml` shows the version; CI YAML parses.

### [SEC-006] Range-validate imported presets with a user-visible clamp notice
- **Files**: `src/app/capture.rs:855` (`import_from_json`), `src/ui/mod.rs:858` (import trigger), toast API in `src/ui/toast.rs`
- **Steps**:
  1. After SEC-001, imports are already clamped at `from_settings`. Add detection: have the clamp helpers (SEC-001) return whether clamping occurred (e.g. `fn clamp_report(...) -> (T, bool)` or accumulate a `clamped: bool` in `from_settings` returning `(Self, bool)` — pick the least invasive: a `&mut bool` out-param threaded through, or compare pre/post values at the import call site).
  2. Simplest robust approach at the import site: deserialize, build params via `from_settings`, then compare a handful of resource fields against the raw deserialized values; if any differ, show a toast: `"Preset values out of range were clamped"` using the existing `Toast` type (`src/ui/toast.rs`).
- **Method**: UX transparency on top of SEC-001's silent safety. Pitfall: don't duplicate the clamp logic — reuse SEC-001's path and only *detect* differences.
- **Verify**: `make checkall`; import a JSON preset with `max_iterations: 999999999` → toast appears, app healthy.

### [SEC-007] Sanitize filenames inside `PresetGallery`/`BookmarkGallery`
- **Files**: `src/fractal/presets.rs:717, 734, 750` (PresetGallery save/load/delete), `:170, 189, 204` (BookmarkGallery)
- **Steps**:
  1. Add one helper in `presets.rs`:
     ```rust
     fn sanitize_name(name: &str) -> String {
         name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect()
     }
     ```
     Match the existing caller-side sanitization at `src/ui/mod.rs:751, 1894` (read it first; use the same replacement rules so existing saved files keep resolving).
  2. Apply `let filename = sanitize_name(filename);` at the top of all six gallery methods before any `join(format!("{}.yaml", ...))`.
  3. Leave the UI-side sanitization in place (harmless duplication, defense in depth).
- **Method**: The API is traversal-prone by contract even though no current caller passes unsanitized input. Pitfall: if the gallery methods' replacement rules differ from the UI's, a preset saved pre-change could stop resolving post-change — matching the UI's existing rules avoids this.
- **Verify**: `make checkall`; unit test: `load_preset("../../etc/passwd")` resolves inside the presets dir (add a small test asserting `sanitize_name("../x") == "___x"` or similar).

### [SEC-008] Track `ttf-parser` advisory (informational)
- **Files**: `audit.toml` (new, if SEC-003's gate needs it), else none
- **Steps**: If the SEC-003 CI gate is added, add `RUSTSEC-2026-0192` to the ignore list with comment `# ttf-parser unmaintained; transitive via winit->ab_glyph; awaiting upstream`. No other action.
- **Verify**: `cargo audit` green.

### [SEC-009] Internal resolution clamp in hi-res render paths
- **Files**: `src/app/capture.rs:233` (`render_high_resolution`), `src/app/capture_web.rs:157` (`render_high_resolution_web`)
- **Steps**: At the top of both functions:
  ```rust
  let width = width.clamp(1, 16384);
  let height = height.clamp(1, 16384);
  ```
  Match the existing UI bound (confirmed at `src/ui/mod.rs:3044`: `1..=16384`). Optionally `log::warn!` when clamping.
- **Method**: Defense in depth — the functions allocate ~6 full-res RGBA16F textures; today only bounded UI calls them, but the API shouldn't rely on that. Pitfall: also consider total-pixel budget (16384×16384 is ~1.6 GB of intermediates and may legitimately fail on smaller GPUs) — that's an accepted existing behavior; do not add device-limit querying here (scope creep).
- **Verify**: `make checkall`; hi-res screenshot from the UI still works at a normal size (e.g. 3840×2160).

### [SEC-010] (≙ part of QA-021) Remove debug line; migrate prints in touched files
- **Files**: `src/ui/toast_ui.rs:102`
- **Steps**: Delete or convert to `log::debug!` the line printing `"DEBUG: Successfully called open::that()"`. Full println→log migration is QA-021; this entry covers only the leftover debug line.
- **Verify**: `grep -rn 'DEBUG:' src/` returns nothing; `make checkall`.

---

## Phase 3b — Architecture (remaining, parallelizable except where noted)

### [ARC-005] (≙ QA-006) Gate the bloom passes on `bloom_enabled`
- **Files**: `src/app/render.rs:360-469`
- **Steps**:
  1. Find `if true { // Always run bloom passes` (render.rs:361). Replace the condition with the actual toggle: `if self.fractal_params.bloom_enabled {` (confirm the field name via `grep -n 'bloom_enabled' src/fractal/mod.rs` — default false per mod.rs:247).
  2. The composite pass samples the bloom texture regardless, so it must contain defined data when bloom is off. On the transition enabled→disabled (or simply when disabled and a `bloom_texture_cleared` flag is false), record one cheap clear of the bloom output texture: begin a render pass on the final blur target with `LoadOp::Clear(BLACK)` and no draw, then set the flag; reset the flag when bloom re-enables.
  3. Keep `PostProcessUniforms.bloom_enabled` logic as-is (composite still multiplies by it — belt and suspenders).
- **Method**: Three full-res Rgba16Float passes are pure waste when disabled (the default). Pitfall: skipping the passes WITHOUT step 2 samples stale/undefined texture memory — on some backends that's garbage pixels when the user toggles bloom off after having it on. The one-time clear is mandatory.
- **Verify**: `make checkall`; runtime: bloom off → screenshot identical to before (compare a screenshot pre/post change with bloom off); toggle bloom on → glow appears; toggle off → image returns to clean (no ghost bloom). Frame rate at default settings should measurably improve on integrated GPUs (report before/after FPS from the overlay if available).

### [ARC-006] (≙ QA-011) Dirty-flag render-on-demand — do AFTER ARC-008
- **Files**: `src/main.rs:277-283`, `src/web_main.rs:331`, `src/app/update.rs`, `src/app/mod.rs` (App state), `src/app/render.rs`
- **Steps**:
  1. Add `scene_dirty: bool` (init `true`) to `App` (`src/app/mod.rs`).
  2. Set it `true` from every state change that affects the image: input handlers (`src/app/input.rs` — any mutation of fractal params/camera), UI actions that change params (`app/render.rs` UiActions handling — after ARC-003 this is one place), LOD transitions in progress (`lod_state.update_transition` returning "still transitioning"), palette/procedural animation active (check the animation flags in `FractalParams` — e.g. procedural phase animation), camera transitions active, 3D mode with autorotate, attractor/Buddhabrot accumulation still converging, video recording active, and egui wanting repaint (`egui_ctx.has_requested_repaint()` or the platform equivalent — check how egui integration exposes it in this codebase).
  3. In `main.rs` `AboutToWait`: `if app.scene_dirty || app.ui_needs_repaint() { app.window().request_redraw(); }` — on native ALSO switch `ControlFlow` to `Wait` when clean so the loop sleeps (verify how ControlFlow is currently set, main.rs:256+). On web (`web_main.rs:331`), keep rAF but early-return the scene pass when clean and re-present.
  4. In `App::render`: when rendering completes and no animation source is active, set `scene_dirty = false`. When clean but a redraw is forced (e.g. window expose), re-present without re-running the fractal pass IF the codebase's pass structure allows presenting `scene_texture` through the post chain cheaply — otherwise (simpler v1) keep the full pipeline but only when dirty, and accept expose-event re-renders.
  5. IMPORTANT scope control: v1 = "skip redraw entirely when clean". Progressive refinement (render finer while idle) is ENH-002, not this fix.
- **Method**: The app re-renders static images at 60 Hz. Depends on ARC-008 because LOD's per-frame param mutation made everything look dirty every frame. Pitfalls: (a) egui needs frames while the user interacts with UI even if the fractal is unchanged — the `ui_needs_repaint` OR-term is essential or the UI freezes; (b) the `time` uniform drives palette animation — gate on whether any time-dependent feature is enabled, else animations stop; (c) window resize/expose must set dirty; (d) FPS-based LOD reads frame timing — with `Wait`, FPS becomes meaningless when idle; guard the LOD FPS update to only run on rendered frames. Test wasm too (`make web-build`).
- **Verify**: `make checkall`; runtime: idle 2D Mandelbrot → GPU usage near zero (check Activity Monitor GPU history on macOS); interact → immediate response; palette animation still animates; egui panels stay responsive when idle; window resize repaints. `make web-build` compiles.

### [ARC-007] 2D LOD: iteration lever + 2D motion detection; hide dead `render_scale` slider — AFTER ARC-008
- **Files**: `src/lod.rs:43-58` (QualityLevel), `:354-406` (update_motion), `src/fractal/mod.rs` (effective_quality merge from ARC-008), `src/renderer/uniforms.rs:299-301` (iteration computation), `src/ui/mod.rs:2502` (render_scale slider)
- **Steps**:
  1. Add `pub iteration_scale: f32` to `QualityLevel` (ultra 1.0, high 0.85, medium 0.6, low 0.35 — starting points; include in `lerp`). Update the four preset constructors and any serialized form (check `LODConfig` serde — new field needs `#[serde(default = ...)]` returning 1.0 for backward compat).
  2. In the iteration computation (uniforms.rs:299-301, post-ARC-008 it reads effective quality): `let effective_iters = ((params.max_iterations + zoom_bonus) as f32 * q.iteration_scale) as u32; self.max_iterations = effective_iters.max(16);`
  3. 2D motion: extend `update_motion` (lod.rs:354-406) with a 2D path — when in 2D mode, compute motion from `d(log2(zoom))/dt` and pan speed in screen-space units (`d(center) * zoom / dt`), mapped to the same magnitude scale as 3D translation (tune so continuous zoom registers as "moving"). The caller passes camera data today — thread `&FractalParams` or precomputed 2D deltas in (check the call site via `grep -n 'update_motion' src/`).
  4. `render_scale` slider (ui/mod.rs:2502): hide it (comment out with a note referencing ENH-003) or label it "(not yet applied)". Do NOT implement render scaling here — that's ENH-003.
- **Method**: Gives LOD its missing 2D levers so deep-zoom interaction degrades gracefully instead of collapsing FPS first. Pitfalls: (a) `iteration_scale` must multiply AFTER the zoom bonus, not before, or deep zoom re-loses detail; (b) floor at ≥16 iterations to avoid blank renders; (c) serde default for old settings.yaml files; (d) motion units for 2D need tuning — expose the mapping constant near the existing motion_threshold config rather than burying it.
- **Verify**: `make checkall`; runtime with LOD debug overlay (`render_lod_debug_overlay`): continuous 2D zoom shows motion detected + reduced iterations, idle restores; old settings.yaml still loads.

### [ARC-004] (≙ QA-008) Decompose `App::render` — AFTER ARC-003 and SEC-004; one owner for render.rs
- **Files**: `src/app/render.rs:12-908`
- **Steps**:
  1. Extract in this order (mechanical moves, no behavior change):
     a. `fn dispatch_accumulation(&mut self, encoder: &mut wgpu::CommandEncoder)` — the attractor/Buddhabrot block (~lines 77-140).
     b. `fn run_post_chain(&mut self, encoder: &mut wgpu::CommandEncoder, ...)` — bloom/blur/composite/FXAA passes (~lines 300-540).
     c. `fn render_ui(&mut self, ...) -> UiActions` — the egui frame (already returns UiActions post-ARC-003).
     d. `fn handle_ui_actions(&mut self, actions: UiActions)` — preset loading, bookmarks, recorder lifecycle, reset, resolution changes. Call it from `App::update` (`src/app/update.rs`) next frame, or immediately after `render_ui` as an interim step if moving to update() breaks same-frame expectations (e.g. screenshot-this-frame). Screenshot/video capture requests that must affect the CURRENT frame stay in render — document which.
  2. Move the GPU enumeration `pollster::block_on` (line 656) — see ARC-018; do it as part of this decomposition or leave a `// TODO(ARC-018)`.
  3. Target: `App::render` body < 150 lines of orchestration.
- **Method**: Behavior-preserving extraction. Pitfalls: (a) borrow-checker friction — the extracted fns need disjoint borrows; pass exactly what each needs, prefer `&mut self` methods and let NLL work; (b) encoder lifecycle: capture forks the encoder mid-function (~lines 550-600) — keep submission order IDENTICAL (screenshot correctness depends on it); (c) do not reorder pass encoding relative to `queue.submit` calls.
- **Verify**: `make checkall`; runtime: screenshot works, video record start/stop works, preset load works, attractor mode accumulates. Diff review: every extracted line identical (pure moves).

### [ARC-009] (≙ QA-015) Split 2D/3D shader pipelines — scope with ENH-004 (see `docs/fable/ENH-004-pipeline-specialization.md`)
- **Files**: `src/shaders/fractal.wgsl`, `src/renderer/initialization.rs:161-163, 208`
- **Steps (minimum viable, if not deferring to ENH-004)**:
  1. Create two fragment entry points in fractal.wgsl: `fs_main_2d` (2D dispatch + palette only) and `fs_main_3d` (ray-march path), sharing helper functions in the same module.
  2. Build two render pipelines at init (same layout, different `entry_point`); select per frame by `params.fractal_type.is_3d()` (an `is_3d()` helper exists or is trivial — check `src/fractal/types.rs`).
- **Method**: Cuts register pressure/occupancy cost for the cheap 2D path. Pitfall: WGSL compiles the whole module regardless — the win comes from the per-entry-point dead-code elimination naga/backends perform; measure before claiming perf wins. Full per-fractal specialization = ENH-004; don't half-do it here.
- **Verify**: `make checkall`; render one 2D and one 3D fractal (screenshots); `make web-build`.

### [ARC-012] Cheap accumulation clear
- **Files**: `src/renderer/compute.rs:231-270` (`AccumulationTexture::clear` and the Buddhabrot equivalent at :393), trigger at `src/app/render.rs:77-108`
- **Steps**:
  1. Replace the `vec![0u8; size]` + staging-buffer upload + dedicated encoder with `encoder.clear_texture(&self.texture, &wgpu::ImageSubresourceRange::default())` using the FRAME's existing encoder (thread `&mut CommandEncoder` into `clear`), guarded by the `CLEAR_TEXTURE` feature — check `initialization.rs` device features; wgpu's `clear_texture` requires `Features::CLEAR_TEXTURE` (widely supported; request it at device creation with a fallback).
  2. Fallback path (if feature unavailable): keep a persistent zero buffer allocated once at texture size, reused for every clear.
  3. For the Buddhabrot storage BUFFER (`BuddhabrotAccumulationBuffer::clear`, :393): use `encoder.clear_buffer(&self.buffer, 0, None)` — always available, no feature needed.
- **Method**: Removes per-frame multi-MB allocation + upload during zoom/pan in accumulation mode. Pitfall: ordering — the clear must be encoded BEFORE this frame's compute dispatch in the same encoder; verify the dispatch happens after the (new) clear encoding point.
- **Verify**: `make checkall`; runtime: attractor mode, continuously zoom — accumulation restarts cleanly each view change, no ghosting; memory profile flat (Activity Monitor).

### [ARC-014] Deduplicate native/web app layer via the platform traits
- **Files**: `src/app/mod.rs:69-213` (`App::new`) vs `:217-345` (`new_async`), `src/main.rs`, `src/web_main.rs`; longer-term `capture.rs`/`capture_web.rs`
- **Steps** (scoped to the constructors, the highest-value dedup):
  1. Diff the two constructors (`App::new` vs `new_async`). Extract the shared body into `async fn App::init_common(window, settings, ...) -> Self`; keep thin `new` (native: `pollster::block_on`) and `new_async` (web) wrappers around it.
  2. While merging, reconcile the KNOWN drift: web currently skips camera-settings load and UI-state restore — after dedup, web gets them too (that's the bug being fixed; note it in the commit).
  3. `capture.rs` vs `capture_web.rs` dedup is larger; do NOT attempt in this pass — file-level TODO comment referencing AUDIT ARC-014.
- **Method**: The `platform::` traits already abstract storage/dialogs; the constructors forked for async reasons only. Pitfall: wasm has no `pollster` — the shared body must be `async` and the native wrapper blocks on it; keep `cfg` at the wrapper level only. Verify web feature compiles: `make web-build`.
- **Verify**: `make checkall && make web-build`; native run restores camera + UI state as before; if a web test harness exists (`make web-serve`), spot-check startup.

### [ARC-015] Split `FractalParams` God object — LAST of the Phase 3b items touching `fractal/mod.rs`
- **Files**: `src/fractal/mod.rs:20-156`, `src/fractal/settings.rs`, `src/ui/history.rs`, all `params.` consumers
- **Steps**:
  1. Group the ~100 fields into three structs INSIDE `FractalParams` (composition, not scattering — keeps most call sites working via one extra path segment):
     - `pub settings: RenderSettings` — everything serialized/undoable (fractal type, iterations, colors, camera prefs…)
     - `pub lod: LodRuntime` — `lod_config` + `lod_state` (FPS ring, transitions)
     - `pub accum: AccumulationState` — `attractor_pending_clear`, `attractor_last_*`, `attractor_total_iterations`
  2. Mechanical field-path migration (`params.max_iterations` → `params.settings.max_iterations` etc.). Use par-mem `get_impact(FractalParams)` / `list_symbols` per consumer file to enumerate; expect ~300 sites across ui/, app/, renderer/, fractal/.
  3. Undo history (`ui/history.rs`): store `RenderSettings` clones only — undo no longer restores FPS buffers or accumulation bookkeeping (that's the fix).
  4. `to_settings()`/`from_settings` become near field-for-field mirrors of `RenderSettings`.
  5. Consider `#[serde(flatten)]` on the settings struct if keeping the YAML schema identical matters (it does — user files must keep loading; test with an existing settings.yaml).
- **Method**: Fixes undo restoring transient state and clarifies ownership. HIGH-CHURN mechanical change. Pitfalls: (a) YAML backward compatibility is the hard requirement — `#[serde(flatten)]` or a manual `Settings` mapping preserves the schema; add a roundtrip test FIRST; (b) coordinate: run this after ARC-008/ARC-007/SEC-001 have landed (they all edit `fractal/mod.rs`); (c) don't combine with any other issue in one commit.
- **Verify**: `make checkall`; load a pre-change settings.yaml and a saved preset — both must parse identically (write the roundtrip test before refactoring); undo/redo works; a session's settings.yaml diff shows no schema change.

### [ARC-016] Fix `compute.rs` module docs; scope the dead_code allow
- **Files**: `src/renderer/compute.rs:16-22` (module docs + `#![allow(dead_code)]`)
- **Steps**:
  1. Rewrite the module header: the compute passes ARE integrated (dispatched from `app/render.rs` each accumulation frame for attractor/Buddhabrot modes).
  2. Remove the blanket `#![allow(dead_code)]`; run `cargo clippy` — for each newly flagged item, either delete it (if genuinely dead, e.g. `AttractorComputeUniforms.param_c/param_d` if always 0.0 — check whether the WGSL reads them: `grep -n 'param_c\|param_d' src/shaders/*.wgsl`; if the shader reads them they are NOT dead, they're reserved — keep with a comment) or add a scoped `#[allow(dead_code)]` with a reason.
- **Method**: Docs drift + blanket-allow hiding real dead code. Pitfall: uniform struct fields that pad GPU layout are load-bearing even if "unread" — never delete fields from `#[repr(C)]` GPU structs without adjusting BOTH sides and the size assert (see ARC-010).
- **Verify**: `make checkall` (clippy clean without the blanket allow).

### [ARC-017] Split animated vs static uniform writes
- **Files**: `src/renderer/update.rs:191-234`
- **Steps**: After ARC-006 lands, most frames skip rendering entirely, which removes most of the waste — so implement the cheap version only: keep one `Uniforms` rebuild per RENDERED frame (unchanged), but move the bloom/composite uniform writes behind change detection (compare cached copies; write only when changed). Do not build a full field-level dirty system (over-engineering for 864 bytes).
- **Method**: `queue.write_buffer` of <1KB is cheap; this is a tidiness fix. Pitfall: the `time` field changes when animation is active — that's fine post-ARC-006 since frames only render when something changed.
- **Verify**: `make checkall`; bloom toggle + color-grade changes still take effect immediately.

### [ARC-018] Non-blocking GPU enumeration
- **Files**: `src/app/render.rs:656` (`pollster::block_on(enumerate_gpus)`)
- **Steps**: Replace the inline block_on with a spawned thread + channel: on button press, `std::thread::spawn` the enumeration (native; `cfg` — on wasm keep current behavior or use wasm_bindgen_futures::spawn_local), store `gpu_scan_receiver: Option<Receiver<Vec<GpuInfo>>>` on `App`/UI state; poll `try_recv()` each frame; show "Scanning…" in the UI meanwhile.
- **Method**: User-initiated, but freezes the frame loop for potentially hundreds of ms. Pitfall: `wgpu::Instance` is `Send` on native — enumerate via a fresh `Instance` in the thread rather than sharing the renderer's.
- **Verify**: `make checkall`; click "scan GPUs" in settings — UI stays responsive, list populates.

### [ARC-019] `VecDeque` undo history
- **Files**: `src/ui/history.rs:36-38`
- **Steps**: Change the container `Vec<FractalParams>` → `VecDeque<FractalParams>`; `remove(0)` → `pop_front()`; `push` → `push_back()`; index accesses adapt (`VecDeque` supports indexing).
- **Verify**: `make checkall`; undo/redo works across >capacity entries.

### [ARC-020] Makefile bundle version + `typecheck` target
- **Files**: `Makefile`
- **Steps**:
  1. Locate the macOS `bundle` target hardcoding `CFBundleVersion 0.7.0`. Add at the top of the Makefile: `VERSION := $(shell grep -m1 '^version' Cargo.toml | cut -d'"' -f2)` and use `$(VERSION)` in the bundle plist substitutions (both `CFBundleVersion` and `CFBundleShortVersionString` if present).
  2. Add `typecheck: check` (alias per house convention) and add `typecheck` to `.PHONY`.
- **Verify**: `make typecheck` runs cargo check; `make -n bundle | grep -i version` shows 0.8.3 (dry-run; don't actually build the bundle).

### [ARC-021] (≙ DOC-012 partial) CLAUDE.md invariants — fold into DOC-012's single CLAUDE.md pass
- See DOC-012 below. Do not edit CLAUDE.md separately here.

---

## Phase 3c — Code Quality (remaining; skip QA-001/002/004/005 [Phase 2 bundle], QA-007 [SEC-004], QA-010 [ARC-008], QA-018 [ARC-010])

### [QA-003] Clamp `acos` input in LOD motion tracking
- **Files**: `src/lod.rs:370`
- **Steps**: `self.prev_camera_forward.dot(camera_forward).acos()` → `self.prev_camera_forward.dot(camera_forward).clamp(-1.0, 1.0).acos()`.
- **Method**: dot of normalized vectors exceeds ±1.0 by ulps; `acos` of that is NaN; the EMA at :386-387 then never recovers. Pitfall: none — one-line fix; but ADD the regression test: `update_motion` twice with identical camera state must leave `camera_velocity` finite (see QA-019).
- **Verify**: `make checkall`; new test green.

### [QA-006] Bloom gating — done as ARC-005. Verify only: `grep -n 'if true' src/app/render.rs` returns nothing.

### [QA-008] Render decomposition — done as ARC-004. Verify only.

### [QA-009] Split `ui/mod.rs` per panel — LAST change to `ui/mod.rs` in the whole plan
- **Files**: `src/ui/mod.rs` (3,287 lines), new files `src/ui/panels/*.rs`
- **Steps**:
  1. Preconditions: ARC-003 (UiActions) and SEC-006 merged; no other pending `ui/mod.rs` work.
  2. Create `src/ui/panels/` with one module per panel, following the existing `ui/command.rs`/`ui/overlays.rs` pattern: `rendering.rs` (fractal/quality settings), `lighting.rs`, `palette.rs`, `capture.rs` (screenshot/video/resolution), `lod.rs`, `presets.rs`, `gpu.rs`. Each exposes `pub fn show(ui: &mut egui::Ui, params: &mut FractalParams, actions: &mut UiActions, state: &mut UIState)` — match the actual data each panel needs by reading its section of `UI::render`.
  3. Move code section-by-section (pure moves; egui code is naturally sectioned by `CollapsingHeader`/`Window` blocks). Compile after each move.
  4. `UI::render` becomes ~50 lines of panel calls. Keep helpers used by multiple panels in `ui/mod.rs` or a `ui/util.rs`.
- **Method**: Mechanical redistribution; CC 288 comes from egui's inline closures — moving them out is safe. Pitfalls: (a) borrow-checker: panels taking `&mut params` AND `&mut actions` is fine (disjoint), but shared `&mut self.something` across panels needs the state threaded explicitly; (b) do NOT change any widget logic while moving; (c) egui `Id` collisions — widgets get IDs from position/label; moving code between containers can change IDs and reset collapse-state; acceptable, but don't "fix" it mid-move.
- **Verify**: `make checkall`; runtime click-through of every panel; `wc -l src/ui/mod.rs` < 800.

### [QA-011] Dirty-flag redraw — done as ARC-006. Verify only.

### [QA-012] GPU-index fallback
- **Files**: `src/renderer/initialization.rs:61`
- **Steps**: Replace `adapters.into_iter().nth(gpu_index).unwrap()` with:
  ```rust
  let count = adapters.len();
  let adapter = adapters.into_iter().nth(gpu_index).unwrap_or_else(|| {
      log::warn!("Saved GPU index {gpu_index} out of range ({count} adapters); using default");
      /* re-enumerate or keep a clone of the first adapter — read the surrounding
         code: if adapters was consumed, restructure to `let mut adapters = ...;
         if gpu_index < adapters.len() { adapters.swap_remove(gpu_index) } else { adapters.swap_remove(0) }` */
  });
  ```
  Simplest correct form: bounds-check BEFORE consuming: `let idx = if gpu_index < adapters.len() { gpu_index } else { log::warn!(...); 0 };`
- **Method**: Saved `preferred_gpu_index` outlives hardware changes. Pitfall: `adapters` may be an iterator not a Vec at that point — collect first if needed; ensure the fallback picks the same "default" the no-preference path picks (read `new_with_gpu_preference`, initialization.rs:44/98).
- **Verify**: `make checkall`; run with a settings.yaml containing `preferred_gpu_index: 99` — app starts on default GPU with a warning instead of panicking.

### [QA-013] `zoom_2d` f32→f64 — done inside ARC-001. Verify only: `grep -n 'zoom_2d: f32' src/` returns nothing.

### [QA-014] Per-fractal bounding radii
- **Files**: `src/shaders/fractal.wgsl:2240-2246`
- **Steps**:
  1. Replace the formula `50.0 * fractal_scale * max(1.0, fold) * max(1.0, iterations*0.1)` with a per-type conservative radius:
     ```wgsl
     fn bounding_radius(fractal_type: u32, fractal_scale: f32) -> f32 {
         // Conservative per-family bounds (world units, pre-scale):
         // Mandelbulb ~1.5, Menger ~1.8, Sierpinski ~2.0, Mandelbox ~ (4·|scale|),
         // Julia3D ~2.0, IFS types ~2.5, Kleinian ~4.0, attractors: no bound (return 1e9).
         var r = 2.5; // safe default
         switch fractal_type { /* per-type values */ default: { r = 2.5; } }
         return r * max(fractal_scale, 1.0) * 1.5; // 1.5 safety margin
     }
     ```
  2. Mandelbox needs `4.0 * abs(mandelbox_scale)` (its attractor can extend that far); Lorenz/Rossler attractors are unbounded in principle — give them a large radius (their DE path may not use the sphere anyway — check).
  3. Verify each radius empirically: render each 3D fractal at default params from a pulled-back camera; the fractal must never clip against the bounding sphere (visible as a hard spherical cutoff).
- **Method**: Iteration count has no relation to spatial extent; the current radius (~3,250) means the camera is always inside → skip never fires. Pitfall: TOO TIGHT a radius visibly clips geometry — err generous (1.5× margin); the win is skipping empty space when the camera is far, not a tight fit.
- **Verify**: `make checkall`; screenshot every 3D fractal type (script a preset sweep with `--preset` + `--screenshot-delay`) — no spherical clipping; FPS from a distant camera improves.

### [QA-015] Pipeline specialization — scoped to ARC-009/ENH-004. No separate action.

### [QA-016] `smooth_iteration_count()` helper
- **Files**: `src/shaders/fractal.wgsl:460-464, 487-490, 515-518, 543-546, 588-590` + every 2D fractal function with the same epilogue (~17 sites; find with `grep -n 'log_zn\|log2(log2' src/shaders/fractal.wgsl`)
- **Steps**:
  1. Add once near the df helpers:
     ```wgsl
     fn smooth_iteration_count(iteration: u32, mag_sq: f32, max_iterations: u32) -> f32 {
         let log_zn = log2(mag_sq) / 2.0;
         let nu = log2(log_zn / log2(2.0)); // simplify only if all call sites use the same escape radius/power
         return (f32(iteration) + 1.0 - nu) / f32(max_iterations);
     }
     ```
     FIRST read 3–4 of the 17 sites: if they differ in escape radius or power (Multibrot variants), parameterize (`escape_log: f32`, `power: f32`) rather than assuming.
  2. Replace each epilogue with a call. Do the hp variants too if their epilogue is identical in f32 space (they compute from `hi` parts — check).
- **Method**: Pure dedup; behavior-identical refactor. Pitfall: subtle per-fractal differences (power-n fractals use `log(power)` not `log(2)`) — parameterize, don't normalize semantics. Compare before/after screenshots for 3 fractal types at the same coordinates — pixel-identical expected.
- **Verify**: `make checkall`; screenshot diff (Mandelbrot, Julia, Burning Ship) pre/post — identical.

### [QA-017] `#[repr(u32)]` enums + dedupe channel match
- **Files**: `src/fractal/types.rs` (`FractalType`, and the color/channel enums — locate with `grep -n 'enum' src/fractal/types.rs`), `src/renderer/uniforms.rs:273-519` (the matches at :307-348 and :427-456)
- **Steps**:
  1. For each enum crossing to the GPU (`FractalType`, `ColorMode`, `ChannelSource`…): add `#[repr(u32)]` and explicit discriminants MATCHING the current match tables exactly (transcribe from uniforms.rs:307-348 — e.g. `Mandelbrot2D = 0, Julia2D = 1, ...`). The existing shader integer IDs are the contract; the enum adopts them, never the reverse.
  2. Replace the 30-arm match with `params.fractal_type as u32`.
  3. Replace the triplicated 8-arm channel matches with one `fn channel_to_u32(c: ChannelSource) -> u32 { c as u32 }` (after step 1 discriminants) used three times.
  4. Add a regression test pinning a few discriminants: `assert_eq!(FractalType::Buddhabrot2D as u32, 25);` etc. (values from the OLD match — write the test against the old table BEFORE deleting it).
- **Method**: Removes the hand-sync drift risk (CC 97 → ~10). Pitfalls: (a) if `FractalType` derives serde by NAME (yaml stores strings), discriminants don't affect serialization — verify `settings.rs` uses string serde (it does, YAML shows names); (b) `shader_index()` in types.rs:260 may already encode a DIFFERENT mapping — reconcile: if `shader_index()` exists and differs from the uniforms.rs match, understand why before unifying (2D/3D index spaces may be separate); the test in step 4 protects the GPU contract.
- **Verify**: `make checkall`; discriminant tests green; render 4 fractal types spanning the enum (first, last, Buddhabrot, one 3D) — correct fractal appears for each.

### [QA-019] Numeric-core test suite
- **Files**: new tests in `src/renderer/uniforms.rs` (or `tests/`), `src/lod.rs`, `src/fractal/settings.rs`/`tests/integration_tests.rs`
- **Steps** — add these tests (each is small and pure):
  1. **DF split roundtrip** (uniforms.rs): for a sweep of f64 values (0.0, ±1.0, π, ±1e-15, ±0.1318259042, 1e10): split into (hi, lo) exactly as `Uniforms::update` does (uniforms.rs:288-296 — extract the split into a testable `pub(crate) fn split_f64(v: f64) -> (f32, f32)` if inline), assert `(hi as f64) + (lo as f64)` relative error < 1e-14 and `|lo| <= ulp(hi)/2`.
  2. **Zoom→iteration bonus**: for zoom 1.0 / 1e6 / 1e12, assert `zoom_bonus` matches `log2(zoom)*15` and total never overflows u32.
  3. **LOD NaN regression** (lod.rs tests): `update_motion` with bit-identical forward vectors twice → `camera_velocity.is_finite()`; with a forward vector whose dot is 1.0+ε (construct via normalize of near-parallel vectors) → finite.
  4. **Settings roundtrip**: `FractalParams::default()` → `to_settings()` → YAML string → parse → `from_settings` → compare key fields. Same for a `Preset`.
  5. **QualityLevel::lerp monotonicity** already exists — extend to the new `iteration_scale` (post-ARC-007).
- **Method**: Targets exactly the code where the audit found real bugs. Pitfall: the DF split test defines correctness the perturbation work (ENH-001) will build on — keep it strict.
- **Verify**: `cargo test` — all new tests green; `make checkall`.

### [QA-020] Remove dead code; scope allows
- **Files**: `src/shaders/fractal.wgsl:363-370` (`df_add`), `src/camera.rs:256` (`is_any_key_pressed`), 13 `#[allow(dead_code)]` sites (`grep -rn '#\[allow(dead_code)\]' src/`), `src/app/mod.rs:247` (TODO)
- **Steps**:
  1. Delete `df_add` (WGSL; `grep -c 'df_add\b' src/shaders/fractal.wgsl` must show only `df_add_full` remains in callers).
  2. Delete `is_any_key_pressed` (verify zero callers: `grep -rn 'is_any_key_pressed' src/`).
  3. For each `#[allow(dead_code)]`: remove the attribute, build, and either delete the flagged item or restore the attribute WITH a one-line reason comment. Do NOT delete: GPU-layout struct fields (see ARC-016 pitfall), trait-impl methods reported dead by static analysis but called dynamically, and platform-cfg'd items (build BOTH `make build` and `make web-build` before deciding anything web-side is dead).
  4. `src/app/mod.rs:247` TODO (web settings-load): after ARC-014's constructor merge this may be resolved; if ARC-014 hasn't landed, leave the TODO but reference AUDIT ARC-014.
- **Method**: par-mem's dead-code list was ~90% false positives for Rust trait dispatch — the grep verification per item is mandatory, not optional. Pitfall: web-only code looks dead to a native-only build.
- **Verify**: `make checkall && make web-build`.

### [QA-021] (≙ SEC-010) Standardize on `log::`
- **Files**: ~20 files; enumerate with `grep -rln 'println!\|eprintln!' src/`
- **Steps**:
  1. Mechanical replacement: `println!` → `log::info!`, `eprintln!` → `log::error!` (or `warn!` where the text implies a warning; `debug!` for chatty diagnostics like per-file save paths).
  2. Exceptions — keep as-is: `main.rs` CLI output that is the program's stdout contract (`--list-presets` output MUST stay `println!`), and anything printed before the logger initializes (check where `env_logger`/logger init happens in main.rs — output before it would vanish; either keep prints or move logger init earlier).
  3. Confirm the logger is initialized on native (env_logger or similar in main.rs) and wasm (console_log in web_main.rs) — both exist per the audit; verify.
- **Method**: `println!` is invisible on wasm and unfilterable. Pitfall: the `--list-presets`/`--help` stdout contract; grep main.rs's CLI handling and leave those.
- **Verify**: `make checkall`; `cargo run --release -- --list-presets` still prints the list; `RUST_LOG=info cargo run --release -- --exit-delay 3` shows logs.

### [QA-022] Explicit Buddhabrot counter wrap
- **Files**: `src/app/render.rs:128`
- **Steps**: Replace `total_iterations: self.fractal_params.attractor_total_iterations as u32` with an explicitly wrapped value: `(self.fractal_params.attractor_total_iterations % (1u64 << 32)) as u32` is identical behavior but silences nothing — instead, check WHAT the shader uses it for (`grep -n 'total_iterations' src/shaders/buddhabrot_compute.wgsl src/shaders/attractor_compute.wgsl`): it seeds RNG/sample sequencing. Correct fix: pass a value that stays useful — `(self.fractal_params.attractor_total_iterations & 0xFFFF_FFFF) as u32` with a comment `// deliberate wrap: shader uses this as an RNG stream offset; wrapping is benign`, OR pass a frame counter if the shader only needs per-frame variation.
- **Method**: Make the wrap explicit and documented, not accidental. Pitfall: if the shader derives PROGRESS (e.g. auto-pause threshold) from this u32, wrapping is NOT benign — read the shader first; the auto-pause logic lives CPU-side (`attractor_max_iterations` check in render.rs) per the audit, so GPU-side wrap should be benign; verify.
- **Verify**: `make checkall`; Buddhabrot accumulates normally past a few minutes (or lower `attractor_iterations_per_frame` × time to cross 2^32 in test).

### [QA-023] Inside-set sentinel
- **Files**: `src/shaders/fractal.wgsl:3030` (`if (t == 0.0)`) and the fractal functions returning `t`
- **Steps**: Change the "inside the set" sentinel from `0.0` to `-1.0`: the escape-time functions return `-1.0` when iteration reaches max without escape (find the `return 0.0`-on-no-escape sites; ~the same 17 functions as QA-016 — coordinate: do QA-016 FIRST, then this touches one helper + the return sites), and the consumer becomes `if (t < 0.0)`.
- **Method**: A legitimately computed smooth value of exactly 0.0 (first-iteration escape) currently colors as "inside". Pitfall: hp variants and the `smooth_iteration_count` helper (post QA-016) must agree; verify no OTHER consumer branches on `t == 0.0` (`grep -n 't == 0.0\|t <= 0.0' src/shaders/fractal.wgsl`).
- **Verify**: `make checkall`; screenshots of Mandelbrot interior (solid region) unchanged; zoom to a boundary — no stray interior-colored pixels on the set edge.

### [QA-024] `array::from_fn` palette copy
- **Files**: `src/app/render.rs:235-284`
- **Steps**: Replace the 50-line unrolled 8-element copy with `std::array::from_fn(|i| ...)` mapping the palette color at index i to the uniform format (read the existing unroll to capture the exact transform — likely `[f32; 4]` from rgb + pad).
- **Verify**: `make checkall`; palette rendering unchanged (screenshot).

### [QA-025] Deduplicate magic numbers
- **Files**: `src/lod.rs:187` and `src/renderer/uniforms.rs:255-257` (zone defaults `[10, 25, 50]`); `src/lod.rs:379-385` (EMA `0.3`, rotation weight `5.0`)
- **Steps**: In `lod.rs`, add `pub const DEFAULT_LOD_ZONES: [f32; 3] = [10.0, 25.0, 50.0];` (match actual types) and use it in both files (uniforms.rs imports it from the lod module). Name the EMA constants `const MOTION_EMA_ALPHA: f32 = 0.3; const ROTATION_WEIGHT: f32 = 5.0;` adjacent to `update_motion`.
- **Verify**: `make checkall`; grep shows the literals only at the const definitions.

### [QA-026] `camera_velocity: f32`
- **Files**: `src/lod.rs:386-391` and the `LODState` field declaration
- **Steps**: Change `camera_velocity: Vec3` to `f32`; update the EMA computation (currently writing `.x`) and every reader (`grep -n 'camera_velocity' src/`).
- **Verify**: `make checkall`; LOD debug overlay still shows sensible motion values.

### [QA-027] winit `ApplicationHandler` migration
- **Files**: `src/main.rs:256` (deprecated `event_loop.run`), `src/web_main.rs`, `src/app/mod.rs`
- **Steps**:
  1. Implement `winit::application::ApplicationHandler` for a wrapper (or for `App` itself): move the current `match event` arms into `window_event()`, `about_to_wait()`, `resumed()` (create the window in `resumed()` — the winit 0.30 contract), `device_event()` as applicable.
  2. `event_loop.run(...)` → `event_loop.run_app(&mut app_handler)`; remove the `#[allow(deprecated)]`.
  3. Web (`web_main.rs`): use `EventLoopExtWebSys::spawn_app` equivalent (check winit version's web API — `run_app` may work via `spawn_app`).
  4. Window creation moves from before-the-loop into `resumed()` — the renderer init (async) needs restructuring on native: `pollster::block_on` inside `resumed()` is the standard pattern; on web this is why `new_async` exists — coordinate with ARC-014's constructor merge (do ARC-014 first or together).
- **Method**: Deprecated API; winit will remove it. This is the largest low-priority item — treat as its own PR. Pitfall: `resumed()` can fire multiple times on some platforms (mobile); guard window/renderer creation with `if self.window.is_none()`.
- **Verify**: `make checkall && make web-build`; native run + resize + screenshot; web serves (`make web-serve` spot check).

### [QA-028] Guard `max_iterations == 0` underflow
- **Files**: `src/shaders/fractal.wgsl:456` and clones (`grep -n 'max_iterations - 1' src/shaders/fractal.wgsl`)
- **Steps**: Either clamp CPU-side (SEC-001 already sets floor 1 — verify) AND make the shader safe anyway: `let last_iter = max(uniforms.max_iterations, 1u) - 1u;` at each site.
- **Verify**: `make checkall`; set iterations slider to minimum — renders without artifacts.

### [QA-029] CLAUDE.md counts — folded into DOC-012. No separate action.

### [QA-030] WGSL `switch` readability
- **Files**: `src/shaders/fractal.wgsl` (`fs_main`, `scene_de_with_material` if/else chains)
- **Steps**: Convert the long `if/else if` fractal-type ladders to `switch uniforms.fractal_type { case 0u: {...} ... default: {...} }`. Pure readability; do AFTER QA-014/QA-016/QA-023 to avoid conflicting hunks.
- **Verify**: `make checkall`; screenshot smoke of 3 fractal types.

---

## Phase 3d — Documentation (all parallelizable; single writer for CLAUDE.md)

> Doc fixers: verify each claimed value against code before writing it — line refs below are the code truth sources. Follow `docs/DOCUMENTATION_STYLE_GUIDE.md`.

### [DOC-001] Fix README keybindings
- **Files**: `README.md` (Key Bindings tables + Command Palette section)
- **Steps**: Screenshot → **F12** (truth: `src/app/input.rs:209`); command palette → **/** or **Ctrl/Cmd+K** (truth: input.rs palette handling; plain P = cycle palettes); 3D vertical movement → **E** (up) / **Q** (down) (truth: `src/camera.rs:110-114`); DELETE the "Mouse Wheel adjusts 3D speed" row (see DOC-007). Cross-check every other row against `docs/CONTROLS.md`, which is verified-accurate — reconcile README TO CONTROLS.md wherever they disagree.
- **Verify**: Every binding in the edited tables appears identically in CONTROLS.md; `grep -n 'F9' README.md` shows F9 only as Kleinian selection (if listed).

### [DOC-002] MSRV 1.85+ everywhere; `rust-version` in Cargo.toml
- **Files**: `README.md:6` (badge), `:185`; `docs/QUICKSTART.md:21, 32, 54, 457`; `CLAUDE.md` (tech stack section); `Cargo.toml`
- **Steps**:
  1. `Cargo.toml` `[package]`: add `rust-version = "1.85"`.
  2. Replace every "1.70" and "Edition 2021" with "1.85+" / "Edition 2024" at the listed locations (`grep -rn '1\.70\|Edition 2021\|edition 2021' README.md docs/ CLAUDE.md`).
  3. README badge: update the shields.io rust badge text to 1.85+.
- **Verify**: `cargo check` (validates rust-version syntax); `grep -rn '1\.70' README.md docs/ CLAUDE.md` → empty.

### [DOC-003] Sync ARCHITECTURE.md
- **Files**: `docs/ARCHITECTURE.md` (lines ~853-857 LOD, ~399-403 compute, 294/595/931 counts, 436 wheel)
- **Steps**:
  1. LOD table → 325/250/175/100 max_steps with the full per-level values copied from `src/lod.rs:62-113` (read the four constructors; transcribe every field).
  2. Rewrite the `renderer/compute.rs` paragraph: fully integrated; `Renderer` holds `attractor_compute` + `buddhabrot_compute` pipelines dispatched per accumulation frame from `app/render.rs`; add `attractor_compute.wgsl`, `attractor_display.wgsl`, `buddhabrot_compute.wgsl`, `buddhabrot_copy.wgsl` to the shader inventory and the GPU Computation mermaid diagram.
  3. Fractal count 34→35; index 25 = Buddhabrot2D (truth: `src/renderer/uniforms.rs:307-348`).
  4. `LODState` field names: `prev_camera_pos` (not `last_camera_pos`); add `camera_velocity`; add `LODConfig.profile` (truth: `src/lod.rs:293+`).
  5. Palette count → 48 (DOC-008); remove wheel-speed at line 436 (DOC-007); render_scale wording per DOC-010.
- **Verify**: Each number cross-checked against the cited code line; mermaid renders (paste into a mermaid-aware previewer or `mmdc` if available).

### [DOC-004] Fix FRACTALS3D.md performance guidance
- **Files**: `docs/FRACTALS3D.md:1371-1396` (profiles), `:1346-1349` (epsilon), `:128-129, 1433` (wheel)
- **Steps**: Replace the Quality Profiles block with the accurate table from `docs/FEATURES.md:449-453` (which matches `src/lod.rs` — verify once more against code); state explicitly that LOD adjusts only the six numeric knobs and never toggles effects/shading models; epsilon default 0.001 → **0.00035** (truth: `src/fractal/mod.rs` default for `min_distance` — confirm with `grep -n 'min_distance' src/fractal/mod.rs | head -3`); delete both wheel-speed rows.
- **Verify**: Values match lod.rs; `grep -in 'wheel' docs/FRACTALS3D.md` shows no speed claims.

### [DOC-005] Document deep-zoom thresholds/limits user-facing
- **Files**: `docs/FRACTALS2D.md:81-104` (High-Precision section), `:1132-1144` (Deep Zoom Guidelines)
- **Steps**:
  1. Add to the High-Precision section: exact auto-enable rule (post-Phase-2 it is DERIVED from pixel spacing — document the new rule from `src/renderer/uniforms.rs` as edited by QA-004; if Phase 2 hasn't landed yet, document `zoom > 1e6` and mark this doc task blocked on the ARC-002 bundle); that the CENTER is f64→hi/lo double-float while zoom storage is f64 CPU-side and f32 on the GPU (post ARC-001) — the practical ceiling ~1e11 comes from the DF center + f32 GPU zoom.
  2. Add to Deep Zoom Guidelines: iterations auto-scale as `max_iterations + log2(zoom)×15` (uniforms.rs:300) — adjust the manual-iteration table so users don't double-count; note supported hp fractal list INCLUDING Tricorn post-QA-001 (verify the gate at time of writing).
- **Verify**: Every threshold/formula matches the CURRENT code (post-Phase-2 values, not the audit-time ones — this entry runs after Phase 2).

### [DOC-006] Refresh docs index
- **Files**: `docs/README.md`
- **Steps**: Version block → 0.8.3 / current date (or delete the hand-maintained block — prefer deletion with a note "see CHANGELOG.md", it's the drift-proof fix); 34/19 → 35/20 fractal counts; 784 → 864 bytes; 47 → 48 palettes; add `FEATURES.md` to the structure diagram, learning paths, and Quick Links table with a one-line description.
- **Verify**: `grep -n '784\|0\.6\.0\|34 fractal\|47' docs/README.md` → empty; FEATURES.md linked ≥2 places.

### [DOC-007] Remove wheel-speed claim everywhere
- **Files**: `README.md`, `docs/QUICKSTART.md:255, 326`, `docs/FRACTALS3D.md:128-129, 1433`, `docs/ARCHITECTURE.md:436`, `CLAUDE.md`
- **Steps**: `grep -rn -i 'wheel' README.md docs/ CLAUDE.md` — remove/replace every claim that the wheel changes 3D movement speed; point to the Camera panel's speed slider instead (CONTROLS.md:203/499 has the correct wording to copy). Note: the wheel DOES zoom in 2D — keep those mentions.
- **Verify**: The grep shows wheel mentions only for 2D zoom and CONTROLS.md's correct caveat.

### [DOC-008] Normalize palette count to 48
- **Files**: `docs/FRACTALS2D.md:70, 143` (says 54), `docs/README.md` (47), `docs/ARCHITECTURE.md:302` (46)
- **Steps**: All → 48 static palettes (truth: count entries in `src/fractal/palettes.rs` — verify with `grep -c 'Palette {' src/fractal/palettes.rs` or however entries are structured; FEATURES.md/QUICKSTART/About window already say 48). Add "(canonical list: FEATURES.md)" where the docs enumerate palettes.
- **Verify**: `grep -rn '54 \|47 \|46 ' docs/*.md | grep -i palette` → empty.

### [DOC-009] Fix CONTROLS.md Advanced Settings
- **Files**: `docs/CONTROLS.md:326-337`
- **Steps**: Read the actual settings panel in `src/ui/mod.rs` (post-QA-009 it may be `src/ui/panels/rendering.rs`); delete the rows for "Render Resolution multiplier", "Anti-Aliasing sample count", "Precision Mode Float vs Double" (none exist; precision is automatic); document what IS there.
- **Verify**: Every row in the rewritten subsection corresponds to a real widget (cite the source file in the commit message).

### [DOC-010] render_scale wording — AFTER the ARC-007/ENH-003 decision
- **Files**: `docs/FEATURES.md:450-453, 525`, `docs/ARCHITECTURE.md:866`
- **Steps**: If ENH-003 wired render scaling: document it as active with its mechanism. If ARC-007 hid the slider: remove render_scale from the quality tables or mark "reserved, not yet applied". Also FEATURES.md:525 "5-point color gradient" → 8-color palettes (truth: palette struct in `src/fractal/palettes.rs`).
- **Verify**: Matches the shipped state of `apply_lod_quality`/renderer at time of writing.

### [DOC-011] 35 fractal types in user docs
- **Files**: `docs/QUICKSTART.md:187, 511`, `docs/CONTROLS.md:360`
- **Steps**: 34 → 35 (20 2D + 15 3D); mention Buddhabrot2D where 2D types are enumerated.
- **Verify**: `grep -rn '34' docs/QUICKSTART.md docs/CONTROLS.md` → no fractal-count hits.

### [DOC-012] (≙ ARC-021, QA-029, parts of DOC-002/007) Single CLAUDE.md sync pass — AFTER ARC-010
- **Files**: `CLAUDE.md`
- **Steps** (one commit, all CLAUDE.md changes):
  1. Tech stack: "Rust 1.85+ (Edition 2024)".
  2. Uniform buffer: 784 → 864 bytes; mention the compile assert AND the new offset tests (ARC-010); if ENH-008 (encase) landed, rewrite the manual-padding guidance entirely.
  3. Fractal counts: 20 2D + 15 3D = 35 (list Buddhabrot2D and the attractors — enumerate from `src/fractal/types.rs`).
  4. CLI options: add `--quality`/`-q` (truth: `src/main.rs` clap definitions — transcribe all current flags).
  5. Remove the wheel-speed claim (DOC-007).
  6. Check remaining sections (module structure, commands) against reality while in the file — the audit found these accurate; verify no drift from remediation-phase changes (e.g. new `ui/panels/` from QA-009, `typecheck` target from ARC-020).
- **Verify**: Every stated number greps back to code; `make typecheck` exists if CLAUDE.md mentions it.

### [DOC-013] Add CONTRIBUTING.md
- **Files**: `CONTRIBUTING.md` (new, project root)
- **Steps**: Short file (~60 lines): prerequisites (Rust 1.85+, `make install-deps` for Linux), setup (`git clone`, `make build`), verification (`make checkall` before every PR — required), test guidance (add tests for numeric code; see `src/fractal/tests.rs` patterns), PR expectations (atomic commits, conventional-ish messages per git log style), the new-fractal checklist (from CLAUDE.md "Adding a New Fractal" — link, don't duplicate), doc-update rule (README + FEATURES + About window for user-visible changes), license note (MIT). Update README's Contributing section and docs/README.md to link it.
- **Verify**: Links resolve; instructions actually work (`make checkall` passes on a clean checkout — it does if CI is green).

### [DOC-014] Fix README anchor
- **Files**: `README.md:28`
- **Steps**: `docs/FEATURES.md#command-line-interface` → `docs/FEATURES.md#cli-options` (verify the heading: `grep -n '^#.*CLI' docs/FEATURES.md`; GitHub anchors are the heading lowercased, spaces→hyphens).
- **Verify**: Anchor matches the heading's generated slug exactly.

### [DOC-015] Rustdoc for public API
- **Files**: `src/lib.rs` (crate-level `//!` + 14 pub items), `src/camera.rs` (32 pub items), `src/renderer/mod.rs` (56), `src/app/mod.rs`, `src/video_recorder.rs` (13), `src/fractal/mod.rs`
- **Steps**:
  1. `src/lib.rs`: add a crate-level `//!` block (what par-fractal is, the module map, a pointer to docs/ARCHITECTURE.md).
  2. For each listed file: doc-comment every `pub` struct/enum/fn — one summary line minimum; parameters/invariants where non-obvious. Use `src/lod.rs` and `src/command_palette.rs` as the style standard (the audit called them exemplary).
  3. Priority order if time-boxed: lib.rs → camera.rs → renderer/mod.rs → fractal/mod.rs → the rest.
  4. Check the result: `cargo doc --no-deps --open` (or `make doc`) — no broken intra-doc links (`cargo doc` warns).
- **Method**: The crate publishes an rlib to crates.io; docs.rs is nearly empty. Pitfall: don't narrate ("returns the camera") — state contracts (units, coordinate conventions, panics). Enumerate pub items per file: `grep -n '^\s*pub ' <file>`.
- **Verify**: `cargo doc --no-deps 2>&1 | grep -c warning` → 0 (or only pre-existing); `make checkall`.

### [DOC-016] CHANGELOG hygiene
- **Files**: `CHANGELOG.md`
- **Steps**: Add a v0.8.1 entry (reconstruct from `git log v0.8.0..v0.8.1 --oneline`); note that 0.8.2 has no tag (either add the missing link refs pointing to the compare URLs that exist, or annotate); extend the link-reference block at the bottom through [0.8.3] following the existing pattern (`[0.8.3]: https://github.com/paulrobello/par-fractal/compare/v0.8.2...v0.8.3` — verify tag names with `git tag -l 'v0.8*'`; if v0.8.2 truly has no tag, link 0.8.3 against v0.8.1).
- **Verify**: Every version heading has a link ref; refs use real tags (`git tag -l`).

### [DOC-017] QUICKSTART style fixes
- **Files**: `docs/QUICKSTART.md`
- **Steps**: Remove `$ ` prefixes from copy-paste command blocks; convert blocks that are prose/instructions ("# Press the '2' key") from ```bash to ```text. Per `docs/DOCUMENTATION_STYLE_GUIDE.md`.
- **Verify**: `grep -n '^\$ ' docs/QUICKSTART.md` → empty.

### [DOC-018] Remove CONTROLS.md F9 note — AFTER DOC-001
- **Files**: `docs/CONTROLS.md:96`
- **Steps**: Delete the note explaining README's F9 error (README is now correct).
- **Verify**: `grep -n 'F9' docs/CONTROLS.md` shows F9 only as the Kleinian binding.

### [DOC-019] README What's New
- **Files**: `README.md`, optionally `CLAUDE.md`
- **Steps**: Choose the lighter fix: amend the CLAUDE.md rule to say the About page tracks **CHANGELOG.md** (which is what actually happens and is current) instead of "README what's new". Only add a README What's New section if the user prefers it — default to the rule amendment (surface the choice in the fix report).
- **Verify**: Rule text matches practice.

### [DOC-020] Quick-switch keys + Mermaid classDef
- **Files**: `README.md:248`, mermaid blocks in `docs/README.md`, `docs/CONTROLS.md`, `docs/QUICKSTART.md`
- **Steps**: README: "1-4" → "1-0 select 2D fractals; F1-F10 select 3D fractals" (truth: `src/app/input.rs` number/function-key handlers — verify the exact ranges). Mermaid: convert per-node `style` lines to `classDef` + `class` per the style guide (mechanical; keep the same colors — see the dark-mode palette in the style guide/user prefs).
- **Verify**: Key ranges match input.rs; diagrams render.

---

## Post-phase verification (Phase 4)

1. `make checkall` — must be fully green.
2. `make web-build` — wasm target compiles.
3. Visual sweep: `cargo run --release -- --screenshot-delay 4 --exit-delay 6` at: default view; Mandelbrot zoom 1e5, 1e8 (via presets); Tricorn 1e6; Burning Ship 1e8; one 3D fractal (Mandelbulb); attractor mode. Compare against the pre-remediation screenshots taken in the ARC-002 bundle.
4. Settings compatibility: a pre-remediation `settings.yaml` and preset file load cleanly.
5. `cargo audit` green (post SEC-003).
6. Confirm no commits were pushed (pushes require explicit user confirmation).
