//! Native capture implementation using image crate and filesystem

use crate::platform::{Capture, PlatformError};
use chrono::Local;
use image::{ImageBuffer, Rgba};
use std::path::Path;

/// Native capture using image crate
pub struct NativeCapture;

impl NativeCapture {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NativeCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl Capture for NativeCapture {
    fn save_screenshot(
        &self,
        width: u32,
        height: u32,
        data: &[u8],
        filename_prefix: &str,
    ) -> Result<String, PlatformError> {
        // Create image from raw RGBA data
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_raw(width, height, data.to_vec())
                .ok_or_else(|| PlatformError::IoError("Failed to create image buffer".into()))?;

        // Generate filename with timestamp
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}.png", filename_prefix, timestamp);

        // Save to current directory
        img.save(&filename)
            .map_err(|e| PlatformError::IoError(e.to_string()))?;

        Ok(filename)
    }

    fn supports_auto_open(&self) -> bool {
        true
    }

    fn open_file(&self, path: &str) -> Result<(), PlatformError> {
        open::that(Path::new(path)).map_err(|e| PlatformError::IoError(e.to_string()))
    }
}
