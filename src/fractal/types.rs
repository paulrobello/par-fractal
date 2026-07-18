//! GPU wire-format enums shared between Rust and the WGSL shaders.
//!
//! Each `#[repr(u32)]` enum below has discriminants that are written verbatim
//! into `FractalUniforms` and read back by the shaders in
//! `src/shaders/fractal.wgsl`. Renumbering any variant is a wire-protocol
//! change — the shader's `switch` arms and the `gpu_discriminant_roundtrip`
//! test at the bottom of this file must move with it.

use serde::{Deserialize, Serialize};

/// Discriminant contract: the integer IDs here are the wire format the WGSL
/// shaders read from `uniforms.fractal_type`. They MUST match the
/// `switch fractal_type` arms in `src/shaders/fractal.wgsl` exactly. The
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

impl FractalType {
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
        matches!(
            self,
            FractalType::Mandelbulb3D
                | FractalType::MengerSponge3D
                | FractalType::SierpinskiPyramid3D
                | FractalType::JuliaSet3D
                | FractalType::Mandelbox3D
                | FractalType::OctahedralIFS3D
                | FractalType::IcosahedralIFS3D
                | FractalType::ApollonianGasket3D
                | FractalType::Kleinian3D
                | FractalType::HybridMandelbulbJulia3D
                | FractalType::QuaternionCubic3D
                | FractalType::SierpinskiGasket3D
                // 3D strange attractors
                | FractalType::Pickover3D
                | FractalType::Lorenz3D
                | FractalType::Rossler3D
        )
    }

    /// Returns true if this fractal type uses accumulation rendering
    pub fn uses_accumulation(&self) -> bool {
        self.is_2d_attractor() || self.is_buddhabrot()
    }

    /// Returns the index of the 2D attractor type for the compute shader.
    /// Returns 0 if not a 2D attractor.
    pub fn attractor_index(&self) -> u32 {
        match self {
            FractalType::Hopalong2D => 0,
            FractalType::Martin2D => 1,
            FractalType::Gingerbreadman2D => 2,
            FractalType::Chip2D => 3,
            FractalType::Quadruptwo2D => 4,
            FractalType::Threeply2D => 5,
            _ => 0,
        }
    }

    /// Returns a filename-safe name for this fractal type
    pub fn filename_safe_name(&self) -> &'static str {
        match self {
            FractalType::Mandelbrot2D => "mandelbrot",
            FractalType::Julia2D => "julia",
            FractalType::Sierpinski2D => "sierpinski",
            FractalType::SierpinskiTriangle2D => "sierpinski_triangle",
            FractalType::BurningShip2D => "burning_ship",
            FractalType::Tricorn2D => "tricorn",
            FractalType::Phoenix2D => "phoenix",
            FractalType::Celtic2D => "celtic",
            FractalType::Newton2D => "newton",
            FractalType::Lyapunov2D => "lyapunov",
            FractalType::Nova2D => "nova",
            FractalType::Magnet2D => "magnet",
            FractalType::Collatz2D => "collatz",
            FractalType::Mandelbulb3D => "mandelbulb",
            FractalType::MengerSponge3D => "menger_sponge",
            FractalType::SierpinskiPyramid3D => "sierpinski_pyramid",
            FractalType::JuliaSet3D => "julia_3d",
            FractalType::Mandelbox3D => "mandelbox",
            FractalType::OctahedralIFS3D => "octahedral_ifs",
            FractalType::IcosahedralIFS3D => "icosahedral_ifs",
            FractalType::ApollonianGasket3D => "apollonian",
            FractalType::Kleinian3D => "kleinian",
            FractalType::HybridMandelbulbJulia3D => "hybrid_bulb_julia",
            FractalType::QuaternionCubic3D => "quaternion_cubic",
            FractalType::SierpinskiGasket3D => "sierpinski_gasket",
            // Density/Accumulation based
            FractalType::Buddhabrot2D => "buddhabrot",
            // Strange Attractors 2D
            FractalType::Hopalong2D => "hopalong",
            FractalType::Martin2D => "martin",
            FractalType::Gingerbreadman2D => "gingerbreadman",
            FractalType::Chip2D => "chip",
            FractalType::Quadruptwo2D => "quadruptwo",
            FractalType::Threeply2D => "threeply",
            // Strange Attractors 3D
            FractalType::Pickover3D => "pickover",
            FractalType::Lorenz3D => "lorenz",
            FractalType::Rossler3D => "rossler",
        }
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
/// `switch color_mode` arms in `fractal.wgsl`.
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
/// channel-source switch in `fractal.wgsl`.
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
/// 0..=12 — keep in lockstep with the `switch procedural_palette_type` arms in
/// `fractal.wgsl`.
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
}
