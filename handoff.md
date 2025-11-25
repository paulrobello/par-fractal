# Handoff Document - Web Video Recording

**Date**: 2025-11-24
**Previous Developer**: Claude
**Status**: FIXED - web video recording now uses async buffer mapping

## Before Starting

Read `plan.md` in the project root for overall project context and planning notes.

## Fix Applied (2025-11-24)

**Changes Made:**

1. **`src/app/mod.rs`**:
   - Added `Rc<RefCell<>>` imports for web builds
   - Changed `video_recorder` type from `WebVideoRecorder` to `Rc<RefCell<WebVideoRecorder>>` for web

2. **`src/app/capture_web.rs`**:
   - Added new `capture_video_frame_web()` function that captures frames asynchronously
   - Uses `wasm_bindgen_futures::spawn_local` and `gloo_timers` for non-blocking buffer mapping
   - Same async pattern as the working `capture_screenshot_web()` function

3. **`src/app/render.rs`**:
   - Replaced synchronous blocking video frame capture with call to `capture_video_frame_web()`
   - Updated all `video_recorder` accesses to use `.borrow()` and `.borrow_mut()` for web
   - Fixed recording indicator to show actual recording state on web

**Why this works:**
- The async pattern avoids blocking the main thread during GPU buffer mapping
- `Rc<RefCell<>>` allows sharing the recorder between the render loop and async callbacks
- Frame capture happens in the background without freezing the browser

## Previous Issue (Now Fixed)

**Problem**: Clicking the "Record" button on the web version causes the browser tab to become unresponsive and eventually get killed by the browser.

**Root Cause Analysis**: The issue is in `src/app/render.rs` around line 270-360. The web video frame capture code attempts to do **synchronous GPU buffer mapping** which blocks the main thread:

```rust
// This is the problematic code (lines ~320-331):
let (sender, receiver) = std::sync::mpsc::channel();
buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
    let _ = sender.send(result);
});

// Poll device to process the mapping - THIS BLOCKS!
let _ = self.renderer.device.poll(wgpu::PollType::Wait {
    submission_index: None,
    timeout: None,
});

if receiver.recv().map(|r| r.is_ok()).unwrap_or(false) {
    // Read buffer data...
}
```

On web (WASM), `device.poll()` with `Wait` and `std::sync::mpsc::channel` do not work the same as native. The browser's single-threaded model means this blocks the entire page.

## Required Fix

The web frame capture needs to be **asynchronous**. Options:

### Option 1: Async Frame Collection (Recommended)
Instead of capturing frames synchronously, collect them asynchronously using `wasm_bindgen_futures::spawn_local`:

```rust
// In render.rs, for web video recording:
#[cfg(target_arch = "wasm32")]
if is_recording {
    // Submit copy command but don't wait for it
    // Use async buffer mapping like capture_web.rs does for screenshots
    // Store frames in a queue that the WebVideoRecorder processes
}
```

Look at `src/app/capture_web.rs` lines 82-149 for the correct async pattern using:
- `wasm_bindgen_futures::spawn_local`
- `gloo_timers::future::TimeoutFuture` for polling

### Option 2: Skip Frame Capture on Web During Recording
A simpler workaround - don't capture every frame. Instead:
1. Only capture keyframes (e.g., every 30th frame)
2. Use a lower capture rate for web

### Option 3: Use Canvas Recording
Use the browser's `MediaRecorder` API to record the canvas directly instead of manual frame capture.

## Files to Modify

1. **`src/app/render.rs`** (lines 265-360)
   - Remove synchronous buffer polling
   - Implement async frame collection similar to `capture_web.rs`

2. **`src/platform/web/video_recorder.rs`**
   - May need to accept frames asynchronously
   - Consider adding a frame queue that gets processed by the async callback

## Related Code Reference

| File | Purpose |
|------|---------|
| `src/app/render.rs` | Main render loop, contains broken frame capture |
| `src/app/capture_web.rs` | Working async screenshot capture - use as reference |
| `src/platform/web/video_recorder.rs` | WebVideoRecorder implementation |
| `src/platform/web/gif_encoder.rs` | GIF encoder (works correctly) |
| `src/platform/web/webm_muxer.rs` | WebM muxer (not currently used) |

## What Was Implemented (Working)

- WebVideoRecorder structure with format detection
- GIF encoder using `gif` + `color_quant` crates
- ZIP fallback (frames as PNG archive)
- WebM muxer foundation (for future WebCodecs use)
- Toast notifications for recording status

## What Was Tested

- Native video recording: Works correctly
- Web screenshot capture: Works correctly
- Web high-resolution render: Works (was fixed earlier in session)
- Web video recording: **BROKEN** - causes browser hang

## Build & Test Commands

```bash
# Native build and test
make checkall

# Web build check
cargo check --no-default-features --features web --target wasm32-unknown-unknown

# Build and serve web version
trunk serve --open
```

## Notes

- The `gif` crate (v0.14) and `color_quant` (v1.1) were added as web dependencies
- web-sys was updated to v0.3.82 with WebCodecs features (for future use)
- The WebCodecs API integration is prepared but not active (falls back to GIF)

## Suggested Approach

1. Start by understanding the async pattern in `capture_web.rs`
2. Refactor the web video frame capture to use the same async approach
3. Consider using `Rc<RefCell<>>` or similar to share the recorder state with async callbacks
4. Test with a simple recording (just a few frames) before full integration
