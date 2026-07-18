//! ENH-001 — perturbation-theory deep-zoom subsystem.
//!
//! The GPU's double-float HP path is correct only through ~1e7 zoom; above ~3e7
//! per-pixel coordinate precision collapses (root-caused 2026-07-17: a fast
//! frame with no device-loss, but the whole frame computes one shared orbit).
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
//! delta shader is `mandelbrot_perturb` in `shaders/fractal.wgsl`. See
//! `docs/fable/ENH-001-perturbation-deep-zoom.md`.

pub mod driver;
pub mod orbit;

pub use driver::{PerturbationDriver, perturbation_eligible};
// Re-exported so library consumers can construct / inspect orbits without
// reaching into the `orbit` submodule. The binary crate compiles
// `deep_zoom` but does not consume these two here, hence the allow — the
// lib's public API surface is what justifies the re-export.
#[allow(unused_imports)]
pub use orbit::{ReferenceOrbit, compute_reference_orbit, precision_bits_for_zoom};
