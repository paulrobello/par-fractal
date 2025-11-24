//! Web storage implementation using localStorage
//!
//! Uses localStorage for simple key-value storage. Data is base64 encoded.
//! For larger datasets, IndexedDB could be used instead.

use crate::platform::{category, PlatformError, Storage};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use wasm_bindgen::JsCast;

/// Web storage using localStorage
pub struct WebStorage {
    storage: web_sys::Storage,
}

impl WebStorage {
    pub fn new() -> Self {
        let window = web_sys::window().expect("no window");
        let storage = window
            .local_storage()
            .expect("no localStorage")
            .expect("localStorage unavailable");
        Self { storage }
    }

    fn make_key(&self, category: &str, key: &str) -> String {
        format!("par-fractal:{}:{}", category, key)
    }

    fn parse_key(&self, storage_key: &str, category: &str) -> Option<String> {
        let prefix = format!("par-fractal:{}:", category);
        if storage_key.starts_with(&prefix) {
            Some(storage_key[prefix.len()..].to_string())
        } else {
            None
        }
    }
}

impl Default for WebStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage for WebStorage {
    fn save(&self, category: &str, key: &str, data: &[u8]) -> Result<(), PlatformError> {
        let storage_key = self.make_key(category, key);
        let encoded = BASE64.encode(data);
        self.storage
            .set_item(&storage_key, &encoded)
            .map_err(|_| PlatformError::StorageNotAvailable)?;
        Ok(())
    }

    fn load(&self, category: &str, key: &str) -> Result<Option<Vec<u8>>, PlatformError> {
        let storage_key = self.make_key(category, key);
        match self.storage.get_item(&storage_key) {
            Ok(Some(encoded)) => {
                let data = BASE64
                    .decode(&encoded)
                    .map_err(|e| PlatformError::SerializationError(e.to_string()))?;
                Ok(Some(data))
            }
            Ok(None) => Ok(None),
            Err(_) => Err(PlatformError::StorageNotAvailable),
        }
    }

    fn delete(&self, category: &str, key: &str) -> Result<(), PlatformError> {
        let storage_key = self.make_key(category, key);
        self.storage
            .remove_item(&storage_key)
            .map_err(|_| PlatformError::StorageNotAvailable)?;
        Ok(())
    }

    fn list_keys(&self, category: &str) -> Result<Vec<String>, PlatformError> {
        let mut keys = Vec::new();
        let length = self
            .storage
            .length()
            .map_err(|_| PlatformError::StorageNotAvailable)?;

        for i in 0..length {
            if let Ok(Some(storage_key)) = self.storage.key(i) {
                if let Some(key) = self.parse_key(&storage_key, category) {
                    keys.push(key);
                }
            }
        }
        keys.sort();
        Ok(keys)
    }

    fn exists(&self, category: &str, key: &str) -> bool {
        let storage_key = self.make_key(category, key);
        matches!(self.storage.get_item(&storage_key), Ok(Some(_)))
    }

    fn clear_category(&self, category: &str) -> Result<(), PlatformError> {
        let keys = self.list_keys(category)?;
        for key in keys {
            self.delete(category, &key)?;
        }
        Ok(())
    }

    fn clear_all(&self) -> Result<(), PlatformError> {
        for cat in [
            category::SETTINGS,
            category::PRESETS,
            category::BOOKMARKS,
            category::PALETTES,
            category::PREFERENCES,
        ] {
            self.clear_category(cat)?;
        }
        Ok(())
    }
}
