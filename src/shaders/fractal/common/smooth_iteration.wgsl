// Smooth (fractional) iteration count for escape-time fractals (QA-016).
// Returns (iteration + 1 - nu) / max_iterations where nu is the fractional
// correction derived from the escape magnitude |z|^2 = mag_sq at bailout.
// Parameterized over `escape_radius` (R, the bailout whose square the caller
// compared against) and `power` (n for z^n + c maps), so it serves both
// fixed-z^2 maps — pass (2.0, 2.0) — and variable-z^n Multibrot variants —
// pass the actual bailout and n. Callers that defensively clamp the inner
// ratio with `max(... , eps)` (nova/magnet/collatz) keep their inline
// epilogues: the clamp is unreachable under their bailouts but is preserved
// verbatim to avoid any source-level semantic drift.
fn smooth_iteration_count(iteration: u32, mag_sq: f32, escape_radius: f32, power: f32, max_iterations: u32) -> f32 {
    let log_zn = log(mag_sq) / 2.0;
    let nu = log(log_zn / log(escape_radius)) / log(abs(power));
    return (f32(iteration) + 1.0 - nu) / f32(max_iterations);
}
