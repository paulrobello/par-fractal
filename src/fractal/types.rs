use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FractalType {
    // 2D Fractals - Escape Time
    Mandelbrot2D,
    Julia2D,
    Sierpinski2D,
    SierpinskiTriangle2D,
    BurningShip2D,
    Tricorn2D,
    Phoenix2D,
    Celtic2D,
    Newton2D,
    Lyapunov2D,
    Nova2D,
    Magnet2D,
    Collatz2D,

    // 3D Fractals
    Mandelbulb3D,
    MengerSponge3D,
    SierpinskiPyramid3D,
    JuliaSet3D,
    Mandelbox3D,
    OctahedralIFS3D,
    IcosahedralIFS3D,
    ApollonianGasket3D,
    Kleinian3D,
    HybridMandelbulbJulia3D,
    QuaternionCubic3D,
    SierpinskiGasket3D,

    // 2D Fractals - Density/Accumulation based
    Buddhabrot2D,

    // 2D Fractals - Strange Attractors (from xfractint)
    Hopalong2D,
    Martin2D,
    Gingerbreadman2D,
    Chip2D,
    Quadruptwo2D,
    Threeply2D,

    // 3D Fractals - Strange Attractors
    Pickover3D,
    Lorenz3D,
    Rossler3D,
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RenderMode {
    TwoD,
    ThreeD,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum ShadingModel {
    BlinnPhong,
    PBR,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ColorMode {
    Palette,          // Standard palette coloring
    RaySteps,         // Visualize number of ray marching steps
    Normals,          // Visualize surface normals
    OrbitTrapXYZ,     // Color based on XYZ coordinates during iteration
    OrbitTrapRadial,  // Color based on radial distance during iteration
    WorldPosition,    // Color based on world position
    LocalPosition,    // Color based on local/fractal-space position
    AmbientOcclusion, // Visualize AO only
    PerChannel,       // Per-channel mapping (custom R/G/B sources)
    // Debug visualization modes
    DistanceField,     // Visualize distance estimator values
    Depth,             // Visualize surface depth from camera
    Convergence,       // Visualize convergence/escape time (2D fractals)
    LightingOnly,      // Show only lighting (no fractal coloring)
    ShadowMap,         // Visualize shadow values
    CameraDistanceLOD, // Visualize camera distance using LOD zone colors
    DistanceGrayscale, // Visualize raw distance from camera as grayscale
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ChannelSource {
    Iterations, // Number of iterations/steps
    Distance,   // Distance to surface
    PositionX,  // X coordinate
    PositionY,  // Y coordinate
    PositionZ,  // Z coordinate
    Normal,     // Surface normal component
    AO,         // Ambient occlusion value
    Constant,   // Fixed value (0.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FogMode {
    Linear,      // Linear fog falloff
    Exponential, // Exponential fog falloff
    Quadratic,   // Quadratic (exponential squared) fog falloff
}

/// Procedural palette types that generate colors mathematically
/// These use cosine-based formulas for smooth, continuous color gradients
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ProceduralPalette {
    /// No procedural palette - use static color palette
    #[default]
    None,
    /// Fire Storm - RGB phase-shifted cosines (classic Fractint firestrm)
    /// r = (cos(a) + 1) / 2
    /// g = (cos(a + 2π/3) + 1) / 2
    /// b = (cos(a + 4π/3) + 1) / 2
    Firestrm,
    /// Rainbow - full spectrum HSV-like gradient
    Rainbow,
    /// Electric Blue - cyan to blue to purple
    Electric,
    /// Sunset - warm oranges to purples
    Sunset,
    /// Forest - greens and earth tones
    Forest,
    /// Ocean - deep blues to cyan
    Ocean,
    /// Grayscale - simple black to white
    Grayscale,
    /// Hot - black to red to yellow to white
    Hot,
    /// Cool - cyan to magenta gradient
    Cool,
    /// Plasma - purple to orange (scientific visualization)
    Plasma,
    /// Viridis - perceptually uniform (scientific visualization)
    Viridis,
    /// Custom - user-defined cosine palette parameters
    Custom,
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
