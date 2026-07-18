use super::App;
use crate::platform::{PlatformContext, category};

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
