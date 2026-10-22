# Audit Remediation Plan — par-fractal (2026-09-28)

Per-issue execution playbook for `/fix-audit`. Entry order matches the `## Remediation Plan`
phases in `AUDIT.md`. Each entry is complete enough to execute without re-deriving the analysis.
Every agent: read the target files fresh before editing (other agents touch overlapping files —
see AUDIT.md's File Conflict Map); run the entry's Verify commands and `make checkall` before
reporting done; commit atomically per issue.

Global gates: `make checkall` (fmt + clippy + tests). For wasm-touching issues after ARC-021:
`cargo clippy --lib --target wasm32-unknown-unknown --no-default-features --features web -- -D warnings`.

---

## Phase 1 — Trust-boundary security (sequential, first)

### [SEC-010] Close the `center_2d_precise` trust-boundary bypass
- **Files**: `src/deep_zoom/orbit.rs` (`parse_center_decimal`, ~line 154), `src/fractal/mod.rs:332-335` (`from_settings` pass-through), `src/fractal/mod.rs:223-230` (doc comment), `src/deep_zoom/driver.rs:233-276,314-320` (worker + retry), `src/ui/panels/rendering.rs:36-37` (UI-thread paste sink), `src/ui/panels/presets.rs:307-317` (SEC-006 import warning)
- **Steps**:
  1. In `parse_center_decimal`, before any dashu call: reject `s.len() > 4096`; if `s` contains `e`/`E`, require the exponent digits (after optional `+`/`-`) to be all-ASCII-digits and at most 4 characters; after parsing, reject `|magnitude| > 1e15`. Return descriptive `Err` strings.
  2. In `from_settings`, validate each element of `center_2d_precise` through `parse_center_decimal`; on failure set the whole pair to `None`.
  3. In `deep_zoom/driver.rs`, change the worker `Disconnected` handler: add a consecutive-failure counter with exponential backoff (e.g. skip retry for `min(2^n, 60)` frames) or a cap (e.g. 5 consecutive failures → disable perturbation for the session and log once). Reset the counter on success.
  4. In `ui/panels/presets.rs`, add `center_2d_precise` to the SEC-006 clamp-warning field comparison.
  5. Update the `from_settings` doc comment (`mod.rs:223-230`) — it currently claims hostile presets cannot smuggle bad values past this constructor, which SEC-010/011 disprove; reword to describe the actual validation.
- **Method**: The cost is spent *during* dashu parsing/conversion, so only a lexical pre-check before `DBig::from_str` works. The single choke point in `parse_center_decimal` covers the driver, the UI paste path and `from_settings`. Do not add validation at individual call sites — `get_symbol_context` on `parse_center_decimal` shows the sink set; keep it to one function.
- **Verify**: `cargo test` (add tests: 200k-digit mantissa accepted; `1e1000000000` rejected; `1e9223372036854775807` rejected; `1e15` magnitude boundary). Manually: `par-fractal --preset` with a crafted settings file exits promptly; pasting `1e1000000000` into Center shows an inline error, no freeze.

### [SEC-011] Clamp LOD `iteration_scale` at the trust boundary
- **Files**: `src/fractal/mod.rs:460` (`from_settings`), `src/lod.rs` (`QualityLevel`), `src/renderer/uniforms.rs:45,449-450` (`effective_2d_max_iterations`), `src/ui/panels/presets.rs:307-317`
- **Steps**:
  1. Add `QualityLevel::sanitized(self) -> Self` in `src/lod.rs`: finite-only clamps — `iteration_scale` ∈ `[0.0, 1.0]`, `render_scale` ∈ `[0.25, 1.0]`, integer counts clamped to the same ceilings SEC-001 uses for their global counterparts (`max_iterations` ≤ 100,000 etc., match the `from_settings` constants).
  2. In `from_settings`, apply `.sanitized()` to every `quality_presets[*]` of the incoming `lod_config` (and to the default `LODConfig`, cheap and harmless).
  3. In `uniforms.rs`, clamp the result of `effective_2d_max_iterations` to `perturbation_max_iterations(params)` (the existing orbit-budget helper) so the shader and orbit budgets stay identical even if a future code path forgets step 2.
  4. Add the LOD fields to the SEC-006 import clamp-warning in `presets.rs`.
- **Method**: The float→`u32` cast saturates, so `1e30 * iterations` becomes `u32::MAX`. The two layers are deliberate: `from_settings` fixes the boundary, the `uniforms.rs` clamp is defense-in-depth on the one place the value reaches the GPU. `min_quality_level` was verified safe (`set_target` applies `.min(3)` before any index) — do not add redundant checks there.
- **Verify**: `cargo test` with a new test: settings carrying `iteration_scale: 1e30`, negative values, NaN load with every quality-preset field inside its clamp; assert `effective_2d_max_iterations(...) <= perturbation_max_iterations(...)` for all quality levels.

---

## Phase 2 — Critical/blocking architecture (sequential)

### [ARC-021] Restore the web/WASM build; add a `web-check` gate
- **Files**: `src/web_main.rs:354`, `Makefile:213` + target list, `.pre-commit-config.yaml`
- **Steps**:
  1. Run `cargo check --lib --target wasm32-unknown-unknown --no-default-features --features web` and fix the reported error: `App::new_async` at `web_main.rs:354` passes 6 args, needs 7 — add the missing `profile_dump_path: None`.
  2. Fix the 13 wasm warnings (unused imports/variables) until clippy is clean.
  3. Add to `Makefile`: `web-check: cargo clippy --lib --target wasm32-unknown-unknown --no-default-features --features web -- -D warnings` (target needs the wasm target installed; note in the target's comment).
  4. Add `web-check` as a prerequisite of `checkall`; add the same command to `.pre-commit-config.yaml`.
- **Method**: Do not restructure the `App::new_async` signature here (that is ARC-032's `AppOptions`, a later phase) — a one-arg fix keeps this issue atomic. The local gate, not CI, is the enforcement point (CI web-build stays dispatch-only by design).
- **Verify**: the wasm clippy command exits 0; `make checkall` runs and passes including the web step.

### [ARC-024] Binary uses the library crate
- **Files**: `src/main.rs:1-20`, `src/lib.rs:39-72`, `Cargo.toml:33-34`
- **Steps**:
  1. In `main.rs`, replace the `mod` declarations with `use par_fractal::…` imports for everything `main` uses. Move `fn main`'s helpers it needs from private modules into `pub` in `lib.rs` (at minimum `clear_settings_files`; check what else fails to resolve).
  2. Delete `#[allow(dead_code)] mod platform` from `main.rs` — after the switch, the binary's copy is gone.
  3. `imgdiff` already uses the lib; confirm it still builds (`cargo build --bin imgdiff`).
  4. Decide on `crate-type`: keep `cdylib` for wasm but note that it links a native dylib on every build; if straightforward, gate it (`[lib] crate-type = ["rlib"]` + a wasm-only `cdylib` via cfg is not possible in one manifest — simplest is leaving it and noting the cost, or moving the cdylib to a workspace member; prefer the simple leave-in-place unless trivial).
  5. Run clippy; remove any `#[allow(dead_code)]` in `src/` that exists only because of the duplicate tree (grep for `dead_code` — those with a QA-020 reason stay).
- **Method**: This must land before QA-046/QA-047/any dead-code sweep because it changes which symbols clippy flags. parsight's zero-caller reports for `App::render/update/resize` resolve themselves once the binary calls through the lib.
- **Verify**: `cargo build` completes; `cargo clippy --all-targets` no longer flags the duplicate-tree dead code; `make checkall` green. Release build binary works: `make build-release && ./target/release/par-fractal --list-presets`.

### [ARC-026] Derive `RenderSettings` serde; unify parameter defaults
- **Files**: `src/fractal/state.rs:51-290`, `src/fractal/settings.rs:24-205`, `src/fractal/mod.rs:135-473`, `src/renderer/uniforms.rs:127-593`, `src/shaders/fractal.wgsl:6`, `src/fractal/tests.rs` (`tests_default_settings.yaml` gate)
- **Steps**:
  1. Add `#[derive(Serialize, Deserialize)]` + `#[serde(default)]` on `RenderSettings`. Give every field the code-side default (10,000 iterations/frame; log scale 4.0 — i.e. make the *serde* defaults match `RenderSettings::default`, not vice versa; pick the `Default` impl's values as canonical).
  2. In `Settings`, replace the flat duplicated fields with `#[serde(flatten)] pub render: RenderSettings` so existing flat YAML keeps loading. Keep the derived `palette`/`render_mode` fields with `#[serde(skip)]`.
  3. Move the trust-boundary clamps from `FractalParams::from_settings` into `RenderSettings::sanitize(&mut self)`; `from_settings` calls it. Keep the NaN-safe helper functions.
  4. Build `Uniforms::new()` from `FractalParams::default()` so lighting/bloom/iteration defaults come from one place; keep any GPU-only defaults (indices, zeroed textures) local.
  5. Keep the `tests_default_settings.yaml` round-trip test green — it is the compatibility gate for existing user files.
- **Method**: SEC-010/011 (Phase 1) already touched `from_settings`; do this after them so the clamp move doesn't conflict. SEC re-review: the clamps must still run on every load path (fresh default, file load, preset import) — verify `from_settings` is still the single entry. `to_settings` shrinks to serialization + derived fields; delete the ~330 hand-copied lines.
- **Verify**: `cargo test` (round-trip: serialize default → deserialize → equals default; a YAML missing every optional key loads equal to a fresh start; clamps still reject NaN/Inf/out-of-range). `make checkall`.

### [ARC-023] Extract `renderer::PostChain`; privatize Renderer fields
- **Files**: `src/renderer/mod.rs:52-184` (new `PostChain`), `src/app/render.rs:643-848`, `src/app/capture.rs:185-564`, `src/app/capture_web.rs:131-490`, `src/renderer/initialization.rs`, `src/renderer/update.rs`
- **Steps**:
  1. Create `renderer::PostChain` owning: bloom-extract target, blur targets, composite target, the four pipelines and all bind groups. API: `PostChain::new(device, format, size)` and `encode(&self, encoder, scene_view, output_view, opts: PostOptions)` where `PostOptions { bloom: bool, fxaa: bool, composite: bool }`.
  2. Interactive path (`render.rs`): replace the inline pass encoding with `Renderer`'s surface-sized `PostChain.encode(...)`; pass `composite: false` in accumulation mode (preserving current on-screen behavior at `render.rs:783`).
  3. Native capture (`capture.rs`) and web capture (`capture_web.rs`): create a temporary `PostChain` at the capture size and encode once. Fix the divergences while moving: pass `fxaa: true` (exports currently always use `copy_pipeline`), use half-resolution bloom targets matching the live path (fixes exported-bloom width), pass `composite` matching the accumulation rule (QA-042's predicate can supply it once it lands; until then reuse the existing condition).
  4. Make the `Renderer` fields private; expose `encode_scene`/`post_chain()` accessors as needed. Update `App` to stop encoding passes directly.
  5. Delete the now-dead inline pass code in all three call sites.
- **Method**: This unblocks ARC-022 (the blur fix lands inside PostChain once, not three times), ARC-035 and most QA/SEC work on the three app files — so it goes late in Phase 2 but before Phase 3b/3c agents touch those files. Preserve current *on-screen* behavior exactly; exports change deliberately (FXAA + correct bloom width) — call that out in the commit message.
- **Verify**: `make checkall`; visual: `par-fractal --screenshot-delay 2 --exit-delay 3` with bloom+FXAA on, then `--quality high` hi-res capture — exported image shows FXAA and bloom width matching the screen; accumulation-mode export matches its on-screen composite behavior.

---

## Phase 3a — Security (remaining)

### [SEC-012] Palette gallery `sanitize_name`
- **Files**: `src/fractal/palettes.rs:968,987,1002`, `src/fractal/presets.rs:23` (`sanitize_name`), its doc reference
- **Steps**: move `sanitize_name` to a shared location (e.g. `pub(crate) fn` in `presets.rs` re-exported, or a small `src/fractal/naming.rs`); call it at the top of `save`, `load` and `delete` in `CustomPaletteGallery`; fix the stale doc reference (UI rule now lives in `ui/panels/presets.rs:150` and `ui/panels/camera.rs:167`). Add traversal tests mirroring `presets.rs:1118-1129`.
- **Verify**: `cargo test` (new traversal tests: `../evil`, absolute paths, empty → rejected or sanitized); `make checkall`.

### [SEC-013] Preset camera fields applied raw
- **Files**: `src/app/render.rs:1092-1098`, `src/app/mod.rs:480-481`
- **Steps**: read `fovy`/`camera_speed` from `self.fractal_params.settings` (the clamped params) instead of `preset.settings`; add a finiteness check on `camera_position`/`camera_target` from presets — non-finite → fall back to `FractalType::default_camera_distance` framing.
- **Verify**: `cargo test` (preset with NaN position loads to the default framing); `make checkall`.

### [SEC-015] Dependency hygiene
- **Files**: `.cargo/audit.toml:14-21`
- **Steps**: remove the stale `RUSTSEC-2026-0194`/`2026-0195` ignores (lockfile has quick-xml 0.41.0, the fixed version); keep the ttf-parser ignore; update the outdated wayland comment. Do **not** migrate YAML crates in this pass — file a note on the serde_yaml evaluation (any replacement must pass `tests_default_settings.yaml`; `serde_yml` is banned, RUSTSEC-2025-0068).
- **Verify**: `cargo audit` reports no findings with the pruned ignores.

### [SEC-016] Release pipeline hardening
- **Files**: `.github/workflows/release.yml:10-12,185-186`, `publish-crates.yml:67`, `publish-homebrew-cask-core.yml:113`, `ci.yml`, `deploy-web.yml`
- **Steps**: top-level `permissions: contents: read`; grant `contents: write`/`id-token: write` only to the publishing jobs; drop `--token` from `cargo publish` (env `CARGO_REGISTRY_TOKEN` already set) and remove the `|| echo` + `continue-on-error` that hides publish failures; replace the tap-token clone URL with `gh auth setup-git` (or `http.extraheader`); SHA-pin `dtolnay/rust-toolchain` (resolve the current stable commit SHA first — verify the ref resolves before committing); add a `SHA256SUMS` step to the release job.
- **Verify**: `actionlint` or workflow YAML parse; next `workflow_dispatch` dry-run (do not trigger a release without the owner's go-ahead — surface this).

### [SEC-017] Sign + notarize macOS; drop xattr instructions
- **Files**: `.github/workflows/release.yml:91-133`, `docs/QUICKSTART.md:475-496`
- **Steps**: in the macOS job, add `codesign --force --options runtime --sign "Developer ID Application: PAUL AARON ROBELLO (QMLVG482FY)"` on the .app, then `xcrun notarytool submit` + `xcrun stapler staple` (secrets `APPLE_ID`/`APPLE_APP_SPECIFIC_PASSWORD`/`APPLE_TEAM_ID` must be configured — flag if absent, don't invent them). Replace the QUICKSTART xattr section with: downloads are signed and notarized; no xattr needed.
- **Verify**: workflow YAML parses; locally, `codesign -dv` on a signed build shows the Developer ID (local dry run optional); docs no longer instruct quarantine stripping.

### [SEC-019] `#![forbid(unsafe_code)]`
- **Files**: `src/lib.rs`, `src/main.rs` (top of each)
- **Steps**: add the attribute to both crate roots. If anything transitively requires unsafe, use targeted `#[allow(unsafe_code)]` with a comment (none should — zero `unsafe` exists today).
- **Verify**: `cargo check --all-targets` and the wasm target both compile.

### [SEC-020] gitleaks ignore entry
- **Files**: new `.gitleaksignore` at repo root
- **Steps**: add the fingerprint `5f382c5f...:docs/DOCUMENTATION_STYLE_GUIDE.md:curl-auth-header:568` (get the full fingerprint by running gitleaks once on the repo; copy it exactly).
- **Verify**: `gitleaks detect --redact` reports clean.

---

## Phase 3b — Architecture (remaining)

### [ARC-022] Bloom blur direction uniforms
- **Files**: `src/renderer/initialization.rs:458-464,929-946`, blur writes in `src/app/render.rs:714-718`, `src/app/capture.rs` (the PostChain after ARC-023), `src/app/capture_web.rs`
- **Steps**: if ARC-023 landed, do this inside `PostChain` only. Create `blur_h_uniform_buffer` (dirs `(1,0)`) and `blur_v_uniform_buffer` (`(0,1)`), written once at init; bind each to its existing bind group; delete every per-frame blur `write_buffer`. Add a bloom-enabled golden image to `tests/golden/manifest.txt` per the existing golden-image workflow.
- **Verify**: `make checkall`; golden test passes; visually: bloom halo is round on screen; hi-res export matches.

### [ARC-027] naga-based Rust/WGSL contract test
- **Files**: `Cargo.toml` (dev-dependency `naga = "=29.0.4"`, features `["wgsl-in"]`), new test (e.g. `tests/shader_layout.rs`), `src/renderer/uniforms.rs:771-815`
- **Steps**: parse each of the six shaders with naga; walk the module's struct types; for `Uniforms`, `BloomUniforms`, `PostProcessUniforms` and the attractor/buddhabrot compute+display uniform structs, compare member order, offsets and sizes against encase's `min_size()`/member offsets. For per-3D-type specialization, compile once per `SPECIALIZED_TYPE` override value (see how the pipeline is built in `initialization.rs`).
- **Method**: pin naga to the lockfile version (`grep naga Cargo.lock`) — a mismatched naga rejects otherwise-valid WGSL. This makes the WGSL half of the contract machine-checked; keep `uniforms_byte_layout` as the Rust-side byte-pattern check.
- **Verify**: `cargo test --test shader_layout` passes; deliberately swap two WGSL struct members → test fails.

### [ARC-028] Single cfg axis
- **Files**: `Cargo.toml:36-56`, 9 `feature = "native"` sites, `Trunk.toml`, `index.html`
- **Steps**: after ARC-021: move native-only deps into `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` and wasm deps into the wasm32 table; delete the `native`/`web` features; replace `#[cfg(feature = "native")]` with `#[cfg(not(target_arch = "wasm32"))]`; update `Trunk.toml`/`index.html` feature references; fix `cargo check --lib --no-default-features` (45 errors: unresolved `directories`, `rfd`, `chrono`, missing `video_recorder`) — with target-gating, no-default-features on native must compile.
- **Verify**: `cargo check --lib` (default), `cargo check --lib --no-default-features`, and the wasm check all exit 0; `make web-build` succeeds.

### [ARC-029] Adopt `Storage` for all galleries
- **Files**: `src/fractal/presets.rs:117-300,768-905`, `src/fractal/palettes.rs:972-1018`, `src/app/persistence.rs:51`, `src/platform/mod.rs`, ~20 `ProjectDirs::from` call sites
- **Steps**: route preset/bookmark/palette/preference file access through the existing `Storage` trait using its `category::*` constants; pass `&dyn Storage` (or `&PlatformContext`) into the galleries instead of constructing one per call; delete direct `ProjectDirs::from` uses behind a single `config_dir()` helper; wire the wasm stubs so preset/bookmark/palette categories report supported (WebStorage already implements them); remove the resulting `allow(dead_code)`.
- **Verify**: `make checkall` + wasm check; on web (trunk serve), saving a preset persists to localStorage.

### [ARC-030] Remove render-thread blocking I/O (merges QA-041)
- **Files**: `src/app/mod.rs:917-925` (`App::resize`), `src/ui/panels/settings.rs:134`, `src/app/capture.rs:132-183` (video readback), `src/app/update.rs:187-195` (existing debounce)
- **Steps**:
  1. `App::resize`: store the pending size in an `Option<(u32,u32)>` field; drop the per-event load+save. Flush it through the existing one-second autosave timer in `update()`, behind the same `is_headless()` guard that guards settings autosave.
  2. Settings-panel GPU list: cache `AppPreferences` in `App` (load once at startup / on panel open, not every frame).
  3. Video readback: replace the per-frame `device.poll(Wait)` with a ring of staging buffers (3-deep, matching `GpuProfiler`'s pattern) mapped asynchronously; the encoder thread keeps consuming completed buffers.
- **Verify**: `make checkall`; manual: drag-resize the window — no stutter; harness run (`--window-size`) does not overwrite the user's saved window size afterwards; record a short video — frames don't stall.

### [ARC-031] Capture respects device limits (merges SEC-014)
- **Files**: `src/renderer/initialization.rs:171-177`, `src/app/capture.rs:190-194`, `src/ui/panels/capture.rs:282`, `src/app/capture_web.rs`
- **Steps**: request limits from the adapter (`adapter.limits()` merged with required defaults, respecting `required_limits`); store `max_texture_dimension_2d` and `max_buffer_size` on `Renderer`; clamp the UI's hi-res input and the SEC-009 clamp to `min(16384, device_max)`; for requests above the limit, tile the export (reuse the scissor tiles in `app/refine.rs`) and stitch, or reject with a clear message (prefer reject-with-message for this pass; tiling is a follow-up); check the readback buffer size against `max_buffer_size`; correct the "~1.6 GB" comment to the real per-target size at the chosen limit.
- **Method**: `request_device().unwrap()` panic handling belongs to QA-040 — coordinate if both land; if QA-040 landed first, return the limit error through its `RendererInitError`.
- **Verify**: `make checkall`; UI shows the device's actual max; requesting 16384 on a device capped at 8192 shows a clear error instead of "Failed to map buffer".

### [ARC-032] `AppOptions`; group `App` state
- **Files**: `src/app/mod.rs:43-174,237-346`, `src/main.rs:460-485`, `src/web_main.rs`
- **Steps**: define `AppOptions` (all 7 positional `Option`s + any harness fields) with `Default` + builder-ish `From<&Cli>`; change `new`/`new_async`/`init_common`/`AppHandler` to take `AppOptions`; then group state: `InputState` (mouse/touch tracking), `HarnessState` (CLI test flags), `FrameScheduler` (wrap the existing pure decision functions), `AutosaveState`. Do not change behavior — pure refactor.
- **Method**: This removes the signature-drift class that caused ARC-021. Keep it a separate commit from any behavior fix.
- **Verify**: `make checkall` + wasm check green; `--help`/harness flags behave identically.

### [ARC-034] Delete the dead post-path and its 12 uniform fields (merges QA-038)
- **Files**: `src/shaders/fractal.wgsl` (`apply_post_processing` ~3165-3300, `box_fold` 1767, `sphere_fold` 1771, `sample_disk` 2893, identity `apply_fxaa` 3276), `src/renderer/uniforms.rs:209-221,554-570` (fields `brightness`→`fxaa_enabled`), the `uniforms_byte_layout` test, `CLAUDE.md` (size claim), plus the `henon/latoocorfian/icon_attractor` bodies (~150 lines) once QA-031 fixed the IDs
- **Steps**: order matters — QA-031 first (final shader IDs), ARC-026 landed (serde). Delete the dead WGSL functions; delete the 12 unused uniform fields on both Rust and WGSL sides; run the `uniforms_byte_layout` test, read its `left:` output, update the test's size + sentinel offsets; update CLAUDE.md's stated size (and any doc mentioning 768). Confirm each deletion target truly has zero call sites (`get_impact`/grep in the shader sources).
- **Method**: `df2_mul` is intentionally excluded (in-flight double-precision board item). Don't delete post-processing fields that `postprocess.wgsl`'s own uniform structs use — only the `Uniforms` copies nobody reads.
- **Verify**: `cargo test` (updated layout test green); wasm + native compile; all 35 types render (spot-check several).

### [ARC-035] Split the renderer constructor
- **Files**: `src/renderer/initialization.rs:137-1064`, `src/renderer/update.rs:48-175` (`recreate_textures`)
- **Steps**: after ARC-023/031: split `initialize_with_adapter` into `create_device`, `create_fractal_pipelines`, `PostChain::new` (already exists post-ARC-023), `create_accumulation_resources`; make `recreate_textures` reuse the same bind-group builders (extract texture+bind-group creation into functions both init and resize call).
- **Verify**: `make checkall`; run the app and resize the window repeatedly across all three post-modes (bloom on/off, accumulation on/off) — no resize-induced black frame or panic.

### [ARC-037] Deduplicate entry-point helpers (Low)
- **Files**: `src/main.rs:56`, `src/web_main.rs:16` (`parse_quality_level`), the two `window_event` handlers, `attractor_compute.wgsl:137`, `buddhabrot_compute.wgsl:64` (`hash`, `world_to_screen`)
- **Steps**: move `parse_quality_level` into the library; share the WGSL snippets via `common.wgsl` + `concat!` (mechanism shared with QA-039 — do them together); unify the two `window_event` handlers behind one function parameterized by the platform hook.
- **Verify**: `make checkall` + wasm; `--quality high` still parses on native and web.

### [ARC-038] Table-driven CLI parser (Low)
- **Files**: `src/main.rs:157-416`
- **Steps**: if QA-035's clap migration happened, this is subsumed — mark done. Otherwise convert the per-flag loop to a `(flag, arity, parser)` table; add delay validation (finite, ≥ 0).
- **Verify**: `cargo test` on the extracted parser; `--help` output unchanged.

### [ARC-039] Repo hygiene (merges DOC-022)
- **Files**: `todos.md`, `plan.md`, `Cargo.toml:16-31` (`exclude`)
- **Steps**: `git rm --cached todos.md plan.md` (both are stale planning files; `todos.md` lists nonexistent attractors, `plan.md` claims mobile touch is in progress); delete or move under `docs/archive/`; add `todos.md` to Cargo `exclude` (or it vanishes with the file — then verify the publish file list).
- **Verify**: `git status` clean; `cargo package --list` no longer includes `todos.md`.

---

## Phase 3c — Code quality

### [QA-031] Fix the 2D attractor shader dispatch (Critical — run FIRST in 3c)
- **Files**: `src/shaders/fractal.wgsl:3413-3422` (`fs_main_2d` switch), `src/fractal/types.rs:56-61,63` (enum + stale "reserved" comment), `src/fractal/mod.rs:511` (`switch_fractal`), `src/ui/panels/fractal_type.rs:81`, new dispatch test
- **Steps**:
  1. Renumber the `fs_main_2d` cases so 26→`hopalong_attractor`, 27→`martin_attractor`, 28→`gingerbreadman_attractor`, 29→`chip_attractor`, 30→`quadruptwo_attractor`, 31→`threeply_attractor` (matching `FractalType as u32`); delete the `henon_attractor`, `latoocorfian_attractor` and `icon_attractor` arms (their bodies die here; deletion of the dead function bodies is ARC-034's step — leave bodies for now if you want to keep this diff small, but remove the arms).
  2. In `switch_fractal`, set `attractor_accumulation_enabled = ft.uses_accumulation()` so no entry path depends on the UI panel having run.
  3. Fix the stale comment at `types.rs:63` (32–34 are not "reserved" — remove or correct it) and `uniforms.rs:140` (QA-043's "2: Mandelbulb, 3: Menger") in the same pass.
  4. New test (in `src/fractal/tests.rs` or a new `tests/shader_dispatch.rs`): `include_str!("../src/shaders/fractal.wgsl")`, extract the `fs_main_2d` switch arms, assert each `FractalType` attractor variant's `as u32` maps to the function named by its own `attractor_index()`. Keep the test simple — a substring/regex check is enough.
- **Method**: The compute path's `attractor_index()` is already correct — the *display* path also uses it, so only the fragment switch is wrong. Do not "fix" the enum discriminants (they're a persisted wire format); fix the shader.
- **Verify**: `make checkall` + new dispatch test; manual: with UI hidden (`--exit-delay` run), screenshot Martin2D, Chip2D, Quadruptwo2D, Threeply2D and compare against the compute-path (accumulation-on) renders — same structure.

### [QA-034] `FractalType::ALL` + `FractalInfo` table (before QA-032/033/035)
- **Files**: `src/fractal/types.rs`, `src/fractal/mod.rs:235-271,513-708,732-751`, `src/app/input.rs:94-215`, `src/command_palette.rs`, `src/ui/panels/fractal_type.rs`, `src/fractal/tests.rs:590-640`
- **Steps**:
  1. Add `pub const ALL: [FractalType; 35]` and `pub struct FractalInfo { label: &'static str, filename: &'static str, dim: Dim, camera_distance: f32, hotkey: Option<&'static str>, uses_accumulation: bool }` plus `const fn info(self) -> &'static FractalInfo` — one 35-entry table (a `const` array indexed by discriminant, or a match returning `&'static` entries).
  2. Replace both `RenderMode` 35-arm matches in `fractal/mod.rs` with `if ft.is_3d() { RenderMode::ThreeD } else { RenderMode::TwoD }`.
  3. Drive the UI panel labels/hover text, command palette entries, randomize list, hotkey table (via the QA-035 CommandAction merge if it landed, else a lookup in `input.rs`) and the in-app help from `ALL`/`info()` — this fixes the wrong F6/F11 help text.
  4. `filename_safe_name`, `attractor_index`, `default_camera_distance` read from the table.
  5. Replace the manual "keep in sync" lists in `tests.rs:590-640` with loops over `ALL`.
  6. Per-type default parameters (`switch_fractal` seeding, `mod.rs:552-707`) stay a match (they set different field sets) but gain a compile-time exhaustiveness guard: add `#[allow(unreachable_patterns)]`-free exhaustive match so a new variant breaks the build.
- **Method**: This is the keystone refactor — QA-032/033 need its per-type 2D defaults and QA-035 needs the single enumeration. After it, adding a fractal type touches: the enum + one `FractalInfo` row + the WGSL estimator + the `switch_fractal` seed match.
- **Verify**: `make checkall`; `--list-presets`, UI panel, command palette and help all agree; the sync tests loop over `ALL` and pass; temporarily add a 36th variant → build breaks in exactly the expected places.

### [QA-032] Working R-key reset view (after QA-034)
- **Files**: `src/app/mod.rs:928-953`, `src/camera.rs`, new `App::reset_view`
- **Steps**: implement `App::reset_view()`: 2D — restore center/zoom from per-type defaults (extend `FractalInfo` or the existing default table with `default_center: [f64;2]`, `default_zoom: f64` if not already there; escape-time default is `[0,0]`/`1.0` — reuse whatever `switch_fractal`/initial spawn uses so there is one source), clear `center_2d_precise`, and *do not* touch user `power`/`max_iterations`; 3D — `apply_default_camera_for_fractal()` + `note_camera_framed()`. Bind R to it (replacing the `switch_fractal(current)` call).
- **Verify**: `cargo test` (per-type reset test over `ALL`: center/zoom restored, power/iterations preserved for escape-time types); manual: zoom deep into Mandelbrot, press R — back to full view; Octahedral IFS press R — framed outside the structure.

### [QA-033] Unify the four reset entry points (after QA-032)
- **Files**: `src/ui/command.rs:382-393`, `src/ui/panels/rendering.rs:464-469`, `src/app/render.rs:1136-1165`, `src/app/mod.rs`
- **Steps**: route all four through `App::reset_view` / add `App::reset_all` (params + camera + UI state); make the command palette return an action enum that `App` executes (matches the QA-035 CommandAction direction) instead of mutating `params` directly; ensure every reset clears `center_2d_precise`.
- **Verify**: behavior test: all four entry points produce byte-identical `FractalParams` + camera state from the same starting state.

### [QA-035] Decompose the cx>40 functions (after QA-034)
- **Files**: `src/app/input.rs:8`, `src/ui/command.rs:154`, `src/ui/panels/palette.rs`, `src/ui/panels/presets.rs`, `src/ui/panels/capture.rs`, `src/main.rs:157`
- **Steps**:
  1. Define `CommandAction` (if QA-033 didn't already) covering toggles, switches, palettes, reset; hoist the magic numbers to named consts (iteration step 32 / max 2048; step step 10 / range 30–500; power ±0.5 / 2–16; orbit speed 0.1–5.0; shadow-mode names — one table, delete the copy at `input.rs:407`). `App::input` maps keys to `CommandAction` and calls `execute_command`; delete the duplicated arms.
  2. `main`: extract `fn parse_args(&[String]) -> Result<Cli, String>` — clap derive is acceptable (new dep) or a table; validate delays finite/≥0; keep `--help` output materially the same.
  3. Split `render_color_viz_panel`, `render_presets_panel`, `render_capture_window` into one function per collapsing group (same pattern the other panels use).
- **Method**: run after QA-034 so hotkey arms use `FractalInfo` instead of another enumeration. The shared `execute_command` also serves ARC-025's commit-path unification — if ARC-025 already landed, route through `App::commit_change` here.
- **Verify**: `cargo test` on `parse_args` (valid/invalid/missing-value cases); `make checkall`; hotkey and palette produce identical state for the same action (behavior test); no function above cx 40 (`calculate_cyclomatic_complexity` spot-check).

### [QA-036] Behavior tests (last in 3c)
- **Files**: `tests/integration_tests.rs:23-57,103-144,146-170`, `src/ui/tests.rs`, `src/fractal/tests.rs:126-143,590-640`
- **Steps**: delete/replace assign-then-assert tests with: reset-restores-defaults over `ALL`; `switch_fractal` leaves `uses_accumulation()` types with accumulation enabled; palette action == hotkey action state equality; unit tests for `parse_pal_content` (valid `.pal`, malformed, non-UTF8), `parse_url_params` (valid, unknown, hostile), `parse_args`. Replace the index-hardcoded palette test with a name-based loop over `ColorPalette::ALL`.
- **Method**: this is the agent that verifies the other 3c fixes from the outside — schedule it last so it tests final behavior.
- **Verify**: `make test`; confirm the deleted vacuous tests' names are gone and the new behavior tests fail if you intentionally re-break one fix (spot-check QA-032).

### [QA-042] One `accumulation_active()` predicate (after QA-031)
- **Files**: `src/app/render.rs:73-76`, `src/renderer/update.rs:274-276`, `src/app/capture.rs:373-379`, `src/app/mod.rs:722-725,818-820`
- **Steps**: add `FractalParams::accumulation_active(&self) -> bool` (flag && (`is_2d_attractor()` || `is_buddhabrot()`), or derive purely from type once QA-031's `switch_fractal` owns the flag); replace the five inline derivations. Note `capture.rs` additionally checks a bind group — keep that part local.
- **Verify**: `make checkall`; accumulation modes render identically before/after (spot-check Buddhabrot + one attractor).

### [QA-040] Result-returning renderer init; web storage fallback (merges SEC-018)
- **Files**: `src/renderer/initialization.rs:57,95,106,123,177`, `src/main.rs:420,449`, `src/platform/web/storage.rs:15-22`, `src/web_main.rs:108` (existing `#error` element)
- **Steps**: define `RendererInitError` (enum: NoAdapter, DeviceLost(String), Surface(String), Limits(String)); change `new*`/`initialize_with_adapter` to return `Result<Self, RendererInitError>`; native: print a human message to stderr and exit non-zero; web: set the `#error` element text ("WebGPU not available or no compatible GPU"); `WebStorage::new` → return `Result`/fall back to an in-memory store (prefer the in-memory fallback so private browsing still runs). Coordinate with ARC-031 if both pending (limits error variant).
- **Verify**: wasm check + native check; manual (if possible): run on a machine without WebGPU → clear message, no panic.

### [QA-039] Shared `common.wgsl` (with ARC-037)
- **Files**: new `src/shaders/common.wgsl`; loaders wherever `include_str!("fractal.wgsl")` etc. appear (grep `include_str!` in `src/`)
- **Steps**: move `rgb_to_hsv`/`hsv_to_rgb` (fractal.wgsl:3165-3219, postprocess.wgsl:151-205) and `hash`/`world_to_screen` (attractor_compute.wgsl:137, buddhabrot_compute.wgsl:64) into `common.wgsl`; change each load site to `concat!(include_str!("common.wgsl"), include_str!("<shader>.wgsl"))`. Watch for duplicate symbols once concatenated — remove the originals from each shader.
- **Verify**: `make checkall`; ARC-027's naga test (if landed) passes; visual spot-check of a palette edit affecting both paths.

### [QA-043] Stale wire-format comments (with QA-031)
- **Files**: `src/renderer/uniforms.rs:140`, `src/fractal/types.rs:63`, `src/ui/panels/fractal_type.rs:141`
- **Steps**: fix the "2: Mandelbulb, 3: Menger" comment (actual 13/14); fix/correct the "32–34 reserved" comment per QA-031's outcome; repoint the `todos.md` reference (file removed by ARC-039) to the board.
- **Verify**: `make checkall`; grep the comments for the corrected values.

### [QA-044] Delete `shader_index()`
- **Files**: `src/fractal/types.rs:353-369`, `src/renderer/uniforms.rs:462`
- **Steps**: replace the call with `*self as u32` (enum already `#[repr(u32)]`); delete the method; keep the deprecated doc comment's removal out of the changelog (trivial).
- **Verify**: `make checkall`; uniform values unchanged (layout test).

### [QA-045] Co-locate guard and use
- **Files**: `src/ui/panels/rendering.rs:406,411`, `src/app/render.rs:488,490`, `src/renderer/compute.rs:306`, `src/app/input.rs:656`
- **Steps**: rewrite each guarded `unwrap()` as `let Some(x) = … else { return/bail }` or `if let` so the guard and the use cannot drift. No behavior change.
- **Verify**: `make checkall`.

### [QA-046] `#[cfg(test)]` camera helpers (after ARC-024)
- **Files**: `src/camera.rs:271,308-338`
- **Steps**: after ARC-024 changes the dead-code landscape, move the six test-only helpers behind `#[cfg(test)]` (or `#[cfg(any(test, feature = "test-util"))]` if non-test consumers appear); delete the `allow(dead_code)`. Re-check which helpers are still unused after ARC-024 — delete any that nothing uses.
- **Verify**: `cargo clippy --all-targets` clean; release build has no dead helpers (spot-check with `nm`/`objdump` optional).

### [QA-047] One `VideoFormat` enum
- **Files**: `src/ui/mod.rs:175-193` (wasm stub), `src/video_recorder.rs:12-61`
- **Steps**: keep one platform-neutral `VideoFormat` in a shared module (`fractal` or `ui`); have the wasm path use it too; make the encoder call `codec()`/`pixel_format()` instead of inlining the arms; delete the `allow(dead_code)`s on those methods.
- **Verify**: `make checkall` + wasm; record a short MP4 and GIF — outputs identical to before.

### [QA-048] Remove no-op `bail` param
- **Files**: `src/reference.rs:234`
- **Steps**: delete the `bail` parameter and `let _ = bail;` (or wire it if the shader mirror genuinely needs it — check the caller).
- **Verify**: `make checkall`; `tests/reference_math.rs` passes.

---

## Phase 3d — Documentation

### [DOC-001] Rewrite uniform-sync docs for encase (Critical — after ARC-026/027/034)
- **Files**: `docs/ARCHITECTURE.md:380,588,645-745,965-989,1044-1049`, `docs/README.md:208-211`, `CLAUDE.md:127,139,193`, `src/fractal/types.rs:4`
- **Steps**: in ARCHITECTURE.md replace the "Uniform Buffer Structure", "Synchronization Checklist", "Memory Layout Example", "Modifying Uniforms" and "Uniform Buffer Mismatch" sections with the encase procedure (glam types, no padding fields, update `uniforms_byte_layout` sentinels from the test's `left:` output); rename `FractalUniforms`→`Uniforms` everywhere (grep all of `docs/` + `CLAUDE.md` + the `types.rs:4` doc comment); remove `to_uniforms()` mentions and its test example from CLAUDE.md and the ARCHITECTURE diagram node; state the *current* size from `min_size()` (recheck after ARC-034 — it will not be 768 anymore).
- **Verify**: grep for `repr(C)`, `_padding`, `FractalUniforms`, `to_uniforms`, `864` across docs → zero hits; the stated size matches `cargo test uniforms_byte_layout` output.

### [DOC-002] Release-state alignment (needs owner decision first)
- **Files**: `CHANGELOG.md:106,285,611-612`, `README.md:16,36,176-186`, `docs/QUICKSTART.md:45-57`
- **Steps**: ask the owner: publish 0.9.0/0.10.0 (then tag via the release workflow — see DOC-017's runbook) or fold under `[Unreleased]`. Implement the chosen option; fix README's install-path claims to match what actually installs; update feature bullets if folding.
- **Verify**: every CHANGELOG version link resolves (`git tag --list` + crates.io check); `cargo install par-fractal` README claim matches the shipped version's features.

### [DOC-003] `WGPU_BACKEND` wiring (needs decision; prefer wiring)
- **Files**: `src/renderer/initialization.rs:16-17,53-55`, `docs/QUICKSTART.md:421-428`, `CLAUDE.md:63-70`, `Makefile:510-525`
- **Steps**: switch both `InstanceDescriptor` constructions to `InstanceDescriptor::new_without_display_handle_from_env()` (verify the exact wgpu-29 API name on the struct — it exists per the audit's wgpu-core grep); keep `Backends::all()` as the default when the env var is unset. Then the docs/Makefile targets become true. (Alternative if rejected: delete the doc sections and Makefile targets.)
- **Verify**: `WGPU_BACKEND=gl make run-linux` fails *differently* (GL unsupported) proving the variable is read; `WGPU_BACKEND=vulkan make run-linux` works on a Vulkan machine; default run unchanged.

### [DOC-004] Complete CLI reference
- **Files**: `docs/FEATURES.md:412-421` (canonical), `CLAUDE.md:73-82`, `docs/README.md:298-331`, `docs/ARCHITECTURE.md:163-169`
- **Steps**: build the table from `main.rs:29-53` `print_help` (all 14 options: name, argument, unit, default, example) — include `--profile-dump`, `--switch-after`, `--camera-pos`, `--resize-after`, `--window-size`, `--screenshot-path`; replace the four partial copies with a link; if QA-035's parser landed, generate the table from the parser definition instead of hand-copying.
- **Verify**: diff FEATURES table against `--help` output — identical option set; no other doc lists flags inline.

### [DOC-005] Deep-zoom documentation
- **Files**: `docs/FRACTALS2D.md:81-107,146,255,261`, `docs/CONTROLS.md:337`, `docs/ARCHITECTURE.md` (new section)
- **Steps**: rewrite "High-Precision Zoom Mode" as "Deep Zoom": double-float path, perturbation path (gate `PERTURBATION_LOG2_GATE = 13.3`, `src/deep_zoom/driver.rs:42`), covered types (Mandelbrot, Julia, Burning Ship, Tricorn), the precise-center UI ("Go to", "Copy", 🔒). Add an ARCHITECTURE `deep_zoom/` subsection: reference-orbit worker → orbit buffer → shader, with a Mermaid data-flow diagram per the style guide.
- **Verify**: the guide no longer states a 1e11 ceiling as current; a reader can tell from the docs alone which path is active at a given zoom; `find_broken_doc_links` clean for the new section.

### [DOC-006] About page regeneration (after DOC-002)
- **Files**: `src/ui/panels/about.rs:59-91`
- **Steps**: rebuild the What's New entries from CHANGELOG.md's actual version sections (post-DOC-002 labels); correct the current 0.9.0 entry (it lists 0.8.3 content); add the missing 0.8.x entries; decide build-time generation (a `build.rs` embedding the changelog head) only if the owner wants it — otherwise add a comment marking the sync rule.
- **Verify**: diff each About entry's bullet list against the corresponding CHANGELOG section — equal.

### [DOC-007] Undo/redo + double-click docs (after ENH-002)
- **Files**: `README.md:248`, `docs/FEATURES.md:388-389`, `src/ui/mod.rs:477,490`, `docs/FRACTALS2D.md:79,118`
- **Steps**: once ENH-002 lands (or is declined): align all four sites with the shipped bindings; make README/FEATURES/tooltips state the same redo key.
- **Verify**: every documented shortcut resolves to a handler in `input.rs`; the redo binding is named identically in all three places.

### [DOC-008] Settings location/format
- **Files**: `CLAUDE.md:221-256`, `docs/ARCHITECTURE.md:209-222`, `docs/QUICKSTART.md` (new section)
- **Steps**: add "Configuration Files" to QUICKSTART: macOS `~/Library/Application Support/com.fractal.par-fractal/`, Windows `%APPDATA%\fractal\par-fractal\config`, Linux `~/.config/par-fractal/`; file `settings/settings.yaml` (legacy `settings.yaml` fallback), `preferences.yaml`, `presets/`; flat YAML sample matching the real serialization (flat keys — verify against a live `settings/settings.yaml`); web note (localStorage). Fix CLAUDE.md and the ARCHITECTURE "Native Only" claim.
- **Verify**: paths verified against `ProjectDirs::from("com","fractal","par-fractal")` resolution on this machine; the YAML sample round-trips through `from_settings`.

### [DOC-009] `render_scale` claim
- **Files**: `docs/FEATURES.md:455`, `docs/ARCHITECTURE.md:866`, `docs/FRACTALS3D.md:1385`
- **Steps**: replace "not currently applied" with the dynamic-scaling description (sub-rect viewport during motion, full res at idle, forced full res for captures — see `renderer/mod.rs:171-179`, `set_render_scale_override`).
- **Verify**: grep docs for "not currently applied" → zero.

### [DOC-010] MSRV reference
- **Files**: `CONTRIBUTING.md:7`
- **Steps**: replace "1.85" with a pointer to `rust-version` in `Cargo.toml`.
- **Verify**: grep CONTRIBUTING for a hard-coded MSRV → none.

### [DOC-011] rustdoc warnings + doc gate
- **Files**: `src/deep_zoom/mod.rs:13,15`, `src/renderer/profiler.rs:16`, `src/renderer/orbit_buffer.rs:20`, `src/fractal/palettes.rs:812` (unresolved links); `src/app/persistence.rs:5`, `src/fractal/mod.rs:70`, `src/reference.rs:315`, `src/app/mod.rs:232,664` (private links); `.github/workflows/ci.yml`, `Makefile`
- **Steps**: fix links (`[orbit](self::orbit)`, `std::cell::Cell`, escape `[[f32;3];8]` as `\[[f32;3];8\]` or reword, link public items or un-link private ones); add `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --lib` to CI and a `doc-check` step in `checkall`.
- **Verify**: `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --lib` exits 0; CI includes the step.

### [DOC-012] Docstring coverage
- **Files**: `src/fractal/palettes.rs` (6/65), `src/fractal/presets.rs` (12/43), `src/fractal/ui_state.rs` (1/16), `src/app/camera_transition.rs` (0/4), `src/renderer/initialization.rs` (4/9), `src/ui/mod.rs` (3/9), 22 files without `//!` module docs
- **Steps**: add `//!` module docs (one or two sentences each: what the module owns, its invariants); document the gallery/preset/palette public APIs including error conditions; add `#![warn(missing_docs)]` to `lib.rs` only if the warning list stays manageable (run it first; if it floods, skip the lint and just fix the galleries).
- **Verify**: `cargo doc --no-deps` builds; spot-check the palette/preset APIs document their errors.

### [DOC-013] ARCHITECTURE module map
- **Files**: `docs/ARCHITECTURE.md:40-149,224-270,312,381,467,593,834-851`, `docs/README.md:197-205`, `docs/FEATURES.md:263`
- **Steps**: regenerate the component diagram from the real `src/` tree (include `deep_zoom/`, `renderer/{compute,orbit_buffer,profiler}.rs`, `app/{refine,capture_common,capture_web}.rs`, `ui/panels/*`, `fractal/state.rs`, `reference.rs`, `bin/imgdiff.rs`); replace verbatim struct dumps (the `App` field list) with short prose summaries; fix: `LODState.camera_velocity` is `f32`; preset categories All/2D/3D/IFS/Apollonian; type 25 = Buddhabrot2D; `NativeCapture` saves to cwd; `docs/README.md` layout diagram to the post-split tree.
- **Verify**: every file in the new diagram exists; the corrected facts match the code (spot-check each).

### [DOC-014] Adding-a-fractal checklist (after DOC-001)
- **Files**: `CLAUDE.md:261-267`, `docs/ARCHITECTURE.md:903-963` (keep one — prefer CONTRIBUTING.md or ARCHITECTURE.md; CLAUDE.md links)
- **Steps**: rewrite the checklist: `#[repr(u32)]` discriminant; WGSL estimator + hook into `scene_de_with_material` (not a nonexistent `map()`); `switch_fractal` seeds every uniform the estimator reads; `FractalType::default_camera_distance`; `filename_safe_name`; command-palette entry (or note it's generated once QA-034 landed); the shader-dispatch regression test (QA-031); `make checkall`. Remove the duplicated older checklist.
- **Verify**: follow the checklist mentally for a hypothetical type — no step references a nonexistent symbol (grep each named symbol).

### [DOC-015] FRACTALS3D ranges/defaults
- **Files**: `docs/FRACTALS3D.md:347,401,493,543,597,649,693,744,806,867,936,1015,1064-1065,1092`
- **Steps**: correct scale range to `FRACTAL_SCALE_RANGE` (0.05–5.0, log slider) — cite the constant, don't repeat numbers; Pickover/Lorenz/Rossler defaults 0.3/0.05/0.1 (`fractal/mod.rs:684-705`); Rossler `dt = 0.02`, 2000 transient steps.
- **Verify**: each corrected value matches the code constant (grep each).

### [DOC-016] CHANGELOG hygiene (after DOC-002)
- **Files**: `CHANGELOG.md:229,344-350,610-626`
- **Steps**: reorder 0.8.1 below 0.8.2; add `[Unreleased]: .../compare/<latest-real-tag>...HEAD`; fix "~1e6e7" → "~1e6"; add the ENH-002 tile-progressive refinement entry (`615fb26`, `1c92fd2`).
- **Verify**: compare links resolve; versions strictly descending top-to-bottom.

### [DOC-017] Release runbook + env-var reference
- **Files**: new `docs/RELEASING.md`; env-var table in QUICKSTART or ARCHITECTURE
- **Steps**: document the release flow: version bump (Cargo.toml + CHANGELOG + About sync), `make release`/`make deploy` (`workflow_dispatch`), required secrets (`CARGO_REGISTRY_TOKEN`, `HOMEBREW_TAP_TOKEN`, `DISCORD_WEBHOOK`), Homebrew cask step, verification per `release-checklist.md`. Env vars: `RUST_LOG`, `PAR_REFINE_FORCE_SIDE` (what it forces, where — `app/refine.rs`), `WGPU_BACKEND` (per DOC-003's outcome).
- **Verify**: a maintainer could execute a release from `docs/RELEASING.md` alone; every env var named in code appears in the table (grep `env::var` / `std::env`).

### [DOC-018] CONTROLS/QUICKSTART/FEATURES UI claims
- **Files**: `docs/CONTROLS.md:304-307,315,324,539`, `docs/QUICKSTART.md:171`
- **Steps**: rewrite the video section from `ui/panels/capture.rs` (MP4/WebM/GIF; FPS slider 24–60, 10–30 GIF; no quality/duration controls); delete preset thumbnail-preview/share-link claims (nonexistent); fix "panel fixed right" (movable `egui::Window`) and the Esc-fullscreen claim (no fullscreen); add Shift+G (GPU profiler) to the global table.
- **Verify**: each documented control resolves to a UI element in the panel sources; Shift+G documented.

### [DOC-019] Screenshot filename claims
- **Files**: `docs/QUICKSTART.md:380-382`, `docs/FRACTALS2D.md:1162-1165`, `docs/ARCHITECTURE.md:467`
- **Steps**: F12 → `{fractal}_{timestamp}.png`; hi-res adds `{WxH}`; both save to the current working directory.
- **Verify**: matches `src/app/capture.rs:94,640`.

### [DOC-020] Style-guide conformance
- **Files**: ~45 fenced blocks across `docs/ARCHITECTURE.md:624,633`, `docs/README.md:198,204,335`, `docs/CONTROLS.md:399-456`, `docs/FRACTALS2D.md` (21), `docs/FRACTALS3D.md` (14); Mermaid `style` lines in ARCHITECTURE (20), FRACTALS2D (6), FRACTALS3D (7)
- **Steps**: add `text` (or the real language) to unlabeled fences; convert per-node `style` to `classDef` (use the user's dark-mode palette: bg `#1E1E1E`, text `#E6E6E6`, success `#4CAF50`, warning `#FFC107`, error `#F44336`, info `#2196F3`); replace repeated Rust-version numbers with "see `rust-version` in Cargo.toml" (README badges stay).
- **Verify**: grep for `^```\s*$` closers with no opener-language and for `style ` in Mermaid blocks → zero (except intentional).

### [DOC-021] README redundancy
- **Files**: `README.md:14-26,315-334`
- **Steps**: move the prose comment lines out of the install code block into bullets; shrink Contributing to one line + link.
- **Verify**: install block contains only commands.

### [DOC-023] `src/`-prefixed doc paths
- **Files**: `CLAUDE.md:86-119`, `docs/ARCHITECTURE.md` (throughout), `CONTRIBUTING.md:87`
- **Steps**: prefix code paths with `src/` so parsight's doc-link graph resolves them (~65 medium-confidence broken links).
- **Verify**: `find_broken_doc_links` medium-confidence count drops substantially.
