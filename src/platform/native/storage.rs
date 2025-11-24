//! Native storage implementation using the filesystem

use crate::platform::{category, PlatformError, Storage};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

/// Native storage using filesystem
pub struct NativeStorage {
    base_dir: PathBuf,
}

impl NativeStorage {
    pub fn new() -> Self {
        let base_dir = ProjectDirs::from("com", "fractal", "par-fractal")
            .map(|dirs| dirs.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".par-fractal"));

        Self { base_dir }
    }

    fn get_category_dir(&self, category: &str) -> PathBuf {
        self.base_dir.join(category)
    }

    fn get_file_path(&self, category: &str, key: &str) -> PathBuf {
        self.get_category_dir(category).join(format!("{}.yaml", key))
    }

    fn ensure_category_dir(&self, category: &str) -> Result<(), PlatformError> {
        let dir = self.get_category_dir(category);
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        Ok(())
    }
}

impl Default for NativeStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage for NativeStorage {
    fn save(&self, category: &str, key: &str, data: &[u8]) -> Result<(), PlatformError> {
        self.ensure_category_dir(category)?;
        let path = self.get_file_path(category, key);
        fs::write(&path, data)?;
        Ok(())
    }

    fn load(&self, category: &str, key: &str) -> Result<Option<Vec<u8>>, PlatformError> {
        let path = self.get_file_path(category, key);
        if path.exists() {
            let data = fs::read(&path)?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    fn delete(&self, category: &str, key: &str) -> Result<(), PlatformError> {
        let path = self.get_file_path(category, key);
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    fn list_keys(&self, category: &str) -> Result<Vec<String>, PlatformError> {
        let dir = self.get_category_dir(category);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut keys = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(stem) = path.file_stem() {
                    if let Some(name) = stem.to_str() {
                        keys.push(name.to_string());
                    }
                }
            }
        }
        keys.sort();
        Ok(keys)
    }

    fn exists(&self, category: &str, key: &str) -> bool {
        self.get_file_path(category, key).exists()
    }

    fn clear_category(&self, category: &str) -> Result<(), PlatformError> {
        let dir = self.get_category_dir(category);
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }

    fn clear_all(&self) -> Result<(), PlatformError> {
        // Clear all known categories
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
