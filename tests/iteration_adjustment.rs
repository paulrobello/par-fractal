use par_fractal::{FractalParams, FractalType};

#[test]
fn iteration_shortcuts_preserve_per_dimension_step_sizes() {
    let mut params = FractalParams::default();
    let initial = params.settings.max_iterations;
    assert_eq!(params.adjust_iterations(32), initial + 32);
    assert_eq!(params.adjust_iterations(-32), initial);

    params.switch_fractal(FractalType::Mandelbulb3D);
    let initial = params.settings.max_steps;
    assert_eq!(params.adjust_iterations(32), initial + 10);
    assert_eq!(params.adjust_iterations(-32), initial);
}
