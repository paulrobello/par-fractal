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
