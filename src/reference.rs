//! CPU reference renderer for the deep-zoom visual-regression harness (ENH-007).
//!
//! A pure-Rust, GPU-free reimplementation of the 2D escape-time math in
//! `shaders/fractal.wgsl`. It exists for two reasons:
//!
//! 1. **Ground truth.** An f64 escape-time renderer is the reference the GPU
//!    pipeline is compared against. f64 has ~16 decimal digits; the GPU's
//!    double-float path has ~14, and plain f32 has ~7. At deep zoom the gap
//!    between f32 and f64 is exactly the bug class the audit found shipping
//!    silently (blocky quantization past ~3e4). Comparing the GPU against this
//!    reference catches that class.
//! 2. **Teeth without a GPU.** The double-float primitives (`two_prod`,
//!    `two_sum`, `df_mul`, …) are mirrored here *byte-for-byte* from the WGSL
//!    so a `cargo test` can prove the DF math is correct (DF-at-zoom-1e8
//!    matches the f64 renderer within tolerance) with no GPU, no display, and
//!    no driver — i.e. it runs in CI. This is where a regression in the DF
//!    algebra (a reverted abs fix, an FMA-collapsed `two_prod`) gets caught.
//!
//! Everything here mirrors the shader's coordinate mapping, escape test, and
//! `smooth_iteration_count` formula so the numbers are directly comparable.
//! ENH-001's perturbation subsystem will reuse the f64 path as its reference
//! orbit generator.
//!
//! **No `image` dependency** — rendering returns `Vec<f32>` smooth values; PNG
//! encoding lives in `src/bin/imgdiff.rs`. This keeps the module lean and
//! testable from `cargo test` with no extra crates.

/// Zoom above which the shader engages its double-float ("high-precision") path
/// (`renderer/uniforms.rs`, `HP_ZOOM_THRESHOLD`). The reference mirrors the
/// same cutoff so a single `render_*` call picks the same iteration semantics
/// the GPU will use at that zoom.
pub const HP_ZOOM_THRESHOLD: f64 = 1e4;

/// Which 2D escape-time map to render. Matches the shader's per-`fractal_type`
/// dispatch (`fs_main_2d`, fractal.wgsl:2998–3049).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FractalKind {
    Mandelbrot,
    BurningShip,
    Tricorn,
}

// ============================================================================
// Coordinate mapping — mirrors `fs_main_2d` (fractal.wgsl:2971–3021)
// ============================================================================

/// Map a pixel (with `py = 0` at the **top**, matching the framebuffer the
/// screenshot reads back) to the complex coordinate the shader evaluates there.
///
/// The fullscreen-quad vertex shader sets `output.uv = input.position` with
/// `clip_position = (position, 0, 1)` (no transform), so `uv` is clip-space
/// `[-1, 1]` with `uv.y = +1` at the **top** of the viewport (WebGPU clip space
/// is y-up). The framebuffer/screenshot reads `py = 0` at the top too, so
/// `py = 0 ↔ uv.y = +1` — the y term is therefore `1 − 2·(py+0.5)/h` (flipped
/// vs the x term, because `py` grows downward while `uv.y` grows upward).
/// Fragments rasterize at pixel centers. The fragment coord is then
/// `c = center + (uv · 2 / zoom) · (aspect, 1)`, where `aspect = w/h`
/// (`camera.aspect`, stored as `uniforms.aspect_ratio.x`). Computed in f64
/// here — strictly more accurate than the GPU's f32 or DF offset.
pub fn pixel_to_c(px: u32, py: u32, size: (u32, u32), center: (f64, f64), zoom: f64) -> (f64, f64) {
    let (w, h) = (size.0 as f64, size.1 as f64);
    let aspect = w / h;
    let uvx = 2.0 * (px as f64 + 0.5) / w - 1.0;
    let uvy = 1.0 - 2.0 * (py as f64 + 0.5) / h;
    let cx = center.0 + (uvx * 2.0 / zoom) * aspect;
    let cy = center.1 + (uvy * 2.0 / zoom);
    (cx, cy)
}

// ============================================================================
// Smooth iteration count — mirrors `smooth_iteration_count` (fractal.wgsl:457)
// ============================================================================

/// `(iteration + 1 - nu) / max_iterations`, where `nu` is the fractional
/// correction from the escape magnitude `mag_sq = |z|^2` at bailout, `R` the
/// bailout radius whose square the caller compared against, and `power` the
/// exponent of the `z^n + c` map. f64 here (ground truth); the DF path uses
/// the f32 variant below to match the shader's `log` precision.
pub fn smooth_f64(
    iteration: u32,
    mag_sq: f64,
    escape_radius: f64,
    power: f64,
    max_iter: u32,
) -> f64 {
    let log_zn = mag_sq.ln() / 2.0;
    let nu = (log_zn / escape_radius.ln()).ln() / power.abs().ln();
    (iteration as f64 + 1.0 - nu) / max_iter as f64
}

/// f32 mirror of `smooth_iteration_count` — used by the DF renderers so their
/// smooth values carry the same `f32::log` rounding as the GPU (the DF-vs-f64
/// teeth tolerance absorbs the f32-vs-f64 `log` difference).
pub fn smooth_f32(
    iteration: u32,
    mag_sq: f32,
    escape_radius: f32,
    power: f32,
    max_iter: u32,
) -> f32 {
    let log_zn = mag_sq.ln() / 2.0;
    let nu = (log_zn / escape_radius.ln()).ln() / power.abs().ln();
    (iteration as f32 + 1.0 - nu) / max_iter as f32
}

// ============================================================================
// f64 reference renderers (ground truth)
// ============================================================================

/// Result of one escape-time walk: `(iteration, mag_sq)` at bailout, or `None`
/// when the point is inside the set (never escaped within `max_iter`).
type Escape = Option<(u32, f64)>;

/// Escape-radius the *standard-precision* shader path derives from `power`
/// (`mandelbrot`, fractal.wgsl:589): `select(4.0, 2^(2/|n|), |n| < 2)`. For the
/// default `power = 2` this is `4.0`, i.e. bailout `|z|^2 > 16`.
fn std_escape_radius(power: f32) -> f32 {
    if power.abs() < 2.0 {
        2.0f32.powf(2.0 / power.abs())
    } else {
        4.0
    }
}

/// `complex_pow(z, n)` (fractal.wgsl:573): polar-form `z^n`, with the shader's
/// `r < 1e-7` short-circuit at the origin.
fn complex_pow_f64(zr: f64, zi: f64, n: f64) -> (f64, f64) {
    let r = (zr * zr + zi * zi).sqrt();
    if r < 1e-7 {
        return (0.0, 0.0);
    }
    let theta = zi.atan2(zr);
    let r_n = r.powf(n);
    let n_theta = n * theta;
    (r_n * n_theta.cos(), r_n * n_theta.sin())
}

/// Standard-precision Mandelbrot walk (`mandelbrot`, fractal.wgsl:585): the
/// escape test sits at the *top* of the loop and `iteration = i` is recorded
/// after the `z` update, so on escape `iteration` is one behind the `z` whose
/// magnitude tripped the test — mirrored exactly here.
fn mandelbrot_std_f64(c: (f64, f64), max_iter: u32, power: f32) -> Escape {
    let n = power as f64;
    let r = std_escape_radius(power) as f64;
    let bail = r * r;
    let mut zr: f64 = 0.0;
    let mut zi: f64 = 0.0;
    let mut iteration = 0u32;
    let mut i = 0u32;
    while i < max_iter {
        if zr * zr + zi * zi > bail {
            break;
        }
        let (pr, pi) = complex_pow_f64(zr, zi, n);
        zr = pr + c.0;
        zi = pi + c.1;
        iteration = i;
        i += 1;
    }
    finalize_std(zr, zi, iteration, max_iter, bail)
}

/// High-precision Mandelbrot walk (`mandelbrot_hp`, fractal.wgsl:464): direct
/// `z = z^2 + c`, bailout `|z|^2 > 4`. (`mandelbrot_hp` ignores `power` and
/// always squares.)
fn mandelbrot_hp_f64(c: (f64, f64), max_iter: u32) -> Escape {
    let mut zr = 0.0;
    let mut zi = 0.0;
    let mut iteration = 0u32;
    let mut i = 0u32;
    while i < max_iter {
        if zr * zr + zi * zi > 4.0 {
            break;
        }
        let zr2 = zr * zr - zi * zi + c.0;
        zi = 2.0 * zr * zi + c.1;
        zr = zr2;
        iteration = i;
        i += 1;
    }
    finalize_hp(zr, zi, iteration, max_iter)
}

/// High-precision Burning Ship (`burning_ship_hp`, fractal.wgsl:513):
/// `z = (|Re| + i|Im|)^2 + c`, bailout `|z|^2 > 4`.
fn burning_ship_hp_f64(c: (f64, f64), max_iter: u32) -> Escape {
    let mut zr: f64 = 0.0;
    let mut zi: f64 = 0.0;
    let mut iteration = 0u32;
    let mut i = 0u32;
    while i < max_iter {
        if zr * zr + zi * zi > 4.0 {
            break;
        }
        let ar = zr.abs();
        let ai = zi.abs();
        let zr2 = ar * ar - ai * ai + c.0;
        zi = 2.0 * ar * ai + c.1;
        zr = zr2;
        iteration = i;
        i += 1;
    }
    finalize_hp(zr, zi, iteration, max_iter)
}

/// High-precision Tricorn (`tricorn_hp`, fractal.wgsl:542): `z = conj(z)^2 + c`
/// where `conj(z) = (re, -im)`, bailout `|z|^2 > 4`.
fn tricorn_hp_f64(c: (f64, f64), max_iter: u32) -> Escape {
    let mut zr = 0.0;
    let mut zi = 0.0;
    let mut iteration = 0u32;
    let mut i = 0u32;
    while i < max_iter {
        if zr * zr + zi * zi > 4.0 {
            break;
        }
        // conj then square: (re - i·im)^2 = (re^2 - im^2) - 2·re·im·i
        let zr2 = zr * zr - zi * zi + c.0;
        zi = -2.0 * zr * zi + c.1;
        zr = zr2;
        iteration = i;
        i += 1;
    }
    finalize_hp(zr, zi, iteration, max_iter)
}

/// Interior sentinel mirrors the shader: if the loop exhausted without escape,
/// `iteration >= max(max_iter,1) - 1` and the function returns `-1.0`
/// (`fs_main_2d` maps `t < 0` to black). Standard path.
fn finalize_std(zr: f64, zi: f64, iteration: u32, max_iter: u32, bail: f64) -> Escape {
    if iteration >= max_iter.max(1) - 1 {
        return None;
    }
    let _ = bail;
    Some((iteration, zr * zr + zi * zi))
}

/// Interior check for the HP paths (bailout `|z|^2 > 4`).
fn finalize_hp(zr: f64, zi: f64, iteration: u32, max_iter: u32) -> Escape {
    if iteration >= max_iter.max(1) - 1 {
        return None;
    }
    Some((iteration, zr * zr + zi * zi))
}

/// Smooth value for a pixel, or `-1.0` for the interior. Picks the iteration
/// semantics the GPU uses at this zoom (HP above `HP_ZOOM_THRESHOLD`, standard
/// below) for Mandelbrot; Burning Ship and Tricorn always use the HP walk
/// (their deep-zoom rows engage HP, and no shallow row is defined for them).
pub fn smooth_at(kind: FractalKind, c: (f64, f64), zoom: f64, max_iter: u32, power: f32) -> f32 {
    let hp = zoom > HP_ZOOM_THRESHOLD;
    let esc = match kind {
        FractalKind::Mandelbrot => {
            if hp {
                mandelbrot_hp_f64(c, max_iter)
            } else {
                mandelbrot_std_f64(c, max_iter, power)
            }
        }
        FractalKind::BurningShip => burning_ship_hp_f64(c, max_iter),
        FractalKind::Tricorn => tricorn_hp_f64(c, max_iter),
    };
    match esc {
        None => -1.0,
        Some((iteration, mag_sq)) => {
            if hp || kind != FractalKind::Mandelbrot {
                smooth_f64(iteration, mag_sq, 2.0, 2.0, max_iter) as f32
            } else {
                let r = std_escape_radius(power);
                smooth_f64(iteration, mag_sq, r as f64, power as f64, max_iter) as f32
            }
        }
    }
}

/// Render `kind` to a row-major `Vec<f32>` of length `w·h` (`py = 0` at top).
/// Each entry is the smooth iteration value in `[0, 1)`, or `-1.0` for pixels
/// inside the set. `center`/`zoom` are f64 (the values `FractalSettings`
/// already keeps as f64); `max_iter` and `power` match the shader uniforms.
pub fn render(
    kind: FractalKind,
    center: (f64, f64),
    zoom: f64,
    size: (u32, u32),
    max_iter: u32,
    power: f32,
) -> Vec<f32> {
    let mut out = Vec::with_capacity((size.0 as usize) * (size.1 as usize));
    for py in 0..size.1 {
        for px in 0..size.0 {
            let c = pixel_to_c(px, py, size, center, zoom);
            out.push(smooth_at(kind, c, zoom, max_iter, power));
        }
    }
    out
}

/// sRGB OECF (linear → sRGB), matching the GPU's sRGB surface encoding. The
/// renderer picks an sRGB display format (`Bgra8UnormSrgb` on macOS,
/// `initialization.rs`), so the fragment's linear `vec3(t)` is hardware-encoded
/// to sRGB before the screenshot reads it back. The CPU reference writes PNG
/// bytes directly, so it must apply the same OECF or its output renders far too
/// dark vs the GPU (linear `t` vs sRGB `t` is the dominant cross-check error).
fn srgb_oecf(v: f32) -> f32 {
    if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// Map a buffer of smooth values to grayscale RGBA bytes (the shader's
/// `color_mode == 2`: `color = vec3(t)`), with the interior (`t < 0`) sent to
/// black — exactly what `fs_main_2d` writes for `t < 0`. The exterior value is
/// run through [`srgb_oecf`] to match the GPU's sRGB surface encoding, so the
/// CPU reference and the GPU screenshot are compared through the *same* color
/// mapping. Used by `imgdiff`'s reference-render mode.
pub fn smooth_to_grayscale_rgba(smooth: &[f32], size: (u32, u32)) -> Vec<u8> {
    let n = (size.0 as usize) * (size.1 as usize);
    let mut rgba = Vec::with_capacity(n * 4);
    for &t in smooth {
        let g = if t < 0.0 {
            0u8
        } else {
            (srgb_oecf(t.clamp(0.0, 1.0)) * 255.0).round() as u8
        };
        rgba.extend_from_slice(&[g, g, g, 255]);
    }
    rgba
}

// ============================================================================
// Double-float primitives — byte-for-byte mirror of fractal.wgsl:359–399
// ============================================================================
//
// `two_prod` is Dekker's split product (NOT hardware FMA): naga/backends may
// lower `a*b + c` to `a*b` (collapsing the error term), so the WGSL uses the
// split form. We mirror it exactly so the CPU DF walk reproduces the GPU DF
// walk bit-for-bit (modulo backend rounding). Do not "simplify" the algebra —
// every intermediate is load-bearing for the error-free transform (QA-005).

/// Reinterpret an f64 coordinate as the `(hi, lo)` f32 pair the shader's HP
/// path consumes (matches `renderer/uniforms.rs::split_f64`). `hi` is the f32
/// cast; `lo` is the f32 residual.
pub fn split_f64(v: f64) -> (f32, f32) {
    let hi = v as f32;
    let lo = (v - hi as f64) as f32;
    (hi, lo)
}

/// Error-free transform of `a + b` (fractal.wgsl:359).
fn two_sum(a: f32, b: f32) -> (f32, f32) {
    let s = a + b;
    let v = s - a;
    let e = (a - (s - v)) + (b - v);
    (s, e)
}

/// Dekker two-product without FMA (fractal.wgsl:381).
fn two_prod(a: f32, b: f32) -> (f32, f32) {
    let p = a * b;
    const SPLIT: f32 = 4097.0; // 2^12 + 1 for f32 (24-bit mantissa)
    let a_t = a * SPLIT;
    let a_hi = a_t - (a_t - a);
    let a_lo = a - a_hi;
    let b_t = b * SPLIT;
    let b_hi = b_t - (b_t - b);
    let b_lo = b - b_hi;
    let e = ((a_hi * b_hi - p) + a_hi * b_lo + a_lo * b_hi) + a_lo * b_lo;
    (p, e)
}

/// Double-float add with full error propagation (fractal.wgsl:369).
fn df_add_full(a_hi: f32, a_lo: f32, b_hi: f32, b_lo: f32) -> (f32, f32) {
    let s1 = two_sum(a_hi, b_hi);
    let s2 = two_sum(a_lo, b_lo);
    let s3 = two_sum(s1.0, s1.1 + s2.0);
    (s3.0, s3.1 + s2.1)
}

/// Double-float multiply (fractal.wgsl:395).
fn df_mul(a_hi: f32, a_lo: f32, b_hi: f32, b_lo: f32) -> (f32, f32) {
    let p = two_prod(a_hi, b_hi);
    let e = a_hi * b_lo + a_lo * b_hi;
    two_sum(p.0, p.1 + e)
}

/// Double-float subtract (fractal.wgsl:402).
fn df_sub(a_hi: f32, a_lo: f32, b_hi: f32, b_lo: f32) -> (f32, f32) {
    df_add_full(a_hi, a_lo, -b_hi, -b_lo)
}

/// Complex double-float: `re` and `im` are each `(hi, lo)`.
#[derive(Clone, Copy)]
struct Df2 {
    re: (f32, f32),
    im: (f32, f32),
}

impl Df2 {
    const fn zero() -> Self {
        Df2 {
            re: (0.0, 0.0),
            im: (0.0, 0.0),
        }
    }

    /// `|z|^2` from the high words only (fractal.wgsl:443) — f32 precision is
    /// ample for the escape test.
    fn mag_sq(self) -> f32 {
        self.re.0 * self.re.0 + self.im.0 * self.im.0
    }

    /// `z^2` (fractal.wgsl:430).
    fn square(self) -> Df2 {
        let a2 = df_mul(self.re.0, self.re.1, self.re.0, self.re.1);
        let b2 = df_mul(self.im.0, self.im.1, self.im.0, self.im.1);
        let real = df_sub(a2.0, a2.1, b2.0, b2.1);
        let ab = df_mul(self.re.0, self.re.1, self.im.0, self.im.1);
        let imag = (ab.0 * 2.0, ab.1 * 2.0);
        Df2 { re: real, im: imag }
    }

    /// `a + b` (fractal.wgsl:407).
    fn add(self, b: Df2) -> Df2 {
        let r = df_add_full(self.re.0, self.re.1, b.re.0, b.re.1);
        let i = df_add_full(self.im.0, self.im.1, b.im.0, b.im.1);
        Df2 { re: r, im: i }
    }
}

// ============================================================================
// Double-float renderers — mirror `mandelbrot_hp` / `burning_ship_hp` /
// `tricorn_hp` (fractal.wgsl:464 / 513 / 542) exactly, including the DF abs
// fix (QA-002: when a component's hi word is negative, BOTH words negate).
// ============================================================================

fn mandelbrot_df(c: Df2, max_iter: u32) -> f32 {
    let mut z = Df2::zero();
    let mut iteration = 0u32;
    let mut i = 0u32;
    while i < max_iter {
        if z.mag_sq() > 4.0 {
            break;
        }
        z = z.square().add(c);
        iteration = i;
        i += 1;
    }
    if iteration >= max_iter.max(1) - 1 {
        return -1.0;
    }
    let mag_sq = z.mag_sq();
    smooth_f32(iteration, mag_sq, 2.0, 2.0, max_iter)
}

fn burning_ship_df(c: Df2, max_iter: u32) -> f32 {
    let mut z = Df2::zero();
    let mut iteration = 0u32;
    let mut i = 0u32;
    while i < max_iter {
        if z.mag_sq() > 4.0 {
            break;
        }
        // DF abs: negate both words when the hi word is negative. Uses a plain
        // sign (cond ? -1 : 1) rather than `sign(hi)` — `sign(0.0)==0.0` would
        // silently zero the lo word when hi==0 (QA-002).
        let re_neg = if z.re.0 < 0.0 { -1.0 } else { 1.0 };
        let im_neg = if z.im.0 < 0.0 { -1.0 } else { 1.0 };
        let z_abs = Df2 {
            re: (z.re.0.abs(), z.re.1 * re_neg),
            im: (z.im.0.abs(), z.im.1 * im_neg),
        };
        z = z_abs.square().add(c);
        iteration = i;
        i += 1;
    }
    if iteration >= max_iter.max(1) - 1 {
        return -1.0;
    }
    let mag_sq = z.mag_sq();
    smooth_f32(iteration, mag_sq, 2.0, 2.0, max_iter)
}

fn tricorn_df(c: Df2, max_iter: u32) -> f32 {
    let mut z = Df2::zero();
    let mut iteration = 0u32;
    let mut i = 0u32;
    while i < max_iter {
        if z.mag_sq() > 4.0 {
            break;
        }
        // conj(z) = (re, -im), both words of im negated (fractal.wgsl:554).
        let z_conj = Df2 {
            re: z.re,
            im: (-z.im.0, -z.im.1),
        };
        z = z_conj.square().add(c);
        iteration = i;
        i += 1;
    }
    if iteration >= max_iter.max(1) - 1 {
        return -1.0;
    }
    let mag_sq = z.mag_sq();
    smooth_f32(iteration, mag_sq, 2.0, 2.0, max_iter)
}

/// Render the DF (high-precision) walk for `kind` to smooth values. `center`
/// and per-pixel offset are split into `(hi, lo)` exactly as the shader's HP
/// path does (`df_add_full(center_hi, center_lo, offset, 0)`), so this
/// reproduces the GPU's deep-zoom math on the CPU.
pub fn render_df(
    kind: FractalKind,
    center: (f64, f64),
    zoom: f64,
    size: (u32, u32),
    max_iter: u32,
) -> Vec<f32> {
    let (cx_hi, cx_lo) = split_f64(center.0);
    let (cy_hi, cy_lo) = split_f64(center.1);
    let (w, h) = (size.0 as f64, size.1 as f64);
    let aspect = w / h;
    let mut out = Vec::with_capacity((size.0 as usize) * (size.1 as usize));
    for py in 0..size.1 {
        for px in 0..size.0 {
            let uvx = 2.0 * (px as f64 + 0.5) / w - 1.0;
            let uvy = 1.0 - 2.0 * (py as f64 + 0.5) / h;
            // offset is the f32 term the shader adds to the (hi,lo) center.
            let off_x = (uvx * 2.0 / zoom * aspect) as f32;
            let off_y = (uvy * 2.0 / zoom) as f32;
            let c = Df2 {
                re: df_add_full(cx_hi, cx_lo, off_x, 0.0),
                im: df_add_full(cy_hi, cy_lo, off_y, 0.0),
            };
            let t = match kind {
                FractalKind::Mandelbrot => mandelbrot_df(c, max_iter),
                FractalKind::BurningShip => burning_ship_df(c, max_iter),
                FractalKind::Tricorn => tricorn_df(c, max_iter),
            };
            out.push(t);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Coordinate mapping sanity: fragments rasterize at pixel centers, so
    /// pixel (128,128) in a 256×256 view sits one half-pixel right and one
    /// half-pixel below the geometric center (127.5, 127.5). Right is +uvx
    /// (→ +x); below is +py, and since `uv.y` grows upward while `py` grows
    /// downward, below is −uvy (→ −y). So the sample lands just inside the
    /// +x/−y quadrant — matching the shader's clip-space `uv = position`.
    #[test]
    fn pixel_to_c_uses_pixel_center_convention() {
        let (cx, cy) = pixel_to_c(128, 128, (256, 256), (-0.5, 0.0), 1.0);
        assert!((cx - -0.4921875).abs() < 1e-12, "cx={cx}");
        assert!((cy - -0.0078125).abs() < 1e-12, "cy={cy}");
    }

    /// Known math facts the reference must satisfy regardless of zoom tier.
    #[test]
    fn known_points() {
        // c = 0 is inside the Mandelbrot set → interior sentinel.
        let interior = smooth_at(FractalKind::Mandelbrot, (0.0, 0.0), 1.0, 64, 2.0);
        assert_eq!(interior, -1.0);
        // c = (2, 2) escapes on the first z update → iteration 0, finite value.
        let esc = smooth_at(FractalKind::Mandelbrot, (2.0, 2.0), 1.0, 64, 2.0);
        assert!(esc > 0.0 && esc < 1.0, "esc={esc}");
    }

    /// Error-free-transform property of `two_sum`: `hi + lo == a + b` *exactly*
    /// in f64. This is the defining correctness condition — if it ever fails,
    /// the DF algebra drifted from the shader's (fractal.wgsl:359).
    #[test]
    fn two_sum_is_error_free() {
        let cases = [
            (1.0f32, 2.0f32),
            (1e8, 1.0),
            (1.0, 1e-8),
            (0.1, 0.2),
            (1.3333333, 2.6666667),
            (-1e6, 1e-6),
        ];
        for &(a, b) in &cases {
            let (hi, lo) = two_sum(a, b);
            let reconstructed = (hi as f64) + (lo as f64);
            let exact = (a as f64) + (b as f64);
            assert_eq!(reconstructed.to_bits(), exact.to_bits(), "two_sum({a},{b})");
        }
    }

    /// Error-free-transform property of `two_prod`: `hi + lo == a * b`
    /// *exactly* in f64. The FMA-free Dekker split (fractal.wgsl:381) must hold
    /// this exactly — a backend that lowered it to `a*b + 0` would collapse `lo`
    /// to 0 and fail here. (QA-005.)
    #[test]
    fn two_prod_is_error_free() {
        let cases = [
            (1.5f32, 3.25f32),
            (1e8, 1e-4),
            (0.1, 0.1),
            (1.3333333, 1.0),
            (65537.0, 65537.0), // exercises the 4097 split
            (-2.5, 4.0),
        ];
        for &(a, b) in &cases {
            let (hi, lo) = two_prod(a, b);
            let reconstructed = (hi as f64) + (lo as f64);
            let exact = (a as f64) * (b as f64);
            assert_eq!(
                reconstructed.to_bits(),
                exact.to_bits(),
                "two_prod({a},{b})"
            );
        }
    }

    /// `split_f64` reconstructs the input to within f64 relative error 1e-14
    /// across the deep-zoom coordinate range (mirrors the contract tested for
    /// `renderer::uniforms::split_f64`, guarding the center hi/lo split the DF
    /// renderer consumes).
    #[test]
    fn split_f64_roundtrip() {
        for &v in &[
            0.0f64,
            1.0,
            -1.0,
            std::f64::consts::PI,
            -0.7436438870,
            0.1318259042,
            1e10,
            1e-10,
        ] {
            let (hi, lo) = split_f64(v);
            let reconstructed = (hi as f64) + (lo as f64);
            let err = (reconstructed - v).abs();
            let rel = if v == 0.0 { err } else { err / v.abs() };
            assert!(rel < 1e-14, "split_f64({v:e}): rel_err={rel:e}");
        }
    }
}
