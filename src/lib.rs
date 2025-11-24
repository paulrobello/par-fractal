// Library exports for testing and external use
pub mod camera;
pub mod command_palette;
pub mod fractal;
pub mod lod;
pub mod renderer;
pub mod ui;
pub mod video_recorder;

// Re-export commonly used types
pub use camera::{Camera, CameraController};
pub use fractal::{
    ColorPalette, FractalParams, FractalType, Preset, PresetGallery, RenderMode, ShadingModel,
};
pub use renderer::{GpuInfo, Renderer};
pub use ui::UI;
