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
//! This module is the CPU foundation (Phase A, step 1): the arbitrary-precision
//! Mandelbrot reference orbit. GPU plumbing, the delta shader, and the
//! view-change driver land in later Phase A steps. See
//! `docs/fable/ENH-001-perturbation-deep-zoom.md`.

pub mod orbit;

pub use orbit::{ReferenceOrbit, compute_reference_orbit, precision_bits_for_zoom};
