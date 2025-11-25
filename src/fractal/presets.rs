use super::{FractalParams, FractalType, Settings};
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum PresetCategory {
    #[default]
    All,
    #[serde(rename = "2D Fractals")]
    TwoDFractals,
    #[serde(rename = "3D Fractals")]
    ThreeDFractals,
    #[serde(rename = "IFS")]
    IFS,
    Apollonian,
}

impl PresetCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            PresetCategory::All => "All",
            PresetCategory::TwoDFractals => "2D Fractals",
            PresetCategory::ThreeDFractals => "3D Fractals",
            PresetCategory::IFS => "IFS",
            PresetCategory::Apollonian => "Apollonian",
        }
    }

    pub fn all_categories() -> Vec<PresetCategory> {
        vec![
            PresetCategory::All,
            PresetCategory::TwoDFractals,
            PresetCategory::ThreeDFractals,
            PresetCategory::IFS,
            PresetCategory::Apollonian,
        ]
    }
}

// Preset system for saving/loading fractal configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub category: PresetCategory,
    pub settings: Settings,
}

// Camera bookmark for saving viewpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraBookmark {
    pub name: String,
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub fov: f32,
    pub timestamp: String,
}

// Application preferences
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppPreferences {
    #[serde(default)]
    pub preferred_gpu_index: Option<usize>,
    #[serde(default)]
    pub preferred_gpu_name: Option<String>,
    #[serde(default)]
    pub window_width: Option<u32>,
    #[serde(default)]
    pub window_height: Option<u32>,
}

impl AppPreferences {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load() -> Self {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
            let prefs_file = config_dir.config_dir().join("preferences.yaml");
            if let Ok(yaml) = fs::read_to_string(prefs_file) {
                if let Ok(prefs) = serde_yaml::from_str(&yaml) {
                    return prefs;
                }
            }
        }
        Self::default()
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load() -> Self {
        Self::default()
    }

    /// Return the preferred window size or sensible defaults (960x540)
    pub fn window_size_or_default(&self) -> (u32, u32) {
        (
            self.window_width.unwrap_or(960),
            self.window_height.unwrap_or(540),
        )
    }

    /// Persist a new window size in preferences.
    pub fn set_window_size(&mut self, width: u32, height: u32) {
        self.window_width = Some(width);
        self.window_height = Some(height);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
            let config_path = config_dir.config_dir();
            fs::create_dir_all(config_path)?;
            let prefs_file = config_path.join("preferences.yaml");
            let yaml = serde_yaml::to_string(self)?;
            fs::write(prefs_file, yaml)?;
            Ok(())
        } else {
            Err("Could not determine config directory".into())
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Preferences saving not supported on web yet
        Ok(())
    }
}

impl CameraBookmark {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(name: String, position: Vec3, target: Vec3, fov: f32) -> Self {
        use chrono::Local;
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        Self {
            name,
            position: position.to_array(),
            target: target.to_array(),
            fov,
            timestamp,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new(name: String, position: Vec3, target: Vec3, fov: f32) -> Self {
        // Web: use a simple timestamp placeholder
        let timestamp = "web".to_string();
        Self {
            name,
            position: position.to_array(),
            target: target.to_array(),
            fov,
            timestamp,
        }
    }

    pub fn get_position(&self) -> Vec3 {
        Vec3::from_array(self.position)
    }

    pub fn get_target(&self) -> Vec3 {
        Vec3::from_array(self.target)
    }
}

// Gallery of camera bookmarks
pub struct BookmarkGallery;

#[cfg(not(target_arch = "wasm32"))]
impl BookmarkGallery {
    pub fn save_bookmark(
        bookmark: &CameraBookmark,
        filename: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
            let bookmarks_dir = config_dir.config_dir().join("bookmarks");
            fs::create_dir_all(&bookmarks_dir)?;

            let bookmark_file = bookmarks_dir.join(format!("{}.yaml", filename));
            let yaml = serde_yaml::to_string(bookmark)?;
            fs::write(bookmark_file, yaml)?;

            println!("Bookmark '{}' saved", bookmark.name);
            Ok(())
        } else {
            Err("Could not determine config directory".into())
        }
    }

    pub fn load_bookmark(filename: &str) -> Result<CameraBookmark, Box<dyn std::error::Error>> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
            let bookmark_file = config_dir
                .config_dir()
                .join("bookmarks")
                .join(format!("{}.yaml", filename));
            let yaml = fs::read_to_string(bookmark_file)?;
            let bookmark: CameraBookmark = serde_yaml::from_str(&yaml)?;
            println!("Bookmark '{}' loaded", bookmark.name);
            Ok(bookmark)
        } else {
            Err("Could not determine config directory".into())
        }
    }

    pub fn delete_bookmark(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
            let bookmark_file = config_dir
                .config_dir()
                .join("bookmarks")
                .join(format!("{}.yaml", filename));
            fs::remove_file(bookmark_file)?;
            println!("Bookmark '{}' deleted", filename);
            Ok(())
        } else {
            Err("Could not determine config directory".into())
        }
    }

    pub fn list_bookmarks() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
            let bookmarks_dir = config_dir.config_dir().join("bookmarks");
            if !bookmarks_dir.exists() {
                return Ok(Vec::new());
            }

            let mut bookmarks = Vec::new();
            for entry in fs::read_dir(bookmarks_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        bookmarks.push(name.to_string());
                    }
                }
            }
            bookmarks.sort();
            Ok(bookmarks)
        } else {
            Ok(Vec::new())
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl BookmarkGallery {
    pub fn save_bookmark(
        _bookmark: &CameraBookmark,
        _filename: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Bookmark saving not yet supported on web".into())
    }

    pub fn load_bookmark(_filename: &str) -> Result<CameraBookmark, Box<dyn std::error::Error>> {
        Err("Bookmark loading not yet supported on web".into())
    }

    pub fn delete_bookmark(_filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        Err("Bookmark deletion not yet supported on web".into())
    }

    pub fn list_bookmarks() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }
}

impl Preset {
    pub fn from_current(
        name: String,
        description: String,
        category: PresetCategory,
        params: &FractalParams,
        camera_pos: Vec3,
        camera_target: Vec3,
    ) -> Self {
        let mut settings = params.to_settings();
        settings.camera_position = camera_pos.to_array();
        settings.camera_target = camera_target.to_array();
        Self {
            name,
            description,
            category,
            settings,
        }
    }
}

// Gallery of built-in presets
pub struct PresetGallery;

impl PresetGallery {
    pub fn get_builtin_presets() -> Vec<Preset> {
        let default_settings = FractalParams::default().to_settings();

        vec![
            // Classic Mandelbulb
            Preset {
                name: "Classic Mandelbulb".to_string(),
                description: "The iconic 3D Mandelbrot set with power 8".to_string(),
                category: PresetCategory::ThreeDFractals,
                settings: Settings {
                    fractal_type: FractalType::Mandelbulb3D,
                    power: 8.0,
                    max_steps: 200,
                    camera_position: [0.0, 0.0, 4.5],
                    camera_target: [0.0, 0.0, 0.0],
                    fractal_scale: 1.0,
                    show_floor: true,
                    ambient_occlusion: true,
                    shadow_mode: 2, // soft
                    ..default_settings.clone()
                },
            },
            // Detailed Mandelbulb
            Preset {
                name: "Detailed Mandelbulb".to_string(),
                description: "High-detail close-up with power 9".to_string(),
                category: PresetCategory::ThreeDFractals,
                settings: Settings {
                    fractal_type: FractalType::Mandelbulb3D,
                    power: 9.0,
                    max_steps: 200,
                    camera_position: [2.5, 1.8, 3.0],
                    camera_target: [0.0, 0.0, 0.0],
                    fractal_scale: 1.2,
                    ambient_occlusion: true,
                    ao_intensity: 0.8,
                    shadow_mode: 2,   // soft
                    palette_index: 2, // Rainbow
                    ..default_settings.clone()
                },
            },
            // Mandelbox Cubic
            Preset {
                name: "Mandelbox Cubic".to_string(),
                description: "Geometric Mandelbox with crisp edges".to_string(),
                category: PresetCategory::ThreeDFractals,
                settings: Settings {
                    fractal_type: FractalType::Mandelbox3D,
                    fractal_scale: 2.0,
                    fractal_fold: 1.0,
                    fractal_min_radius: 0.4,
                    max_steps: 200,
                    camera_position: [4.5, 3.0, 4.5],
                    camera_target: [0.0, 0.0, 0.0],
                    show_floor: true,
                    ambient_occlusion: true,
                    shadow_mode: 2,   // soft
                    palette_index: 1, // Ocean
                    bloom_enabled: true,
                    fxaa_enabled: true,
                    ..default_settings.clone()
                },
            },
            // Julia 3D
            Preset {
                name: "Julia Crystal".to_string(),
                description: "Beautiful 3D Julia set".to_string(),
                category: PresetCategory::ThreeDFractals,
                settings: Settings {
                    fractal_type: FractalType::JuliaSet3D,
                    julia_c: [-0.4, 0.6],
                    max_steps: 200,
                    camera_position: [0.0, 0.0, 4.0],
                    camera_target: [0.0, 0.0, 0.0],
                    fractal_scale: 1.0,
                    ambient_occlusion: true,
                    depth_of_field: true,
                    dof_focal_length: 4.0,
                    dof_aperture: 0.01,
                    dof_samples: 4,
                    palette_index: 3, // Forest
                    ..default_settings.clone()
                },
            },
            // Menger Sponge
            Preset {
                name: "Menger Sponge".to_string(),
                description: "Classic fractal cube with infinite holes".to_string(),
                category: PresetCategory::ThreeDFractals,
                settings: Settings {
                    fractal_type: FractalType::MengerSponge3D,
                    max_iterations: 7,
                    max_steps: 200,
                    camera_position: [3.5, 3.0, 3.5],
                    camera_target: [0.0, 0.0, 0.0],
                    show_floor: true,
                    ambient_occlusion: true,
                    ao_intensity: 0.7,
                    shadow_mode: 2,   // soft
                    palette_index: 4, // Sunset
                    ..default_settings.clone()
                },
            },
            // Mandelbrot Classic
            Preset {
                name: "Mandelbrot Classic".to_string(),
                description: "The original Mandelbrot set".to_string(),
                category: PresetCategory::TwoDFractals,
                settings: Settings {
                    fractal_type: FractalType::Mandelbrot2D,
                    center_2d: [-0.5f64, 0.0f64],
                    zoom_2d: 1.0,
                    max_iterations: 256,
                    palette_index: 0, // Fire
                    ..default_settings.clone()
                },
            },
            // Julia 2D
            Preset {
                name: "Julia Swirl".to_string(),
                description: "Beautiful Julia set pattern".to_string(),
                category: PresetCategory::TwoDFractals,
                settings: Settings {
                    fractal_type: FractalType::Julia2D,
                    julia_c: [-0.7269, 0.1889],
                    zoom_2d: 1.0,
                    max_iterations: 256,
                    palette_index: 2, // Rainbow
                    ..default_settings.clone()
                },
            },
            // Octahedral IFS
            Preset {
                name: "Octahedron Kaleidoscope".to_string(),
                description: "8-fold symmetric kaleidoscopic fractal".to_string(),
                category: PresetCategory::IFS,
                settings: Settings {
                    fractal_type: FractalType::OctahedralIFS3D,
                    fractal_scale: 2.0,
                    fractal_fold: 1.2,
                    max_steps: 200,
                    camera_position: [4.0, 3.5, 4.0],
                    camera_target: [0.0, 0.0, 0.0],
                    show_floor: true,
                    ambient_occlusion: true,
                    shadow_mode: 2,    // soft
                    palette_index: 14, // Purple Dream
                    ..default_settings.clone()
                },
            },
            // Icosahedral IFS
            Preset {
                name: "Icosahedron Kaleidoscope".to_string(),
                description: "20-fold symmetric kaleidoscopic fractal with intricate detail"
                    .to_string(),
                category: PresetCategory::IFS,
                settings: Settings {
                    fractal_type: FractalType::IcosahedralIFS3D,
                    fractal_scale: 1.7,
                    fractal_fold: 1.5,
                    max_steps: 200,
                    camera_position: [4.5, 4.0, 4.5],
                    camera_target: [0.0, 0.0, 0.0],
                    show_floor: true,
                    ambient_occlusion: true,
                    ao_intensity: 1.2,
                    shadow_mode: 2,    // soft
                    palette_index: 18, // Galaxy
                    ..default_settings.clone()
                },
            },
            // Apollonian Gasket
            Preset {
                name: "Apollonian Sphere Packing".to_string(),
                description: "Beautiful sphere-packing fractal".to_string(),
                category: PresetCategory::Apollonian,
                settings: Settings {
                    fractal_type: FractalType::ApollonianGasket3D,
                    fractal_scale: 1.3,
                    fractal_fold: 1.35,
                    fractal_min_radius: 1.12,
                    max_steps: 200,
                    camera_position: [5.0, 4.0, 5.0],
                    camera_target: [0.0, 0.0, 0.0],
                    show_floor: false,
                    ambient_occlusion: true,
                    shadow_mode: 2,    // soft
                    palette_index: 19, // Mint
                    ..default_settings.clone()
                },
            },
            // Octahedral IFS - Tight Structure
            Preset {
                name: "Octahedron Crystal".to_string(),
                description: "Tight octahedral structure with high fold".to_string(),
                category: PresetCategory::IFS,
                settings: Settings {
                    fractal_type: FractalType::OctahedralIFS3D,
                    fractal_scale: 1.5,
                    fractal_fold: 2.5,
                    max_steps: 200,
                    camera_position: [3.5, 3.0, 3.5],
                    camera_target: [0.0, 0.0, 0.0],
                    show_floor: true,
                    ambient_occlusion: true,
                    ao_intensity: 1.5,
                    shadow_mode: 2,    // soft
                    palette_index: 16, // Ice
                    ..default_settings.clone()
                },
            },
            // Octahedral IFS - Open Structure
            Preset {
                name: "Octahedron Lattice".to_string(),
                description: "Open lattice structure with moderate fold".to_string(),
                category: PresetCategory::IFS,
                settings: Settings {
                    fractal_type: FractalType::OctahedralIFS3D,
                    fractal_scale: 2.5,
                    fractal_fold: 1.5,
                    max_steps: 200,
                    camera_position: [5.0, 4.0, 5.0],
                    camera_target: [0.0, 0.0, 0.0],
                    show_floor: false,
                    ambient_occlusion: true,
                    shadow_mode: 2,    // soft
                    palette_index: 11, // Cool
                    ..default_settings.clone()
                },
            },
            // Icosahedral IFS - Detailed
            Preset {
                name: "Icosahedron Cathedral".to_string(),
                description: "Highly detailed 20-fold structure".to_string(),
                category: PresetCategory::IFS,
                settings: Settings {
                    fractal_type: FractalType::IcosahedralIFS3D,
                    fractal_scale: 1.4,
                    fractal_fold: 2.0,
                    max_steps: 200,
                    camera_position: [4.0, 3.5, 4.0],
                    camera_target: [0.0, 0.0, 0.0],
                    show_floor: true,
                    ambient_occlusion: true,
                    ao_intensity: 1.5,
                    shadow_mode: 2, // soft
                    shadow_softness: 12.0,
                    palette_index: 7, // Plasma
                    ..default_settings.clone()
                },
            },
            // Icosahedral IFS - Geometric
            Preset {
                name: "Icosahedron Geometry".to_string(),
                description: "Sharp geometric 20-fold fractal".to_string(),
                category: PresetCategory::IFS,
                settings: Settings {
                    fractal_type: FractalType::IcosahedralIFS3D,
                    fractal_scale: 2.2,
                    fractal_fold: 1.1,
                    max_steps: 200,
                    camera_position: [5.5, 4.5, 5.5],
                    camera_target: [0.0, 0.0, 0.0],
                    show_floor: true,
                    ambient_occlusion: true,
                    shadow_mode: 2,    // soft
                    palette_index: 13, // Neon
                    ..default_settings.clone()
                },
            },
            // Apollonian Gasket - Dense Packing
            Preset {
                name: "Apollonian Dense Pack".to_string(),
                description: "Tightly packed spheres with high min_radius".to_string(),
                category: PresetCategory::Apollonian,
                settings: Settings {
                    fractal_type: FractalType::ApollonianGasket3D,
                    fractal_scale: 1.4,
                    fractal_fold: 1.3,
                    fractal_min_radius: 1.2,
                    max_steps: 200,
                    camera_position: [4.5, 3.5, 4.5],
                    camera_target: [0.0, 0.0, 0.0],
                    show_floor: true,
                    ambient_occlusion: true,
                    ao_intensity: 1.8,
                    shadow_mode: 2,    // soft
                    palette_index: 10, // Copper
                    ..default_settings.clone()
                },
            },
            // Apollonian Gasket - Wispy
            Preset {
                name: "Apollonian Wispy".to_string(),
                description: "Delicate sphere arrangement with low min_radius".to_string(),
                category: PresetCategory::Apollonian,
                settings: Settings {
                    fractal_type: FractalType::ApollonianGasket3D,
                    fractal_scale: 1.3,
                    fractal_fold: 1.5,
                    fractal_min_radius: 0.8,
                    max_steps: 200,
                    camera_position: [4.5, 3.5, 4.5],
                    camera_target: [0.0, 0.0, 0.0],
                    show_floor: false,
                    ambient_occlusion: true,
                    shadow_mode: 2, // soft
                    shadow_softness: 10.0,
                    palette_index: 20, // Cherry
                    ..default_settings.clone()
                },
            },
            // Kleinian Group
            Preset {
                name: "Kleinian Limit Set".to_string(),
                description: "Intricate Kleinian group limit set fractal".to_string(),
                category: PresetCategory::ThreeDFractals,
                settings: Settings {
                    fractal_type: FractalType::Kleinian3D,
                    fractal_scale: 1.0,
                    fractal_fold: 1.2,
                    fractal_min_radius: 1.5,
                    max_steps: 200,
                    camera_position: [3.0, 2.5, 3.0],
                    camera_target: [0.0, 0.0, 0.0],
                    show_floor: true,
                    ambient_occlusion: true,
                    ao_intensity: 1.0,
                    shadow_mode: 2,    // soft
                    palette_index: 18, // Galaxy
                    ..default_settings.clone()
                },
            },
            // Hybrid Mandelbulb-Julia
            Preset {
                name: "Hybrid Bulb-Julia".to_string(),
                description: "Fascinating blend of Mandelbulb and Julia set behaviors".to_string(),
                category: PresetCategory::ThreeDFractals,
                settings: Settings {
                    fractal_type: FractalType::HybridMandelbulbJulia3D,
                    power: 8.0,
                    julia_c: [-0.2, 0.8],
                    max_iterations: 12,
                    max_steps: 200,
                    camera_position: [3.5, 3.0, 3.5],
                    camera_target: [0.0, 0.0, 0.0],
                    fractal_scale: 1.0,
                    show_floor: true,
                    ambient_occlusion: true,
                    shadow_mode: 2,    // soft
                    palette_index: 14, // Purple Dream
                    ..default_settings.clone()
                },
            },
            // Quaternion Cubic Julia
            Preset {
                name: "Quaternion Cubic".to_string(),
                description: "4D quaternion cubic Julia set (z³ + c)".to_string(),
                category: PresetCategory::ThreeDFractals,
                settings: Settings {
                    fractal_type: FractalType::QuaternionCubic3D,
                    julia_c: [-0.2, 0.6],
                    max_iterations: 16,
                    max_steps: 200,
                    camera_position: [3.0, 2.5, 3.0],
                    camera_target: [0.0, 0.0, 0.0],
                    fractal_scale: 1.0,
                    show_floor: true,
                    ambient_occlusion: true,
                    ao_intensity: 1.2,
                    shadow_mode: 2,   // soft
                    palette_index: 8, // Inferno
                    ..default_settings.clone()
                },
            },
            // Sierpinski Gasket
            Preset {
                name: "Sierpinski Gasket".to_string(),
                description: "Tetrahedral IFS fractal with sphere folding".to_string(),
                category: PresetCategory::IFS,
                settings: Settings {
                    fractal_type: FractalType::SierpinskiGasket3D,
                    fractal_scale: 1.5,
                    fractal_fold: 1.0,
                    fractal_min_radius: 0.5,
                    max_iterations: 8,
                    max_steps: 200,
                    camera_position: [3.5, 3.0, 3.5],
                    camera_target: [0.0, 0.0, 0.0],
                    show_floor: true,
                    ambient_occlusion: true,
                    ao_intensity: 1.3,
                    shadow_mode: 2,    // soft
                    palette_index: 15, // Forest
                    ..default_settings.clone()
                },
            },
        ]
    }

    /// Get a builtin preset by name (for web use)
    #[allow(dead_code)]
    pub fn get_builtin_preset(name: &str) -> Option<&'static Preset> {
        // Use a static to avoid repeated allocation
        static PRESETS: std::sync::OnceLock<Vec<Preset>> = std::sync::OnceLock::new();
        let presets = PRESETS.get_or_init(Self::get_builtin_presets);
        presets.iter().find(|p| p.name == name)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_preset(preset: &Preset, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
            let presets_dir = config_dir.config_dir().join("presets");
            fs::create_dir_all(&presets_dir)?;

            let preset_file = presets_dir.join(format!("{}.yaml", filename));
            let yaml = serde_yaml::to_string(preset)?;
            fs::write(preset_file, yaml)?;

            println!("Preset '{}' saved", preset.name);
            Ok(())
        } else {
            Err("Could not determine config directory".into())
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_preset(filename: &str) -> Result<Preset, Box<dyn std::error::Error>> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
            let preset_file = config_dir
                .config_dir()
                .join("presets")
                .join(format!("{}.yaml", filename));
            let yaml = fs::read_to_string(preset_file)?;
            let preset: Preset = serde_yaml::from_str(&yaml)?;
            println!("Preset '{}' loaded", preset.name);
            Ok(preset)
        } else {
            Err("Could not determine config directory".into())
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn delete_preset(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
            let preset_file = config_dir
                .config_dir()
                .join("presets")
                .join(format!("{}.yaml", filename));
            if preset_file.exists() {
                fs::remove_file(&preset_file)?;
                println!("Preset '{}' deleted", filename);
                Ok(())
            } else {
                Err(format!("Preset '{}' not found", filename).into())
            }
        } else {
            Err("Could not determine config directory".into())
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn list_user_presets() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
            let presets_dir = config_dir.config_dir().join("presets");
            if !presets_dir.exists() {
                return Ok(Vec::new());
            }

            let mut presets = Vec::new();
            for entry in fs::read_dir(presets_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        presets.push(name.to_string());
                    }
                }
            }
            Ok(presets)
        } else {
            Ok(Vec::new())
        }
    }

    /// Export current settings to a JSON file (user chooses location)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn export_to_json(
        settings: &Settings,
        camera_position: [f32; 3],
        camera_target: [f32; 3],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file_dialog = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .set_file_name("fractal_settings.json");

        if let Some(path) = file_dialog.save_file() {
            // Create a preset with current settings
            let preset = Preset {
                name: "Exported Settings".to_string(),
                description: "Exported fractal configuration".to_string(),
                category: PresetCategory::All,
                settings: Settings {
                    camera_position,
                    camera_target,
                    ..settings.clone()
                },
            };

            let json = serde_json::to_string_pretty(&preset)?;
            fs::write(&path, json)?;
            println!("Settings exported to {}", path.display());
            Ok(())
        } else {
            Err("Export cancelled by user".into())
        }
    }

    /// Import settings from a JSON file (user chooses file)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn import_from_json() -> Result<Preset, Box<dyn std::error::Error>> {
        let file_dialog = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .set_title("Import Fractal Settings");

        if let Some(path) = file_dialog.pick_file() {
            let json = fs::read_to_string(&path)?;
            let preset: Preset = serde_json::from_str(&json)?;
            println!("Settings imported from {}", path.display());
            Ok(preset)
        } else {
            Err("Import cancelled by user".into())
        }
    }

    // Web stubs
    #[cfg(target_arch = "wasm32")]
    pub fn save_preset(
        _preset: &Preset,
        _filename: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Preset saving not yet supported on web".into())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load_preset(_filename: &str) -> Result<Preset, Box<dyn std::error::Error>> {
        Err("User preset loading not yet supported on web".into())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn list_user_presets() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn export_to_json(
        _settings: &Settings,
        _camera_position: [f32; 3],
        _camera_target: [f32; 3],
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Export not yet supported on web".into())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn import_from_json() -> Result<Preset, Box<dyn std::error::Error>> {
        Err("Import not yet supported on web".into())
    }
}
