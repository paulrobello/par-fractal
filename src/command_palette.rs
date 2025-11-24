// Command Palette System
// Provides a quick command interface for accessing all application features

use crate::fractal::{ColorMode, FractalType, ShadingModel};
use crate::lod::LODProfile;
use serde::{Deserialize, Serialize};

/// Category of command for grouping and filtering
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandCategory {
    Fractal,
    Preset,
    Effect,
    Color,
    Camera,
    Recording,
    LOD,
    UI,
    Settings,
    Debug,
}

impl CommandCategory {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Fractal => "Fractal",
            Self::Preset => "Preset",
            Self::Effect => "Effect",
            Self::Color => "Color",
            Self::Camera => "Camera",
            Self::Recording => "Recording",
            Self::LOD => "LOD",
            Self::UI => "UI",
            Self::Settings => "Settings",
            Self::Debug => "Debug",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Fractal => "🔷",
            Self::Preset => "📋",
            Self::Effect => "✨",
            Self::Color => "🎨",
            Self::Camera => "📷",
            Self::Recording => "🎬",
            Self::LOD => "⚡",
            Self::UI => "🖥️",
            Self::Settings => "⚙️",
            Self::Debug => "🔍",
        }
    }
}

/// Action to execute when a command is triggered
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum CommandAction {
    SetFractalType(FractalType),
    LoadPreset(String),
    ToggleEffect(EffectType),
    SetColorMode(ColorMode),
    SetPalette(usize),
    SetShadowMode(u32), // 0=off, 1=hard, 2=soft
    SetShadingModel(ShadingModel),
    SetLODProfile(LODProfile),
    ToggleLOD,
    ToggleLODDebug,
    ToggleUI,
    ToggleStats,
    ResetView,
    ResetAll,
    SavePreset,
    ExportSettings,
    ImportSettings,
    StartRecording(String), // Format: mp4, webm, gif
    StopRecording,
    Screenshot,
    CycleTheme,
}

#[allow(dead_code, clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectType {
    AmbientOcclusion,
    Shadows,
    SoftShadows,
    DepthOfField,
    Fog,
    Bloom,
    Vignette,
    FXAA,
    SSR,
}

/// A single command in the palette
#[derive(Debug, Clone)]
pub struct Command {
    pub name: String,
    pub aliases: Vec<String>,
    pub category: CommandCategory,
    pub action: CommandAction,
    pub description: String,
    pub shortcut: Option<String>,
}

impl Command {
    pub fn new(
        name: impl Into<String>,
        category: CommandCategory,
        action: CommandAction,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            aliases: Vec::new(),
            category,
            action,
            description: description.into(),
            shortcut: None,
        }
    }

    pub fn with_aliases(mut self, aliases: Vec<impl Into<String>>) -> Self {
        self.aliases = aliases.into_iter().map(|a| a.into()).collect();
        self
    }

    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Calculate fuzzy match score for a query
    pub fn match_score(&self, query: &str) -> Option<f32> {
        if query.is_empty() {
            return Some(1.0);
        }

        let query_lower = query.to_lowercase();

        // Check exact match first (highest score)
        if self.name.to_lowercase() == query_lower {
            return Some(100.0);
        }

        // Check if name starts with query
        if self.name.to_lowercase().starts_with(&query_lower) {
            return Some(90.0);
        }

        // Check aliases
        for alias in &self.aliases {
            if alias.to_lowercase() == query_lower {
                return Some(95.0);
            }
            if alias.to_lowercase().starts_with(&query_lower) {
                return Some(85.0);
            }
        }

        // Fuzzy match on name
        if let Some(score) = fuzzy_match(&query_lower, &self.name.to_lowercase()) {
            return Some(score);
        }

        // Fuzzy match on aliases
        for alias in &self.aliases {
            if let Some(score) = fuzzy_match(&query_lower, &alias.to_lowercase()) {
                return Some(score * 0.9); // Slightly lower score for alias matches
            }
        }

        // Check description for keyword match
        if self.description.to_lowercase().contains(&query_lower) {
            return Some(30.0);
        }

        None
    }
}

/// Simple fuzzy matching algorithm
/// Returns a score from 0.0 to 100.0 if there's a match, None otherwise
fn fuzzy_match(query: &str, text: &str) -> Option<f32> {
    if query.is_empty() {
        return Some(1.0);
    }

    let query_chars: Vec<char> = query.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();

    let mut query_idx = 0;
    let mut text_idx = 0;
    let mut consecutive_matches = 0;
    let mut max_consecutive = 0;
    let mut total_matches = 0;

    while query_idx < query_chars.len() && text_idx < text_chars.len() {
        if query_chars[query_idx] == text_chars[text_idx] {
            query_idx += 1;
            total_matches += 1;
            consecutive_matches += 1;
            max_consecutive = max_consecutive.max(consecutive_matches);
        } else {
            consecutive_matches = 0;
        }
        text_idx += 1;
    }

    // Must match all query characters
    if query_idx < query_chars.len() {
        return None;
    }

    // Calculate score based on:
    // - Match ratio (how many characters matched vs text length)
    // - Consecutive matches (prefer contiguous matches)
    // - Position of first match (prefer matches at start)
    let match_ratio = total_matches as f32 / text.len() as f32;
    let consecutive_bonus = max_consecutive as f32 / query.len() as f32;
    let position_bonus = if text.starts_with(query) { 1.0 } else { 0.5 };

    let score = (match_ratio * 40.0) + (consecutive_bonus * 40.0) + (position_bonus * 20.0);

    Some(score.min(80.0)) // Cap fuzzy matches at 80 to prioritize exact/prefix matches
}

/// Command palette state
pub struct CommandPalette {
    pub open: bool,
    pub query: String,
    pub selected_index: usize,
    pub filtered_commands: Vec<(f32, Command)>,
    commands: Vec<Command>,
}

impl CommandPalette {
    pub fn new() -> Self {
        let commands = Self::build_commands();

        Self {
            open: false,
            query: String::new(),
            selected_index: 0,
            filtered_commands: Vec::new(),
            commands,
        }
    }

    /// Open the command palette
    pub fn open(&mut self) {
        self.open = true;
        self.query.clear();
        self.selected_index = 0;
        self.update_filtered_commands();
    }

    /// Close the command palette
    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected_index = 0;
        self.filtered_commands.clear();
    }

    /// Toggle the command palette
    #[allow(dead_code)]
    pub fn toggle(&mut self) {
        if self.open {
            self.close();
        } else {
            self.open();
        }
    }

    /// Update the search query and refresh filtered commands
    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.selected_index = 0;
        self.update_filtered_commands();
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if self.selected_index < self.filtered_commands.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    /// Get the currently selected command
    pub fn get_selected_command(&self) -> Option<&Command> {
        self.filtered_commands
            .get(self.selected_index)
            .map(|(_, cmd)| cmd)
    }

    /// Update filtered commands based on current query
    fn update_filtered_commands(&mut self) {
        self.filtered_commands = self
            .commands
            .iter()
            .filter_map(|cmd| {
                cmd.match_score(&self.query)
                    .map(|score| (score, cmd.clone()))
            })
            .collect();

        // Sort by score (highest first)
        self.filtered_commands
            .sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Limit to top 10 results
        self.filtered_commands.truncate(10);
    }

    /// Build all available commands
    #[allow(clippy::vec_init_then_push)]
    fn build_commands() -> Vec<Command> {
        let mut commands = Vec::new();

        // === Fractal Type Commands ===
        commands.push(
            Command::new(
                "Mandelbrot (2D)",
                CommandCategory::Fractal,
                CommandAction::SetFractalType(FractalType::Mandelbrot2D),
                "Classic Mandelbrot set fractal",
            )
            .with_aliases(vec!["mandelbrot", "mb", "mbrot"])
            .with_shortcut("1"),
        );

        commands.push(
            Command::new(
                "Julia Set (2D)",
                CommandCategory::Fractal,
                CommandAction::SetFractalType(FractalType::Julia2D),
                "Julia set fractal with customizable constant",
            )
            .with_aliases(vec!["julia", "js"])
            .with_shortcut("2"),
        );

        commands.push(
            Command::new(
                "Burning Ship (2D)",
                CommandCategory::Fractal,
                CommandAction::SetFractalType(FractalType::BurningShip2D),
                "Burning Ship fractal variation",
            )
            .with_aliases(vec!["burning ship", "ship"])
            .with_shortcut("3"),
        );

        commands.push(
            Command::new(
                "Mandelbulb (3D)",
                CommandCategory::Fractal,
                CommandAction::SetFractalType(FractalType::Mandelbulb3D),
                "3D extension of Mandelbrot set",
            )
            .with_aliases(vec!["mandelbulb", "bulb"])
            .with_shortcut("F1"),
        );

        commands.push(
            Command::new(
                "Kleinian Group (3D)",
                CommandCategory::Fractal,
                CommandAction::SetFractalType(FractalType::Kleinian3D),
                "Kleinian limit set fractal",
            )
            .with_aliases(vec!["kleinian", "klein"])
            .with_shortcut("F10"),
        );

        // Add all other fractal types...
        let fractal_types = vec![
            (
                FractalType::Sierpinski2D,
                "Sierpinski Carpet (2D)",
                vec!["sierpinski", "carpet"],
                Some("4"),
            ),
            (
                FractalType::Tricorn2D,
                "Tricorn (2D)",
                vec!["tricorn"],
                Some("5"),
            ),
            (
                FractalType::Phoenix2D,
                "Phoenix (2D)",
                vec!["phoenix"],
                Some("6"),
            ),
            (
                FractalType::Celtic2D,
                "Celtic (2D)",
                vec!["celtic"],
                Some("7"),
            ),
            (FractalType::Newton2D, "Newton (2D)", vec!["newton"], None),
            (
                FractalType::Lyapunov2D,
                "Lyapunov (2D)",
                vec!["lyapunov"],
                None,
            ),
            (FractalType::Nova2D, "Nova (2D)", vec!["nova"], None),
            (FractalType::Magnet2D, "Magnet (2D)", vec!["magnet"], None),
            (
                FractalType::Collatz2D,
                "Collatz (2D)",
                vec!["collatz"],
                None,
            ),
            (
                FractalType::MengerSponge3D,
                "Menger Sponge (3D)",
                vec!["menger", "sponge"],
                Some("F2"),
            ),
            (
                FractalType::SierpinskiPyramid3D,
                "Sierpinski Pyramid (3D)",
                vec!["pyramid"],
                Some("F3"),
            ),
            (
                FractalType::JuliaSet3D,
                "Julia Set (3D)",
                vec!["julia3d"],
                Some("F4"),
            ),
            (
                FractalType::Mandelbox3D,
                "Mandelbox (3D)",
                vec!["mandelbox", "box"],
                Some("F5"),
            ),
            (
                FractalType::TgladFormula3D,
                "Tglad Formula (3D)",
                vec!["tglad"],
                Some("F6"),
            ),
            (
                FractalType::OctahedralIFS3D,
                "Octahedral IFS (3D)",
                vec!["octahedral", "oct"],
                Some("F7"),
            ),
            (
                FractalType::IcosahedralIFS3D,
                "Icosahedral IFS (3D)",
                vec!["icosahedral", "ico"],
                Some("F8"),
            ),
            (
                FractalType::ApollonianGasket3D,
                "Apollonian Gasket (3D)",
                vec!["apollonian", "gasket"],
                Some("F9"),
            ),
            (
                FractalType::HybridMandelbulbJulia3D,
                "Hybrid Mandelbulb-Julia (3D)",
                vec!["hybrid"],
                Some("F11"),
            ),
            (
                FractalType::QuaternionCubic3D,
                "Quaternion Cubic (3D)",
                vec!["quaternion", "quat"],
                None,
            ),
        ];

        for (ftype, name, aliases, shortcut) in fractal_types {
            let mut cmd = Command::new(
                name,
                CommandCategory::Fractal,
                CommandAction::SetFractalType(ftype),
                format!("{} fractal", name),
            )
            .with_aliases(aliases);

            if let Some(sc) = shortcut {
                cmd = cmd.with_shortcut(sc);
            }

            commands.push(cmd);
        }

        // === Effect Commands ===
        commands.push(
            Command::new(
                "Toggle Ambient Occlusion",
                CommandCategory::Effect,
                CommandAction::ToggleEffect(EffectType::AmbientOcclusion),
                "Toggle ambient occlusion effect",
            )
            .with_aliases(vec!["ao", "occlusion", "ambient"]),
        );

        commands.push(
            Command::new(
                "Toggle Soft Shadows",
                CommandCategory::Effect,
                CommandAction::ToggleEffect(EffectType::SoftShadows),
                "Toggle soft shadow rendering",
            )
            .with_aliases(vec!["soft shadows", "shadows soft"])
            .with_shortcut("B"),
        );

        commands.push(
            Command::new(
                "Toggle Depth of Field",
                CommandCategory::Effect,
                CommandAction::ToggleEffect(EffectType::DepthOfField),
                "Toggle depth of field effect",
            )
            .with_aliases(vec!["dof", "depth of field", "blur"]),
        );

        commands.push(
            Command::new(
                "Toggle Fog",
                CommandCategory::Effect,
                CommandAction::ToggleEffect(EffectType::Fog),
                "Toggle atmospheric fog",
            )
            .with_aliases(vec!["fog", "atmosphere"]),
        );

        commands.push(
            Command::new(
                "Toggle Bloom",
                CommandCategory::Effect,
                CommandAction::ToggleEffect(EffectType::Bloom),
                "Toggle bloom/glow effect",
            )
            .with_aliases(vec!["bloom", "glow"]),
        );

        commands.push(
            Command::new(
                "Toggle Vignette",
                CommandCategory::Effect,
                CommandAction::ToggleEffect(EffectType::Vignette),
                "Toggle vignette edge darkening",
            )
            .with_aliases(vec!["vignette", "vign"]),
        );

        commands.push(
            Command::new(
                "Toggle FXAA",
                CommandCategory::Effect,
                CommandAction::ToggleEffect(EffectType::FXAA),
                "Toggle FXAA anti-aliasing",
            )
            .with_aliases(vec!["fxaa", "antialiasing", "aa"]),
        );

        commands.push(
            Command::new(
                "Toggle Screen-Space Reflections",
                CommandCategory::Effect,
                CommandAction::ToggleEffect(EffectType::SSR),
                "Toggle floor reflections",
            )
            .with_aliases(vec!["ssr", "reflections", "reflect"]),
        );

        // === Color Mode Commands ===
        let color_modes = vec![
            (
                ColorMode::Palette,
                "Palette Coloring",
                vec!["palette", "pal"],
            ),
            (
                ColorMode::Normals,
                "Normal Visualization",
                vec!["normals", "normal"],
            ),
            (
                ColorMode::RaySteps,
                "Ray Steps Visualization",
                vec!["ray steps", "steps"],
            ),
            (
                ColorMode::DistanceField,
                "Distance Field Debug",
                vec!["distance field", "df", "debug df"],
            ),
            (
                ColorMode::Depth,
                "Depth Debug",
                vec!["depth", "debug depth"],
            ),
            (
                ColorMode::Convergence,
                "Convergence Debug",
                vec!["convergence", "debug conv"],
            ),
            (
                ColorMode::LightingOnly,
                "Lighting Only Debug",
                vec!["lighting", "debug light"],
            ),
            (
                ColorMode::ShadowMap,
                "Shadow Map Debug",
                vec!["shadow map", "debug shadow"],
            ),
            (
                ColorMode::CameraDistanceLOD,
                "Camera Distance LOD Debug",
                vec!["lod distance", "debug lod"],
            ),
        ];

        for (mode, name, aliases) in color_modes {
            commands.push(
                Command::new(
                    name,
                    CommandCategory::Color,
                    CommandAction::SetColorMode(mode),
                    format!("Switch to {} mode", name),
                )
                .with_aliases(aliases),
            );
        }

        // === LOD Commands ===
        commands.push(
            Command::new(
                "Enable LOD System",
                CommandCategory::LOD,
                CommandAction::ToggleLOD,
                "Toggle adaptive Level of Detail system",
            )
            .with_aliases(vec!["lod", "lod on", "lod enable", "enable lod"]),
        );

        commands.push(
            Command::new(
                "LOD Debug Visualization",
                CommandCategory::LOD,
                CommandAction::ToggleLODDebug,
                "Toggle LOD debug overlay",
            )
            .with_aliases(vec!["lod debug", "debug lod", "lod viz"]),
        );

        let lod_profiles = vec![
            (
                LODProfile::Balanced,
                "LOD: Balanced",
                vec!["lod balanced", "balanced"],
            ),
            (
                LODProfile::QualityFirst,
                "LOD: Quality First",
                vec!["lod quality", "quality first"],
            ),
            (
                LODProfile::PerformanceFirst,
                "LOD: Performance First",
                vec!["lod performance", "performance first"],
            ),
            (
                LODProfile::DistanceOnly,
                "LOD: Distance Only",
                vec!["lod distance only", "distance only"],
            ),
            (
                LODProfile::MotionOnly,
                "LOD: Motion Only",
                vec!["lod motion only", "motion only"],
            ),
        ];

        for (profile, name, aliases) in lod_profiles {
            commands.push(
                Command::new(
                    name,
                    CommandCategory::LOD,
                    CommandAction::SetLODProfile(profile),
                    format!("Set LOD profile to {}", name),
                )
                .with_aliases(aliases),
            );
        }

        // === UI Commands ===
        commands.push(
            Command::new(
                "Toggle UI",
                CommandCategory::UI,
                CommandAction::ToggleUI,
                "Show/hide the user interface",
            )
            .with_aliases(vec!["ui", "hide ui", "show ui", "toggle ui"])
            .with_shortcut("H"),
        );

        commands.push(
            Command::new(
                "Toggle Performance Overlay",
                CommandCategory::UI,
                CommandAction::ToggleStats,
                "Show/hide FPS and performance graph",
            )
            .with_aliases(vec!["stats", "fps", "performance", "perf"])
            .with_shortcut("V"),
        );

        commands.push(
            Command::new(
                "Toggle Theme",
                CommandCategory::UI,
                CommandAction::CycleTheme,
                "Switch between dark and light themes",
            )
            .with_aliases(vec!["theme", "dark", "light"]),
        );

        // === Camera Commands ===
        commands.push(
            Command::new(
                "Reset View",
                CommandCategory::Camera,
                CommandAction::ResetView,
                "Reset camera to default position",
            )
            .with_aliases(vec!["reset", "reset camera", "reset view"])
            .with_shortcut("R"),
        );

        // === Recording Commands ===
        commands.push(
            Command::new(
                "Start MP4 Recording",
                CommandCategory::Recording,
                CommandAction::StartRecording("mp4".to_string()),
                "Start recording video as MP4",
            )
            .with_aliases(vec!["record mp4", "mp4", "video"]),
        );

        commands.push(
            Command::new(
                "Start WebM Recording",
                CommandCategory::Recording,
                CommandAction::StartRecording("webm".to_string()),
                "Start recording video as WebM",
            )
            .with_aliases(vec!["record webm", "webm"]),
        );

        commands.push(
            Command::new(
                "Start GIF Recording",
                CommandCategory::Recording,
                CommandAction::StartRecording("gif".to_string()),
                "Start recording animated GIF",
            )
            .with_aliases(vec!["record gif", "gif", "animate"]),
        );

        commands.push(
            Command::new(
                "Stop Recording",
                CommandCategory::Recording,
                CommandAction::StopRecording,
                "Stop current recording",
            )
            .with_aliases(vec!["stop", "stop recording", "stop video"]),
        );

        commands.push(
            Command::new(
                "Take Screenshot",
                CommandCategory::Recording,
                CommandAction::Screenshot,
                "Save current view as PNG image",
            )
            .with_aliases(vec!["screenshot", "capture", "snap", "save image"]),
        );

        // === Settings Commands ===
        commands.push(
            Command::new(
                "Save Preset",
                CommandCategory::Settings,
                CommandAction::SavePreset,
                "Save current settings as a preset",
            )
            .with_aliases(vec!["save", "save preset", "preset save"]),
        );

        commands.push(
            Command::new(
                "Export Settings",
                CommandCategory::Settings,
                CommandAction::ExportSettings,
                "Export current settings to JSON file",
            )
            .with_aliases(vec!["export", "export settings", "save settings"]),
        );

        commands.push(
            Command::new(
                "Import Settings",
                CommandCategory::Settings,
                CommandAction::ImportSettings,
                "Import settings from JSON file",
            )
            .with_aliases(vec!["import", "import settings", "load settings"]),
        );

        commands.push(
            Command::new(
                "Reset All Settings",
                CommandCategory::Settings,
                CommandAction::ResetAll,
                "Reset all parameters to defaults",
            )
            .with_aliases(vec!["reset all", "defaults", "reset settings"]),
        );

        commands
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match() {
        assert!(fuzzy_match("man", "mandelbrot").is_some());
        assert!(fuzzy_match("mdb", "mandelbulb").is_some());
        assert!(fuzzy_match("xyz", "mandelbrot").is_none());
    }

    #[test]
    fn test_command_matching() {
        let cmd = Command::new(
            "Mandelbrot",
            CommandCategory::Fractal,
            CommandAction::SetFractalType(FractalType::Mandelbrot2D),
            "Classic Mandelbrot",
        )
        .with_aliases(vec!["mb", "mbrot"]);

        assert!(cmd.match_score("man").is_some());
        assert!(cmd.match_score("mb").is_some());
        assert!(cmd.match_score("xyz").is_none());
    }

    #[test]
    fn test_palette_filtering() {
        let mut palette = CommandPalette::new();
        palette.set_query("mandel".to_string());

        assert!(!palette.filtered_commands.is_empty());
        assert!(palette
            .filtered_commands
            .iter()
            .any(|(_, cmd)| cmd.name.contains("Mandel")));
    }
}
