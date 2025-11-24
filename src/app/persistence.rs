use super::App;

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

        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
            let config_dir = proj_dirs.config_dir();
            if let Err(e) = std::fs::create_dir_all(config_dir) {
                eprintln!("Failed to create config directory: {}", e);
                return;
            }

            let settings_path = config_dir.join("settings.yaml");
            match serde_yaml::to_string(&settings) {
                Ok(yaml) => {
                    if let Err(e) = std::fs::write(&settings_path, yaml) {
                        eprintln!("Failed to save settings: {}", e);
                    } else {
                        println!("Settings auto-saved to {:?}", settings_path);
                    }
                }
                Err(e) => eprintln!("Failed to serialize settings: {}", e),
            }
        }
    }

    pub(super) fn save_camera_settings(&self) {
        self.save_all_settings();
    }
}
