//! ENH-001 — perturbation-theory deep-zoom subsystem.
//!
//! The GPU's double-float HP path loses precision on Metal across the mid-zoom
//! band: a 2026-07-18 crosscheck (GPU vs CPU f64) showed per-pixel noise from
//! ~1e4, worsening to a fully collapsed frame (one shared orbit) above ~3e7
//! (root-caused 2026-07-17 as downstream Metal FTZ / sub-ULP lo-word loss).
//! Perturbation removes the ceiling: compute ONE reference orbit per view here
//! in arbitrary precision, upload `Z_n` as f32 pairs to a GPU storage buffer,
//! and iterate only per-pixel *deltas* on the GPU in plain f32
//! (`Δz ← 2·Z_n·Δz + Δz² + Δc`).
//!
//! Module layout:
//! - [`orbit`] — CPU foundation (Phase A step 1): the arbitrary-precision
//!   Mandelbrot reference orbit.
//! - [`driver`] — Phase A step 5: the off-render-thread scheduler that
//!   recomputes the orbit on view change and drains the result on the main
//!   thread.
//!
//! GPU plumbing (storage buffer + uniforms) lives in
//! [`crate::renderer::orbit_buffer`] and [`crate::renderer::uniforms`]; the
//! delta shader is `mandelbrot_perturb` in `shaders/fractal.wgsl`.

pub mod driver;
pub mod orbit;

pub use driver::{PerturbationDriver, perturbation_eligible};
// Re-exported so library consumers can construct / inspect orbits without
// reaching into the `orbit` submodule. The binary crate compiles
// `deep_zoom` but does not consume these two here, hence the allow — the
// lib's public API surface is what justifies the re-export.
#[allow(unused_imports)]
pub use orbit::{
    FractalKind, ReferenceOrbit, compute_reference_orbit, compute_reference_orbit_best,
    precision_bits_for_zoom,
};
