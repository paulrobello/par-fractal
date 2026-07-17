use super::*;

#[test]
fn test_default_fractal_params() {
    let params = FractalParams::default();
    assert_eq!(params.fractal_type, FractalType::Mandelbrot2D);
    assert_eq!(params.render_mode, RenderMode::TwoD);
    assert_eq!(params.zoom_2d, 1.0);
    assert_eq!(params.max_iterations, 80);
}

#[test]
fn test_switch_fractal_2d_to_3d() {
    let mut params = FractalParams::default();
    params.switch_fractal(FractalType::Mandelbulb3D);
    assert_eq!(params.fractal_type, FractalType::Mandelbulb3D);
    assert_eq!(params.render_mode, RenderMode::ThreeD);
}

#[test]
fn test_switch_fractal_3d_to_2d() {
    let mut params = FractalParams::default();
    params.switch_fractal(FractalType::Mandelbulb3D);
    params.switch_fractal(FractalType::Julia2D);
    assert_eq!(params.fractal_type, FractalType::Julia2D);
    assert_eq!(params.render_mode, RenderMode::TwoD);
}

#[test]
fn test_palette_cycling() {
    let mut params = FractalParams::default();
    assert_eq!(params.palette_index, 0);
    assert_eq!(params.palette.name, "Fire");

    params.next_palette();
    assert_eq!(params.palette_index, 1);
    assert_eq!(params.palette.name, "Ocean");

    params.next_palette();
    assert_eq!(params.palette_index, 2);
    assert_eq!(params.palette.name, "Rainbow");
}

#[test]
fn test_palette_cycling_wraps() {
    let mut params = FractalParams::default();
    params.palette_index = ColorPalette::ALL.len() - 1;
    params.palette = ColorPalette::ALL[params.palette_index];

    params.next_palette();
    assert_eq!(params.palette_index, 0);
    assert_eq!(params.palette.name, "Fire");
}

#[test]
fn test_palette_prev() {
    let mut params = FractalParams {
        palette_index: 2,
        palette: ColorPalette::ALL[2],
        ..Default::default()
    };

    params.prev_palette();
    assert_eq!(params.palette_index, 1);
    assert_eq!(params.palette.name, "Ocean");
}

#[test]
fn test_palette_prev_wraps() {
    let mut params = FractalParams::default();
    assert_eq!(params.palette_index, 0);

    params.prev_palette();
    assert_eq!(params.palette_index, ColorPalette::ALL.len() - 1);
    assert_eq!(params.palette.name, "Volcano"); // Last palette (xfractint)
}

#[test]
fn test_all_palettes_exist() {
    // 21 original + 27 xfractint palettes = 48 total
    assert_eq!(ColorPalette::ALL.len(), 48);
    // Original 6 palettes
    assert_eq!(ColorPalette::ALL[0].name, "Fire");
    assert_eq!(ColorPalette::ALL[1].name, "Ocean");
    assert_eq!(ColorPalette::ALL[2].name, "Rainbow");
    assert_eq!(ColorPalette::ALL[3].name, "Forest");
    assert_eq!(ColorPalette::ALL[4].name, "Sunset");
    assert_eq!(ColorPalette::ALL[5].name, "Grayscale");
    // Scientific palettes
    assert_eq!(ColorPalette::ALL[6].name, "Viridis");
    assert_eq!(ColorPalette::ALL[7].name, "Plasma");
    assert_eq!(ColorPalette::ALL[8].name, "Inferno");
    assert_eq!(ColorPalette::ALL[9].name, "Magma");
    assert_eq!(ColorPalette::ALL[10].name, "Copper");
    assert_eq!(ColorPalette::ALL[11].name, "Cool");
    assert_eq!(ColorPalette::ALL[12].name, "Hot");
    // Artistic palettes
    assert_eq!(ColorPalette::ALL[13].name, "Neon");
    assert_eq!(ColorPalette::ALL[14].name, "Purple Dream");
    assert_eq!(ColorPalette::ALL[15].name, "Earth");
    assert_eq!(ColorPalette::ALL[16].name, "Ice");
    assert_eq!(ColorPalette::ALL[17].name, "Lava");
    assert_eq!(ColorPalette::ALL[18].name, "Galaxy");
    assert_eq!(ColorPalette::ALL[19].name, "Mint");
    assert_eq!(ColorPalette::ALL[20].name, "Cherry");
    // Xfractint palettes (first and last)
    assert_eq!(ColorPalette::ALL[21].name, "Alternating Grey");
    assert_eq!(ColorPalette::ALL[47].name, "Volcano");
}

#[test]
fn test_palette_colors_valid() {
    for palette in ColorPalette::ALL {
        for color in &palette.colors {
            // Check that colors are in valid range [0, 1]
            assert!(color.x >= 0.0 && color.x <= 1.0);
            assert!(color.y >= 0.0 && color.y <= 1.0);
            assert!(color.z >= 0.0 && color.z <= 1.0);
        }
    }
}

#[test]
fn test_material_properties_valid() {
    let params = FractalParams::default();
    assert!(params.roughness >= 0.0 && params.roughness <= 1.0);
    assert!(params.metallic >= 0.0 && params.metallic <= 1.0);
    assert!(params.albedo.x >= 0.0 && params.albedo.x <= 1.0);
    assert!(params.albedo.y >= 0.0 && params.albedo.y <= 1.0);
    assert!(params.albedo.z >= 0.0 && params.albedo.z <= 1.0);
}

#[test]
fn test_3d_parameters_valid() {
    let params = FractalParams::default();
    assert!(params.power > 0.0);
    assert!(params.max_steps > 0);
    assert!(params.min_distance > 0.0);
    assert!(params.dof_focal_length > 0.0);
    assert!(params.dof_aperture > 0.0);
}

// ===========================================================================
// ARC-015 YAML-backward-compat gate.
//
// These tests pin the on-disk `Settings` schema (field names, nesting, types)
// BEFORE the `FractalParams` God-object split so the refactor cannot silently
// drift it. `Settings` is the contract with every saved settings.yaml and
// preset on disk; the roundtrip must be byte-for-byte stable for default
// values and field-for-field stable for non-default values.
//
// Run green against the PRE-refactor code first (baseline), then keep green
// through the split.
// ===========================================================================

/// Captured default-`Settings` YAML emitted by the PRE-refactor code
/// (`FractalParams::default().to_settings()` serialized with `serde_yaml`).
/// If any field is renamed, re-nested, or changes type under ARC-015, the
/// `old_settings_yaml_loads_default` test fires.
const PRE_REFRAC_DEFAULT_YAML: &str = include_str!("tests_default_settings.yaml");

/// Round-trip `FractalParams::default()` through the full Settings pipeline
/// and assert the reconstructed params match the default on every field that
/// `Settings` actually persists (the derived fields — `palette`, `palette_offset`,
/// `render_mode` — are recomputed and checked separately).
#[test]
fn default_roundtrips_through_settings_yaml() {
    let original = FractalParams::default();
    let settings = original.to_settings();
    let yaml = serde_yaml::to_string(&settings).expect("serialize settings");
    let parsed: Settings = serde_yaml::from_str(&yaml).expect("parse settings yaml");
    let rebuilt = FractalParams::from_settings(parsed);

    // Derived fields that from_settings recomputes (not stored in Settings).
    assert_eq!(rebuilt.fractal_type, original.fractal_type);
    assert_eq!(rebuilt.render_mode, original.render_mode);
    assert_eq!(rebuilt.palette_index, original.palette_index);
    assert_eq!(rebuilt.palette.name, original.palette.name);
    assert_eq!(rebuilt.palette_offset, original.palette_offset);

    // Representative serialized knobs across every section — if the split
    // drops or renames any of these, this fires.
    assert_eq!(rebuilt.max_iterations, original.max_iterations);
    assert_eq!(rebuilt.zoom_2d, original.zoom_2d);
    assert_eq!(rebuilt.center_2d, original.center_2d);
    assert_eq!(rebuilt.julia_c, original.julia_c);
    assert_eq!(rebuilt.power, original.power);
    assert_eq!(rebuilt.max_steps, original.max_steps);
    assert_eq!(rebuilt.min_distance, original.min_distance);
    assert_eq!(rebuilt.ao_intensity, original.ao_intensity);
    assert_eq!(rebuilt.shadow_mode, original.shadow_mode);
    assert_eq!(rebuilt.shadow_samples, original.shadow_samples);
    assert_eq!(rebuilt.dof_samples, original.dof_samples);
    assert_eq!(rebuilt.fractal_scale, original.fractal_scale);
    assert_eq!(rebuilt.fractal_fold, original.fractal_fold);
    assert_eq!(rebuilt.roughness, original.roughness);
    assert_eq!(rebuilt.metallic, original.metallic);
    assert_eq!(rebuilt.albedo, original.albedo);
    assert_eq!(rebuilt.light_intensity, original.light_intensity);
    assert_eq!(rebuilt.floor_height, original.floor_height);
    assert_eq!(rebuilt.fog_density, original.fog_density);
    assert_eq!(rebuilt.camera_fov, original.camera_fov);
    assert_eq!(rebuilt.auto_orbit, original.auto_orbit);
    assert_eq!(rebuilt.brightness, original.brightness);
    assert_eq!(rebuilt.bloom_enabled, original.bloom_enabled);
    assert_eq!(rebuilt.bloom_intensity, original.bloom_intensity);
    assert_eq!(rebuilt.fxaa_enabled, original.fxaa_enabled);
    assert_eq!(
        rebuilt.attractor_accumulation_enabled,
        original.attractor_accumulation_enabled
    );
    assert_eq!(
        rebuilt.attractor_iterations_per_frame,
        original.attractor_iterations_per_frame
    );
    assert_eq!(rebuilt.attractor_log_scale, original.attractor_log_scale);
}

/// A pre-refactor `Settings` YAML (captured baseline) must still parse under
/// `Settings` and reconstruct via `from_settings`. This is the hard YAML-compat
/// gate: any field rename / re-nesting in the refactor breaks this.
#[test]
fn old_settings_yaml_loads_default() {
    let parsed: Settings = serde_yaml::from_str(PRE_REFRAC_DEFAULT_YAML)
        .expect("pre-refactor default YAML must still parse");
    let params = FractalParams::from_settings(parsed);

    // Spot-check representative fields against the known default values.
    assert_eq!(params.fractal_type, FractalType::Mandelbrot2D);
    assert_eq!(params.render_mode, RenderMode::TwoD);
    assert_eq!(params.max_iterations, 80);
    assert_eq!(params.zoom_2d, 1.0);
    assert_eq!(params.power, 2.0);
    assert_eq!(params.max_steps, 200);
    assert_eq!(params.shadow_mode, 2);
    assert!(!params.bloom_enabled);
    assert!(!params.attractor_accumulation_enabled);
    assert_eq!(params.attractor_iterations_per_frame, 10_000);
}

/// Construct a `Settings` with non-default values everywhere and assert the
/// YAML round-trip preserves them. This catches type/nesting regressions that
/// the default-only test would miss (e.g. a `default` attribute masking a
/// renamed field).
#[test]
fn nondefault_settings_roundtrip_identical() {
    let mut s = FractalParams::default().to_settings();
    s.fractal_type = FractalType::Julia2D;
    s.shading_model = ShadingModel::PBR;
    s.color_mode = ColorMode::Normals;
    s.palette_index = 5;
    s.orbit_trap_scale = 2.5;
    s.channel_r = ChannelSource::Distance;
    s.channel_g = ChannelSource::PositionX;
    s.channel_b = ChannelSource::Iterations;
    s.procedural_palette = ProceduralPalette::Firestrm;
    s.procedural_brightness = [0.1, 0.2, 0.3];
    s.procedural_contrast = [0.4, 0.5, 0.6];
    s.procedural_frequency = [2.0, 3.0, 4.0];
    s.procedural_phase = [0.1, 0.2, 0.3];
    s.center_2d = [-1.5, 0.25];
    s.zoom_2d = 1e6;
    s.julia_c = [0.123, -0.456];
    s.max_iterations = 1234;
    s.power = 6.0;
    s.max_steps = 333;
    s.min_distance = 1e-5;
    s.ambient_occlusion = false;
    s.ao_intensity = 2.5;
    s.ao_step_size = 0.07;
    s.shadow_mode = 1;
    s.shadow_softness = 12.0;
    s.shadow_max_distance = 8.0;
    s.shadow_samples = 64;
    s.shadow_step_factor = 0.5;
    s.depth_of_field = true;
    s.dof_focal_length = 12.0;
    s.dof_aperture = 0.05;
    s.dof_samples = 4;
    s.fractal_scale = 1.7;
    s.fractal_fold = 2.2;
    s.fractal_min_radius = 0.7;
    s.roughness = 0.65;
    s.metallic = 0.4;
    s.albedo = [0.5, 0.6, 0.7];
    s.light_intensity = 4.5;
    s.ambient_light = 0.25;
    s.light_azimuth = 60.0;
    s.light_elevation = 20.0;
    s.show_floor = false;
    s.floor_height = -1.0;
    s.floor_color1 = [0.9, 0.8, 0.7];
    s.floor_color2 = [0.1, 0.2, 0.3];
    s.floor_reflections = true;
    s.floor_reflection_strength = 0.75;
    s.fog_enabled = false;
    s.fog_mode = FogMode::Linear;
    s.fog_density = 0.02;
    s.fog_color = [0.25, 0.5, 0.75];
    s.use_adaptive_step = false;
    s.fixed_step_size = 0.05;
    s.step_multiplier = 0.9;
    s.max_distance = 50.0;
    s.camera_position = [1.0, 2.0, 3.0];
    s.camera_target = [0.5, 0.0, -0.5];
    s.camera_speed = 3.5;
    s.camera_fov = 60.0;
    s.auto_orbit = true;
    s.orbit_speed = 0.5;
    s.brightness = 1.25;
    s.contrast = 1.1;
    s.saturation = 0.9;
    s.hue_shift = 30.0;
    s.vignette_enabled = true;
    s.vignette_intensity = 0.7;
    s.vignette_radius = 0.6;
    s.bloom_enabled = true;
    s.bloom_threshold = 0.6;
    s.bloom_intensity = 0.4;
    s.bloom_radius = 0.01;
    s.fxaa_enabled = true;
    s.attractor_accumulation_enabled = true;
    s.attractor_iterations_per_frame = 25_000;
    s.attractor_log_scale = 3.0;

    let yaml = serde_yaml::to_string(&s).expect("serialize nondefault settings");
    let parsed: Settings = serde_yaml::from_str(&yaml).expect("parse nondefault yaml");
    // Compare the YAML strings byte-for-byte for stable serialization.
    let reparsed_yaml = serde_yaml::to_string(&parsed).expect("reserialize");
    assert_eq!(yaml, reparsed_yaml, "YAML round-trip is not byte-stable");

    let params = FractalParams::from_settings(parsed);
    assert_eq!(params.fractal_type, FractalType::Julia2D);
    assert_eq!(params.render_mode, RenderMode::TwoD);
    assert_eq!(params.max_iterations, 1234);
    assert_eq!(params.zoom_2d, 1e6);
    assert_eq!(params.center_2d, [-1.5, 0.25]);
    assert_eq!(params.power, 6.0);
    assert_eq!(params.max_steps, 333);
    assert_eq!(params.shadow_mode, 1);
    assert_eq!(params.shadow_samples, 64);
    assert!(params.bloom_enabled);
    assert_eq!(params.bloom_intensity, 0.4);
    assert!(params.fxaa_enabled);
    assert!(params.attractor_accumulation_enabled);
    assert_eq!(params.attractor_iterations_per_frame, 25_000);
    assert_eq!(params.attractor_log_scale, 3.0);
}
