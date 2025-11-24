use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Copy)]
pub struct ColorPalette {
    pub colors: [Vec3; 5],
    pub name: &'static str,
}

impl ColorPalette {
    pub const FIRE: ColorPalette = ColorPalette {
        name: "Fire",
        colors: [
            Vec3::new(0.0, 0.0, 0.0), // Black
            Vec3::new(0.5, 0.0, 0.5), // Purple
            Vec3::new(1.0, 0.0, 0.0), // Red
            Vec3::new(1.0, 0.5, 0.0), // Orange
            Vec3::new(1.0, 1.0, 0.0), // Yellow
        ],
    };

    pub const OCEAN: ColorPalette = ColorPalette {
        name: "Ocean",
        colors: [
            Vec3::new(0.0, 0.0, 0.1), // Deep blue
            Vec3::new(0.0, 0.2, 0.4), // Dark blue
            Vec3::new(0.0, 0.4, 0.7), // Blue
            Vec3::new(0.0, 0.7, 0.9), // Light blue
            Vec3::new(0.5, 1.0, 1.0), // Cyan
        ],
    };

    pub const RAINBOW: ColorPalette = ColorPalette {
        name: "Rainbow",
        colors: [
            Vec3::new(1.0, 0.0, 0.0), // Red
            Vec3::new(1.0, 1.0, 0.0), // Yellow
            Vec3::new(0.0, 1.0, 0.0), // Green
            Vec3::new(0.0, 0.0, 1.0), // Blue
            Vec3::new(0.5, 0.0, 0.5), // Purple
        ],
    };

    pub const FOREST: ColorPalette = ColorPalette {
        name: "Forest",
        colors: [
            Vec3::new(0.1, 0.2, 0.1), // Dark green
            Vec3::new(0.2, 0.4, 0.2), // Green
            Vec3::new(0.4, 0.6, 0.3), // Light green
            Vec3::new(0.6, 0.8, 0.4), // Yellow-green
            Vec3::new(0.8, 0.9, 0.6), // Pale green
        ],
    };

    pub const SUNSET: ColorPalette = ColorPalette {
        name: "Sunset",
        colors: [
            Vec3::new(0.2, 0.0, 0.4), // Purple
            Vec3::new(0.8, 0.2, 0.3), // Pink
            Vec3::new(1.0, 0.4, 0.2), // Orange
            Vec3::new(1.0, 0.7, 0.3), // Light orange
            Vec3::new(1.0, 0.9, 0.6), // Yellow
        ],
    };

    pub const GRAYSCALE: ColorPalette = ColorPalette {
        name: "Grayscale",
        colors: [
            Vec3::new(0.0, 0.0, 0.0),    // Black
            Vec3::new(0.25, 0.25, 0.25), // Dark gray
            Vec3::new(0.5, 0.5, 0.5),    // Gray
            Vec3::new(0.75, 0.75, 0.75), // Light gray
            Vec3::new(1.0, 1.0, 1.0),    // White
        ],
    };

    // Scientific visualization palettes
    pub const VIRIDIS: ColorPalette = ColorPalette {
        name: "Viridis",
        colors: [
            Vec3::new(0.267, 0.005, 0.329), // Dark purple
            Vec3::new(0.283, 0.141, 0.458), // Purple
            Vec3::new(0.128, 0.567, 0.551), // Teal
            Vec3::new(0.369, 0.788, 0.382), // Green
            Vec3::new(0.993, 0.906, 0.144), // Yellow
        ],
    };

    pub const PLASMA: ColorPalette = ColorPalette {
        name: "Plasma",
        colors: [
            Vec3::new(0.050, 0.030, 0.529), // Deep blue
            Vec3::new(0.540, 0.071, 0.689), // Purple
            Vec3::new(0.885, 0.218, 0.478), // Magenta
            Vec3::new(0.988, 0.553, 0.235), // Orange
            Vec3::new(0.940, 0.975, 0.131), // Yellow
        ],
    };

    pub const INFERNO: ColorPalette = ColorPalette {
        name: "Inferno",
        colors: [
            Vec3::new(0.000, 0.000, 0.014), // Black
            Vec3::new(0.341, 0.065, 0.319), // Dark purple
            Vec3::new(0.785, 0.186, 0.180), // Red
            Vec3::new(0.988, 0.553, 0.100), // Orange
            Vec3::new(0.988, 0.998, 0.645), // Yellow
        ],
    };

    pub const MAGMA: ColorPalette = ColorPalette {
        name: "Magma",
        colors: [
            Vec3::new(0.001, 0.000, 0.014), // Black
            Vec3::new(0.341, 0.065, 0.380), // Purple
            Vec3::new(0.735, 0.215, 0.331), // Pink-red
            Vec3::new(0.992, 0.624, 0.427), // Orange
            Vec3::new(0.987, 0.991, 0.750), // Pale yellow
        ],
    };

    pub const COPPER: ColorPalette = ColorPalette {
        name: "Copper",
        colors: [
            Vec3::new(0.0, 0.0, 0.0),   // Black
            Vec3::new(0.4, 0.25, 0.16), // Dark brown
            Vec3::new(0.7, 0.44, 0.28), // Brown
            Vec3::new(1.0, 0.63, 0.40), // Copper
            Vec3::new(1.0, 0.78, 0.50), // Light copper
        ],
    };

    pub const COOL: ColorPalette = ColorPalette {
        name: "Cool",
        colors: [
            Vec3::new(0.0, 1.0, 1.0),   // Cyan
            Vec3::new(0.25, 0.75, 1.0), // Light blue
            Vec3::new(0.5, 0.5, 1.0),   // Blue-purple
            Vec3::new(0.75, 0.25, 1.0), // Purple
            Vec3::new(1.0, 0.0, 1.0),   // Magenta
        ],
    };

    pub const HOT: ColorPalette = ColorPalette {
        name: "Hot",
        colors: [
            Vec3::new(0.0, 0.0, 0.0), // Black
            Vec3::new(0.5, 0.0, 0.0), // Dark red
            Vec3::new(1.0, 0.5, 0.0), // Orange
            Vec3::new(1.0, 1.0, 0.5), // Yellow
            Vec3::new(1.0, 1.0, 1.0), // White
        ],
    };

    // Artistic palettes
    pub const NEON: ColorPalette = ColorPalette {
        name: "Neon",
        colors: [
            Vec3::new(1.0, 0.0, 1.0), // Magenta
            Vec3::new(0.0, 1.0, 1.0), // Cyan
            Vec3::new(0.0, 1.0, 0.0), // Green
            Vec3::new(1.0, 1.0, 0.0), // Yellow
            Vec3::new(1.0, 0.2, 0.8), // Pink
        ],
    };

    pub const PURPLE_DREAM: ColorPalette = ColorPalette {
        name: "Purple Dream",
        colors: [
            Vec3::new(0.1, 0.0, 0.2), // Dark purple
            Vec3::new(0.4, 0.0, 0.6), // Purple
            Vec3::new(0.7, 0.3, 0.9), // Violet
            Vec3::new(0.9, 0.6, 1.0), // Lavender
            Vec3::new(1.0, 0.9, 1.0), // Pale purple
        ],
    };

    pub const EARTH: ColorPalette = ColorPalette {
        name: "Earth",
        colors: [
            Vec3::new(0.2, 0.15, 0.1), // Dark brown
            Vec3::new(0.4, 0.3, 0.2),  // Brown
            Vec3::new(0.6, 0.5, 0.3),  // Tan
            Vec3::new(0.5, 0.6, 0.3),  // Olive
            Vec3::new(0.7, 0.8, 0.5),  // Beige-green
        ],
    };

    pub const ICE: ColorPalette = ColorPalette {
        name: "Ice",
        colors: [
            Vec3::new(0.9, 0.95, 1.0),  // Pale blue
            Vec3::new(0.7, 0.85, 0.95), // Light blue
            Vec3::new(0.5, 0.7, 0.9),   // Blue
            Vec3::new(0.3, 0.5, 0.8),   // Deep blue
            Vec3::new(0.2, 0.3, 0.6),   // Dark blue
        ],
    };

    pub const LAVA: ColorPalette = ColorPalette {
        name: "Lava",
        colors: [
            Vec3::new(0.1, 0.0, 0.0), // Almost black
            Vec3::new(0.5, 0.0, 0.0), // Dark red
            Vec3::new(0.8, 0.2, 0.0), // Red
            Vec3::new(1.0, 0.5, 0.0), // Orange
            Vec3::new(1.0, 0.8, 0.2), // Yellow-orange
        ],
    };

    pub const GALAXY: ColorPalette = ColorPalette {
        name: "Galaxy",
        colors: [
            Vec3::new(0.05, 0.0, 0.15), // Deep space blue
            Vec3::new(0.2, 0.0, 0.5),   // Purple
            Vec3::new(0.5, 0.1, 0.7),   // Violet
            Vec3::new(0.8, 0.3, 0.8),   // Pink-purple
            Vec3::new(1.0, 0.7, 0.9),   // Pink
        ],
    };

    pub const MINT: ColorPalette = ColorPalette {
        name: "Mint",
        colors: [
            Vec3::new(0.0, 0.3, 0.3),  // Dark teal
            Vec3::new(0.2, 0.5, 0.5),  // Teal
            Vec3::new(0.4, 0.7, 0.6),  // Mint
            Vec3::new(0.6, 0.9, 0.8),  // Light mint
            Vec3::new(0.8, 1.0, 0.95), // Pale mint
        ],
    };

    pub const CHERRY: ColorPalette = ColorPalette {
        name: "Cherry",
        colors: [
            Vec3::new(0.3, 0.0, 0.1),  // Dark red
            Vec3::new(0.6, 0.0, 0.2),  // Cherry red
            Vec3::new(0.9, 0.2, 0.4),  // Red-pink
            Vec3::new(1.0, 0.5, 0.6),  // Pink
            Vec3::new(1.0, 0.8, 0.85), // Pale pink
        ],
    };

    pub const ALL: &'static [ColorPalette] = &[
        Self::FIRE,
        Self::OCEAN,
        Self::RAINBOW,
        Self::FOREST,
        Self::SUNSET,
        Self::GRAYSCALE,
        Self::VIRIDIS,
        Self::PLASMA,
        Self::INFERNO,
        Self::MAGMA,
        Self::COPPER,
        Self::COOL,
        Self::HOT,
        Self::NEON,
        Self::PURPLE_DREAM,
        Self::EARTH,
        Self::ICE,
        Self::LAVA,
        Self::GALAXY,
        Self::MINT,
        Self::CHERRY,
    ];

    /// Create a custom palette from Vec3 colors and a String name
    #[allow(dead_code)]
    pub fn custom(name: String, colors: [Vec3; 5]) -> CustomPalette {
        CustomPalette::new(name, colors)
    }
}

// Custom palette that can be saved and loaded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPalette {
    pub name: String,
    pub colors: [[f32; 3]; 5],
}

impl CustomPalette {
    pub fn new(name: String, colors: [Vec3; 5]) -> Self {
        Self {
            name,
            colors: [
                colors[0].to_array(),
                colors[1].to_array(),
                colors[2].to_array(),
                colors[3].to_array(),
                colors[4].to_array(),
            ],
        }
    }

    pub fn to_color_palette(&self) -> (String, [Vec3; 5]) {
        (
            self.name.clone(),
            [
                Vec3::from_array(self.colors[0]),
                Vec3::from_array(self.colors[1]),
                Vec3::from_array(self.colors[2]),
                Vec3::from_array(self.colors[3]),
                Vec3::from_array(self.colors[4]),
            ],
        )
    }

    #[allow(dead_code)]
    pub fn from_current(name: String, palette: &ColorPalette) -> Self {
        Self::new(name, palette.colors)
    }

    /// Import palette from .pal file (JASC-PAL or simple RGB format)
    pub fn from_pal_file(path: &std::path::Path) -> Result<Self, String> {
        let contents =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Imported")
            .to_string();

        Self::parse_pal_content(&contents, name)
    }

    /// Parse PAL format content
    fn parse_pal_content(contents: &str, name: String) -> Result<Self, String> {
        let lines: Vec<&str> = contents
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();

        // Check for JASC-PAL format
        if lines.first().copied() == Some("JASC-PAL") {
            if lines.len() < 3 {
                return Err("Invalid JASC-PAL file: too few lines".to_string());
            }

            // Parse color count (we'll sample 5 colors evenly)
            let num_colors: usize = lines
                .get(2)
                .ok_or("Missing color count")?
                .parse()
                .map_err(|_| "Invalid color count")?;

            if num_colors < 2 {
                return Err("Need at least 2 colors".to_string());
            }

            // Parse RGB values (0-255 range)
            let mut rgb_values = Vec::new();
            for line in lines.iter().skip(3) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let r: u8 = parts[0].parse().map_err(|_| "Invalid R value")?;
                    let g: u8 = parts[1].parse().map_err(|_| "Invalid G value")?;
                    let b: u8 = parts[2].parse().map_err(|_| "Invalid B value")?;
                    rgb_values.push(Vec3::new(
                        r as f32 / 255.0,
                        g as f32 / 255.0,
                        b as f32 / 255.0,
                    ));
                }
            }

            if rgb_values.len() < 2 {
                return Err("Need at least 2 valid colors".to_string());
            }

            // Sample 5 colors evenly from the palette
            let colors = Self::sample_colors(&rgb_values, 5);
            Ok(Self::new(name, colors))
        } else {
            // Try parsing as simple text format (one RGB per line, 0-255 or 0.0-1.0)
            let mut rgb_values = Vec::new();
            for line in lines {
                // Split by whitespace or comma
                let parts: Vec<&str> = line
                    .split(|c: char| c.is_whitespace() || c == ',')
                    .filter(|s| !s.is_empty())
                    .collect();

                if parts.len() >= 3 {
                    // Try parsing as 0-255 integer values
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        parts[0].parse::<u8>(),
                        parts[1].parse::<u8>(),
                        parts[2].parse::<u8>(),
                    ) {
                        rgb_values.push(Vec3::new(
                            r as f32 / 255.0,
                            g as f32 / 255.0,
                            b as f32 / 255.0,
                        ));
                    }
                    // Try parsing as 0.0-1.0 float values
                    else if let (Ok(r), Ok(g), Ok(b)) = (
                        parts[0].parse::<f32>(),
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                    ) {
                        if r <= 1.0 && g <= 1.0 && b <= 1.0 {
                            rgb_values.push(Vec3::new(r, g, b));
                        }
                    }
                }
            }

            if rgb_values.len() < 2 {
                return Err(
                    "Need at least 2 valid RGB colors (format: R G B per line, 0-255 or 0.0-1.0)"
                        .to_string(),
                );
            }

            let colors = Self::sample_colors(&rgb_values, 5);
            Ok(Self::new(name, colors))
        }
    }

    /// Sample N colors evenly from a color list using linear interpolation
    fn sample_colors(colors: &[Vec3], n: usize) -> [Vec3; 5] {
        let mut result = [Vec3::ZERO; 5];
        if colors.is_empty() {
            return result;
        }

        if colors.len() == 1 {
            // All same color
            result.fill(colors[0]);
        } else {
            // Sample evenly across the palette
            for (i, slot) in result.iter_mut().enumerate().take(n.min(5)) {
                let t = i as f32 / (n - 1).max(1) as f32;
                let idx_f = t * (colors.len() - 1) as f32;
                let idx = idx_f.floor() as usize;
                let frac = idx_f - idx as f32;

                if idx + 1 < colors.len() {
                    // Interpolate between two colors
                    *slot = colors[idx].lerp(colors[idx + 1], frac);
                } else {
                    *slot = colors[idx];
                }
            }
        }

        result
    }
}

// Gallery for managing custom palettes
pub struct CustomPaletteGallery;

#[cfg(not(target_arch = "wasm32"))]
impl CustomPaletteGallery {
    pub fn save_palette(
        palette: &CustomPalette,
        filename: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
            let palettes_dir = config_dir.config_dir().join("palettes");
            fs::create_dir_all(&palettes_dir)?;

            let palette_file = palettes_dir.join(format!("{}.yaml", filename));
            let yaml = serde_yaml::to_string(palette)?;
            fs::write(palette_file, yaml)?;

            println!("Custom palette '{}' saved", palette.name);
            Ok(())
        } else {
            Err("Could not determine config directory".into())
        }
    }

    pub fn load_palette(filename: &str) -> Result<CustomPalette, Box<dyn std::error::Error>> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
            let palette_file = config_dir
                .config_dir()
                .join("palettes")
                .join(format!("{}.yaml", filename));
            let yaml = fs::read_to_string(palette_file)?;
            let palette: CustomPalette = serde_yaml::from_str(&yaml)?;
            println!("Custom palette '{}' loaded", palette.name);
            Ok(palette)
        } else {
            Err("Could not determine config directory".into())
        }
    }

    pub fn delete_palette(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
            let palette_file = config_dir
                .config_dir()
                .join("palettes")
                .join(format!("{}.yaml", filename));
            fs::remove_file(palette_file)?;
            println!("Custom palette '{}' deleted", filename);
            Ok(())
        } else {
            Err("Could not determine config directory".into())
        }
    }

    pub fn list_palettes() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "fractal", "par-fractal") {
            let palettes_dir = config_dir.config_dir().join("palettes");
            if !palettes_dir.exists() {
                return Ok(Vec::new());
            }

            let mut palettes = Vec::new();
            for entry in fs::read_dir(palettes_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        palettes.push(name.to_string());
                    }
                }
            }
            palettes.sort();
            Ok(palettes)
        } else {
            Ok(Vec::new())
        }
    }
}

// Web stub - returns not supported errors
#[cfg(target_arch = "wasm32")]
impl CustomPaletteGallery {
    pub fn save_palette(
        _palette: &CustomPalette,
        _filename: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("Custom palette saving not yet supported on web".into())
    }

    pub fn load_palette(_filename: &str) -> Result<CustomPalette, Box<dyn std::error::Error>> {
        Err("Custom palette loading not yet supported on web".into())
    }

    pub fn delete_palette(_filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        Err("Custom palette deletion not yet supported on web".into())
    }

    pub fn list_palettes() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }
}
