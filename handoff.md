# Handoff Document - Par Fractal Development

**Date**: 2025-11-24
**Status**: Active Development - Sierpinski Triangle Rendering Issue

---

## Current Issue: Sierpinski Triangle Not Rendering Visibly

### Problem Statement
The Sierpinski Triangle 2D fractal (`SierpinskiTriangle2D`) has been implemented but is **not rendering visibly** on screen. User reports "not visible" after multiple attempted fixes.

### What Has Been Completed

#### ✅ Successfully Implemented
1. **Added SierpinskiTriangle2D fractal type** to the codebase:
   - Added enum variant in `src/fractal/types.rs:9`
   - Added filename-safe name mapping
   - Updated all pattern matching in `src/fractal/mod.rs` (lines 307, 445)
   - Updated fractal type indexing in `src/renderer/uniforms.rs:290` (mapped to index 3)
   - Updated shader dispatch logic in `src/shaders/fractal.wgsl:2298-2299`
   - Renumbered all 3D fractals (shifted from indices 12-24 to 13-25)

2. **Added UI button** for Sierpinski Triangle:
   - Located in `src/ui/mod.rs:360`
   - Renamed "Sierpinski" → "Sierpinski Carpet" for clarity
   - Added "Sierpinski Triangle" button next to Carpet
   - Layout: Row 2 of 2D fractals

3. **Fixed Unicode rendering issues** throughout UI:
   - Command palette footer: replaced `↑↓` with `[Up/Down]` (`src/ui/command.rs:135`)
   - Performance overlay: replaced `→` with `>` (`src/ui/overlays.rs:401`)
   - Color key labels: replaced `←...→` with `<-...->` (4 locations in `src/ui/mod.rs`)
   - LOD distance sliders: replaced arrows with `->` (3 locations in `src/ui/mod.rs`)

4. **Updated Controls panel** with complete keyboard shortcuts:
   - All 2D fractals (0-9 keys): `src/ui/mod.rs:2215-2226`
   - All 3D fractals (F1-F11): `src/ui/mod.rs:2229-2241`
   - Effects, camera, parameters: `src/ui/mod.rs:2244-2262`

5. **Made other improvements**:
   - Increased preset list panel height from 200px to 400px (`src/ui/mod.rs:491`)
   - Disabled floor for Apollonian Gasket preset (`src/fractal/presets.rs:424`)
   - Added `presets_open` state to UIState for persistence (`src/fractal/ui_state.rs:6,88`)
   - All drawer states now properly persisted

### ❌ Current Blocking Issue: Sierpinski Triangle Shader

**Location**: `src/shaders/fractal.wgsl:490-530` (function `sierpinski_triangle`)

**Problem**: Multiple shader implementations attempted, all failing to render visibly:

#### Attempt #1: Binary Bitwise Method
- Used Pascal's triangle modulo 2: `(x & y) == 0`
- Result: Rendered but extremely faint/barely visible

#### Attempt #2: IFS (Iterated Function System)
- Three affine transformations for equilateral triangle
- Result: Not visible

#### Attempt #3: Recursive Subdivision (like Sierpinski Carpet)
- Used 3x3 grid with `cell_y > cell_x` constraint
- Result: Worse than previous attempts

#### Attempt #4: Right Triangle Approach (Current)
- Maps to [0,1] x [0,1] space
- Triangle where `y ≤ x` (lower-left diagonal half)
- Upper-right quadrant (x≥1 AND y≥1) removed as hole
- Base brightness: `0.5 + (iteration / 40.0)`
- **Status**: Still not visible per user feedback

### What Should Be Working (Reference)

The **Sierpinski Carpet** (`sierpinski` function at line 449) renders **perfectly** with this algorithm:
```wgsl
fn sierpinski(coord: vec2<f32>) -> f32 {
    var p = coord * 0.5 + 0.5;  // Map to [0,1]
    var scale = 1.0;

    for (var i = 0u; i < uniforms.max_iterations; i = i + 1u) {
        let scaled_x = p.x * scale;
        let scaled_y = p.y * scale;

        let cell_x = floor(fract(scaled_x / 3.0) * 3.0);
        let cell_y = floor(fract(scaled_y / 3.0) * 3.0);

        if (cell_x == 1.0 && cell_y == 1.0) {
            return 0.0;  // Removed center cell
        }

        scale = scale * 3.0;
        if (scale > 1000000.0) { break; }
    }

    return f32(iteration) / f32(uniforms.max_iterations);
}
```

This works perfectly - bright, clear, zoom-able. The triangle version needs similar clarity.

---

## Recommended Next Steps

### Option 1: Debug Current Implementation
1. **Test with solid color return** - Replace line 529 with `return 1.0;` to verify shader is being called
2. **Check coordinate bounds** - Add debug visualization of `p.x` and `p.y` values
3. **Verify fractal type dispatch** - Ensure `uniforms.fractal_type == 3u` correctly routes to this function

### Option 2: Use Proven Algorithm Pattern
The Sierpinski Carpet works perfectly. Try adapting it directly:
1. Keep the exact same 3x3 grid subdivision (`scale * 3.0`)
2. Keep the exact same return value (`f32(iteration) / f32(uniforms.max_iterations)`)
3. Just change the removal condition from center cell to triangular pattern:
   ```wgsl
   // Instead of: if (cell_x == 1.0 && cell_y == 1.0)
   // Try: if (cell_y >= cell_x) or similar triangular constraint
   ```

### Option 3: Research Reference Implementation
- Check Shadertoy or other WGSL/GLSL Sierpinski triangle implementations
- The reference image provided by user shows clear, bright, recursive triangular holes
- Key requirement: equilateral or right triangle with upward-pointing triangular holes

### Option 4: Simplify to Absolute Basics
Start with the simplest possible approach:
```wgsl
fn sierpinski_triangle(coord: vec2<f32>) -> f32 {
    var p = coord * 0.5 + 0.5;  // Exact same as carpet
    // Just test if point is in triangle: if (p.y > p.x) return 0.0;
    // Return solid color: return 1.0;
}
```
Then incrementally add subdivision logic once basic triangle renders.

---

## Key Files to Review

### Shader Files
- `src/shaders/fractal.wgsl:490-530` - **Sierpinski Triangle function (BROKEN)**
- `src/shaders/fractal.wgsl:449-488` - Sierpinski Carpet function (WORKING - use as reference)
- `src/shaders/fractal.wgsl:2298-2299` - Dispatch logic for fractal type 3

### Type Definitions
- `src/fractal/types.rs:9` - `SierpinskiTriangle2D` enum variant
- `src/fractal/types.rs:43` - Filename mapping
- `src/renderer/uniforms.rs:290` - Fractal type index mapping (3u)

### UI Integration
- `src/ui/mod.rs:360` - Sierpinski Triangle button
- `src/app/input.rs:78-81` - Keyboard shortcut (Key 3 maps to Sierpinski Carpet, not Triangle)

### Documentation
- `docs/FRACTALS2D.md` - Should document Sierpinski Triangle once working
- `CLAUDE.md` - Project development guidelines

---

## Testing Instructions

### To Test Sierpinski Triangle:
```bash
make r                           # Run in release mode
# Then in UI: Click "Sierpinski Triangle" button (Row 2 of 2D fractals)
# Or keyboard: Currently NO hotkey assigned
```

### Expected Result:
Should look like reference image - equilateral triangle with recursive triangular holes, bright and clearly visible like the Sierpinski Carpet.

### Current Result:
Not visible / extremely faint / black screen.

---

## Additional Context

### Why This Matters
- User explicitly requested triangle version separate from carpet version
- Reference image shows it should be prominently visible
- All infrastructure is in place - only the shader rendering logic is broken

### What's NOT the Problem
- ✅ Type system integration - all enums/matches updated correctly
- ✅ UI integration - button works, state persists
- ✅ Shader dispatch - correct function is being called
- ✅ Uniform buffer sync - fractal type index mapping correct
- ❌ **Only the actual rendering algorithm in the shader is broken**

### Constraints
- Must render in 2D mode (not 3D ray marching)
- Should support zoom like other 2D fractals
- Should be bright/visible like Sierpinski Carpet
- Should show recursive triangular holes (not square holes)

---

## Questions to Resolve

1. **Why is the current right-triangle approach not rendering?**
   - Is the coordinate mapping wrong?
   - Is the hole detection logic wrong?
   - Is the return value too dim?

2. **Should we use equilateral or right triangle?**
   - Reference image shows equilateral
   - Right triangle is simpler mathematically
   - User may accept either if it's visible

3. **Can we directly adapt the working Carpet algorithm?**
   - The Carpet uses 3x3 grid subdivision
   - Could we use same approach with triangular constraints?

---

## Build and Test Commands

```bash
# Format and lint
make checkall

# Quick compile check
cargo check

# Run in release mode (required for good performance)
make r

# Run with cleared settings
make run-reset
```

---

## Contact / Escalation

If you encounter persistent rendering issues:
1. Compare byte-for-byte with working Sierpinski Carpet shader
2. Use a reference Shadertoy implementation
3. Consider temporarily using a simpler pattern (right triangle) instead of equilateral
4. Test with WebGL/WebGPU documentation examples

**Priority**: HIGH - User is actively waiting for this feature to work

---

**End of Handoff**
