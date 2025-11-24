//! Web capture implementation using Blob download

use crate::platform::{Capture, PlatformError};
use image::{ImageBuffer, Rgba};
use std::io::Cursor;
use wasm_bindgen::JsCast;

/// Web capture using Blob download
pub struct WebCapture;

impl WebCapture {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl Capture for WebCapture {
    fn save_screenshot(
        &self,
        width: u32,
        height: u32,
        data: &[u8],
        filename_prefix: &str,
    ) -> Result<String, PlatformError> {
        let window = web_sys::window().ok_or(PlatformError::NotSupported("no window".into()))?;
        let document = window
            .document()
            .ok_or(PlatformError::NotSupported("no document".into()))?;

        // Create image from raw RGBA data
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_raw(width, height, data.to_vec())
                .ok_or_else(|| PlatformError::IoError("Failed to create image buffer".into()))?;

        // Encode as PNG
        let mut png_bytes = Vec::new();
        img.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
            .map_err(|e| PlatformError::IoError(e.to_string()))?;

        // Generate filename with timestamp
        let date = js_sys::Date::new_0();
        let timestamp = format!(
            "{:04}{:02}{:02}_{:02}{:02}{:02}",
            date.get_full_year(),
            date.get_month() + 1,
            date.get_date(),
            date.get_hours(),
            date.get_minutes(),
            date.get_seconds()
        );
        let filename = format!("{}_{}.png", filename_prefix, timestamp);

        // Create blob and trigger download
        let array = js_sys::Uint8Array::from(&png_bytes[..]);
        let blob_parts = js_sys::Array::new();
        blob_parts.push(&array);

        let mut options = web_sys::BlobPropertyBag::new();
        options.type_("image/png");

        let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&blob_parts, &options)
            .map_err(|_| PlatformError::IoError("Failed to create blob".into()))?;

        let url = web_sys::Url::create_object_url_with_blob(&blob)
            .map_err(|_| PlatformError::IoError("Failed to create URL".into()))?;

        let anchor: web_sys::HtmlAnchorElement = document
            .create_element("a")
            .map_err(|_| PlatformError::IoError("Failed to create anchor".into()))?
            .dyn_into()
            .map_err(|_| PlatformError::IoError("Failed to cast to anchor".into()))?;

        anchor.set_href(&url);
        anchor.set_download(&filename);
        anchor.click();

        // Clean up
        web_sys::Url::revoke_object_url(&url).ok();

        Ok(filename)
    }

    fn supports_auto_open(&self) -> bool {
        false // Can't auto-open files in browser
    }

    fn open_file(&self, _path: &str) -> Result<(), PlatformError> {
        Err(PlatformError::NotSupported(
            "Cannot open files in browser".into(),
        ))
    }
}
