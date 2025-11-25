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
    assert_eq!(params.render_mode, RenderMode::ThreeD);

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
    assert_eq!(params.fractal_type, FractalType::Mandelbrot2D);
    assert_eq!(params.render_mode, RenderMode::TwoD);

    // Change palette
    let original_palette = params.palette.name;
    params.next_palette();
    assert_ne!(params.palette.name, original_palette);

    // Switch to Julia
    params.switch_fractal(FractalType::Julia2D);
    assert_eq!(params.fractal_type, FractalType::Julia2D);
    assert_eq!(params.render_mode, RenderMode::TwoD);

    // Modify Julia parameters
    params.julia_c = [-0.8, 0.156];
    assert_eq!(params.julia_c, [-0.8, 0.156]);

    // Switch to 3D
    params.switch_fractal(FractalType::Mandelbulb3D);
    assert_eq!(params.render_mode, RenderMode::ThreeD);

    // Enable effects
    params.ambient_occlusion = true;
    params.shadow_mode = 2; // Soft shadows
    params.depth_of_field = true;

    assert!(params.ambient_occlusion);
    assert_eq!(params.shadow_mode, 2); // Soft shadows enabled
    assert!(params.depth_of_field);
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
        assert_eq!(params.palette_index, i);
        params.next_palette();
    }

    // Should wrap around
    assert_eq!(params.palette_index, 0);

    // Test backward cycling
    for i in (0..num_palettes).rev() {
        params.prev_palette();
        assert_eq!(params.palette_index, i);
    }
}

#[test]
fn test_shading_model_switching() {
    let mut params = FractalParams::default();
    params.switch_fractal(FractalType::Mandelbulb3D);

    // Start with PBR (default)
    assert_eq!(params.shading_model, ShadingModel::PBR);

    // Switch to Blinn-Phong
    params.shading_model = ShadingModel::BlinnPhong;
    assert_eq!(params.shading_model, ShadingModel::BlinnPhong);

    // Modify PBR parameters
    params.roughness = 0.7;
    params.metallic = 0.9;
    assert_eq!(params.roughness, 0.7);
    assert_eq!(params.metallic, 0.9);

    // Switch back
    params.shading_model = ShadingModel::BlinnPhong;
    assert_eq!(params.shading_model, ShadingModel::BlinnPhong);
}

#[test]
fn test_2d_fractal_parameters() {
    let mut params = FractalParams {
        zoom_2d: 2.0,
        ..Default::default()
    };
    assert_eq!(params.zoom_2d, 2.0);

    // Test panning
    params.center_2d = [0.5, -0.3];
    assert_eq!(params.center_2d, [0.5, -0.3]);

    // Test iterations
    params.max_iterations = 512;
    assert_eq!(params.max_iterations, 512);
}

#[test]
fn test_3d_fractal_parameters() {
    let mut params = FractalParams::default();
    params.switch_fractal(FractalType::Mandelbulb3D);

    // Test power
    params.power = 12.0;
    assert_eq!(params.power, 12.0);

    // Test ray marching parameters
    params.max_steps = 256;
    params.min_distance = 0.0001;
    assert_eq!(params.max_steps, 256);
    assert_eq!(params.min_distance, 0.0001);
}

#[test]
fn test_depth_of_field_parameters() {
    let mut params = FractalParams::default();
    params.switch_fractal(FractalType::Mandelbulb3D);

    params.depth_of_field = true;
    params.dof_focal_length = 10.0;
    params.dof_aperture = 0.5;

    assert!(params.depth_of_field);
    assert_eq!(params.dof_focal_length, 10.0);
    assert_eq!(params.dof_aperture, 0.5);
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
        assert_eq!(params.fractal_type, fractal_type);

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
            | FractalType::Threeply2D => {
                assert_eq!(params.render_mode, RenderMode::TwoD);
            }
            FractalType::Mandelbulb3D
            | FractalType::MengerSponge3D
            | FractalType::SierpinskiPyramid3D
            | FractalType::JuliaSet3D
            | FractalType::Mandelbox3D
            | FractalType::TgladFormula3D
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
                assert_eq!(params.render_mode, RenderMode::ThreeD);
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
    params.shading_model = ShadingModel::PBR;

    // Test boundary values
    params.roughness = 0.0;
    params.metallic = 0.0;
    assert_eq!(params.roughness, 0.0);
    assert_eq!(params.metallic, 0.0);

    params.roughness = 1.0;
    params.metallic = 1.0;
    assert_eq!(params.roughness, 1.0);
    assert_eq!(params.metallic, 1.0);

    // Test mid values
    params.roughness = 0.5;
    params.metallic = 0.5;
    assert_eq!(params.roughness, 0.5);
    assert_eq!(params.metallic, 0.5);
}

#[test]
fn test_fractal_state_consistency() {
    let params = FractalParams::default();

    // Check that all default values are valid
    assert!(params.zoom_2d > 0.0);
    assert!(params.max_iterations > 0);
    assert!(params.power > 0.0);
    assert!(params.max_steps > 0);
    assert!(params.min_distance > 0.0);
    assert!(params.dof_focal_length > 0.0);
    assert!(params.dof_aperture > 0.0);
    assert!(params.roughness >= 0.0 && params.roughness <= 1.0);
    assert!(params.metallic >= 0.0 && params.metallic <= 1.0);
}

#[test]
fn test_render_mode_consistency() {
    let mut params = FractalParams::default();

    // 2D fractals should always be in 2D mode
    params.switch_fractal(FractalType::Mandelbrot2D);
    assert_eq!(params.render_mode, RenderMode::TwoD);

    params.switch_fractal(FractalType::Julia2D);
    assert_eq!(params.render_mode, RenderMode::TwoD);

    // 3D fractals should always be in 3D mode
    params.switch_fractal(FractalType::Mandelbulb3D);
    assert_eq!(params.render_mode, RenderMode::ThreeD);

    params.switch_fractal(FractalType::MengerSponge3D);
    assert_eq!(params.render_mode, RenderMode::ThreeD);
}
