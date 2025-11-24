// Library exports for testing and external use
pub mod app;
pub mod camera;
pub mod command_palette;
pub mod fractal;
pub mod lod;
pub mod platform;
pub mod renderer;
pub mod ui;

#[cfg(not(target_arch = "wasm32"))]
pub mod video_recorder;

// Web entry point
#[cfg(target_arch = "wasm32")]
pub mod web_main;

// Re-export commonly used types
pub use camera::{Camera, CameraController};
pub use fractal::{
    ColorPalette, FractalParams, FractalType, Preset, PresetGallery, RenderMode, ShadingModel,
};
pub use renderer::{GpuInfo, Renderer};
pub use ui::UI;
