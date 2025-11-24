//! Platform abstraction layer for cross-platform support (native and web)
//!
//! This module provides trait abstractions for platform-specific functionality:
//! - Storage: Save/load settings, presets, bookmarks, palettes
//! - FileDialog: Open/save file dialogs
//! - Capture: Screenshot saving

use std::fmt;

#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(target_arch = "wasm32")]
pub mod web;

/// Platform-specific error type
#[derive(Debug)]
pub enum PlatformError {
    StorageNotAvailable,
    SerializationError(String),
    IoError(String),
    OperationCancelled,
    NotSupported(String),
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlatformError::StorageNotAvailable => write!(f, "Storage not available"),
            PlatformError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            PlatformError::IoError(msg) => write!(f, "I/O error: {}", msg),
            PlatformError::OperationCancelled => write!(f, "Operation cancelled"),
            PlatformError::NotSupported(msg) => write!(f, "Not supported: {}", msg),
        }
    }
}

impl std::error::Error for PlatformError {}

impl From<std::io::Error> for PlatformError {
    fn from(err: std::io::Error) -> Self {
        PlatformError::IoError(err.to_string())
    }
}

impl From<serde_yaml::Error> for PlatformError {
    fn from(err: serde_yaml::Error) -> Self {
        PlatformError::SerializationError(err.to_string())
    }
}

impl From<serde_json::Error> for PlatformError {
    fn from(err: serde_json::Error) -> Self {
        PlatformError::SerializationError(err.to_string())
    }
}

/// Storage categories for organizing saved data
pub mod category {
    pub const SETTINGS: &str = "settings";
    pub const PRESETS: &str = "presets";
    pub const BOOKMARKS: &str = "bookmarks";
    pub const PALETTES: &str = "palettes";
    pub const PREFERENCES: &str = "preferences";
}

/// Storage abstraction for persisting data
/// Note: Send + Sync bounds removed to support web_sys types
pub trait Storage {
    /// Save data to storage
    fn save(&self, category: &str, key: &str, data: &[u8]) -> Result<(), PlatformError>;

    /// Load data from storage
    fn load(&self, category: &str, key: &str) -> Result<Option<Vec<u8>>, PlatformError>;

    /// Delete data from storage
    fn delete(&self, category: &str, key: &str) -> Result<(), PlatformError>;

    /// List all keys in a category
    fn list_keys(&self, category: &str) -> Result<Vec<String>, PlatformError>;

    /// Check if a key exists
    fn exists(&self, category: &str, key: &str) -> bool;

    /// Clear all data in a category
    fn clear_category(&self, category: &str) -> Result<(), PlatformError>;

    /// Clear all stored data
    fn clear_all(&self) -> Result<(), PlatformError>;
}

/// File dialog abstraction for open/save dialogs
pub trait FileDialog: Send + Sync {
    /// Save data to a file (triggers download on web)
    fn save_file(&self, filename: &str, data: &[u8], mime_type: &str) -> Result<(), PlatformError>;

    /// Open a file and return its contents
    /// Returns None if the user cancels
    fn open_file(
        &self,
        filters: &[(&str, &[&str])],
    ) -> Result<Option<(String, Vec<u8>)>, PlatformError>;
}

/// Capture abstraction for screenshots
pub trait Capture: Send + Sync {
    /// Save a screenshot and return the filename
    fn save_screenshot(
        &self,
        width: u32,
        height: u32,
        data: &[u8],
        filename_prefix: &str,
    ) -> Result<String, PlatformError>;

    /// Whether auto-open is supported on this platform
    fn supports_auto_open(&self) -> bool;

    /// Open a file with the system default application
    fn open_file(&self, path: &str) -> Result<(), PlatformError>;
}

/// Platform context containing all platform-specific implementations
pub struct PlatformContext {
    pub storage: Box<dyn Storage>,
    pub file_dialog: Box<dyn FileDialog>,
    pub capture: Box<dyn Capture>,
}

impl PlatformContext {
    /// Create a new platform context for the current platform
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Self {
        Self {
            storage: Box::new(native::NativeStorage::new()),
            file_dialog: Box::new(native::NativeFileDialog::new()),
            capture: Box::new(native::NativeCapture::new()),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new() -> Self {
        Self {
            storage: Box::new(web::WebStorage::new()),
            file_dialog: Box::new(web::WebFileDialog::new()),
            capture: Box::new(web::WebCapture::new()),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for PlatformContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
impl Default for PlatformContext {
    fn default() -> Self {
        Self::new()
    }
}
