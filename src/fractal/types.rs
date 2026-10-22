//! GPU wire-format enums shared between Rust and the WGSL shaders.
//!
//! Each `#[repr(u32)]` enum below has discriminants that are written verbatim
//! into `Uniforms` (renderer/uniforms.rs) and read back by the shaders in
//! `src/shaders/fractal/` fragments. Renumbering any variant is a wire-protocol
//! change — the shader's `switch` arms and the `gpu_discriminant_roundtrip`
//! test at the bottom of this file must move with it.

use serde::{Deserialize, Serialize};

/// Discriminant contract: the integer IDs here are the wire format the WGSL
/// shaders read from `uniforms.fractal_type`. They MUST match the
/// `switch fractal_type` arms in `src/shaders/fractal/` exactly. The
/// `gpu_discriminant_roundtrip` test in this file pins representative values;
/// update both the shader and that test together if you renumber.
///
/// Serde derives by NAME (settings.yaml stores `"Mandelbrot2D"` etc.), so
/// renumbering does not break saved settings — but it WILL render the wrong
/// fractal until the shader is updated.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FractalType {
    // 2D Fractals - Escape Time (shader IDs 0..=12)
    Mandelbrot2D = 0,
    Julia2D = 1,
    Sierpinski2D = 2,
    SierpinskiTriangle2D = 3,
    BurningShip2D = 4,
    Tricorn2D = 5,
    Phoenix2D = 6,
    Celtic2D = 7,
    Newton2D = 8,
    Lyapunov2D = 9,
    Nova2D = 10,
    Magnet2D = 11,
    Collatz2D = 12,

    // 3D Fractals (shader IDs 13..=24)
    Mandelbulb3D = 13,
    MengerSponge3D = 14,
    SierpinskiPyramid3D = 15,
    JuliaSet3D = 16,
    Mandelbox3D = 17,
    OctahedralIFS3D = 18,
    IcosahedralIFS3D = 19,
    ApollonianGasket3D = 20,
    Kleinian3D = 21,
    HybridMandelbulbJulia3D = 22,
    QuaternionCubic3D = 23,
    SierpinskiGasket3D = 24,

    // 2D Fractals - Density/Accumulation based (shader ID 25)
    Buddhabrot2D = 25,

    // 2D Fractals - Strange Attractors, from xfractint (shader IDs 26..=31)
    Hopalong2D = 26,
    Martin2D = 27,
    Gingerbreadman2D = 28,
    Chip2D = 29,
    Quadruptwo2D = 30,
    Threeply2D = 31,

    // 3D Fractals - Strange Attractors (shader IDs 35..=37; 32..=34 reserved
    // for future 2D attractor expansion — do not renumber)
    Pickover3D = 35,
    Lorenz3D = 36,
    Rossler3D = 37,
}

/// Static per-variant metadata (QA-034).
///
/// One table entry per variant, written once in [`FractalType::info`]; the
/// accessor methods (`is_3d`, `filename_safe_name`, `default_camera_distance`,
/// `uses_accumulation`, `attractor_index`) delegate here so a new variant
/// fills in one row instead of one match arm per accessor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FractalInfo {
    /// Human-readable name (help overlay, UI lists).
    pub label: &'static str,
    /// Filename-safe identifier (screenshots, exports).
    pub filename: &'static str,
    /// True for the 3D ray-marched pipeline, false for 2D.
    pub is_3d: bool,
    /// Default 3D camera framing distance (ignored for 2D types).
    pub camera_distance: f32,
    /// Keyboard shortcut from `app/input.rs` (`"1"`..`"0"`, `"F1"`..`"F10"`),
    /// or `None` for types without a hotkey (UI-only selection).
    pub hotkey: Option<&'static str>,
    /// Renders via the accumulation (compute) path: 2D attractors + Buddhabrot.
    pub uses_accumulation: bool,
    /// 2D-attractor index for the compute shader (0 for non-attractors).
    pub attractor_index: u32,
}

impl FractalType {
    /// Every fractal variant, in discriminant order (QA-034).
    ///
    /// The single enumeration consumers iterate — help overlay, tests, metadata —
    /// so adding a variant updates ONE list here instead of a match arm per call
    /// site.
    pub const ALL: &[FractalType] = &[
        FractalType::Mandelbrot2D,
        FractalType::Julia2D,
        FractalType::Sierpinski2D,
        FractalType::SierpinskiTriangle2D,
        FractalType::BurningShip2D,
        FractalType::Tricorn2D,
        FractalType::Phoenix2D,
        FractalType::Celtic2D,
        FractalType::Newton2D,
        FractalType::Lyapunov2D,
        FractalType::Nova2D,
        FractalType::Magnet2D,
        FractalType::Collatz2D,
        FractalType::Mandelbulb3D,
        FractalType::MengerSponge3D,
        FractalType::SierpinskiPyramid3D,
        FractalType::JuliaSet3D,
        FractalType::Mandelbox3D,
        FractalType::OctahedralIFS3D,
        FractalType::IcosahedralIFS3D,
        FractalType::ApollonianGasket3D,
        FractalType::Kleinian3D,
        FractalType::HybridMandelbulbJulia3D,
        FractalType::QuaternionCubic3D,
        FractalType::SierpinskiGasket3D,
        FractalType::Buddhabrot2D,
        FractalType::Hopalong2D,
        FractalType::Martin2D,
        FractalType::Gingerbreadman2D,
        FractalType::Chip2D,
        FractalType::Quadruptwo2D,
        FractalType::Threeply2D,
        FractalType::Pickover3D,
        FractalType::Lorenz3D,
        FractalType::Rossler3D,
    ];

    /// Per-variant metadata. Single source for the accessors below; add new
    /// variants here (and to [`ALL`]) rather than editing each accessor.
    pub fn info(self) -> FractalInfo {
        use FractalType::*;
        match self {
            Mandelbrot2D => FractalInfo {
                label: "Mandelbrot",
                filename: "mandelbrot",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: Some("1"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            Julia2D => FractalInfo {
                label: "Julia",
                filename: "julia",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: Some("2"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            Sierpinski2D => FractalInfo {
                label: "Sierpinski",
                filename: "sierpinski",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: Some("3"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            SierpinskiTriangle2D => FractalInfo {
                label: "Sierpinski Triangle",
                filename: "sierpinski_triangle",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: None,
                uses_accumulation: false,
                attractor_index: 0,
            },
            BurningShip2D => FractalInfo {
                label: "Burning Ship",
                filename: "burning_ship",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: Some("4"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            Tricorn2D => FractalInfo {
                label: "Tricorn",
                filename: "tricorn",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: Some("5"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            Phoenix2D => FractalInfo {
                label: "Phoenix",
                filename: "phoenix",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: Some("6"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            Celtic2D => FractalInfo {
                label: "Celtic",
                filename: "celtic",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: Some("7"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            Newton2D => FractalInfo {
                label: "Newton",
                filename: "newton",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: Some("8"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            Lyapunov2D => FractalInfo {
                label: "Lyapunov",
                filename: "lyapunov",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: Some("9"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            Nova2D => FractalInfo {
                label: "Nova",
                filename: "nova",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: Some("0"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            Magnet2D => FractalInfo {
                label: "Magnet",
                filename: "magnet",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: None,
                uses_accumulation: false,
                attractor_index: 0,
            },
            Collatz2D => FractalInfo {
                label: "Collatz",
                filename: "collatz",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: None,
                uses_accumulation: false,
                attractor_index: 0,
            },
            Mandelbulb3D => FractalInfo {
                label: "Mandelbulb",
                filename: "mandelbulb",
                is_3d: true,
                camera_distance: 5.0,
                hotkey: Some("F1"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            MengerSponge3D => FractalInfo {
                label: "Menger Sponge",
                filename: "menger_sponge",
                is_3d: true,
                camera_distance: 5.0,
                hotkey: Some("F2"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            SierpinskiPyramid3D => FractalInfo {
                label: "Sierpinski Pyramid",
                filename: "sierpinski_pyramid",
                is_3d: true,
                camera_distance: 5.0,
                hotkey: Some("F3"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            JuliaSet3D => FractalInfo {
                label: "Julia Set 3D",
                filename: "julia_3d",
                is_3d: true,
                camera_distance: 5.0,
                hotkey: Some("F4"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            Mandelbox3D => FractalInfo {
                label: "Mandelbox",
                filename: "mandelbox",
                is_3d: true,
                camera_distance: 5.0,
                hotkey: Some("F5"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            OctahedralIFS3D => FractalInfo {
                label: "Octahedral IFS",
                filename: "octahedral_ifs",
                is_3d: true,
                camera_distance: 9.0,
                hotkey: Some("F6"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            IcosahedralIFS3D => FractalInfo {
                label: "Icosahedral IFS",
                filename: "icosahedral_ifs",
                is_3d: true,
                camera_distance: 9.0,
                hotkey: Some("F7"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            ApollonianGasket3D => FractalInfo {
                label: "Apollonian Gasket",
                filename: "apollonian",
                is_3d: true,
                camera_distance: 8.0,
                hotkey: Some("F8"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            Kleinian3D => FractalInfo {
                label: "Kleinian",
                filename: "kleinian",
                is_3d: true,
                camera_distance: 5.0,
                hotkey: Some("F9"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            HybridMandelbulbJulia3D => FractalInfo {
                label: "Hybrid Bulb-Julia",
                filename: "hybrid_bulb_julia",
                is_3d: true,
                camera_distance: 5.0,
                hotkey: Some("F10"),
                uses_accumulation: false,
                attractor_index: 0,
            },
            QuaternionCubic3D => FractalInfo {
                label: "Quaternion Cubic",
                filename: "quaternion_cubic",
                is_3d: true,
                camera_distance: 5.0,
                hotkey: None,
                uses_accumulation: false,
                attractor_index: 0,
            },
            SierpinskiGasket3D => FractalInfo {
                label: "Sierpinski Gasket",
                filename: "sierpinski_gasket",
                is_3d: true,
                camera_distance: 5.0,
                hotkey: None,
                uses_accumulation: false,
                attractor_index: 0,
            },
            Buddhabrot2D => FractalInfo {
                label: "Buddhabrot",
                filename: "buddhabrot",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: None,
                uses_accumulation: true,
                attractor_index: 0,
            },
            Hopalong2D => FractalInfo {
                label: "Hopalong",
                filename: "hopalong",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: None,
                uses_accumulation: true,
                attractor_index: 0,
            },
            Martin2D => FractalInfo {
                label: "Martin",
                filename: "martin",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: None,
                uses_accumulation: true,
                attractor_index: 1,
            },
            Gingerbreadman2D => FractalInfo {
                label: "Gingerbreadman",
                filename: "gingerbreadman",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: None,
                uses_accumulation: true,
                attractor_index: 2,
            },
            Chip2D => FractalInfo {
                label: "Chip",
                filename: "chip",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: None,
                uses_accumulation: true,
                attractor_index: 3,
            },
            Quadruptwo2D => FractalInfo {
                label: "Quadruptwo",
                filename: "quadruptwo",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: None,
                uses_accumulation: true,
                attractor_index: 4,
            },
            Threeply2D => FractalInfo {
                label: "Threeply",
                filename: "threeply",
                is_3d: false,
                camera_distance: 5.0,
                hotkey: None,
                uses_accumulation: true,
                attractor_index: 5,
            },
            Pickover3D => FractalInfo {
                label: "Pickover",
                filename: "pickover",
                is_3d: true,
                camera_distance: 3.0,
                hotkey: None,
                uses_accumulation: false,
                attractor_index: 0,
            },
            Lorenz3D => FractalInfo {
                label: "Lorenz",
                filename: "lorenz",
                is_3d: true,
                camera_distance: 5.0,
                hotkey: None,
                uses_accumulation: false,
                attractor_index: 0,
            },
            Rossler3D => FractalInfo {
                label: "Rössler",
                filename: "rossler",
                is_3d: true,
                camera_distance: 5.0,
                hotkey: None,
                uses_accumulation: false,
                attractor_index: 0,
            },
        }
    }

    /// Returns true if this is a 2D strange attractor type
    pub fn is_2d_attractor(&self) -> bool {
        matches!(
            self,
            FractalType::Hopalong2D
                | FractalType::Martin2D
                | FractalType::Gingerbreadman2D
                | FractalType::Chip2D
                | FractalType::Quadruptwo2D
                | FractalType::Threeply2D
        )
    }

    /// Returns true if this is the Buddhabrot fractal type
    pub fn is_buddhabrot(&self) -> bool {
        matches!(self, FractalType::Buddhabrot2D)
    }

    /// Returns true if this is a 3D fractal type (ray-marched or 3D attractor).
    ///
    /// Used to select between the 2D and 3D fragment pipelines (ARC-009/ENH-004):
    /// `fs_main_2d` runs the escape-time + palette path; `fs_main_3d` runs the
    /// ray-march + lighting path. The set of 3D types must stay in lockstep with
    /// the `render_mode` uniform written by `Uniforms::update` (3D types set it
    /// to `1`) so pipeline selection and shader dispatch agree.
    pub fn is_3d(&self) -> bool {
        // QA-034: delegated to the FractalInfo table (single source of truth).
        self.info().is_3d
    }

    /// Distance along +Z at which the default camera frames this fractal,
    /// looking at the origin.
    ///
    /// World-space extent varies widely across the 3D types: a Mandelbulb fits
    /// inside radius ~2, while the folding IFS types sprawl past radius 5. A
    /// single shared default therefore left the camera *inside* the structure
    /// for the larger ones, which renders as a wall of clipped geometry rather
    /// than a fractal. Values are measured per type via `--camera-pos`.
    ///
    /// Meaningless for 2D types (which ignore the camera); they get the
    /// baseline so the value is always well defined.
    pub fn default_camera_distance(&self) -> f32 {
        self.info().camera_distance
    }

    /// Returns true if this fractal type uses accumulation rendering
    pub fn uses_accumulation(&self) -> bool {
        self.info().uses_accumulation
    }

    /// Returns the index of the 2D attractor type for the compute shader.
    /// Returns 0 if not a 2D attractor.
    pub fn attractor_index(&self) -> u32 {
        self.info().attractor_index
    }

    /// Returns a filename-safe name for this fractal type
    pub fn filename_safe_name(&self) -> &'static str {
        self.info().filename
    }
}

/// GPU wire format: 0 = TwoD, 1 = ThreeD. Matches `uniforms.render_mode`.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RenderMode {
    TwoD = 0,
    ThreeD = 1,
}

/// GPU wire format: 0 = BlinnPhong, 1 = PBR. Matches `uniforms.shading_model`.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum ShadingModel {
    BlinnPhong = 0,
    PBR = 1,
}

/// GPU wire format: the discriminant is the value read by the shader from
/// `uniforms.color_mode`. Sequential 0..=15 — keep in lockstep with the
/// `color_mode` arms in `render_ray.wgsl`.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ColorMode {
    Palette = 0,          // Standard palette coloring
    RaySteps = 1,         // Visualize number of ray marching steps
    Normals = 2,          // Visualize surface normals
    OrbitTrapXYZ = 3,     // Color based on XYZ coordinates during iteration
    OrbitTrapRadial = 4,  // Color based on radial distance during iteration
    WorldPosition = 5,    // Color based on world position
    LocalPosition = 6,    // Color based on local/fractal-space position
    AmbientOcclusion = 7, // Visualize AO only
    PerChannel = 8,       // Per-channel mapping (custom R/G/B sources)
    // Debug visualization modes
    DistanceField = 9,      // Visualize distance estimator values
    Depth = 10,             // Visualize surface depth from camera
    Convergence = 11,       // Visualize convergence/escape time (2D fractals)
    LightingOnly = 12,      // Show only lighting (no fractal coloring)
    ShadowMap = 13,         // Visualize shadow values
    CameraDistanceLOD = 14, // Visualize camera distance using LOD zone colors
    DistanceGrayscale = 15, // Visualize raw distance from camera as grayscale
}

/// GPU wire format: the discriminant is the value written to
/// `uniforms.channel_r/g/b`. Sequential 0..=7 — keep in lockstep with the
/// channel-source arms in `render_ray.wgsl`.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ChannelSource {
    Iterations = 0, // Number of iterations/steps
    Distance = 1,   // Distance to surface
    PositionX = 2,  // X coordinate
    PositionY = 3,  // Y coordinate
    PositionZ = 4,  // Z coordinate
    Normal = 5,     // Surface normal component
    AO = 6,         // Ambient occlusion value
    Constant = 7,   // Fixed value (0.0)
}

/// GPU wire format: 0 = Linear, 1 = Exponential, 2 = Quadratic. Matches
/// `uniforms.fog_mode`.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FogMode {
    Linear = 0,      // Linear fog falloff
    Exponential = 1, // Exponential fog falloff
    Quadratic = 2,   // Quadratic (exponential squared) fog falloff
}

/// Procedural palette types that generate colors mathematically
/// These use cosine-based formulas for smooth, continuous color gradients
///
/// GPU wire format: the discriminant (matching the deprecated `shader_index()`
/// return values) is written to `uniforms.procedural_palette_type`. Sequential
/// 0..=12 — keep in lockstep with the `procedural_palette_type` arms in
/// `coloring.wgsl`.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ProceduralPalette {
    /// No procedural palette - use static color palette
    #[default]
    None = 0,
    /// Fire Storm - RGB phase-shifted cosines (classic Fractint firestrm)
    /// r = (cos(a) + 1) / 2
    /// g = (cos(a + 2π/3) + 1) / 2
    /// b = (cos(a + 4π/3) + 1) / 2
    Firestrm = 1,
    /// Rainbow - full spectrum HSV-like gradient
    Rainbow = 2,
    /// Electric Blue - cyan to blue to purple
    Electric = 3,
    /// Sunset - warm oranges to purples
    Sunset = 4,
    /// Forest - greens and earth tones
    Forest = 5,
    /// Ocean - deep blues to cyan
    Ocean = 6,
    /// Grayscale - simple black to white
    Grayscale = 7,
    /// Hot - black to red to yellow to white
    Hot = 8,
    /// Cool - cyan to magenta gradient
    Cool = 9,
    /// Plasma - purple to orange (scientific visualization)
    Plasma = 10,
    /// Viridis - perceptually uniform (scientific visualization)
    Viridis = 11,
    /// Custom - user-defined cosine palette parameters
    Custom = 12,
}

impl ProceduralPalette {
    /// All procedural palette variants (excluding None)
    pub const ALL: &'static [ProceduralPalette] = &[
        ProceduralPalette::Firestrm,
        ProceduralPalette::Rainbow,
        ProceduralPalette::Electric,
        ProceduralPalette::Sunset,
        ProceduralPalette::Forest,
        ProceduralPalette::Ocean,
        ProceduralPalette::Grayscale,
        ProceduralPalette::Hot,
        ProceduralPalette::Cool,
        ProceduralPalette::Plasma,
        ProceduralPalette::Viridis,
        ProceduralPalette::Custom,
    ];

    /// Returns the display name for this palette
    pub fn name(&self) -> &'static str {
        match self {
            ProceduralPalette::None => "None (Static)",
            ProceduralPalette::Firestrm => "Fire Storm",
            ProceduralPalette::Rainbow => "Rainbow",
            ProceduralPalette::Electric => "Electric",
            ProceduralPalette::Sunset => "Sunset",
            ProceduralPalette::Forest => "Forest",
            ProceduralPalette::Ocean => "Ocean",
            ProceduralPalette::Grayscale => "Grayscale",
            ProceduralPalette::Hot => "Hot",
            ProceduralPalette::Cool => "Cool",
            ProceduralPalette::Plasma => "Plasma",
            ProceduralPalette::Viridis => "Viridis",
            ProceduralPalette::Custom => "Custom",
        }
    }

    /// Returns the shader index for this procedural palette type
    pub fn shader_index(&self) -> u32 {
        match self {
            ProceduralPalette::None => 0,
            ProceduralPalette::Firestrm => 1,
            ProceduralPalette::Rainbow => 2,
            ProceduralPalette::Electric => 3,
            ProceduralPalette::Sunset => 4,
            ProceduralPalette::Forest => 5,
            ProceduralPalette::Ocean => 6,
            ProceduralPalette::Grayscale => 7,
            ProceduralPalette::Hot => 8,
            ProceduralPalette::Cool => 9,
            ProceduralPalette::Plasma => 10,
            ProceduralPalette::Viridis => 11,
            ProceduralPalette::Custom => 12,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// QA-017: Pin the GPU wire-format discriminants for every enum that
    /// crosses into WGSL. These IDs are the contract with the shader's
    /// `switch` arms — renumbering must be deliberate and paired with a
    /// shader edit. The values here were transcribed from the hand-written
    /// match tables that used to live in `renderer/uniforms.rs` (and the
    /// `shader_index()` impl for `ProceduralPalette`); they predate
    /// `#[repr(u32)]` and must not change as long as the shader does.
    #[test]
    fn gpu_discriminant_roundtrip() {
        // FractalType — boundaries, gap crossings, and a representative of
        // each contiguous block. The enum has intentional gaps (32..=34
        // reserved for future 2D attractor expansion).
        assert_eq!(FractalType::Mandelbrot2D as u32, 0); // first 2D escape
        assert_eq!(FractalType::Collatz2D as u32, 12); // last 2D escape
        assert_eq!(FractalType::Mandelbulb3D as u32, 13); // first 3D ray-marched
        assert_eq!(FractalType::SierpinskiGasket3D as u32, 24); // last 3D ray-marched
        assert_eq!(FractalType::Buddhabrot2D as u32, 25); // density (gap before)
        assert_eq!(FractalType::Hopalong2D as u32, 26); // first 2D attractor
        assert_eq!(FractalType::Threeply2D as u32, 31); // last 2D attractor
        assert_eq!(FractalType::Pickover3D as u32, 35); // first 3D attractor (gap)
        assert_eq!(FractalType::Rossler3D as u32, 37); // last variant overall

        // RenderMode
        assert_eq!(RenderMode::TwoD as u32, 0);
        assert_eq!(RenderMode::ThreeD as u32, 1);

        // ShadingModel
        assert_eq!(ShadingModel::BlinnPhong as u32, 0);
        assert_eq!(ShadingModel::PBR as u32, 1);

        // ColorMode — sequential 0..=15
        assert_eq!(ColorMode::Palette as u32, 0);
        assert_eq!(ColorMode::PerChannel as u32, 8);
        assert_eq!(ColorMode::DistanceGrayscale as u32, 15);

        // ChannelSource — sequential 0..=7
        assert_eq!(ChannelSource::Iterations as u32, 0);
        assert_eq!(ChannelSource::Constant as u32, 7);

        // FogMode
        assert_eq!(FogMode::Linear as u32, 0);
        assert_eq!(FogMode::Quadratic as u32, 2);

        // ProceduralPalette — agrees with `shader_index()` for every variant.
        assert_eq!(ProceduralPalette::None as u32, 0);
        assert_eq!(ProceduralPalette::Firestrm as u32, 1);
        assert_eq!(ProceduralPalette::Custom as u32, 12);
        for p in ProceduralPalette::ALL {
            assert_eq!(p.shader_index(), *p as u32);
        }
        assert_eq!(ProceduralPalette::None.shader_index(), 0);
    }

    /// QA-031: the 2D-attractor preview switch in `main_2d.wgsl` must map
    /// each 2D-attractor enum discriminant to the attractor function of the
    /// same name. The dispatch drifted once (Rust sent 26..=31, the shader
    /// mapped 27..=34 to henon/latoocarfian/icon among others), so five of
    /// six attractor types rendered the wrong fractal whenever accumulation
    /// was off. Parsing the shader here pins the wire format for both sides.
    #[test]
    fn wgsl_attractor_dispatch_matches_enum_discriminants() {
        let src = crate::fractal_sourced::SHADER;

        // (variant, discriminant, WGSL function name). Keep in sync with the
        // preview switch at the bottom of `main_2d.wgsl`.
        let cases: &[(&str, u32, &str)] = &[
            (
                stringify!(Hopalong2D),
                FractalType::Hopalong2D as u32,
                "hopalong_attractor",
            ),
            (
                stringify!(Martin2D),
                FractalType::Martin2D as u32,
                "martin_attractor",
            ),
            (
                stringify!(Gingerbreadman2D),
                FractalType::Gingerbreadman2D as u32,
                "gingerbreadman_attractor",
            ),
            (
                stringify!(Chip2D),
                FractalType::Chip2D as u32,
                "chip_attractor",
            ),
            (
                stringify!(Quadruptwo2D),
                FractalType::Quadruptwo2D as u32,
                "quadruptwo_attractor",
            ),
            (
                stringify!(Threeply2D),
                FractalType::Threeply2D as u32,
                "threeply_attractor",
            ),
        ];
        for (variant, discriminant, fn_name) in cases {
            let arm = format!("case {discriminant}u: {{ t = {fn_name}(coord); }}");
            assert!(
                src.contains(&arm),
                "{variant} (discriminant {discriminant}) has no matching preview-switch arm in \
                 main_2d.wgsl; update the shader switch and this test together"
            );
        }

        // The retired arms (henon / latoocarfian / icon — never in the enum)
        // must not appear in any preview-switch case line.
        for retired in [
            "henon_attractor",
            "latoocarfian_attractor",
            "icon_attractor",
        ] {
            for line in src.lines() {
                let trimmed = line.trim_start();
                assert!(
                    !(trimmed.starts_with("case") && trimmed.contains(retired)),
                    "preview switch still references {retired}, which is not a FractalType"
                );
            }
        }
    }

    /// QA-034: `ALL` iterates every variant exactly once.
    #[test]
    fn all_iterates_every_variant() {
        assert_eq!(FractalType::ALL.len(), 35);
        // Strictly increasing discriminant order (the enum has gaps at the
        // block boundaries — 25, 26..=31, 35..=37 — so equality is wrong).
        let mut prev = -1i32;
        for t in FractalType::ALL {
            let d = *t as u32 as i32;
            assert!(d > prev, "ALL not in discriminant order at {t:?}");
            prev = d;
        }
        for (i, t) in FractalType::ALL.iter().enumerate() {
            for other in &FractalType::ALL[i + 1..] {
                assert_ne!(t, other, "duplicate variant in ALL");
            }
        }
    }

    /// QA-034: the info table is complete and collision-free — filenames and
    /// hotkeys are unique, and the hotkeys the help overlay prints match
    /// app/input.rs (F6 = Octahedral IFS, F10 = Hybrid — the pair the old
    /// hand-written help got wrong).
    #[test]
    fn info_table_is_complete_and_consistent() {
        let mut filenames = std::collections::HashSet::new();
        let mut hotkeys = std::collections::HashSet::new();
        for t in FractalType::ALL {
            let info = t.info();
            assert!(!info.label.is_empty());
            assert!(!info.filename.is_empty());
            assert!(
                filenames.insert(info.filename),
                "duplicate filename {info:?}"
            );
            if let Some(key) = info.hotkey {
                assert!(hotkeys.insert(key), "duplicate hotkey {key}");
            }
            assert_eq!(info.is_3d, t.is_3d());
            assert_eq!(info.uses_accumulation, t.uses_accumulation());
            assert_eq!(info.camera_distance, t.default_camera_distance());
        }
        assert_eq!(
            FractalType::OctahedralIFS3D.info().hotkey,
            Some("F6"),
            "help/input drift the audit caught"
        );
        assert_eq!(
            FractalType::HybridMandelbulbJulia3D.info().hotkey,
            Some("F10")
        );
    }

    /// QA-034: render_mode derives from `is_3d()` for every variant — adding
    /// a variant requires no new match arm in `from_settings`/`switch_fractal`.
    #[test]
    fn render_mode_derives_from_is_3d_for_all_types() {
        for t in FractalType::ALL {
            let mut settings = crate::FractalParams::default().to_settings();
            settings.fractal_type = *t;
            let params = crate::FractalParams::from_settings(settings);
            let expected = if t.is_3d() {
                RenderMode::ThreeD
            } else {
                RenderMode::TwoD
            };
            assert_eq!(params.settings.render_mode, expected, "{t:?}");
            assert_eq!(params.settings.fractal_type, *t);
        }
    }
}
