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
    assert_eq!(params.palette.name, "Cherry"); // Last palette after adding new ones
}

#[test]
fn test_all_palettes_exist() {
    assert_eq!(ColorPalette::ALL.len(), 21);
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
