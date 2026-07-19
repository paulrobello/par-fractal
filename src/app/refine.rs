//! ENH-002 v2 — tile-progressive refinement (pure math).
//!
//! When a 2D view is too expensive to render in one frame at full quality
//! (deep zoom at high iterations), `render()` refines it tile-by-tile across
//! frames instead of freezing on one costly frame. Each frame rasterizes a
//! single scissor-rect tile at full quality with `LoadOp::Load` on
//! `scene_texture`, so finished tiles persist and detail pours in
//! center-out. This module holds the pure decision math so it is unit-testable
//! without a GPU or an `App`.

/// Per-tile frame budget in milliseconds. Each tile frame should land near
/// this so the UI stays responsive (~60 FPS) while the full-quality image
/// converges. Tunable.
pub(crate) const TILE_BUDGET_MS: f32 = 16.0;

/// Maximum tiles along one side of the (square) refinement grid. `8` → up to
/// 64 tiles, enough to split even a very expensive frame into ~16 ms slices.
pub(crate) const MAX_GRID_SIDE: u32 = 8;

/// Estimate the full-resolution, full-quality frame cost (ms) from the last
/// rendered frame's measured time and the LOD render scale then in effect.
///
/// Pixel count scales with `render_scale²`, so a frame rendered at scale `s`
/// cost roughly `1/s²` of the full-resolution frame. The estimate inverts that
/// to predict the full-quality cost.
///
/// Iteration scaling is deliberately NOT folded in: at deep zoom — where
/// refinement matters most — the perturbation path pins iterations to the
/// orbit length regardless of LOD (`activate_perturbation`), so only the
/// pixel term applies. For shallow non-perturbation zoom this under-estimates,
/// which merely yields coarser (fewer) tiles — safe, never incorrect.
pub(crate) fn estimate_full_quality_ms(last_frame_ms: f32, render_scale: f32) -> f32 {
    let s = render_scale.clamp(0.05, 1.0);
    (last_frame_ms / (s * s)).max(0.0)
}

/// Grid side length (tiles per row and column) for an estimated full-quality
/// cost. Returns `1` (→ one tile = the whole frame, i.e. no tiling) when the
/// view is cheap enough to render in a single frame; otherwise the smallest
/// square grid whose per-tile slice fits the budget, clamped to
/// `[1, MAX_GRID_SIDE]`.
pub(crate) fn grid_side_for_cost(est_full_ms: f32) -> u32 {
    if est_full_ms <= TILE_BUDGET_MS {
        return 1;
    }
    let tiles_needed = ((est_full_ms / TILE_BUDGET_MS).ceil() as u32).max(1);
    let side = (tiles_needed as f32).sqrt().ceil() as u32;
    side.clamp(1, MAX_GRID_SIDE)
}

/// Center-out tile visit order for a `grid_side`×`grid_side` grid, as linear
/// indices `row * grid_side + col`. The center tile(s) come first so detail
/// appears where the user is looking, then expanding outward. Ties (equal
/// distance, e.g. the four center-neighbors of an even grid) are broken by
/// index for determinism.
pub(crate) fn center_out_order(grid_side: u32) -> Vec<u32> {
    let n = grid_side;
    if n == 0 {
        return Vec::new();
    }
    let center = (n as f32 - 1.0) / 2.0;
    let mut tiles: Vec<(u32, f32)> = (0..n)
        .flat_map(|r| (0..n).map(move |c| (r, c)))
        .map(|(r, c)| {
            let dr = r as f32 - center;
            let dc = c as f32 - center;
            let dist = (dr * dr + dc * dc).sqrt();
            (r * n + c, dist)
        })
        .collect();
    tiles.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    tiles.into_iter().map(|(i, _)| i).collect()
}

/// Runtime state of an in-flight tile-progressive refinement: a square grid,
/// the center-out visit order, and how far through it we are. One tile is
/// rasterized per frame (`render_refine_tile`); when `next` reaches `total()`
/// the scene is fully refined → Converged.
#[derive(Clone)]
pub(crate) struct RefineState {
    pub grid_side: u32,
    pub order: Vec<u32>,
    pub next: usize,
}

impl RefineState {
    pub fn new(grid_side: u32) -> Self {
        let order = center_out_order(grid_side);
        Self {
            grid_side,
            order,
            next: 0,
        }
    }

    pub fn total(&self) -> usize {
        self.order.len()
    }
}

/// The pixel scissor rect `(x, y, w, h)` for tile `tile_idx` (linear
/// `row * grid_side + col`) of a `grid_side`×`grid_side` grid covering a
/// `width`×`height` surface. Floor-divides the surface so the whole texture is
/// covered with no gaps or overlaps, and clamps defensively so the result is
/// always a valid non-empty scissor within bounds (wgpu rejects empty or
/// out-of-bounds scissors). For surfaces smaller than the grid, multiple tiles
/// map to the same single-pixel rect — no gaps, never invalid.
pub(crate) fn tile_rect(tile_idx: u32, grid_side: u32, width: u32, height: u32) -> [u32; 4] {
    let grid_side = grid_side.max(1);
    let col = tile_idx % grid_side;
    let row = tile_idx / grid_side;
    let width = width.max(1);
    let height = height.max(1);
    let x = ((col * width) / grid_side).min(width - 1);
    let y = ((row * height) / grid_side).min(height - 1);
    let x_end = (((col + 1) * width) / grid_side).max(x + 1);
    let y_end = (((row + 1) * height) / grid_side).max(y + 1);
    let w = (x_end - x).min(width - x);
    let h = (y_end - y).min(height - y);
    [x, y, w, h]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cheap_view_needs_no_tiling() {
        // A view that already renders within budget → grid side 1 (one tile =
        // the whole frame, i.e. fall back to the v1 single-frame converge).
        assert_eq!(grid_side_for_cost(0.0), 1);
        assert_eq!(grid_side_for_cost(TILE_BUDGET_MS), 1);
        assert_eq!(grid_side_for_cost(TILE_BUDGET_MS - 0.01), 1);
    }

    #[test]
    fn cost_scales_to_a_square_grid() {
        // ~2× budget → 2 tiles needed → ceil(sqrt(2)) = 2 → 2×2 grid.
        assert_eq!(grid_side_for_cost(TILE_BUDGET_MS * 2.0), 2);
        // ~4× budget → 4 tiles → sqrt(4) = 2 → still 2×2.
        assert_eq!(grid_side_for_cost(TILE_BUDGET_MS * 4.0), 2);
        // ~5× budget → 5 tiles → ceil(sqrt(5)) = 3 → 3×3.
        assert_eq!(grid_side_for_cost(TILE_BUDGET_MS * 5.0), 3);
    }

    #[test]
    fn cost_is_clamped_to_max_grid() {
        // An absurdly expensive view still caps at MAX_GRID_SIDE per side.
        assert_eq!(grid_side_for_cost(TILE_BUDGET_MS * 1000.0), MAX_GRID_SIDE);
    }

    #[test]
    fn full_quality_extrapolates_by_pixel_count() {
        // A 12 ms frame at half resolution → ~48 ms at full resolution
        // (4× the pixels). render_scale is the only term.
        let est = estimate_full_quality_ms(12.0, 0.5);
        assert!(
            (est - 48.0).abs() < 0.01,
            "half-res 12 ms should extrapolate to ~48 ms, got {est}",
        );
        // Full-resolution frame extrapolates to itself.
        assert!(
            (estimate_full_quality_ms(20.0, 1.0) - 20.0).abs() < 0.01,
            "full-res should be unchanged",
        );
        // Quarter-res → 16× pixels.
        assert!(
            (estimate_full_quality_ms(10.0, 0.25) - 160.0).abs() < 0.01,
            "quarter-res 10 ms should extrapolate to ~160 ms",
        );
    }

    #[test]
    fn center_out_starts_at_center() {
        // Odd grid: the single center tile is first.
        let order = center_out_order(3);
        assert_eq!(order.len(), 9);
        // Center of a 3×3 is index 4 (row 1, col 1).
        assert_eq!(order[0], 4, "3×3 center tile (idx 4) must be first");
        // Last tiles are the corners (indices 0, 2, 6, 8), in some order.
        let last_four: std::collections::HashSet<u32> = order[5..].iter().copied().collect();
        assert_eq!(last_four, [0, 2, 6, 8].into_iter().collect(),);
    }

    #[test]
    fn center_out_even_grid_starts_near_center() {
        // Even grid has no single center; the four inner tiles come first.
        let order = center_out_order(4);
        assert_eq!(order.len(), 16);
        let first_four: std::collections::HashSet<u32> = order[0..4].iter().copied().collect();
        // Inner 2×2 of a 4×4 = indices 5, 6, 9, 10.
        assert_eq!(first_four, [5, 6, 9, 10].into_iter().collect(),);
        // Corners last.
        let last_four: std::collections::HashSet<u32> = order[12..].iter().copied().collect();
        assert_eq!(last_four, [0, 3, 12, 15].into_iter().collect(),);
    }

    #[test]
    fn center_out_visits_every_tile_exactly_once() {
        for &side in &[1u32, 2, 3, 4, 5, 8] {
            let order = center_out_order(side);
            let n = (side * side) as usize;
            assert_eq!(order.len(), n, "side {side} should yield {n} tiles");
            let mut sorted = order.clone();
            sorted.sort();
            let expected: Vec<u32> = (0..n as u32).collect();
            assert_eq!(sorted, expected, "side {side}: missing or duplicate tiles");
        }
    }

    /// The scissor rect must always be a valid, non-empty, in-bounds sub-rect of
    /// the surface — wgpu rejects anything else. This sweeps many grid/window
    /// combinations (including non-divisible sizes and tiny surfaces) and checks
    /// every tile.
    #[test]
    fn tile_rect_is_always_valid_and_in_bounds() {
        for &side in &[1u32, 2, 3, 4, 5, 8] {
            for &(w, h) in &[(1920u32, 1080), (256, 256), (100, 100), (7, 13), (1, 1)] {
                for idx in 0..side * side {
                    let [x, y, tw, th] = tile_rect(idx, side, w, h);
                    assert!(tw >= 1, "side {side} idx {idx} ({w}x{h}): empty width");
                    assert!(th >= 1, "side {side} idx {idx} ({w}x{h}): empty height");
                    assert!(
                        x + tw <= w,
                        "side {side} idx {idx} ({w}x{h}): x{x}+w{tw} > {w}",
                    );
                    assert!(
                        y + th <= h,
                        "side {side} idx {idx} ({w}x{h}): y{y}+h{th} > {h}",
                    );
                }
            }
        }
    }

    /// The union of all tiles must cover the whole surface with no gaps (and, for
    /// surfaces larger than the grid, no overlap). This is the invariant that
    /// makes `LoadOp::Load` refinement produce a complete image.
    #[test]
    fn tile_rect_covers_surface_without_gaps() {
        for &side in &[1u32, 2, 3, 4, 5, 8] {
            for &(w, h) in &[(1920u32, 1080), (256, 256), (100, 100)] {
                // Check column coverage: tiles in row 0 must span [0, w).
                let mut xs: Vec<u32> = (0..side).map(|c| tile_rect(c, side, w, h)[0]).collect();
                xs.sort();
                assert_eq!(
                    xs[0], 0,
                    "side {side} ({w}x{h}): first column doesn't start at x=0"
                );
                let last_end = tile_rect(side - 1, side, w, h);
                assert_eq!(
                    last_end[0] + last_end[2],
                    w,
                    "side {side} ({w}x{h}): last column doesn't reach x={w}",
                );
                // Adjacent columns are contiguous (next start == prev end).
                for c in 0..side - 1 {
                    let cur = tile_rect(c, side, w, h);
                    let nxt = tile_rect(c + 1, side, w, h);
                    assert_eq!(
                        cur[0] + cur[2],
                        nxt[0],
                        "side {side} ({w}x{h}): gap/overlap between cols {c} and {}",
                        c + 1
                    );
                }
                // Same for rows.
                let last_row = tile_rect((side - 1) * side, side, w, h);
                assert_eq!(
                    last_row[1] + last_row[3],
                    h,
                    "side {side} ({w}x{h}): last row doesn't reach y={h}"
                );
            }
        }
    }

    /// A 1×1 grid is the whole frame (no tiling) — the fallback path.
    #[test]
    fn tile_rect_single_tile_is_full_surface() {
        let [x, y, w, h] = tile_rect(0, 1, 1920, 1080);
        assert_eq!([x, y, w, h], [0, 0, 1920, 1080]);
    }
}
