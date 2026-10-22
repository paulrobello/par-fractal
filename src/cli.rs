//! CLI definition (ENH-003): clap-derive parser replacing main.rs's
//! hand-rolled loop. Lives in the library so `tests/cli.rs` can exercise it
//! with `try_parse_from`; gated to the `native` feature (the binary is
//! native-only).

use clap::Parser;

/// Par Fractal - GPU Accelerated Fractal Renderer
#[derive(Parser, Debug)]
#[command(name = "par-fractal", version, about, propagate_version = true)]
pub struct Cli {
    /// Clear all saved settings and start fresh
    #[arg(long)]
    pub clear_settings: bool,

    /// Load a specific preset on startup
    #[arg(long, value_name = "NAME")]
    pub preset: Option<String>,

    /// Set quality level: low, medium, high, ultra
    #[arg(short, long, value_parser = parse_quality, value_name = "LEVEL")]
    pub quality: Option<usize>,

    /// List available presets and exit
    #[arg(long)]
    pub list_presets: bool,

    /// Take a screenshot after N seconds
    #[arg(long, value_parser = parse_delay, value_name = "SECONDS")]
    pub screenshot_delay: Option<f32>,

    /// Write the screenshot to this exact path (no timestamp)
    #[arg(long, value_name = "PATH")]
    pub screenshot_path: Option<std::path::PathBuf>,

    /// Exit application after N seconds
    #[arg(long, value_parser = parse_delay, value_name = "SECONDS")]
    pub exit_delay: Option<f32>,

    /// Write EMA-smoothed GPU profile timings (YAML) after warmup
    #[arg(long, value_name = "PATH")]
    pub profile_dump: Option<std::path::PathBuf>,

    /// Force the window to WxH physical pixels
    #[arg(long, value_parser = parse_size, value_name = "WxH")]
    pub window_size: Option<(u32, u32)>,

    /// Resize the window to WxH after N seconds
    #[arg(long, num_args = 2, value_names = ["SECONDS", "WxH"])]
    pub resize_after: Option<Vec<String>>,

    /// Switch to FractalType after N seconds (agent testing)
    #[arg(long, num_args = 2, value_names = ["FRACTAL_TYPE", "SECONDS"])]
    pub switch_after: Option<Vec<String>>,

    /// Place the 3D camera at x,y,z, looking at the origin (agent testing)
    #[arg(long, value_parser = parse_vec3, value_name = "x,y,z")]
    pub camera_pos: Option<[f32; 3]>,

    /// Generate shell completions for the given shell and exit
    #[arg(long, hide = true, value_name = "SHELL")]
    pub generate_completions: Option<clap_complete::Shell>,
}

/// Quality level parser delegating to the shared launch-params table
/// (launch_params::parse_quality_level returns Option; clap wants Result).
fn parse_quality(s: &str) -> Result<usize, String> {
    crate::launch_params::parse_quality_level(s)
        .ok_or_else(|| format!("invalid quality level `{s}` (valid: low, medium, high, ultra)"))
}

/// Delay value parser: finite and non-negative (ENH-003 — the old loop
/// accepted `-1` and `inf` silently).
fn parse_delay(s: &str) -> Result<f32, String> {
    let v: f32 = s
        .parse()
        .map_err(|_| format!("invalid delay value `{s}` (expected seconds)"))?;
    if !v.is_finite() {
        return Err(format!("delay must be finite, got `{s}`"));
    }
    if v < 0.0 {
        return Err(format!("delay must be >= 0, got `{s}`"));
    }
    Ok(v)
}

/// `WxH` window-size parser (both dimensions > 0).
fn parse_size(s: &str) -> Result<(u32, u32), String> {
    let (w, h) = s
        .split_once('x')
        .ok_or_else(|| format!("invalid size `{s}` (expected WxH, e.g. 256x256)"))?;
    let w: u32 = w
        .parse()
        .map_err(|_| format!("invalid width in `{s}` (expected WxH, e.g. 256x256)"))?;
    let h: u32 = h
        .parse()
        .map_err(|_| format!("invalid height in `{s}` (expected WxH, e.g. 256x256)"))?;
    if w == 0 || h == 0 {
        return Err(format!("size must be positive, got `{s}`"));
    }
    Ok((w, h))
}

/// `x,y,z` camera-position parser (finite floats).
fn parse_vec3(s: &str) -> Result<[f32; 3], String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 3 {
        return Err(format!("invalid position `{s}` (expected x,y,z)"));
    }
    let mut out = [0.0f32; 3];
    for (slot, part) in out.iter_mut().zip(&parts) {
        *slot = part
            .trim()
            .parse()
            .map_err(|_| format!("invalid coordinate `{s}` (expected x,y,z)"))?;
    }
    Ok(out)
}
