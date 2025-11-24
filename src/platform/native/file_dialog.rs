//! Native file dialog implementation using rfd

use crate::platform::{FileDialog, PlatformError};
use std::fs;

/// Native file dialog using rfd crate
pub struct NativeFileDialog;

impl NativeFileDialog {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NativeFileDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl FileDialog for NativeFileDialog {
    fn save_file(
        &self,
        filename: &str,
        data: &[u8],
        _mime_type: &str,
    ) -> Result<(), PlatformError> {
        let dialog = rfd::FileDialog::new()
            .set_file_name(filename)
            .add_filter("All Files", &["*"]);

        if let Some(path) = dialog.save_file() {
            fs::write(&path, data)?;
            Ok(())
        } else {
            Err(PlatformError::OperationCancelled)
        }
    }

    fn open_file(
        &self,
        filters: &[(&str, &[&str])],
    ) -> Result<Option<(String, Vec<u8>)>, PlatformError> {
        let mut dialog = rfd::FileDialog::new();

        for (name, extensions) in filters {
            dialog = dialog.add_filter(*name, extensions);
        }

        if let Some(path) = dialog.pick_file() {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            let data = fs::read(&path)?;
            Ok(Some((filename, data)))
        } else {
            Ok(None)
        }
    }
}
