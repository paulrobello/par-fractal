use par_fractal::{
    Camera, CameraController, ColorPalette, FractalParams, FractalType, RenderMode, ShadingModel,
    UI,
};

#[test]
fn test_fractal_and_camera_integration() {
    let mut params = FractalParams::default();
    let mut camera = Camera::new(1280, 720);

    // Switch to 3D mode
    params.switch_fractal(FractalType::Mandelbulb3D);
    assert_eq!(params.settings.render_mode, RenderMode::ThreeD);

    // Camera should be initialized properly
    assert_eq!(camera.aspect, 1280.0 / 720.0);

    // Resize should update aspect ratio
    camera.resize(1920, 1080);
    assert!((camera.aspect - 1920.0 / 1080.0).abs() < 0.001);
}

#[test]
fn test_complete_fractal_workflow() {
    let mut params = FractalParams::default();

    // Start with Mandelbrot
    assert_eq!(params.settings.fractal_type, FractalType::Mandelbrot2D);
    assert_eq!(params.settings.render_mode, RenderMode::TwoD);

    // Change palette
    let original_palette = params.settings.palette.name;
    params.next_palette();
    assert_ne!(params.settings.palette.name, original_palette);

    // Switch to Julia
    params.switch_fractal(FractalType::Julia2D);
    assert_eq!(params.settings.fractal_type, FractalType::Julia2D);
    assert_eq!(params.settings.render_mode, RenderMode::TwoD);

    // Switching types keeps the authored settings the user set (Julia's c
    // survives a type switch; switch_fractal only re-derives mode + view).
    let julia_c = [-0.8, 0.156];
    params.settings.julia_c = julia_c;
    params.switch_fractal(FractalType::Mandelbulb3D);
    assert_eq!(params.settings.render_mode, RenderMode::ThreeD);
    assert_eq!(params.settings.julia_c, julia_c, "user julia_c clobbered");

    // Effects toggle round-trips through the settings that persist.
    params.settings.ambient_occlusion = true;
    params.settings.depth_of_field = true;
    params.switch_fractal(FractalType::MengerSponge3D);
    assert!(
        params.settings.ambient_occlusion && params.settings.depth_of_field,
        "effect toggles lost across a type switch"
    );
}

#[test]
fn test_camera_controller_integration() {
    let mut controller = CameraController::new(2.5);
    let mut camera = Camera::new(800, 600);

    let original_position = camera.position;

    // Simulate movement
    controller.simulate_forward_press(true);
    controller.update_camera(&mut camera, 0.1);

    // Camera should have moved
    assert_ne!(camera.position, original_position);

    // Stop movement
    controller.simulate_forward_press(false);
    let stopped_position = camera.position;
    controller.update_camera(&mut camera, 0.1);

    // Position should remain stable
    assert!((camera.position - stopped_position).length() < 0.01);
}

#[test]
fn test_palette_cycle_integration() {
    let mut params = FractalParams::default();

    // Test cycling through all palettes
    let num_palettes = ColorPalette::ALL.len();
    for i in 0..num_palettes {
        assert_eq!(params.settings.palette_index, i);
        params.next_palette();
    }

    // Should wrap around
    assert_eq!(params.settings.palette_index, 0);

    // Test backward cycling
    for i in (0..num_palettes).rev() {
        params.prev_palette();
        assert_eq!(params.settings.palette_index, i);
    }
}

#[test]
fn test_shading_model_switching() {
    let mut params = FractalParams::default();
    params.switch_fractal(FractalType::Mandelbulb3D);

    // Start with PBR (default)
    assert_eq!(params.settings.shading_model, ShadingModel::PBR);

    // Switch to Blinn-Phong
    params.settings.shading_model = ShadingModel::BlinnPhong;
    assert_eq!(params.settings.shading_model, ShadingModel::BlinnPhong);

    // Modify PBR parameters
    params.settings.roughness = 0.7;
    params.settings.metallic = 0.9;
    assert_eq!(params.settings.roughness, 0.7);
    assert_eq!(params.settings.metallic, 0.9);

    // Switch back
    params.settings.shading_model = ShadingModel::BlinnPhong;
    assert_eq!(params.settings.shading_model, ShadingModel::BlinnPhong);
}

#[test]
fn test_zoom_at_keeps_clicked_point_fixed() {
    // QA-036: behavior, not assign-then-assert. zoom_at must keep the
    // fractal point under the cursor stationary across the zoom.
    let mut params = FractalParams::default();
    params.settings.center_2d = [0.0, 0.0];
    params.settings.zoom_2d = 1.0;

    let aspect = 1.0;
    let cursor = (0.5, -0.25);
    // Fractal point under the cursor BEFORE the zoom.
    let fx = params.settings.center_2d[0] + (cursor.0 * 2.0 / params.settings.zoom_2d) * aspect;
    let fy = params.settings.center_2d[1] + cursor.1 * 2.0 / params.settings.zoom_2d;

    params.zoom_at(cursor, 2.0, aspect);

    let fx_after =
        params.settings.center_2d[0] + (cursor.0 * 2.0 / params.settings.zoom_2d) * aspect;
    let fy_after = params.settings.center_2d[1] + cursor.1 * 2.0 / params.settings.zoom_2d;
    assert!(
        (fx - fx_after).abs() < 1e-9 && (fy - fy_after).abs() < 1e-9,
        "clicked point moved under zoom: ({fx},{fy}) -> ({fx_after},{fy_after})"
    );
}

#[test]
fn test_3d_fractal_parameters() {
    let mut params = FractalParams::default();
    params.switch_fractal(FractalType::Mandelbulb3D);

    // Test power
    params.settings.power = 12.0;
    assert_eq!(params.settings.power, 12.0);

    // Test ray marching parameters
    params.settings.max_steps = 256;
    params.settings.min_distance = 0.0001;
    assert_eq!(params.settings.max_steps, 256);
    assert_eq!(params.settings.min_distance, 0.0001);
}

#[test]
fn test_depth_of_field_parameters() {
    let mut params = FractalParams::default();
    params.switch_fractal(FractalType::Mandelbulb3D);

    params.settings.depth_of_field = true;
    params.settings.dof_focal_length = 10.0;
    params.settings.dof_aperture = 0.5;

    assert!(params.settings.depth_of_field);
    assert_eq!(params.settings.dof_focal_length, 10.0);
    assert_eq!(params.settings.dof_aperture, 0.5);
}

#[test]
fn test_ui_creation_and_state() {
    let ui = UI::new();
    assert!(ui.show_ui);

    let ui_default = UI::default();
    assert!(ui_default.show_ui);
}

#[test]
fn test_all_fractal_types_switchable() {
    let mut params = FractalParams::default();

    let fractal_types = vec![
        FractalType::Mandelbrot2D,
        FractalType::Julia2D,
        FractalType::Mandelbulb3D,
        FractalType::MengerSponge3D,
    ];

    for fractal_type in fractal_types {
        params.switch_fractal(fractal_type);
        assert_eq!(params.settings.fractal_type, fractal_type);

        // Check that render mode is correctly set
        match fractal_type {
            FractalType::Mandelbrot2D
            | FractalType::Julia2D
            | FractalType::Sierpinski2D
            | FractalType::SierpinskiTriangle2D
            | FractalType::BurningShip2D
            | FractalType::Tricorn2D
            | FractalType::Phoenix2D
            | FractalType::Celtic2D
            | FractalType::Newton2D
            | FractalType::Lyapunov2D
            | FractalType::Nova2D
            | FractalType::Magnet2D
            | FractalType::Collatz2D
            | FractalType::Hopalong2D
            | FractalType::Martin2D
            | FractalType::Gingerbreadman2D
            | FractalType::Chip2D
            | FractalType::Quadruptwo2D
            | FractalType::Threeply2D
            | FractalType::Buddhabrot2D => {
                assert_eq!(params.settings.render_mode, RenderMode::TwoD);
            }
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
            | FractalType::Pickover3D
            | FractalType::Lorenz3D
            | FractalType::Rossler3D => {
                assert_eq!(params.settings.render_mode, RenderMode::ThreeD);
            }
        }
    }
}

#[test]
fn test_camera_multiple_resizes() {
    let mut camera = Camera::new(1280, 720);

    let resolutions = vec![
        (1920, 1080),
        (800, 600),
        (2560, 1440),
        (1024, 768),
        (3840, 2160),
    ];

    for (width, height) in resolutions {
        camera.resize(width, height);
        let expected_aspect = width as f32 / height as f32;
        assert!((camera.aspect - expected_aspect).abs() < 0.001);
    }
}

#[test]
fn test_camera_movement_all_directions() {
    // Test forward
    let mut test_camera = Camera::new(1280, 720);
    let mut test_controller = CameraController::new(1.0);
    let start_pos = test_camera.position;
    test_controller.simulate_forward_press(true);
    test_controller.update_camera(&mut test_camera, 1.0);
    assert_ne!(test_camera.position, start_pos);

    // Test backward
    let mut test_camera = Camera::new(1280, 720);
    let mut test_controller = CameraController::new(1.0);
    let start_pos = test_camera.position;
    test_controller.simulate_backward_press(true);
    test_controller.update_camera(&mut test_camera, 1.0);
    assert_ne!(test_camera.position, start_pos);

    // Test left
    let mut test_camera = Camera::new(1280, 720);
    let mut test_controller = CameraController::new(1.0);
    let start_pos = test_camera.position;
    test_controller.simulate_left_press(true);
    test_controller.update_camera(&mut test_camera, 1.0);
    assert_ne!(test_camera.position, start_pos);

    // Test right
    let mut test_camera = Camera::new(1280, 720);
    let mut test_controller = CameraController::new(1.0);
    let start_pos = test_camera.position;
    test_controller.simulate_right_press(true);
    test_controller.update_camera(&mut test_camera, 1.0);
    assert_ne!(test_camera.position, start_pos);

    // Test up
    let mut test_camera = Camera::new(1280, 720);
    let mut test_controller = CameraController::new(1.0);
    let start_pos = test_camera.position;
    test_controller.simulate_up_press(true);
    test_controller.update_camera(&mut test_camera, 1.0);
    assert_ne!(test_camera.position, start_pos);

    // Test down
    let mut test_camera = Camera::new(1280, 720);
    let mut test_controller = CameraController::new(1.0);
    let start_pos = test_camera.position;
    test_controller.simulate_down_press(true);
    test_controller.update_camera(&mut test_camera, 1.0);
    assert_ne!(test_camera.position, start_pos);
}

#[test]
fn test_palette_color_validity_all_palettes() {
    for palette in ColorPalette::ALL {
        for color in &palette.colors {
            // All color components should be in [0, 1] range
            assert!(color.x >= 0.0 && color.x <= 1.0);
            assert!(color.y >= 0.0 && color.y <= 1.0);
            assert!(color.z >= 0.0 && color.z <= 1.0);
        }
    }
}

#[test]
fn test_material_property_constraints() {
    let mut params = FractalParams::default();
    params.switch_fractal(FractalType::Mandelbulb3D);
    params.settings.shading_model = ShadingModel::PBR;

    // Test boundary values
    params.settings.roughness = 0.0;
    params.settings.metallic = 0.0;
    assert_eq!(params.settings.roughness, 0.0);
    assert_eq!(params.settings.metallic, 0.0);

    params.settings.roughness = 1.0;
    params.settings.metallic = 1.0;
    assert_eq!(params.settings.roughness, 1.0);
    assert_eq!(params.settings.metallic, 1.0);

    // Test mid values
    params.settings.roughness = 0.5;
    params.settings.metallic = 0.5;
    assert_eq!(params.settings.roughness, 0.5);
    assert_eq!(params.settings.metallic, 0.5);
}

#[test]
fn test_fractal_state_consistency() {
    let params = FractalParams::default();

    // Check that all default values are valid
    assert!(params.settings.zoom_2d > 0.0);
    assert!(params.settings.max_iterations > 0);
    assert!(params.settings.power > 0.0);
    assert!(params.settings.max_steps > 0);
    assert!(params.settings.min_distance > 0.0);
    assert!(params.settings.dof_focal_length > 0.0);
    assert!(params.settings.dof_aperture > 0.0);
    assert!(params.settings.roughness >= 0.0 && params.settings.roughness <= 1.0);
    assert!(params.settings.metallic >= 0.0 && params.settings.metallic <= 1.0);
}

#[test]
fn test_render_mode_consistency() {
    let mut params = FractalParams::default();

    // 2D fractals should always be in 2D mode
    params.switch_fractal(FractalType::Mandelbrot2D);
    assert_eq!(params.settings.render_mode, RenderMode::TwoD);

    params.switch_fractal(FractalType::Julia2D);
    assert_eq!(params.settings.render_mode, RenderMode::TwoD);

    // 3D fractals should always be in 3D mode
    params.switch_fractal(FractalType::Mandelbulb3D);
    assert_eq!(params.settings.render_mode, RenderMode::ThreeD);

    params.switch_fractal(FractalType::MengerSponge3D);
    assert_eq!(params.settings.render_mode, RenderMode::ThreeD);
}

/// QA-036: R-style reset behavior for every 2D type — restores the per-type
/// default view, clears a stale precise center, and does NOT clobber user
/// power/iterations. 3D types are covered by the camera-framing tests.
#[test]
fn reset_2d_view_restores_defaults_for_every_2d_type() {
    for t in FractalType::ALL {
        if t.info().is_3d {
            continue;
        }
        let mut params = FractalParams::default();
        params.switch_fractal(*t);
        params.settings.power = 3.5;
        params.settings.max_iterations = 750;
        params.settings.center_2d = [42.0, -42.0];
        params.settings.zoom_2d = 1e-9;
        params.settings.center_2d_precise = Some(["-0.75".to_string(), "0.1".to_string()]);

        params.reset_2d_view();

        let (center, zoom) = FractalParams::default_2d_view(*t);
        assert_eq!(params.settings.center_2d, center, "{t:?} center");
        assert_eq!(params.settings.zoom_2d, zoom, "{t:?} zoom");
        assert_eq!(params.settings.center_2d_precise, None, "{t:?} precise");
        assert_eq!(params.settings.power, 3.5, "{t:?} power must survive");
        assert_eq!(
            params.settings.max_iterations, 750,
            "{t:?} iterations must survive"
        );
    }
}
