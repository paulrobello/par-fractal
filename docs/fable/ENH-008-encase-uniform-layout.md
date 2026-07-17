# ENH-008 — `encase`-Based Uniform Layout Automation

> **Impact**: Medium — eliminates the project's documented #1 silent-corruption bug class
> (hand-maintained Rust↔WGSL layout with ~14 manual padding fields), and makes the uniform
> additions ENH-001/003/006 need safe.
> **Effort**: Medium (~2 days).
> **Prerequisites**: AUDIT ARC-010 merged (offset tests exist — they are the migration safety net).

## Goal

Replace the hand-mirrored `#[repr(C)] + bytemuck + manual padding` `Uniforms` struct with
`encase`'s `#[derive(ShaderType)]`, which computes WGSL-correct uniform layout (alignment,
padding, struct size) automatically. The WGSL side stays the source of truth; Rust stops
hand-encoding its alignment rules.

## Current state (verified at HEAD 8ee42cc)

- `src/renderer/uniforms.rs`: `Uniforms` (~:6-147) — 864 bytes, ~14 `_padding_*` fields,
  `#[repr(C)]`, `derive(Pod, Zeroable)`, uploaded via
  `queue.write_buffer(&buf, 0, bytemuck::bytes_of(&uniforms))` (find upload sites:
  `grep -n 'bytes_of\|cast_slice' src/renderer/ src/app/`). Compile assert
  `size_of::<Uniforms>() == 864` (:532-535); post-ARC-010 there are `offset_of!` sentinel tests.
- Additional GPU structs in the same style: `PostProcessUniforms`, bloom/composite uniforms
  (`renderer/uniforms.rs` tail / `renderer/update.rs`), `AttractorComputeUniforms`
  (`renderer/compute.rs`). Same migration applies; do `Uniforms` first, others follow the pattern.
- WGSL uniform structs: `src/shaders/fractal.wgsl:60-147`, `postprocess.wgsl`,
  `attractor_compute.wgsl`.
- `encase` is the ecosystem standard (Bevy uses it); current version supports wgpu 29 workflows
  (it's wgpu-agnostic — it produces bytes).

## Design decisions

1. **Keep field ORDER identical to the WGSL struct** — encase computes offsets from declaration
   order exactly like WGSL does for `var<uniform>` (std140-like rules). Same order in ⇒ same
   layout out. The migration DELETES padding fields and lets encase re-derive them.
2. **Upload path**: `encase::UniformBuffer` writes into a `Vec<u8>`/reused buffer:
   ```rust
   let mut ub = encase::UniformBuffer::new(Vec::<u8>::new()); // or reuse a scratch Vec
   ub.write(&uniforms)?;
   queue.write_buffer(&self.uniform_buffer, 0, ub.as_ref());
   ```
   Types change: `[f32; 4]` → `glam::Vec4`/`[f32; 4]` (encase impls exist for glam behind the
   `glam` feature — the project already depends on glam; enable `encase = { version = "...",
   features = ["glam"] }`). `vec3` fields become `glam::Vec3` and encase handles the pad.
3. **Keep the assert + tests**: the `size_of` assert becomes
   `assert_eq!(<Uniforms as ShaderType>::min_size().get(), 864)` and the ARC-010 offset tests are
   RE-POINTED at encase's computed offsets during migration, then kept as WGSL-contract pins.

## Implementation steps

1. `cargo add encase --features glam` (pin the version).
2. **Migrate `Uniforms`** (`renderer/uniforms.rs`):
   - Remove `#[repr(C)]`, `Pod`, `Zeroable` derives; add `#[derive(encase::ShaderType)]`.
   - Delete every `_padding_*` field. Convert `[f32; 3]`+pad pairs to `glam::Vec3`; `[f32; 4]` to
     `glam::Vec4` (or keep arrays — encase supports `[f32; N]`, but WGSL `array<f32,N>` in
     uniforms has 16-byte stride! **CRITICAL**: a WGSL `array<vec4<f32>, 8>` palette maps to
     `[Vec4; 8]` fine; a WGSL `vec4` used as 4 floats maps to Vec4 — check EACH array field's
     WGSL declaration and match the WGSL type, not the old Rust shape).
   - `Uniforms::update` logic unchanged (field assignments are type-adjusted only).
3. **Migrate the upload sites**: replace `bytemuck::bytes_of(&self.uniforms)` with the
   UniformBuffer write (step above). Keep a reusable `Vec<u8>` scratch on the Renderer to avoid
   per-frame allocation (encase can write into `&mut Vec` — clear+reuse).
4. **Re-point the layout tests** (ARC-010's): during migration, run them — every sentinel offset
   must be UNCHANGED (encase agreeing with the old hand layout proves equivalence). If any offset
   differs, the OLD hand layout and WGSL disagreed (a latent bug — reconcile against WGSL, which
   is truth) or a type mapping in step 2 is wrong. Do not proceed until all sentinels match.
   Rewrite the tests to use `encase`'s offset reporting or keep `mem::offset_of!` (still works —
   the struct is still a plain struct, just without manual padding; note `offset_of` measures RUST
   offsets which encase may differ from once padding fields are gone — therefore switch the tests
   to compare `ub.as_ref()` byte patterns instead: write a Uniforms with sentinel values
   (1.0f32, 2.0, …) and assert the bytes at the WGSL-documented offsets equal those sentinels.
   **This byte-pattern test is the strongest form — implement it exactly.**)
5. **Migrate the sibling structs**: `PostProcessUniforms`, bloom/composite uniforms,
   `AttractorComputeUniforms` (storage-buffer structs use `StorageBuffer`/`ShaderType` the same
   way; note storage layout (std430-like) differs from uniform layout — encase handles it via the
   buffer type used at write time; the attractor uniforms are `var<uniform>` — check each WGSL
   declaration's address space and use the matching encase buffer writer).
6. **Update CLAUDE.md** (single-writer rule — coordinate with DOC-012 if in flight): the
   "Modifying Uniforms" section's manual-padding procedure becomes: "field order must match WGSL;
   encase derives layout; run the byte-pattern layout test after any change."
7. **Buddhabrot/attractor storage buffers** that use `bytemuck::cast_slice` on plain data arrays
   (not structs) are FINE as-is — don't churn them.

## Files to touch

| File | Change |
|------|--------|
| `Cargo.toml` | + `encase` (glam feature) |
| `src/renderer/uniforms.rs` | Uniforms + post uniforms migration; byte-pattern tests |
| `src/renderer/update.rs` | upload-path change (UniformBuffer scratch) |
| `src/renderer/compute.rs` | AttractorComputeUniforms migration + its upload site |
| `src/app/capture.rs` / `capture_web.rs` | hi-res paths build uniforms too — same upload change (grep `bytes_of`) |
| `CLAUDE.md` | uniform-procedure rewrite (with DOC-012) |

## Verification

1. Byte-pattern layout tests green BEFORE any visual testing (they are the proof of equivalence).
2. `make checkall`.
3. ENH-007 harness: full golden sweep — pixel-identical output (layout equivalence end-to-end).
4. Manual sweep of features whose uniforms live late in the struct (procedural palettes, DoF,
   fog, LOD debug channel) — late-struct fields are where offset drift shows.
5. `make web-build` + browser smoke.
6. Add one NEW field as a dry run (e.g. a `_reserved: f32` at the end), update WGSL, run tests,
   remove it — confirms the new workflow is actually simpler (document the dry run in the PR).

## Rollback

Revert the branch — the old hand-layout struct is fully self-contained in git history. No
settings/schema impact. Because the byte-pattern tests pin the GPU contract, a revert is provably
safe too.

## Pitfalls

- WGSL `array<T, N>` in uniform address space has 16-byte element stride — `[f32; 8]` in Rust is
  NOT `array<f32, 8>` in WGSL uniforms. Check every array field against its WGSL declaration; the
  palette (`array<vec4<f32>, 8>`) is safe as `[Vec4; 8]`.
- encase's `min_size` can be larger than `size_of` the Rust struct (trailing padding) — the GPU
  buffer size must come from encase, not `mem::size_of`. Grep buffer creation for
  `size_of::<Uniforms>()` (`initialization.rs`) and switch to `Uniforms::min_size()`.
- bool fields: WGSL uniforms use u32 flags — keep them u32 in Rust (encase has no bool mapping
  for uniforms; the current code already uses u32 flags — don't "improve" them to bool).
- Do the migration as ONE PR with no functional changes mixed in — the diff is large but must be
  provably behavior-neutral via the tests + harness.
