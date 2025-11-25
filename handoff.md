# Handoff Document - Strange Attractor Screenshot Bug

**Date**: 2025-11-25
**Status**: FIXED

## Summary

The screenshot capture bug for strange attractors has been resolved. The issue was caused by improper handling of window resize events in the accumulation texture lifecycle.

## Root Cause

When the window was resized:
1. `resize()` in `update.rs` cleared `accumulation_texture` and `accumulation_display_bind_group` to `None`
2. BUT it did NOT clear `attractor_compute`
3. On the next frame, `init_accumulation_compute()` would return early because `attractor_compute.is_some()`
4. This left `accumulation_texture` and `accumulation_display_bind_group` as `None`
5. The render loop would then fail to dispatch compute or display anything

When the texture eventually got recreated (or was present from before resize), there was a mismatch between:
- The compute uniforms using NEW window dimensions
- The accumulation texture having OLD dimensions

This caused the attractor to be rendered into a portion of the texture, which when displayed would appear "zoomed" or showing only a portion of the pattern.

## Fix Applied

1. **`src/renderer/initialization.rs:900-941`** - Refactored `init_accumulation_compute()`:
   - Now checks if texture needs recreation (None OR wrong dimensions)
   - Separates compute pipeline creation from texture creation
   - Properly recreates texture and bind group when dimensions change

2. **`src/app/render.rs:27-43`** - Added texture recreation detection:
   - Checks before calling `init_accumulation_compute()` if texture needs recreation
   - Resets `attractor_total_iterations` counter when texture is recreated

3. **`src/app/capture.rs:408-452`** - Fixed high-resolution render for strange attractors:
   - Added check for accumulation mode (`use_accumulation`)
   - When rendering attractors, uses `accumulation_display_pipeline` instead of `render_pipeline`
   - Samples from existing accumulation texture for high-res output

4. **`tests/integration_tests.rs:218`** - Removed reference to deleted `Icon2D` variant

## Verification

- All 54 tests pass
- Clippy and format checks pass
- Application builds and runs successfully in release mode
- High-resolution screenshot capture working

## Previous Work This Session

- Fixed Icon attractor formula (was completely wrong) - removed entirely
- Fixed Quadruptwo attractor formula (atan argument was wrong)
- Added UI controls for attractor parameters
- Increased Threeply zoom 10x
- Removed Icon attractor from codebase

## Test Commands

```bash
# Build and run
make r

# Run all quality checks
make checkall

# Run tests only
make test
```
