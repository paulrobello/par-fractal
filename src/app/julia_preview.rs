//! Issue #1: live Julia-under-cursor preview.
//!
//! While viewing the Mandelbrot set in 2D, a square inset in the top-right
//! corner renders the Julia set whose `c` is the complex coordinate under
//! the mouse cursor. The inset reuses the 2D fragment entry point
//! (`fs_main_2d`) with a dedicated uniform buffer + pipeline
//! (`Renderer::julia_preview_*`); the pass draws on the post-fork encoder
//! (right before egui) so screenshots and video stay fractal-only.
//!
//! Pure helpers (`should_show`, `cursor_to_complex`, `inset_rect`) are free
//! functions so the seam math is unit-testable without a GPU.

use super::App;
use crate::fractal::{FractalType, RenderMode};

/// Margin between the inset and the window edges, physical px.
const INSET_MARGIN_PX: f32 = 16.0;

/// Inset side as a fraction of the window height.
const INSET_HEIGHT_FRACTION: f32 = 0.25;

/// True when the preview inset should be drawn this frame.
pub fn should_show(enabled: bool, fractal_type: FractalType, render_mode: RenderMode) -> bool {
    enabled && fractal_type == FractalType::Mandelbrot2D && render_mode == RenderMode::TwoD
}

/// Map a cursor position (physical px, y-down) to the complex-plane
/// coordinate under it, or `None` for a zero-sized surface.
///
/// Formula matches the shader's 2D coordinate (`main_2d.wgsl`) and
/// `zoom_at`: `coord = center + ndc · 2/zoom` with x scaled by the aspect
/// ratio and ndc.y flipped so +y is up.
pub fn cursor_to_complex(
    cursor_px: (f32, f32),
    size_px: (u32, u32),
    center: [f64; 2],
    zoom: f64,
) -> Option<[f64; 2]> {
    let (w, h) = (size_px.0 as f64, size_px.1 as f64);
    if w <= 0.0 || h <= 0.0 || zoom <= 0.0 {
        return None;
    }
    let aspect = w / h;
    let ndc_x = cursor_px.0 as f64 / w * 2.0 - 1.0;
    let ndc_y = 1.0 - cursor_px.1 as f64 / h * 2.0;
    Some([
        center[0] + ndc_x * 2.0 / zoom * aspect,
        center[1] + ndc_y * 2.0 / zoom,
    ])
}

/// Inset rectangle in physical px as `[x, y, w, h]`, origin top-left:
/// a square in the top-right corner, clamped to stay inside tiny windows
/// (the scissor rect is validated against the attachment size).
pub fn inset_rect(size_px: (u32, u32)) -> [f32; 4] {
    let (w, h) = (size_px.0 as f32, size_px.1 as f32);
    let side = (h * INSET_HEIGHT_FRACTION).max(24.0).min(w * 0.5);
    let side = side.min((w - INSET_MARGIN_PX).max(8.0)).max(8.0);
    let x = (w - INSET_MARGIN_PX - side).max(0.0);
    let y = INSET_MARGIN_PX.min((h - side).max(0.0));
    // Degenerate windows can clamp x to 0 with `side` overflowing the
    // surface — the scissor must stay inside the attachment, so shrink.
    let side = side.min(w - x).max(1.0);
    [x, y, side, side]
}

impl App {
    /// True when the preview inset should be drawn this frame.
    pub(super) fn julia_preview_visible(&self) -> bool {
        should_show(
            self.ui.ui_state.julia_preview_enabled,
            self.fractal_params.settings.fractal_type,
            self.fractal_params.settings.render_mode,
        )
    }

    /// Physical-px inset rect for the current surface. Shared by the GPU
    /// pass and the egui frame border so the two can never drift.
    pub(super) fn julia_preview_rect(&self) -> [f32; 4] {
        inset_rect((self.renderer.size.width, self.renderer.size.height))
    }

    /// Draw the inset: one extra pass of the 2D fractal path into the
    /// swap-chain view, viewport+scissor limited to the inset rect, using
    /// the dedicated preview uniform buffer (Julia2D at the cursor's `c`,
    /// written each frame by `Renderer::update_julia_preview`).
    pub(super) fn render_julia_preview_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        if !self.julia_preview_visible() {
            return;
        }
        let [x, y, w, h] = self.julia_preview_rect();
        let sw = self.renderer.size.width;
        let sh = self.renderer.size.height;

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Julia Preview Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        // Same forget_lifetime pattern as the scene pass in app/render.rs.
        let mut render_pass = render_pass.forget_lifetime();
        render_pass.set_pipeline(&self.renderer.julia_preview_pipeline);
        render_pass.set_viewport(x, y, w, h, 0.0, 1.0);
        // Clamp the scissor into the attachment (validation requires it).
        let x0 = x as u32;
        let y0 = y as u32;
        let w0 = (w as u32).min(sw.saturating_sub(x0));
        let h0 = (h as u32).min(sh.saturating_sub(y0));
        render_pass.set_scissor_rect(x0, y0, w0.max(1), h0.max(1));
        render_pass.set_bind_group(0, &self.renderer.julia_preview_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.renderer.vertex_buffer.slice(..));
        render_pass.draw(0..4, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Center pixel maps to the view center exactly.
    #[test]
    fn center_pixel_maps_to_center() {
        let c = cursor_to_complex((500.0, 250.0), (1000, 500), [-0.5, 0.25], 2.0).unwrap();
        assert_eq!(c, [-0.5, 0.25]);
    }

    /// Top-left pixel is min-x / max-y in complex space (y flips).
    #[test]
    fn top_left_pixel_maps_to_upper_left_complex() {
        let c = cursor_to_complex((0.0, 0.0), (1000, 500), [0.0, 0.0], 2.0).unwrap();
        // ndc = (-1, +1), aspect = 2 → x: -2/2·2 = -2, y: +2/2 = 1
        assert!((c[0] - (-2.0)).abs() < 1e-12);
        assert!((c[1] - 1.0).abs() < 1e-12);
    }

    /// THE seam: the fractal point under the cursor is preserved by
    /// `zoom_at` — the same pixel maps to the same complex coordinate before
    /// and after a zoom-at-cursor (what makes the preview feel attached to
    /// the cursor while scrolling).
    #[test]
    fn cursor_complex_is_zoom_at_invariant() {
        use crate::fractal::FractalParams;
        let size = (1280.0_f32, 800.0_f32);
        let aspect = (size.0 / size.1) as f64;
        let cursor = (730.0_f32, 260.0_f32);
        let ndc = (
            (cursor.0 as f64 / size.0 as f64) * 2.0 - 1.0,
            1.0 - (cursor.1 as f64 / size.1 as f64) * 2.0,
        );

        let mut params = FractalParams::default();
        let before = cursor_to_complex(
            cursor,
            (1280, 800),
            params.settings.center_2d,
            params.settings.zoom_2d,
        )
        .unwrap();
        params.zoom_at(ndc, 1.7, aspect);
        let after = cursor_to_complex(
            cursor,
            (1280, 800),
            params.settings.center_2d,
            params.settings.zoom_2d,
        )
        .unwrap();

        assert!(
            (before[0] - after[0]).abs() < 1e-6,
            "{before:?} vs {after:?}"
        );
        assert!(
            (before[1] - after[1]).abs() < 1e-6,
            "{before:?} vs {after:?}"
        );
    }

    /// 1080p: 25%-of-height square, top-right, 16px margins.
    #[test]
    fn inset_rect_standard_sizes() {
        assert_eq!(inset_rect((1920, 1080)), [1634.0, 16.0, 270.0, 270.0]);
        assert_eq!(inset_rect((1280, 720)), [1084.0, 16.0, 180.0, 180.0]);
    }

    /// The rect always fits inside the surface, even for degenerate sizes
    /// (a scissor rect outside the attachment is a wgpu validation error).
    #[test]
    fn inset_rect_stays_in_bounds() {
        for &(w, h) in &[
            (1u32, 1u32),
            (20, 20),
            (40, 30),
            (100, 100),
            (640, 480),
            (2560, 1440),
        ] {
            let [x, y, sw, sh] = inset_rect((w, h));
            assert!(x >= 0.0 && y >= 0.0, "{w}x{h}: origin {x},{y}");
            assert!(sw >= 1.0 && sh >= 1.0, "{w}x{h}: size {sw}x{sh}");
            assert!(x + sw <= w as f32, "{w}x{h}: x+w = {} > {w}", x + sw);
            assert!(y + sh <= h as f32, "{w}x{h}: y+h = {} > {h}", y + sh);
        }
    }

    /// The inset zoom frames |z| ≤ 2.5 — the full Julia "home" region
    /// (a 2D view spans `2 / zoom` complex units).
    #[test]
    fn inset_zoom_frames_julia_home() {
        let zoom = crate::renderer::uniforms::Uniforms::INSET_ZOOM as f64;
        assert!((2.0 / zoom - 5.0).abs() < 1e-5);
    }

    /// Only an explicitly-enabled Mandelbrot 2D view shows the inset.
    #[test]
    fn should_show_gates_type_and_mode() {
        use crate::fractal::{FractalType, RenderMode};
        assert!(should_show(
            true,
            FractalType::Mandelbrot2D,
            RenderMode::TwoD
        ));
        assert!(!should_show(
            false,
            FractalType::Mandelbrot2D,
            RenderMode::TwoD
        ));
        assert!(!should_show(true, FractalType::Julia2D, RenderMode::TwoD));
        assert!(!should_show(
            true,
            FractalType::Mandelbulb3D,
            RenderMode::ThreeD
        ));
    }
}
