# ENH-007 — Deep-Zoom Visual Regression Harness

> **Impact**: High — guards the ARC-002 correctness bundle, ENH-001, and every future shader edit.
> **Effort**: Low–Medium (~1–2 days).
> **Prerequisites**: none. **Do this before other deep-zoom work.**

## Goal

A `cargo test`-runnable (and CI-runnable) harness that renders known fractal views through the real
GPU pipeline and compares them against CPU-computed references, failing loudly on precision or
rendering regressions — the class of bug the audit found shipping silently (unreachable hp path,
wrong DF abs, FMA-dependent `two_prod`).

## Current state (verified at HEAD 8ee42cc)

- The app already has agent-operability flags in `src/main.rs`: `--screenshot-delay <s>`,
  `--exit-delay <s>`, `--preset <name>`, `--clear-settings`, `--quality`. Screenshots save via
  `src/app/capture.rs` (PNG to a predictable path — read `save_screenshot` to confirm the output
  directory and whether a `--screenshot-path` override exists; **if not, add one** — trivial clap
  flag threaded to the capture call; this is the only app change needed).
- Presets are YAML files in the user config dir; `--preset` loads by name
  (`src/fractal/presets.rs:734`). A preset fully determines fractal type, center, zoom, iterations,
  palette.
- No existing image-comparison tooling in-repo. Tests run with `make test`.
- CI: GitHub Actions runners have NO GPU — the harness must be split into a local/GPU part and a
  CPU-only part that still runs in CI.

## Design

Two layers:

1. **CPU reference layer (runs everywhere, including CI)** — a pure-Rust f64 escape-time renderer
   (~100 lines) + tests that pin the *math*: DF split roundtrip, smooth-coloring formula, and
   iteration counts at sampled points for known coordinates. No GPU needed.
2. **GPU golden-image layer (local + optional self-hosted CI)** — drives the real binary via the
   CLI flags, renders fixed views, compares PNGs against committed golden tiles with tolerance.
   Run via `make visual-test`; skipped automatically when no GPU/display is present.

## Implementation steps

1. **CPU reference renderer** — new file `src/reference.rs` (or `tests/support/reference.rs` if
   kept test-only; prefer `src/` behind `#[cfg(any(test, feature = "reference"))]` so ENH-001 can
   reuse it):
   ```rust
   /// f64 escape-time Mandelbrot; returns smooth iteration value per pixel,
   /// matching the shader's smooth_iteration_count formula exactly.
   pub fn render_mandelbrot_f64(center: (f64, f64), zoom: f64, size: (u32, u32),
                                max_iter: u32) -> Vec<f32>
   ```
   Mirror the shader's coordinate mapping from `src/shaders/fractal.wgsl:2946-2972` exactly
   (aspect handling, y direction) and its smooth-color epilogue (post-QA-016: the
   `smooth_iteration_count` helper). Support Julia/Burning Ship/Tricorn with an enum parameter —
   they're 5-line variants.
2. **Math pin tests** (`tests/reference_math.rs`):
   - Known-point tests: e.g. c = (0,0) never escapes; c = (2,2) escapes at iter 1; a table of
     ~10 (coordinate, zoom, expected smooth value ±1e-6) entries generated ONCE by the reference
     renderer and hard-coded (self-consistency + future drift alarm).
   - DF split roundtrip (shared with AUDIT QA-019 — don't duplicate if that landed; extend).
   - CPU DF simulation: implement `two_prod`/`two_sum`/df-mul in Rust f32 mirroring the WGSL, and
     assert a DF Mandelbrot at zoom 1e8 matches the f64 renderer within tolerance on a 32×32 tile.
     **This test catches QA-002/QA-005-class bugs with zero GPU.**
3. **Golden tile script** — `scripts/visual_test.sh` + `make visual-test` target:
   - Test matrix (committed as `tests/golden/manifest.yaml`):
     | id | fractal | center | zoom | iters | size |
     |----|---------|--------|------|-------|------|
     | mandel-shallow | Mandelbrot2D | (-0.5, 0) | 1 | 256 | 256×256 |
     | mandel-seahorse-1e5 | Mandelbrot2D | (-0.7436438870, 0.1318259042) | 1e5 | 1000 | 256×256 |
     | mandel-seahorse-1e8 | Mandelbrot2D | same | 1e8 | 2000 | 256×256 |
     | tricorn-1e6 | Tricorn2D | (-0.3, 0.8) | 1e6 | 1000 | 256×256 |
     | ship-1e8 | BurningShip2D | (-1.7625, -0.0333) | 1e8 | 2000 | 256×256 |
     | mandelbulb-default | Mandelbulb3D | default preset | — | — | 256×256 |
   - For each row: generate a preset YAML into the config presets dir (script writes it), run
     `cargo run --release -- --preset <id> --screenshot-delay 3 --exit-delay 5 --screenshot-path target/visual/<id>.png`
     with a fixed window size (confirm/add a `--window-size WxH` flag if none exists — check
     `src/main.rs` clap args; settings.yaml `window:` section can be pre-written by the script as
     an alternative), then compare.
   - Comparison tool: small Rust bin `src/bin/imgdiff.rs` (avoid new system deps): loads two PNGs
     (the `image` crate is already a dependency for capture — verify in Cargo.toml), computes
     (a) fraction of pixels with channel delta > 8/255, (b) mean absolute error. Pass if
     `bad_pixel_fraction < 0.5%` and `mae < 2.0` — loose enough for GPU/driver dither, tight
     enough to catch quantization blocks, wrong-fractal, black frames.
   - First run with `--bless` writes `tests/golden/<id>.png`; goldens are committed (256×256 PNGs,
     ~50–150 KB each — acceptable).
4. **CPU-vs-GPU cross-check** (the precision teeth): for the 2D rows, ALSO compare the GPU PNG
   against a reference-renderer PNG (map smooth values through the same palette — simplest: use a
   grayscale palette preset for harness rows so palette code isn't part of the comparison).
   Tolerance looser (`mae < 4.0`) — DF vs f64 differ legitimately at the last bits.
5. **Wire into make/CI**: `make visual-test` runs the script; `make test` gains the CPU-layer tests
   automatically (they're normal `cargo test`). CI: CPU layer runs in the existing test workflow;
   add `visual-test` as a separate optional job marked `continue-on-error: true` if a macOS runner
   with GPU is available (macos-14 runners DO have a Metal GPU — try it; if flaky, keep local-only).
6. **Document** in `CONTRIBUTING.md` (DOC-013) and `docs/ARCHITECTURE.md`: when goldens
   legitimately change (palette rework etc.), re-bless with `make visual-bless` and include
   before/after in the PR.

## Files to touch

| File | Change |
|------|--------|
| `src/reference.rs` | new: f64 reference renderer + Rust DF mirror |
| `src/bin/imgdiff.rs` | new: PNG comparison bin |
| `tests/reference_math.rs` | new: math pin tests (CI-safe) |
| `tests/golden/manifest.yaml` + `tests/golden/*.png` | new: test matrix + blessed tiles |
| `scripts/visual_test.sh` | new: drives the binary per manifest |
| `src/main.rs` | add `--screenshot-path` (and `--window-size` if absent) |
| `Makefile` | `visual-test`, `visual-bless` targets |
| `.github/workflows/<test>.yml` | CPU tests already run; optional GPU job |

## Verification

1. `make checkall` — new unit tests green.
2. `make visual-test` twice in a row — deterministic pass (run-to-run GPU variance within tolerance).
3. Mutation check (proves the harness has teeth): temporarily reintroduce the QA-002 bug
   (`abs(z.lo) * sign(z.hi)`) — `ship-1e8` must FAIL; revert. Temporarily set hp threshold back to
   1e6 — `mandel-seahorse-1e5` must FAIL (blocky) once the ARC-002 bundle's derived gate is in;
   revert.
4. CI run green with the CPU layer.

## Rollback

Purely additive (new files + two CLI flags). Rollback = delete the files and targets; no runtime
behavior depends on them.

## Pitfalls

- Determinism: fixed window size, fixed quality (`--quality ultra` to pin LOD), LOD *disabled* for
  harness runs if a flag exists (add `--no-lod` if needed — one clap flag + one boolean check),
  no palette animation (pick static palette in the preset).
- macOS screenshot color: capture reads back the render target BEFORE OS color management
  (verify — `capture.rs` reads the wgpu texture, so yes); goldens are portable across displays but
  NOT necessarily across GPU vendors — tolerance handles dither-level differences; if a second
  platform diverges structurally, keep per-platform goldens (`tests/golden/metal/`, `vulkan/`).
- Don't gate `make checkall` on the GPU layer — it must stay runnable headless.
