//! Web file dialog implementation using HTML5 File API

use crate::platform::{FileDialog, PlatformError};
use wasm_bindgen::JsCast;

/// Web file dialog using HTML5 APIs
pub struct WebFileDialog;

impl WebFileDialog {
    pub fn new() -> Self {
        Self
    }

    /// Trigger a file download in the browser
    fn download_blob(&self, filename: &str, data: &[u8], mime_type: &str) -> Result<(), PlatformError> {
        let window = web_sys::window().ok_or(PlatformError::NotSupported("no window".into()))?;
        let document = window
            .document()
            .ok_or(PlatformError::NotSupported("no document".into()))?;

        // Create a Uint8Array from the data
        let array = js_sys::Uint8Array::from(data);
        let blob_parts = js_sys::Array::new();
        blob_parts.push(&array);

        // Create blob with MIME type
        let mut options = web_sys::BlobPropertyBag::new();
        options.type_(mime_type);

        let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&blob_parts, &options)
            .map_err(|_| PlatformError::IoError("Failed to create blob".into()))?;

        // Create object URL
        let url = web_sys::Url::create_object_url_with_blob(&blob)
            .map_err(|_| PlatformError::IoError("Failed to create URL".into()))?;

        // Create and click anchor element to trigger download
        let anchor: web_sys::HtmlAnchorElement = document
            .create_element("a")
            .map_err(|_| PlatformError::IoError("Failed to create anchor".into()))?
            .dyn_into()
            .map_err(|_| PlatformError::IoError("Failed to cast to anchor".into()))?;

        anchor.set_href(&url);
        anchor.set_download(filename);
        anchor.click();

        // Clean up
        web_sys::Url::revoke_object_url(&url).ok();

        Ok(())
    }
}

impl Default for WebFileDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl FileDialog for WebFileDialog {
    fn save_file(&self, filename: &str, data: &[u8], mime_type: &str) -> Result<(), PlatformError> {
        self.download_blob(filename, data, mime_type)
    }

    fn open_file(
        &self,
        _filters: &[(&str, &[&str])],
    ) -> Result<Option<(String, Vec<u8>)>, PlatformError> {
        // File input requires async handling with callbacks
        // For now, return not supported - this would need to be implemented
        // using a hidden file input element and event listeners
        Err(PlatformError::NotSupported(
            "File open dialog not yet implemented for web".into(),
        ))
    }
}
