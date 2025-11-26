# Handoff: Mobile Touch Gesture Tuning

**Date:** 2025-11-26
**Status:** Panning works, pinch zoom needs phantom touch detection tuning
**Latest Commit:** `f6ea910` - fix(touch): add time-distance heuristic to detect and reject phantom touches

> **Note:** This project has a `plan.md` file in the root. Read that file first for context on the overall web/WASM implementation before proceeding with touch gesture improvements.

## Current State

### ✅ What's Working
- **Touch panning (2D)** - Single finger drag pans smoothly after phantom touch fix
- **Pinch zoom centering** - Zoom now centers at the pinch point, not screen center
- **Touch camera rotation (3D)** - Single finger drag rotates camera
- **Preset management** - Built-in presets scroll area 2x taller (800px), export buttons (💾) on all presets
- **Custom preset storage** - Save/load/delete works on web via localStorage
- **iOS Safari viewport** - Properly fills entire screen on iPhone
- **Browser window resize** - Dynamically adjusts when window resized or orientation changes

### ⚠️ Outstanding Issues

#### 1. **CRITICAL: Pinch Zoom Detection Too Strict** (User Feedback)
**Problem:** Pinch zoom is "flaky" - user must place first finger, wait, then place second finger or it won't register as zoom.

**Root Cause:** The phantom touch detection heuristic is **too aggressive**. Current logic in `src/app/input.rs:460`:
```rust
// Phantom touch heuristic: if arrives within 100ms AND is far away (>300px), reject it
if elapsed_ms < 100 && distance > 300.0 {
    // Reject as phantom
}
```

**Why It's Too Strict:**
- **100ms window is too short** for natural pinch gestures
- Users typically place two fingers within **50-200ms** of each other
- The 100ms threshold catches legitimate pinches, forcing users to "wait" before placing second finger

**Analysis:**
From the logs, phantom touches have these characteristics:
- Touch 1: `(108, 81)` - top left (user's actual finger)
- Touch 2: `(306, 720)` - middle bottom (phantom from settings panel or palm)
- **Distance: ~640 pixels**
- **Timing: Within ~10ms** (essentially simultaneous)

Real pinch gestures typically:
- **Distance: 50-400 pixels** (comfortable two-finger spread)
- **Timing: 100-300ms** (deliberate second finger placement)

**Recommended Solution:**

**Option A: Adjust Thresholds (Conservative)**
```rust
// Increase time window but keep distance check
if elapsed_ms < 50 && distance > 400.0 {
    // Only reject truly simultaneous AND far apart touches
}
```
- Time: 100ms → **50ms** (only catches truly simultaneous phantoms)
- Distance: 300px → **400px** (phantom was 640px, valid pinch usually <400px)

**Option B: Dual Heuristic (More Robust)**
```rust
// Reject if EITHER very fast with large distance, OR absurdly fast
let is_instant = elapsed_ms < 20;  // Phantom arrives nearly instantly
let is_far_fast = elapsed_ms < 50 && distance > 500.0;

if is_instant || is_far_fast {
    // Reject as phantom
}
```
- Rejects touches within 20ms (nearly impossible for human)
- Rejects touches within 50ms AND >500px (likely phantom)
- Allows normal pinch gestures (100-300ms, 50-400px)

**Option C: Distance-Only (Simplest)**
```rust
// Only use distance threshold
if distance > 500.0 {
    // Reject - no legitimate pinch gesture spans >500px
}
```
- Removes time constraint entirely
- Relies on fact that phantom is very far away (640px)
- Risk: Fast but wide pinches might work, but safer

**Recommendation:** Start with **Option A** (50ms/400px), test, then try Option B if needed.

**File Location:** `src/app/input.rs:460`

**Testing:** After changing thresholds:
1. Close settings panel, immediately drag → Should pan (no phantom zoom)
2. Place two fingers naturally for pinch → Should zoom immediately (no delay needed)
3. Check logs for "PHANTOM DETECTED" messages - should only appear for actual phantoms

#### 2. **Debug Logging Still Enabled**
**Status:** Extensive `log::info!()` statements throughout touch handlers with 🔧 emoji markers

**Impact:**
- Verbose console output (helps debugging but clutters production)
- Minor performance overhead

**Action Needed:** Once phantom touch detection is tuned and working:
1. Remove all `log::info!()` calls with 🔧 prefix
2. Keep structure and logic, just remove logging
3. Consider keeping a few critical logs at `log::debug!()` level

**Files to Clean:**
- `src/app/input.rs:429-545` - Touch event handlers
- Look for lines containing `🔧` emoji

**Decision:** Keep logs until phantom detection is confirmed working reliably.

## Implementation Summary

### Recent Changes (Session Commits)

**1. Preset UI Improvements** (`06db164`)
- Increased built-in presets scroll area: 400px → 800px
- Added 💾 export button next to each preset (built-in and user)
- Implemented `export_preset_to_json()` for both web and native
- Fixed delete button for user presets on web

**2. Touch Zoom Centering** (`3505235`)
- Implemented zoom-to-pinch-center calculation
- Converts pinch center from screen coords to fractal coords
- Adjusts fractal center so zoom point stays in place
- Previously zoomed at screen center, now zooms where fingers are

**3. Phantom Touch Detection** (`f6ea910`)
- Added `last_touch_time` field to track touch timing
- Implemented time-distance heuristic to reject phantom touches
- Current thresholds: <100ms AND >300px (TOO STRICT - needs adjustment)
- Successfully prevents phantom zoom from settings panel close

### Files Modified This Session
1. **`src/app/input.rs`** - Touch event handling, phantom detection, zoom centering
2. **`src/app/mod.rs`** - Added `last_touch_time` field to App struct
3. **`src/ui/mod.rs`** - Preset scroll area height, export buttons
4. **`src/fractal/presets.rs`** - Export functions for web and native

### Technical Details

#### Phantom Touch Detection Logic
**File:** `src/app/input.rs:433-472`

Current implementation:
```rust
TouchPhase::Started => {
    let now = web_time::Instant::now();

    if self.active_touches.len() == 1 {
        // Second touch arriving
        if let Some(last_time) = self.last_touch_time {
            let elapsed_ms = now.duration_since(last_time).as_millis();
            let distance = /* calculate from first touch */;

            if elapsed_ms < 100 && distance > 300.0 {
                // REJECT: Phantom detected
                self.active_touches.clear();
                // Reset state, return early
            }
        }
    }

    self.active_touches.insert(touch.id, current_pos);
    // Continue processing...
}
```

**Key Points:**
- Phantom rejection happens BEFORE touch is added to `active_touches`
- First touch always sets `last_touch_time`
- Distance calculated between new touch and existing touch
- Both time AND distance conditions must be met to reject

#### Zoom-to-Point Algorithm
**File:** `src/app/input.rs:475-509`

Fractal coordinate transformation:
```rust
// Convert pinch center from screen coords to fractal coords
let screen_x = (center_x / width) * 2.0 - 1.0;  // [-1, 1]
let screen_y = 1.0 - (center_y / height) * 2.0;  // [-1, 1]

// Map to fractal space
let fractal_x = center_2d[0] + screen_x * aspect / zoom;
let fractal_y = center_2d[1] + screen_y / zoom;

// Apply zoom
zoom *= zoom_factor;

// Adjust center to keep fractal_x/y at same screen position
let zoom_ratio = old_zoom / new_zoom;
center_2d[0] += (fractal_x - center_2d[0]) * (1.0 - zoom_ratio);
center_2d[1] += (fractal_y - center_2d[1]) * (1.0 - zoom_ratio);
```

This ensures the point between fingers stays visually fixed during zoom.

## Next Steps

### Immediate (High Priority)
1. **Adjust phantom detection thresholds** (see Options A/B/C above)
   - Start with Option A: `elapsed_ms < 50 && distance > 400.0`
   - Test on actual device (not just desktop mobile emulation)
   - Monitor logs for "PHANTOM DETECTED" and "Valid second touch" messages
   - Iterate based on feel - should feel natural, no forced waiting

2. **Verify pinch zoom feels natural**
   - Two fingers placed naturally (100-300ms apart) should zoom immediately
   - No need to "wait" before placing second finger
   - Fast pinch gestures should work

3. **Confirm phantom touches still blocked**
   - Settings panel close should not trigger zoom
   - Accidental palm touches should not trigger zoom
   - Check logs - phantom detection should still catch actual phantoms

### Short Term
1. **Remove debug logging** once behavior is stable
   - Remove 🔧 prefixed `log::info!()` statements
   - Optionally keep some as `log::debug!()` for troubleshooting
   - Clean up `src/app/input.rs:429-545`

2. **Test on multiple devices**
   - iPhone Safari (primary target)
   - Android Chrome
   - Desktop browser mobile mode (limited - can't test real pinch)

3. **Consider adaptive thresholds** based on screen size
   - Larger screens might need different distance threshold
   - Tablets vs phones have different typical pinch distances

### Future Enhancements
1. **Pinch zoom for 3D camera** - Currently only works in 2D, could control FOV or distance
2. **Two-finger rotate gesture** - Could add rotation support for 2D fractals
3. **Three-finger gestures** - Reset view, switch modes, etc.
4. **Gesture velocity/momentum** - Add inertia to pan/zoom for more natural feel
5. **Accessibility** - Alternative zoom controls for users who can't do pinch gestures

## Testing Instructions

### Local Testing (Desktop)
```bash
# Start local dev server (listens on all interfaces)
trunk serve --address 0.0.0.0 --port 8081

# Test in desktop browser mobile mode
# Open: http://localhost:8081/
# Press F12 → Enable mobile device mode (Cmd+Shift+M)
# Press H to hide UI panel
# Note: Cannot test pinch gestures in desktop emulation
```

### Production Testing (iPhone)
```bash
# Deploy
git add <files>
git commit -m "fix: adjust phantom touch detection thresholds"
git push origin main
make web-deploy

# Monitor deployment
# https://github.com/paulrobello/par-fractal/actions

# Test on iPhone
# https://par-fractal.pardev.net
# Settings → Advanced → Web Inspector (ON) for console logs
```

### Test Checklist
- [ ] Close settings panel, immediately drag → Pans (no zoom)
- [ ] Place two fingers naturally for pinch → Zooms immediately (no delay)
- [ ] Rapid pinch gestures → Work smoothly
- [ ] Check console for phantom detection logs
- [ ] Verify zoom centers at pinch point (not screen center)
- [ ] Single finger drag → Pans smoothly
- [ ] Preset export buttons (💾) → Download JSON files
- [ ] User preset delete (🗑) → Removes from localStorage

## Code References

### Touch Event Handler (2D)
**File:** `src/app/input.rs:426-575`

**Key Sections:**
- Line 433-472: `TouchPhase::Started` - Phantom detection happens here
- Line 473-551: `TouchPhase::Moved` - Pan and pinch logic
- Line 552-575: `TouchPhase::Ended` - Cleanup and state reset

### Phantom Touch Detection
**File:** `src/app/input.rs:448-472`

Critical threshold values:
```rust
let elapsed_ms = now.duration_since(last_time).as_millis();
let distance = (dx * dx + dy * dy).sqrt();

// ⚠️ ADJUST THESE VALUES:
if elapsed_ms < 100 && distance > 300.0 {  // Too strict!
    // Recommended: < 50 && > 400.0
    // Or try dual heuristic approach
}
```

### Zoom-to-Point Calculation
**File:** `src/app/input.rs:475-509`

Screen-to-fractal coordinate conversion and center adjustment.

### Preset Export (Web)
**File:** `src/fractal/presets.rs:915-954`

Blob download implementation with sanitized filenames.

## Known Issues & Caveats

1. **Phantom detection too strict** - Main outstanding issue, requires threshold tuning

2. **Desktop mobile emulation limitations** - Two-finger gestures don't work in browser DevTools mobile mode. Must test on actual device.

3. **HTTPS requirement for WebGPU** - Local dev server uses HTTP, so iPhone shows "WebGPU Not Available". GitHub Pages works because it uses HTTPS.

4. **Touch timing variability** - Different users place fingers at different speeds. Current 100ms threshold is too fast for some users.

5. **Touch position accuracy** - Touch coordinates on mobile can be slightly imprecise, especially on smaller screens.

## Resources

- **Project repo:** https://github.com/paulrobello/par-fractal
- **Live site:** https://par-fractal.pardev.net
- **Actions/Deploy:** https://github.com/paulrobello/par-fractal/actions
- **Local dev server:** http://localhost:8081/ (or http://192.168.1.207:8081/)

## Final Notes

The mobile touch support is **~95% complete**. The core functionality works well:
- ✅ Panning works smoothly after phantom touch detection
- ✅ Zoom centers correctly at pinch point
- ✅ Preset management greatly improved (2x taller scroll, export buttons)
- ⚠️ Pinch zoom detection needs threshold adjustment

**Single remaining task:** Adjust phantom touch detection thresholds so legitimate pinch gestures register immediately without forcing users to wait. This is a **small tuning change** (1-2 lines in `src/app/input.rs:460`) but needs careful testing on actual mobile devices to find the right balance.

The current heuristic successfully blocks phantom touches (settings panel close, palm touches) but is too aggressive, catching some legitimate pinches. The solution is to:
1. Reduce time threshold: 100ms → 50ms (only catch truly simultaneous phantoms)
2. Increase distance threshold: 300px → 400px (phantom was 640px, real pinch <400px)

After threshold adjustment and testing, remove the debug logging (🔧 emoji markers) for production cleanliness.

**Great progress!** The phantom touch detection approach is sound - it just needs fine-tuning. The logs clearly show the difference between phantom touches (~10ms, 640px) and real pinches, so finding the right threshold values should be straightforward with device testing.
