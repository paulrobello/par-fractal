//! GPU storage buffer for the perturbation reference orbit (ENH-001 Phase A,
//! step 3 — plumbing only; perturbation stays OFF here, step 5 will populate).
//!
//! Backs the `@group(0) @binding(1) var<storage, read> ref_orbit:
//! array<vec2<f32>>;` declaration in [`shaders/fractal.wgsl`]. The buffer is
//! `STORAGE | COPY_DST`: the CPU writes the orbit's `Z_n` f32 pairs via
//! [`wgpu::Queue::write_buffer`]; the fragment shader reads them per pixel as
//! the reference orbit for delta iteration
//! (`Δz ← 2·Z_n·Δz + Δz² + Δc`). Recreated only when the orbit length grows
//! past the current capacity — growing only avoids a per-view reallocation
//! when the user pans/zooms within a similar iteration budget.
//!
//! This mirrors the storage-buffer plumbing pattern established by
//! [`crate::renderer::compute::BuddhabrotAccumulationBuffer`] (read-only here
//! vs. read-write there).
//!
//! [`shaders/fractal.wgsl`]: ../../shaders/fractal.wgsl

/// Metadata for the currently-active reference orbit, held on the
/// [`Renderer`] alongside the [`OrbitBuffer`] that stores the `Z_n` values.
///
/// `Some` on the renderer means perturbation is live for the frame: the
/// orbit has been computed, uploaded, and the view still meets the
/// activation gate. The CPU-side `z` vector itself is NOT retained — once
/// uploaded to the GPU, only the length, escape index, and reference offset
/// are needed to populate the matching uniforms and let the shader
/// bounds-check.
///
/// (ENH-001 Phase A step 5.)
#[derive(Debug, Clone, Copy)]
pub struct ActiveOrbit {
    /// Number of `Z_n` entries actually populated in the buffer. The shader
    /// uses this (via the `orbit_len` uniform) to bounds-check iteration
    /// indices — `arrayLength` would report the over-allocated capacity.
    pub len: u32,
    /// First index at which the reference escaped (`|Z|² > 4`), or `0` if
    /// the reference stayed bounded through `max_iter`. The `0` sentinel is
    /// safe for the bounded case because the shader's per-pixel delta orbit
    /// never needs to rebase before iteration 0.
    pub escaped_at: u32,
    /// `c_center − c_ref` as an f32 pair (the orbit's `reference_offset`).
    /// The shader adds this to `uv * delta_c_scale` to reconstruct each
    /// pixel's Δc — see `Uniforms::activate_perturbation`. `[0,0]` when the
    /// reference is the view center.
    pub reference_offset: [f32; 2],
}

/// GPU storage buffer holding a perturbation reference orbit's `Z_n` values.
///
/// One entry per iteration: `z[n] = (Z_n.re, Z_n.im)` as f32 pairs (the
/// low-precision mirror — deltas carry the precision, so f32 pairs are
/// sufficient; this is standard for perturbation rendering). The shader
/// indexes with the iteration index and bounds-checks against the
/// `orbit_len` uniform (NOT `arrayLength` — the buffer may be
/// over-allocated; see ENH-001 pitfalls).
pub struct OrbitBuffer {
    /// The underlying wgpu storage buffer. Replaced in-place by
    /// [`Self::ensure_capacity`] when the orbit outgrows the current
    /// allocation; any bind group referencing the previous buffer must be
    /// rebuilt by the caller.
    pub buffer: wgpu::Buffer,
    /// Capacity in `vec2<f32>` entries (NOT bytes). Always ≥ 1 once
    /// allocated; the placeholder created by [`Self::new_placeholder`] holds
    /// a single zeroed entry so the bind group is valid even before the
    /// first real orbit is uploaded.
    pub capacity: usize,
}

impl OrbitBuffer {
    /// Minimum capacity kept allocated so a storage binding is always valid
    /// even when perturbation is OFF (step 3 ships with perturbation OFF;
    /// step 5 will populate this with a real orbit). One `vec2<f32>` entry
    /// is the smallest binding that satisfies `min_binding_size` in the
    /// bind group layout.
    const PLACEHOLDER_CAPACITY: usize = 1;

    /// Create the placeholder buffer used at renderer initialization.
    ///
    /// One zeroed `vec2<f32>` entry; the shader gates on `orbit_len == 0`
    /// and `perturbation_enabled == 0`, so this entry is never read as a
    /// real orbit. The bind group can still bind this buffer (storage
    /// bindings must point at a real buffer even when unused).
    pub fn new_placeholder(device: &wgpu::Device) -> Self {
        let bytes = (Self::PLACEHOLDER_CAPACITY * std::mem::size_of::<[f32; 2]>()) as u64;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reference Orbit Buffer (placeholder)"),
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            buffer,
            capacity: Self::PLACEHOLDER_CAPACITY,
        }
    }

    /// Ensure the buffer can hold `len` `vec2<f32>` entries, recreating
    /// (growing only) if needed. Returns `true` when the buffer was
    /// reallocated, signaling the caller to rebuild any bind group that
    /// references it. Returns `false` for no-op (existing capacity
    /// suffices).
    ///
    /// Capacity policy: grow-to-fit (no slack). Orbits are bounded by
    /// `max_iterations` and capacity is monotonic over a session (the user
    /// rarely drops iteration count below a previous high), so reallocs are
    /// rare after the first deep zoom.
    ///
    /// ENH-001 step 3: unused until step 5 (CPU driver) calls it on the
    /// first real orbit upload.
    pub fn ensure_capacity(&mut self, device: &wgpu::Device, len: usize) -> bool {
        if len <= self.capacity {
            return false;
        }
        let new_capacity = len.max(Self::PLACEHOLDER_CAPACITY);
        let bytes = (new_capacity * std::mem::size_of::<[f32; 2]>()) as u64;
        self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reference Orbit Buffer"),
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.capacity = new_capacity;
        true
    }

    /// Upload `Z_n` pairs as a flattened `[[f32; 2]]` byte stream.
    ///
    /// Takes the raw `&[[f32; 2]]` slice rather than a typed
    /// `&ReferenceOrbit` so this module stays decoupled from the CPU-side
    /// orbit representation (the orbit lives in `crate::deep_zoom`; step 5's
    /// driver calls this as `orbit_buffer.write(&queue, &orbit.z)`).
    ///
    /// The caller MUST first ensure capacity via [`Self::ensure_capacity`]
    /// (this is a no-op check, not a runtime guard — writing more than the
    /// buffer holds is a GPU-validation error). `z` is already contiguous
    /// `[[f32; 2]]`, so `bytemuck::cast_slice` writes it without a copy.
    /// Only `z.len()` entries are written; the buffer may be larger
    /// (over-allocated capacity is left untouched and never read because
    /// the shader bounds-checks via the `orbit_len` uniform).
    ///
    /// ENH-001 step 3: unused until step 5 (CPU driver) calls it to upload
    /// a computed reference orbit.
    pub fn write(&self, queue: &wgpu::Queue, z: &[[f32; 2]]) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(z));
    }
}
