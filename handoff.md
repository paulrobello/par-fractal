# Handoff: Mobile Web Touch Support for Par Fractal

**Date:** 2025-11-26
**Status:** Pan working, pinch zoom working but too slow
**Latest Commit:** `37d4f4a` - fix(web): clear stale touches to prevent phantom touch accumulation

> **Note:** This project has a `plan.md` file in the root. Read that file first for context on the overall web/WASM implementation before proceeding with mobile touch improvements.

## Current State

### ✅ What's Working
- **iOS Safari viewport** - Properly fills entire screen on iPhone
- **Browser window resize** - Dynamically adjusts when window resized or orientation changes
- **Touch panning (2D)** - Single finger drag pans the fractal correctly
- **Touch camera rotation (3D)** - Single finger drag rotates camera
- **Pinch-to-zoom (2D)** - Two finger pinch works but is **very slow**

### ❌ Outstanding Issues

#### 1. **CRITICAL: Pinch Zoom Speed Too Slow** (User Feedback)
**Problem:** Pinch-to-zoom works but is approximately 10x too slow. Users need to pinch dramatically to see any zoom effect.

**Root Cause:** The zoom sensitivity factor is set to 5% in `src/app/input.rs:458`:
```rust
let zoom_factor = 1.0 + (zoom_delta - 1.0) * 0.05;  // 5% sensitivity
```

**Solution:** Increase sensitivity by at least 10x. Try:
```rust
let zoom_factor = 1.0 + (zoom_delta - 1.0) * 0.5;   // 50% sensitivity (10x faster)
// OR
let zoom_factor = zoom_delta;  // 100% sensitivity (no dampening)
```

**File Location:** `src/app/input.rs:458` in the `TouchPhase::Moved` handler for 2-finger pinch

**Testing:** After changing, test on actual iPhone to ensure zoom feels natural. May need to iterate on the exact value.

#### 2. **Debug Logging Still Enabled**
**Status:** Extensive `log::info!()` statements throughout touch handlers for debugging

**Impact:** Performance overhead (minor), verbose console logs

**Action Needed:** Remove or comment out debug logging before next production release
- Lines to clean up: `src/app/input.rs:424-537` (all the log::info! statements)
- Keep code structure, just remove logging

**Decision:** Keep logs for now if still debugging zoom speed, remove after confirming fix works.

## Implementation Summary

### Files Modified
1. **`index.html`** - Viewport meta tags, CSS for iOS Safari touch handling
2. **`src/web_main.rs`** - Canvas sizing with device pixel ratio, resize event listeners
3. **`src/app/mod.rs`** - Added `active_touches` HashMap and `initial_pinch_distance` fields
4. **`src/app/input.rs`** - Complete touch event handling implementation
5. **`src/camera.rs`** - Touch support for 3D camera rotation
6. **`CHANGELOG.md`** - Documented all mobile web fixes

### Key Technical Solutions

#### Touch Event Routing
**Problem:** On iOS/web, `egui.wants_pointer_input()` returns true by default even when not touching UI, blocking all touch events.

**Solution:** Special-case touch events to bypass egui pointer checks (`src/app/input.rs:21-33`):
```rust
let is_touch = matches!(event, WindowEvent::Touch(_));
let egui_blocks_mouse = if !is_touch {
    // Full egui checks for mouse
    ctx.wants_pointer_input() || ...
} else {
    false  // Touch: always allow (consumed checked earlier)
};
```

#### Mouse/Touch Event Collision
**Problem:** On web, touch events generate BOTH `WindowEvent::Touch` AND `WindowEvent::CursorMoved`/`MouseInput`, causing double-processing.

**Solution:** Guard mouse handlers to ignore events when touches active (`src/app/input.rs:412-417, 529`):
```rust
if self.active_touches.is_empty() {
    // Only process mouse events if no touches active
}
```

#### Stale Touch Cleanup
**Problem:** `TouchPhase::Ended` events occasionally lost, causing ghost touches to accumulate in HashMap. With 3+ touches, matches neither pan (needs 1) nor pinch (needs 2).

**Solution:** Clear stale touches on new touch if count >= 2 (`src/app/input.rs:442-449`):
```rust
if self.active_touches.len() >= 2 {
    self.active_touches.clear();
    self.initial_pinch_distance = None;
}
```

### Touch Gesture Logic

**Single Finger (Pan - 2D mode):**
```rust
TouchPhase::Started → Set mouse_pressed, track position
TouchPhase::Moved → Calculate delta, update center_2d
TouchPhase::Ended → Clear mouse_pressed
```

**Two Fingers (Pinch Zoom - 2D mode):**
```rust
TouchPhase::Started (2nd finger) → Calculate initial distance between fingers
TouchPhase::Moved → Calculate new distance, apply zoom_factor
TouchPhase::Ended → Reset pinch state, resume pan if 1 finger remains
```

## Next Steps

### Immediate (High Priority)
1. **Increase pinch zoom sensitivity** - Change line 458 in `src/app/input.rs` from `* 0.05` to `* 0.5` or higher
2. **Test on iPhone** - Verify zoom feels natural at new speed
3. **Iterate if needed** - May need to adjust between 0.3-1.0 based on feel

### Short Term
1. **Remove debug logging** - Clean up log::info statements once zoom speed confirmed working
2. **Test 3D mode on mobile** - Camera rotation works, but should verify feel is good
3. **Consider zoom center calculation** - Currently centers between fingers, may want to zoom toward touch midpoint in fractal coordinates

### Future Enhancements
1. **Pinch zoom for 3D camera** - Currently only works in 2D, could control camera FOV or distance in 3D
2. **Two-finger rotate gesture** - Could add rotation support for 2D fractals
3. **Three-finger gestures** - Reset view, switch modes, etc.
4. **Gesture velocity/momentum** - Add inertia to pan/zoom for more natural feel

## Testing Instructions

### Local Testing
```bash
# Start local dev server (listens on all interfaces)
trunk serve --address 0.0.0.0 --port 8081

# Access from iPhone (same WiFi)
# Open Safari: http://192.168.1.207:8081/
# Note: Will show "WebGPU Not Available" - this is expected (requires HTTPS)

# Test in desktop browser mobile mode instead
# Open: http://localhost:8081/
# Press F12 → Enable mobile device mode (Cmd+Shift+M)
# Press H to hide UI panel
# Drag to test pan (works)
# Shift+drag does NOT simulate pinch (desktop limitation)
```

### Production Testing
```bash
# Deploy
git add <files>
git commit -m "fix: increase pinch zoom sensitivity 10x"
git push origin main
make web-deploy

# Monitor deployment
# https://github.com/paulrobello/par-fractal/actions

# Test on iPhone
# https://par-fractal.pardev.net
# Press H to hide UI
# Single finger drag → Pan
# Two finger pinch → Zoom (verify speed feels good)
```

### Accessing iPhone Logs
**Option 1: Safari Web Inspector (Mac + iPhone)**
1. iPhone: Settings → Safari → Advanced → Web Inspector (ON)
2. Mac: Safari → Develop → [iPhone Name] → par-fractal.pardev.net
3. View console logs in real-time

**Option 2: Desktop Browser Mobile Mode**
1. Chrome/Safari → DevTools → Device Mode
2. Console tab shows logs
3. Can test pan (works), but pinch doesn't work in emulation

## Code References

### Touch Event Handler (2D)
**File:** `src/app/input.rs:420-557`
**Key sections:**
- Line 440-456: TouchPhase::Started
- Line 458-545: TouchPhase::Moved (pan and pinch logic)
- Line 546-566: TouchPhase::Ended (cleanup)

### Pinch Zoom Calculation
**File:** `src/app/input.rs:458-483`
```rust
if self.active_touches.len() == 2 {
    // Calculate distance between two fingers
    let touches: Vec<&(f32, f32)> = self.active_touches.values().collect();
    let current_distance = /* pythagoras */;

    if let Some(initial_distance) = self.initial_pinch_distance {
        let zoom_delta = current_distance / initial_distance;
        let zoom_factor = 1.0 + (zoom_delta - 1.0) * 0.05;  // ← CHANGE THIS
        self.fractal_params.zoom_2d *= zoom_factor;
        self.initial_pinch_distance = Some(current_distance);
    }
}
```

### Mouse Event Guards
**File:** `src/app/input.rs:412-417, 529`

### 3D Touch Support
**File:** `src/camera.rs:152-186`

## Known Issues & Caveats

1. **Desktop mobile emulation limitations** - Two-finger gestures don't work in browser DevTools mobile mode. Must test on actual device.

2. **HTTPS requirement for WebGPU** - Local dev server uses HTTP, so iPhone shows "WebGPU Not Available". GitHub Pages works because it uses HTTPS.

3. **egui-winit touch handling** - On web, egui-winit doesn't properly update pointer position from touch events, causing stale `is_pointer_over_area()` data. Solved by bypassing egui checks for touches.

4. **Touch event ordering** - Occasionally `TouchPhase::Ended` events are lost (browser/OS issue). Mitigated by clearing stale touches on new gesture start.

5. **HashMap ordering** - Using HashMap for `active_touches` means finger order is random. This is fine for pinch (just measures distance), but could be issue for future gestures that care about finger identity.

## Documentation

- **CHANGELOG.md** - Full details of mobile web implementation in v0.5.0 section
- **README.md** - No changes needed (general-purpose)
- **CLAUDE.md** - Project-specific instructions (already has web build info)

## Deployment Pipeline

**Current version:** 0.5.0
**Branch:** main
**Production URL:** https://par-fractal.pardev.net

**Workflow:**
1. Commit changes to main branch
2. Run `make web-deploy` to trigger GitHub Actions workflow
3. Workflow builds WASM with trunk, deploys to GitHub Pages
4. Site updates in ~2-3 minutes

**GitHub Actions:** `.github/workflows/deploy-web.yml`

## Resources

- **Project repo:** https://github.com/paulrobello/par-fractal
- **Live site:** https://par-fractal.pardev.net
- **Local dev server IPs:** 192.168.1.207, 192.168.1.106 (port 8081)

## Final Notes

The mobile web touch support is **95% complete**. The core functionality works:
- Viewport fills screen correctly on iPhone
- Window resize and orientation change handled
- Touch events route correctly (not blocked by egui)
- Pan works smoothly
- Pinch zoom works but needs speed adjustment

**Single remaining task:** Increase pinch zoom sensitivity from 5% to ~50% (10x faster). This is a **one-line change** that should take 5 minutes to implement and test.

After this fix, consider removing debug logging for production and updating CHANGELOG with final polish notes.

Great work getting touch support working on iOS Safari - this was a tricky problem with multiple subtle issues (egui pointer blocking, touch/mouse event collision, stale touch cleanup). The solution is robust and should work well once zoom speed is adjusted.
