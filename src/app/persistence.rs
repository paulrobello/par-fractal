use super::App;
use crate::platform::{PlatformContext, category};

/// Delete every persisted settings file that
/// [`App::load_settings_via_platform`] reads, under `config_dir`.
///
/// The loader consults two locations, so `--clear-settings` must clear both or
/// a reset silently no-ops and stale parameters keep loading:
///   1. `<config_dir>/settings/settings.yaml` — platform storage (where saves go)
///   2. `<config_dir>/settings.yaml` — pre-ARC-014 legacy path
///
/// Returns the paths that were actually removed.
#[cfg(not(target_arch = "wasm32"))]
pub fn clear_settings_files(
    config_dir: &std::path::Path,
) -> std::io::Result<Vec<std::path::PathBuf>> {
    let candidates = [
        config_dir.join(category::SETTINGS).join("settings.yaml"),
        config_dir.join("settings.yaml"),
    ];

    let mut removed = Vec::new();
    for path in candidates {
        if path.exists() {
            std::fs::remove_file(&path)?;
            removed.push(path);
        }
    }
    Ok(removed)
}

/// Settings persistence methods
impl App {
    pub(super) fn save_all_settings(&self) {
        let mut settings = self.fractal_params.to_settings();
        settings.camera_position = self.camera.position.to_array();
        settings.camera_target = self.camera.target.to_array();
        settings.ui_state = self.ui.get_ui_state().clone();
        settings.auto_open_captures = self.ui.auto_open_captures;
        settings.custom_width = self.ui.custom_width.clone();
        settings.custom_height = self.ui.custom_height.clone();

        match serde_yaml::to_string(&settings) {
            Ok(yaml) => {
                // ARC-014: persist via the platform `Storage` abstraction so the
                // same path works on native (`<config_dir>/settings/settings.yaml`)
                // and web (localStorage — previously web loaded settings but never
                // saved them). The load path reads this location first and falls
                // back to the legacy `<config_dir>/settings.yaml`, so migrating
                // existing users over is non-destructive.
                let storage = PlatformContext::new().storage;
                if let Err(e) = storage.save(category::SETTINGS, "settings", yaml.as_bytes()) {
                    log::error!("Failed to save settings: {}", e);
                } else {
                    log::debug!("Settings auto-saved via platform storage");
                }
            }
            Err(e) => log::error!("Failed to serialize settings: {}", e),
        }
    }

    pub(super) fn save_camera_settings(&self) {
        self.save_all_settings();
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::clear_settings_files;
    use crate::platform::{Storage, category, native::NativeStorage};
    use std::path::PathBuf;

    fn scratch_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("par-fractal-test-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    }

    /// The bug this pins: saves went to `<config>/settings/settings.yaml` while
    /// `--clear-settings` deleted `<config>/settings.yaml`, so a reset never
    /// cleared anything and stale parameters kept loading.
    #[test]
    fn clear_removes_the_file_platform_storage_actually_writes() {
        let config_dir = scratch_dir("clear-storage");
        let storage = NativeStorage::with_base_dir(config_dir.clone());
        storage
            .save(category::SETTINGS, "settings", b"fractal_type: JuliaSet3D")
            .expect("save settings");
        assert!(storage.exists(category::SETTINGS, "settings"));

        let removed = clear_settings_files(&config_dir).expect("clear settings");

        assert!(
            !storage.exists(category::SETTINGS, "settings"),
            "clear_settings_files left the settings the save path wrote"
        );
        assert_eq!(
            removed.len(),
            1,
            "expected exactly the storage entry: {removed:?}"
        );

        std::fs::remove_dir_all(&config_dir).ok();
    }

    /// The loader also falls back to the pre-ARC-014 legacy path, so a reset
    /// that skips it still resurrects old settings on the next launch.
    #[test]
    fn clear_removes_legacy_settings_file() {
        let config_dir = scratch_dir("clear-legacy");
        let legacy = config_dir.join("settings.yaml");
        std::fs::write(&legacy, b"fractal_type: JuliaSet3D").expect("write legacy settings");

        let removed = clear_settings_files(&config_dir).expect("clear settings");

        assert!(!legacy.exists(), "legacy settings.yaml survived the clear");
        assert_eq!(removed, vec![legacy]);

        std::fs::remove_dir_all(&config_dir).ok();
    }

    #[test]
    fn clear_on_a_clean_config_dir_removes_nothing() {
        let config_dir = scratch_dir("clear-empty");
        let removed = clear_settings_files(&config_dir).expect("clear settings");
        assert!(removed.is_empty());
        std::fs::remove_dir_all(&config_dir).ok();
    }
}
