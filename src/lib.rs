//! Par Fractal — a cross-platform GPU-accelerated fractal renderer built with
//! Rust and wgpu.
//!
//! Par Fractal renders both 2D escape-time fractals (Mandelbrot, Julia, Burning
//! Ship, etc.) and 3D ray-marched fractals (Mandelbulb, Mandelbox, Menger
//! Sponge, etc.) with PBR shading, soft shadows, ambient occlusion, depth of
//! field, and a multi-pass post-processing pipeline (bloom, FXAA, color
//! grading, vignette).
//!
//! # Module map
//!
//! - `app` — application state, the winit event loop, and orchestration of
//!   input, update, render, and capture
//! - `camera` — 3D camera state and a keyboard/mouse/touch camera controller
//! - `fractal` — fractal parameter model (`FractalParams`), fractal types,
//!   color palettes, presets, and serializable settings
//! - `renderer` — wgpu device/surface setup, uniform buffers, and the render
//!   and post-processing pipelines
//! - `lod` — adaptive Level-of-Detail system that trades quality for
//!   performance based on distance, motion, and measured FPS
//! - `ui` — egui-based immediate-mode user interface
//! - `command_palette` — quick command palette for fractal, effect, color,
//!   camera, and recording actions
//! - `platform` — platform-specific helpers
//! - `video_recorder` — ffmpeg-backed video capture (native targets only)
//! - `web_main` — WASM web entry point (only compiled on `wasm32`)
//!
//! For the system design and data flow see `docs/ARCHITECTURE.md`; for the
//! feature catalog see `docs/FEATURES.md`.
//!
//! # Crate types
//!
//! The crate builds both as an `rlib` (for consumption from crates.io as a
//! Rust library) and as a `cdylib` (for the web/WASM build loaded from
//! JavaScript). The native binary entry point lives in `src/main.rs`.

/// Application state, the winit event loop, and orchestration of input,
/// update, render, and capture.
pub mod app;
/// 3D camera state and a keyboard/mouse/touch camera controller.
pub mod camera;
/// Quick command palette for fractal, effect, color, camera, and recording
/// actions.
pub mod command_palette;
/// Fractal parameter model (`FractalParams`), fractal types, color palettes,
/// presets, and serializable settings.
pub mod fractal;
/// Adaptive Level-of-Detail system that trades quality for performance based
/// on distance, motion, and measured FPS.
pub mod lod;
/// Platform-specific helpers.
pub mod platform;
/// wgpu device/surface setup, uniform buffers, and the render and
/// post-processing pipelines.
pub mod renderer;
/// egui-based immediate-mode user interface.
pub mod ui;

/// ffmpeg-backed video capture (native targets only).
#[cfg(not(target_arch = "wasm32"))]
pub mod video_recorder;

/// WASM web entry point (only compiled on `wasm32`).
#[cfg(target_arch = "wasm32")]
pub mod web_main;

// Re-export commonly used types

/// Re-exports the camera types for convenience.
pub use camera::{Camera, CameraController};
/// Re-exports the core fractal types for convenience.
pub use fractal::{
    ColorPalette, FractalParams, FractalType, Preset, PresetGallery, RenderMode, ShadingModel,
};
/// Re-exports the renderer and GPU-info types for convenience.
pub use renderer::{GpuInfo, Renderer};
/// Re-exports the UI type for convenience.
pub use ui::UI;
