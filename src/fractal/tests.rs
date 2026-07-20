use super::*;

#[test]
fn test_default_fractal_params() {
    let params = FractalParams::default();
    assert_eq!(params.settings.fractal_type, FractalType::Mandelbrot2D);
    assert_eq!(params.settings.render_mode, RenderMode::TwoD);
    assert_eq!(params.settings.zoom_2d, 1.0);
    assert_eq!(params.settings.max_iterations, 80);
}

#[test]
fn test_switch_fractal_2d_to_3d() {
    let mut params = FractalParams::default();
    params.switch_fractal(FractalType::Mandelbulb3D);
    assert_eq!(params.settings.fractal_type, FractalType::Mandelbulb3D);
    assert_eq!(params.settings.render_mode, RenderMode::ThreeD);
}

#[test]
fn test_switch_fractal_3d_to_2d() {
    let mut params = FractalParams::default();
    params.switch_fractal(FractalType::Mandelbulb3D);
    params.switch_fractal(FractalType::Julia2D);
    assert_eq!(params.settings.fractal_type, FractalType::Julia2D);
    assert_eq!(params.settings.render_mode, RenderMode::TwoD);
}

#[test]
fn test_palette_cycling() {
    let mut params = FractalParams::default();
    assert_eq!(params.settings.palette_index, 0);
    assert_eq!(params.settings.palette.name, "Fire");

    params.next_palette();
    assert_eq!(params.settings.palette_index, 1);
    assert_eq!(params.settings.palette.name, "Ocean");

    params.next_palette();
    assert_eq!(params.settings.palette_index, 2);
    assert_eq!(params.settings.palette.name, "Rainbow");
}

#[test]
fn test_palette_cycling_wraps() {
    let mut params = FractalParams::default();
    params.settings.palette_index = ColorPalette::ALL.len() - 1;
    params.settings.palette = ColorPalette::ALL[params.settings.palette_index];

    params.next_palette();
    assert_eq!(params.settings.palette_index, 0);
    assert_eq!(params.settings.palette.name, "Fire");
}

#[test]
fn test_palette_prev() {
    let mut params = FractalParams {
        settings: RenderSettings {
            palette_index: 2,
            palette: ColorPalette::ALL[2],
            ..Default::default()
        },
        ..Default::default()
    };

    params.prev_palette();
    assert_eq!(params.settings.palette_index, 1);
    assert_eq!(params.settings.palette.name, "Ocean");
}

#[test]
fn test_palette_prev_wraps() {
    let mut params = FractalParams::default();
    assert_eq!(params.settings.palette_index, 0);

    params.prev_palette();
    assert_eq!(params.settings.palette_index, ColorPalette::ALL.len() - 1);
    assert_eq!(params.settings.palette.name, "Volcano"); // Last palette (xfractint)
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
    assert!(params.settings.roughness >= 0.0 && params.settings.roughness <= 1.0);
    assert!(params.settings.metallic >= 0.0 && params.settings.metallic <= 1.0);
    assert!(params.settings.albedo.x >= 0.0 && params.settings.albedo.x <= 1.0);
    assert!(params.settings.albedo.y >= 0.0 && params.settings.albedo.y <= 1.0);
    assert!(params.settings.albedo.z >= 0.0 && params.settings.albedo.z <= 1.0);
}

#[test]
fn test_3d_parameters_valid() {
    let params = FractalParams::default();
    assert!(params.settings.power > 0.0);
    assert!(params.settings.max_steps > 0);
    assert!(params.settings.min_distance > 0.0);
    assert!(params.settings.dof_focal_length > 0.0);
    assert!(params.settings.dof_aperture > 0.0);
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
    assert_eq!(
        rebuilt.settings.fractal_type,
        original.settings.fractal_type
    );
    assert_eq!(rebuilt.settings.render_mode, original.settings.render_mode);
    assert_eq!(
        rebuilt.settings.palette_index,
        original.settings.palette_index
    );
    assert_eq!(
        rebuilt.settings.palette.name,
        original.settings.palette.name
    );
    assert_eq!(
        rebuilt.settings.palette_offset,
        original.settings.palette_offset
    );

    // Representative serialized knobs across every section — if the split
    // drops or renames any of these, this fires.
    assert_eq!(
        rebuilt.settings.max_iterations,
        original.settings.max_iterations
    );
    assert_eq!(rebuilt.settings.zoom_2d, original.settings.zoom_2d);
    assert_eq!(rebuilt.settings.center_2d, original.settings.center_2d);
    assert_eq!(rebuilt.settings.julia_c, original.settings.julia_c);
    assert_eq!(rebuilt.settings.power, original.settings.power);
    assert_eq!(rebuilt.settings.max_steps, original.settings.max_steps);
    assert_eq!(
        rebuilt.settings.min_distance,
        original.settings.min_distance
    );
    assert_eq!(
        rebuilt.settings.ao_intensity,
        original.settings.ao_intensity
    );
    assert_eq!(rebuilt.settings.shadow_mode, original.settings.shadow_mode);
    assert_eq!(
        rebuilt.settings.shadow_samples,
        original.settings.shadow_samples
    );
    assert_eq!(rebuilt.settings.dof_samples, original.settings.dof_samples);
    assert_eq!(
        rebuilt.settings.fractal_scale,
        original.settings.fractal_scale
    );
    assert_eq!(
        rebuilt.settings.fractal_fold,
        original.settings.fractal_fold
    );
    assert_eq!(rebuilt.settings.roughness, original.settings.roughness);
    assert_eq!(rebuilt.settings.metallic, original.settings.metallic);
    assert_eq!(rebuilt.settings.albedo, original.settings.albedo);
    assert_eq!(
        rebuilt.settings.light_intensity,
        original.settings.light_intensity
    );
    assert_eq!(
        rebuilt.settings.floor_height,
        original.settings.floor_height
    );
    assert_eq!(rebuilt.settings.fog_density, original.settings.fog_density);
    assert_eq!(rebuilt.settings.camera_fov, original.settings.camera_fov);
    assert_eq!(rebuilt.settings.auto_orbit, original.settings.auto_orbit);
    assert_eq!(rebuilt.settings.brightness, original.settings.brightness);
    assert_eq!(
        rebuilt.settings.bloom_enabled,
        original.settings.bloom_enabled
    );
    assert_eq!(
        rebuilt.settings.bloom_intensity,
        original.settings.bloom_intensity
    );
    assert_eq!(
        rebuilt.settings.fxaa_enabled,
        original.settings.fxaa_enabled
    );
    assert_eq!(
        rebuilt.settings.attractor_accumulation_enabled,
        original.settings.attractor_accumulation_enabled
    );
    assert_eq!(
        rebuilt.settings.attractor_iterations_per_frame,
        original.settings.attractor_iterations_per_frame
    );
    assert_eq!(
        rebuilt.settings.attractor_log_scale,
        original.settings.attractor_log_scale
    );
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
    assert_eq!(params.settings.fractal_type, FractalType::Mandelbrot2D);
    assert_eq!(params.settings.render_mode, RenderMode::TwoD);
    assert_eq!(params.settings.max_iterations, 80);
    assert_eq!(params.settings.zoom_2d, 1.0);
    assert_eq!(params.settings.power, 2.0);
    assert_eq!(params.settings.max_steps, 200);
    assert_eq!(params.settings.shadow_mode, 2);
    assert!(!params.settings.bloom_enabled);
    assert!(!params.settings.attractor_accumulation_enabled);
    assert_eq!(params.settings.attractor_iterations_per_frame, 10_000);
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
    assert_eq!(params.settings.fractal_type, FractalType::Julia2D);
    assert_eq!(params.settings.render_mode, RenderMode::TwoD);
    assert_eq!(params.settings.max_iterations, 1234);
    assert_eq!(params.settings.zoom_2d, 1e6);
    assert_eq!(params.settings.center_2d, [-1.5, 0.25]);
    assert_eq!(params.settings.power, 6.0);
    assert_eq!(params.settings.max_steps, 333);
    assert_eq!(params.settings.shadow_mode, 1);
    assert_eq!(params.settings.shadow_samples, 64);
    assert!(params.settings.bloom_enabled);
    assert_eq!(params.settings.bloom_intensity, 0.4);
    assert!(params.settings.fxaa_enabled);
    assert!(params.settings.attractor_accumulation_enabled);
    assert_eq!(params.settings.attractor_iterations_per_frame, 25_000);
    assert_eq!(params.settings.attractor_log_scale, 3.0);
}

/// The 3D julia-family fractals all feed `julia_c` into their distance
/// estimators, so switching into one must re-seed it. Without this, `julia_c`
/// carries over from the previous fractal — Threeply2D leaves `[-55.0, -1.0]`,
/// which the Julia slider clamps to `[-2.0, -1.0]`, a constant far outside the
/// escape radius. The set is then empty and only its floor shadow renders.
#[test]
fn test_switch_to_3d_julia_family_reseeds_julia_c() {
    for fractal_type in [
        FractalType::JuliaSet3D,
        FractalType::HybridMandelbulbJulia3D,
        FractalType::QuaternionCubic3D,
    ] {
        let mut params = FractalParams::default();
        params.switch_fractal(FractalType::Threeply2D);
        assert_eq!(params.settings.julia_c, [-55.0, -1.0]);

        params.switch_fractal(fractal_type);

        let c = params.settings.julia_c;
        let magnitude = (c[0] * c[0] + c[1] * c[1]).sqrt();
        assert!(
            magnitude < 2.0,
            "{fractal_type:?} kept |julia_c| = {magnitude} (>= escape radius 2), \
             which renders an empty set"
        );
    }
}

/// `mandelbox_de` derives its internal fold scale as `-(power / 4.0)`, so the
/// classic scale of -2.0 needs `power == 8.0`. Nothing else sets `power` for
/// Mandelbox — the UI slider is Mandelbulb-only — so without a reset here it
/// carries over from the previous fractal. The 2.0 default leaves an internal
/// scale of -0.5, and |scale| < 1 contracts the iteration into a smooth blob
/// with no Mandelbox structure at all.
#[test]
fn test_switch_to_mandelbox_sets_power_for_classic_internal_scale() {
    let mut params = FractalParams::default();
    assert_eq!(params.settings.power, 2.0, "default is Mandelbrot's z^2");

    params.switch_fractal(FractalType::Mandelbox3D);

    let internal_scale = -(params.settings.power / 4.0);
    assert!(
        internal_scale.abs() > 1.0,
        "|internal scale| = {} collapses the Mandelbox to a blob",
        internal_scale.abs()
    );
    assert_eq!(internal_scale, -2.0, "classic Mandelbox scale");
}

/// The shipped preset bypasses `switch_fractal` entirely
/// (`FractalParams::from_settings`), so it has to carry `power` itself.
#[test]
fn test_mandelbox_preset_carries_its_own_power() {
    let preset = PresetGallery::get_builtin_preset("Mandelbox Cubic")
        .expect("Mandelbox Cubic preset exists");
    let internal_scale = -(preset.settings.power / 4.0);
    assert_eq!(
        internal_scale, -2.0,
        "preset renders a blob without an explicit power"
    );
}

/// Class-wide guard for the defect behind the Julia 3D, Mandelbox, and
/// Sierpinski Gasket bugs: a shape parameter that a fractal's distance
/// estimator reads but `switch_fractal` never seeds silently inherits whatever
/// the previously selected fractal left behind. Selecting a type from the menu
/// must produce the same fractal regardless of what you were looking at before.
///
/// Each entry lists the uniforms that type's distance estimator in
/// `shaders/fractal.wgsl` actually reads; only those are compared, since an
/// unread field may legitimately carry over. Threeply2D and MengerSponge3D are
/// the adversarial predecessors — between them they leave extreme values in
/// every shared shape field.
#[test]
fn test_3d_defaults_do_not_depend_on_the_previous_fractal() {
    use Shape::*;

    #[derive(Clone, Copy)]
    enum Shape {
        Scale,
        Fold,
        MinRadius,
        Iterations,
        Power,
        JuliaC,
    }

    // (type, uniforms its DE reads)
    let de_reads: &[(FractalType, &[Shape])] = &[
        (FractalType::Mandelbulb3D, &[Scale, Power]),
        (FractalType::MengerSponge3D, &[Scale, Iterations]),
        (FractalType::SierpinskiPyramid3D, &[Scale, Iterations]),
        (
            FractalType::SierpinskiGasket3D,
            &[Scale, Fold, MinRadius, Iterations],
        ),
        (FractalType::JuliaSet3D, &[Scale, JuliaC]),
        (FractalType::Mandelbox3D, &[Scale, Fold, MinRadius, Power]),
        (FractalType::OctahedralIFS3D, &[Scale, Fold]),
        (FractalType::IcosahedralIFS3D, &[Scale, Fold]),
        (FractalType::ApollonianGasket3D, &[Scale, Fold, MinRadius]),
        (FractalType::Kleinian3D, &[Scale, Fold, MinRadius]),
        (
            FractalType::HybridMandelbulbJulia3D,
            &[Scale, JuliaC, Iterations, Power],
        ),
        (FractalType::QuaternionCubic3D, &[Scale, JuliaC, Iterations]),
    ];

    for (target, reads) in de_reads {
        let shape_via = |via: FractalType| -> Vec<String> {
            let mut p = FractalParams::default();
            p.switch_fractal(via);
            p.switch_fractal(*target);
            reads
                .iter()
                .map(|f| match f {
                    Scale => format!("scale={}", p.settings.fractal_scale),
                    Fold => format!("fold={}", p.settings.fractal_fold),
                    MinRadius => format!("min_radius={}", p.settings.fractal_min_radius),
                    Iterations => format!("iterations={}", p.settings.max_iterations),
                    Power => format!("power={}", p.settings.power),
                    JuliaC => format!("julia_c={:?}", p.settings.julia_c),
                })
                .collect()
        };

        assert_eq!(
            shape_via(FractalType::Threeply2D),
            shape_via(FractalType::MengerSponge3D),
            "{target:?} renders differently depending on the previously \
             selected fractal — a shape parameter its DE reads is not seeded"
        );
    }
}

/// An IFS multiplies its point by ~2.5 per step, so it needs a low iteration
/// count; the 80 escape-time default diverges and the fractal disappears.
#[test]
fn test_ifs_fractals_use_low_iteration_counts() {
    for fractal_type in [
        FractalType::SierpinskiGasket3D,
        FractalType::SierpinskiPyramid3D,
        FractalType::MengerSponge3D,
    ] {
        let mut params = FractalParams::default();
        assert_eq!(params.settings.max_iterations, 80, "escape-time default");

        params.switch_fractal(fractal_type);

        assert!(
            params.settings.max_iterations <= 16,
            "{fractal_type:?} kept max_iterations = {}, which diverges",
            params.settings.max_iterations
        );
    }
}

/// The default camera sits at `+Z` looking at the origin. A 3D fractal whose
/// world-space extent reaches past that distance renders as a wall of clipped
/// geometry, because the camera is *inside* it — which is what the sprawling
/// IFS types did while every 3D type shared one hardcoded distance.
///
/// Verified visually per type via `--camera-pos`; this pins the measured
/// values so a future default change cannot silently re-bury the camera.
#[test]
fn test_sprawling_3d_fractals_are_framed_from_outside() {
    // (type, extent that must fit in front of the camera)
    let min_distance = [
        (FractalType::OctahedralIFS3D, 8.0),
        (FractalType::IcosahedralIFS3D, 8.0),
        (FractalType::ApollonianGasket3D, 7.0),
    ];
    for (fractal_type, minimum) in min_distance {
        let distance = fractal_type.default_camera_distance();
        assert!(
            distance >= minimum,
            "{fractal_type:?} frames from {distance}, inside its ~{minimum} extent"
        );
    }
}

/// Every 3D type must declare a usable framing distance; a zero or negative
/// value would put the camera at or behind the origin.
///
/// The list is explicit because `FractalType` has no iterator; keep it in sync
/// with `FractalType::is_3d` when adding a type.
#[test]
fn test_every_3d_fractal_declares_a_positive_camera_distance() {
    for fractal_type in [
        FractalType::Mandelbulb3D,
        FractalType::MengerSponge3D,
        FractalType::SierpinskiPyramid3D,
        FractalType::SierpinskiGasket3D,
        FractalType::JuliaSet3D,
        FractalType::Mandelbox3D,
        FractalType::OctahedralIFS3D,
        FractalType::IcosahedralIFS3D,
        FractalType::ApollonianGasket3D,
        FractalType::Kleinian3D,
        FractalType::HybridMandelbulbJulia3D,
        FractalType::QuaternionCubic3D,
        FractalType::Pickover3D,
        FractalType::Lorenz3D,
        FractalType::Rossler3D,
    ] {
        assert!(fractal_type.is_3d(), "{fractal_type:?} is not a 3D type");
        let distance = fractal_type.default_camera_distance();
        assert!(
            distance > 0.0 && distance.is_finite(),
            "{fractal_type:?} declares camera distance {distance}"
        );
    }
}

/// An egui slider clamps whatever value it is bound to as a side effect of
/// being drawn. The "Scale" slider is ungated, so it renders for every 3D type
/// — and its old 0.5 lower bound silently rewrote the three 3D attractors'
/// defaults (Lorenz 0.05 -> 0.5) the first frame the 3D Parameters panel was
/// open. Lorenz then rendered ten times oversized, with the camera inside it
/// and nothing in the saved settings to explain why.
///
/// Any per-type default outside the slider's range reintroduces that, so pin
/// every one of them against the range the slider actually uses.
#[test]
fn fractal_scale_defaults_survive_the_ui_slider() {
    for fractal_type in [
        FractalType::Mandelbulb3D,
        FractalType::MengerSponge3D,
        FractalType::SierpinskiPyramid3D,
        FractalType::SierpinskiGasket3D,
        FractalType::JuliaSet3D,
        FractalType::Mandelbox3D,
        FractalType::OctahedralIFS3D,
        FractalType::IcosahedralIFS3D,
        FractalType::ApollonianGasket3D,
        FractalType::Kleinian3D,
        FractalType::HybridMandelbulbJulia3D,
        FractalType::QuaternionCubic3D,
        FractalType::Pickover3D,
        FractalType::Lorenz3D,
        FractalType::Rossler3D,
    ] {
        let mut params = FractalParams::default();
        params.switch_fractal(fractal_type);
        let scale = params.settings.fractal_scale;
        assert!(
            FRACTAL_SCALE_RANGE.contains(&scale),
            "{fractal_type:?} defaults to fractal_scale {scale}, outside the \
             slider range {:?} — drawing the panel would clamp it",
            FRACTAL_SCALE_RANGE,
        );
    }
}
