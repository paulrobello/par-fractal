//! PNG comparison + harness helpers for the ENH-007 visual-regression layer.
//!
//! Three modes (the first positional arg selects; two bare paths = compare):
//!
//! - `imgdiff <a.png> <b.png>` — compare two PNGs. Prints JSON metrics and
//!   exits `0` if within tolerance, `1` otherwise. Default tolerance (overridable
//!   via `--mae` / `--frac`): bad-pixel fraction < 0.5% *and* mean abs error <
//!   2.0 — pixel-exact, for goldens. `--min-corr R` switches the gate to Pearson
//!   correlation over luma (`pass = corr >= R`), the metric the f32-GPU-vs-f64-
//!   reference cross-check needs: fractal-boundary drift makes ~half the pixels
//!   differ on a *correct* render, so per-pixel MAE is unpassable there, while
//!   correlation stays ~0.7-0.9 and collapses to ~0 on a black frame / wrong
//!   fractal.
//! - `imgdiff render-ref <kind> <cx> <cy> <zoom> <iters> <WxH> <out.png>` —
//!   render the CPU f64 reference to a grayscale PNG (`color_mode == 2`:
//!   `vec3(t)`), for the optional CPU-vs-GPU cross-check. `<kind>` ∈
//!   {mandelbrot, burning_ship, tricorn}.
//! - `imgdiff gen-preset <id> <FractalType> <cx> <cy> <zoom> <iters>
//!   <color_mode>` — build a preset from `FractalParams::default()` with the
//!   row's view overrides and save it where `--preset` reads, so the GPU
//!   script never hand-writes the (large) `Settings` schema.
//!
//! Native-only: `gen-preset` calls `PresetGallery::save_preset` (filesystem).

use std::process::ExitCode;

const DEFAULT_MAE: f64 = 2.0;
const DEFAULT_FRAC: f64 = 0.005; // 0.5 %

fn usage() -> ! {
    eprintln!(
        "usage:\n  \
         imgdiff <a.png> <b.png> [--mae M] [--frac F] [--min-corr R]\n  \
         imgdiff render-ref <kind> <cx> <cy> <zoom> <iters> <WxH> <out.png>\n  \
         imgdiff gen-preset <id> <FractalType> <cx> <cy> <zoom> <iters> <color_mode>"
    );
    std::process::exit(2);
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        usage();
    }

    match args[0].as_str() {
        "render-ref" => run_render_ref(&args[1..]),
        "gen-preset" => run_gen_preset(&args[1..]),
        _ => run_compare(&args),
    }
}

// ---------------------------------------------------------------------------
// compare
// ---------------------------------------------------------------------------

fn run_compare(args: &[String]) -> ExitCode {
    // Pull optional --mae/--frac/--min-corr off the end, leaving the two PNG paths.
    let (paths, mae, frac, min_corr) = parse_compare_args(args);
    if paths.len() != 2 {
        usage();
    }

    let a = match image::open(paths[0]) {
        Ok(i) => i.to_rgba8(),
        Err(e) => {
            eprintln!("imgdiff: failed to open {}: {e}", paths[0]);
            return ExitCode::from(2);
        }
    };
    let b = match image::open(paths[1]) {
        Ok(i) => i.to_rgba8(),
        Err(e) => {
            eprintln!("imgdiff: failed to open {}: {e}", paths[1]);
            return ExitCode::from(2);
        }
    };
    if (a.width(), a.height()) != (b.width(), b.height()) {
        eprintln!(
            "imgdiff: dimension mismatch {}x{} vs {}x{}",
            a.width(),
            a.height(),
            b.width(),
            b.height()
        );
        return ExitCode::from(2);
    }

    let (bad_frac, mae_value) = compare_rgba(&a, &b);
    let corr = compare_corr(&a, &b);
    // `--min-corr` switches the gate to structural correlation (Pearson over
    // luma) — the only metric that tolerates f32-GPU-vs-f64-reference boundary
    // drift, where ~half the pixels differ >8/255 on a *correct* render. Without
    // it, the default MAE+frac gate (pixel-exact) is used for goldens.
    let pass = match min_corr {
        Some(mc) => corr >= mc,
        None => bad_frac < frac && mae_value < mae,
    };
    println!(
        "{{\"bad_pixel_fraction\": {bad_frac:.6}, \"mae\": {mae_value:.4}, \"corr\": {corr:.4}, \
         \"threshold_frac\": {frac}, \"threshold_mae\": {mae}, \"pass\": {pass}}}"
    );
    if pass {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Split `args` into PNG paths and the (possibly overridden) thresholds.
fn parse_compare_args(args: &[String]) -> (Vec<&str>, f64, f64, Option<f64>) {
    let mut mae = DEFAULT_MAE;
    let mut frac = DEFAULT_FRAC;
    let mut min_corr: Option<f64> = None;
    let mut paths = Vec::with_capacity(2);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--mae" => {
                mae = args
                    .get(i + 1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| usage());
                i += 2;
            }
            "--frac" => {
                frac = args
                    .get(i + 1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| usage());
                i += 2;
            }
            "--min-corr" => {
                min_corr = Some(
                    args.get(i + 1)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or_else(|| usage()),
                );
                i += 2;
            }
            other => {
                paths.push(other);
                i += 1;
            }
        }
    }
    (paths, mae, frac, min_corr)
}

/// Pearson correlation over per-pixel luma `(R+G+B)/3`. Captures structural
/// agreement (is the same fractal in the same place/orientation?) independent
/// of per-pixel drift, so a correct f32-GPU-vs-f64-reference render scores
/// ~0.7-0.9 while a black frame or wrong fractal scores ~0. Returns 0.0 for a
/// degenerate (solid) image — such a match should not pass a positive gate.
fn compare_corr(a: &image::RgbaImage, b: &image::RgbaImage) -> f64 {
    let av = a.as_raw();
    let bv = b.as_raw();
    let n = (av.len() / 4) as f64;
    let luma = |p: &[u8]| (p[0] as f64 + p[1] as f64 + p[2] as f64) / 3.0;
    let la: Vec<f64> = av.chunks_exact(4).map(luma).collect();
    let lb: Vec<f64> = bv.chunks_exact(4).map(luma).collect();
    let ma = la.iter().sum::<f64>() / n;
    let mb = lb.iter().sum::<f64>() / n;
    let cov = la
        .iter()
        .zip(&lb)
        .map(|(x, y)| (x - ma) * (y - mb))
        .sum::<f64>()
        / n;
    let sa = (la.iter().map(|x| (x - ma).powi(2)).sum::<f64>() / n).sqrt();
    let sb = (lb.iter().map(|y| (y - mb).powi(2)).sum::<f64>() / n).sqrt();
    if sa == 0.0 || sb == 0.0 {
        0.0
    } else {
        cov / (sa * sb)
    }
}

/// `(bad_pixel_fraction, mean_abs_error)` over the RGBA channels. A pixel is
/// "bad" if any channel differs by more than 8/255.
fn compare_rgba(a: &image::RgbaImage, b: &image::RgbaImage) -> (f64, f64) {
    let av = a.as_raw();
    let bv = b.as_raw();
    let n = av.len() as f64;
    let mut bad = 0u64;
    let mut sum_abs: f64 = 0.0;
    let threshold = 8i32;
    for (pa, pb) in av.chunks_exact(4).zip(bv.chunks_exact(4)) {
        let mut pixel_bad = false;
        for (ca, cb) in pa.iter().zip(pb.iter()) {
            let d = (*ca as i32 - *cb as i32).abs();
            sum_abs += d as f64;
            if d > threshold {
                pixel_bad = true;
            }
        }
        if pixel_bad {
            bad += 1;
        }
    }
    let pixels = (av.len() / 4) as f64;
    // sum_abs covers all 4 channels of every pixel; n = 4·pixels, so sum_abs/n
    // is the mean per-channel absolute error on the 0–255 scale.
    (bad as f64 / pixels, sum_abs / n)
}

// ---------------------------------------------------------------------------
// render-ref
// ---------------------------------------------------------------------------

fn run_render_ref(args: &[String]) -> ExitCode {
    if args.len() != 7 {
        usage();
    }
    let kind = parse_kind(&args[0]);
    let cx: f64 = args[1].parse().unwrap_or_else(|_| usage());
    let cy: f64 = args[2].parse().unwrap_or_else(|_| usage());
    let zoom: f64 = args[3].parse().unwrap_or_else(|_| usage());
    let iters: u32 = args[4].parse().unwrap_or_else(|_| usage());
    let (w, h) = parse_size(&args[5]);
    let out = &args[6];

    // Iterate the same budget the GPU runs: the manifest's `iters` is
    // `settings.max_iterations`, but the shader loop uses max_iterations +
    // zoom_iteration_bonus (and perturbation pins to that same length when the
    // reference is bounded). Without the bonus the f64 reference normalizes
    // smooth-`t` by the wrong divisor and pixels near the boundary flip
    // interior/exterior vs the GPU — a structural mismatch no threshold hides.
    let effective_iters = iters + par_fractal::renderer::uniforms::zoom_iteration_bonus(zoom);

    let smooth = par_fractal::reference::render(kind, (cx, cy), zoom, (w, h), effective_iters, 2.0);
    let rgba = par_fractal::reference::smooth_to_grayscale_rgba(&smooth, (w, h));
    match image::RgbaImage::from_raw(w, h, rgba) {
        Some(img) => match img.save(out) {
            Ok(_) => {
                println!("render-ref: wrote {out} ({w}x{h})");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("render-ref: save failed: {e}");
                ExitCode::FAILURE
            }
        },
        None => {
            eprintln!("render-ref: buffer/dimension mismatch");
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------------
// gen-preset
// ---------------------------------------------------------------------------

fn run_gen_preset(args: &[String]) -> ExitCode {
    if args.len() != 7 {
        usage();
    }
    let id = &args[0];
    let ft = parse_fractal_type(&args[1]);
    let cx: f64 = args[2].parse().unwrap_or_else(|_| usage());
    let cy: f64 = args[3].parse().unwrap_or_else(|_| usage());
    let zoom: f64 = args[4].parse().unwrap_or_else(|_| usage());
    let iters: u32 = args[5].parse().unwrap_or_else(|_| usage());
    let color_mode: u32 = args[6].parse().unwrap_or_else(|_| usage());

    let mut params = par_fractal::FractalParams::default();
    params.switch_fractal(ft);
    params.settings.center_2d = [cx, cy];
    params.settings.zoom_2d = zoom;
    params.settings.max_iterations = iters;
    params.settings.color_mode = int_to_color_mode(color_mode);

    let preset = par_fractal::Preset {
        name: id.clone(),
        description: format!("ENH-007 visual-regression row ({ft:?})"),
        category: Default::default(),
        settings: params.to_settings(),
    };
    match par_fractal::PresetGallery::save_preset(&preset, id) {
        Ok(_) => {
            println!("gen-preset: saved preset '{id}' ({ft:?}, zoom={zoom})");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("gen-preset: save failed: {e}");
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------------
// small parsers
// ---------------------------------------------------------------------------

fn parse_kind(s: &str) -> par_fractal::reference::FractalKind {
    match s {
        "mandelbrot" => par_fractal::reference::FractalKind::Mandelbrot,
        "burning_ship" => par_fractal::reference::FractalKind::BurningShip,
        "tricorn" => par_fractal::reference::FractalKind::Tricorn,
        _ => {
            eprintln!("render-ref: unknown kind '{s}' (mandelbrot|burning_ship|tricorn)");
            std::process::exit(2);
        }
    }
}

fn parse_fractal_type(s: &str) -> par_fractal::FractalType {
    use par_fractal::FractalType::*;
    match s {
        "Mandelbrot2D" => Mandelbrot2D,
        "BurningShip2D" => BurningShip2D,
        "Tricorn2D" => Tricorn2D,
        _ => {
            eprintln!("gen-preset: unsupported FractalType '{s}'");
            std::process::exit(2);
        }
    }
}

fn int_to_color_mode(v: u32) -> par_fractal::fractal::ColorMode {
    // Value 2 is the 2D shader's grayscale iteration mode (`vec3(t)`) —
    // time-independent, so GPU screenshots are deterministic and directly
    // comparable to the CPU reference's grayscale output.
    use par_fractal::fractal::ColorMode::*;
    match v {
        0 => Palette,
        1 => RaySteps,
        2 => Normals,
        3 => OrbitTrapXYZ,
        4 => OrbitTrapRadial,
        5 => WorldPosition,
        6 => LocalPosition,
        _ => Palette,
    }
}

fn parse_size(s: &str) -> (u32, u32) {
    let (w, h) = s
        .split_once('x')
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        .unwrap_or_else(|| usage());
    if w == 0 || h == 0 {
        usage();
    }
    (w, h)
}
