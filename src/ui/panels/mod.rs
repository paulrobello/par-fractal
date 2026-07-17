//! Per-panel rendering split out of `UI::render` (AUDIT QA-009).
//!
//! Each module owns one or more `CollapsingHeader` / `Window` blocks that
//! previously lived inline in the 3,300-line `UI::render`. The orchestrator
//! in `super::mod.rs` calls the panel methods in the same order they
//! originally appeared.
//!
//! These are pure code moves — widget logic is unchanged. Local action
//! flags (`changed`, `preset_to_load`, …) that the closures captured by
//! reference now live on the `UiActions` struct (AUDIT ARC-003) and are
//! threaded through as `&mut UiActions`.

mod about;
mod camera;
mod capture;
mod fractal_type;
mod lighting;
mod lod;
mod palette;
mod presets;
mod rendering;
mod settings;
