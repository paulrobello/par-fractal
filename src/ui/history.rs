use super::UI;
use crate::fractal::{FractalParams, RenderSettings};

/// History entry for undo/redo functionality.
///
/// ARC-015: stores ONLY the authored [`RenderSettings`] snapshot, not a full
/// `FractalParams`. Undo/redo restores the user's render knobs without
/// clobbering transient runtime state that the user did not author and that
/// the audit explicitly flagged should not be resurrected:
///
/// - `LodRuntime` (the multi-KB FPS deque + motion EMA + transition timers)
/// - `AccumulationState` (counters, last-view snapshots, pending-clear flag)
///
/// Restoring those on undo produced visible glitches: redoing back to a slow
/// scene inherited the FPS measurements of the scene you just left, and
/// undoing an attractor tweak resurrected stale accumulation buffers. Both
/// subsystems now keep their runtime state across undo/redo; only the user's
/// authored preferences roll back.
#[derive(Clone)]
pub(super) struct HistoryEntry {
    pub(super) settings: RenderSettings,
    // QA-020: stamp captured for the future history panel (entry age, "5m ago"
    // labels). The field is written on every push but no UI reads it yet;
    // deleting it would lose provenance that's expensive to reconstruct.
    #[allow(dead_code)]
    pub(super) timestamp: web_time::Instant,
}

/// Undo/redo functionality for UI
impl UI {
    pub(super) fn save_to_history(&mut self, params: &FractalParams) {
        // If we're not at the end of history, truncate everything after current position
        if self.history_index < self.history.len() {
            self.history.truncate(self.history_index);
        }

        // Check if settings actually changed from last saved state. Only the
        // authored `RenderSettings` is compared — runtime state (LOD FPS ring,
        // accumulation counters) is intentionally excluded so it doesn't
        // generate spurious history entries on every frame.
        let should_save = if let Some(ref last) = self.last_saved_settings {
            !settings_equal(&params.settings, last)
        } else {
            true
        };

        if should_save {
            self.history.push_back(HistoryEntry {
                settings: params.settings.clone(),
                timestamp: web_time::Instant::now(),
            });

            // Maintain max history size. ARC-019: VecDeque keeps the
            // Oldest entry at the front, so pop_front is O(1) (vs the
            // previous Vec::remove(0) which shifted every entry).
            if self.history.len() > self.max_history_size {
                self.history.pop_front();
            }
            // Index always tracks the just-pushed tail.
            self.history_index = self.history.len();

            self.last_saved_settings = Some(params.settings.clone());
        }
    }

    /// Roll the user's authored render settings back one step.
    ///
    /// Returns the previous [`RenderSettings`] snapshot, or `None` if there is
    /// nothing to undo. The caller writes it back into `params.settings`,
    /// leaving `params.lod` and `params.accum` untouched (ARC-015).
    pub(super) fn undo(&mut self) -> Option<RenderSettings> {
        if self.can_undo() {
            self.history_index = self.history_index.saturating_sub(1);
            Some(self.history[self.history_index].settings.clone())
        } else {
            None
        }
    }

    /// Roll the user's authored render settings forward one step.
    ///
    /// Returns the next [`RenderSettings`] snapshot, or `None` if there is
    /// nothing to redo. As with [`undo`](Self::undo), only `RenderSettings`
    /// is restored.
    pub(super) fn redo(&mut self) -> Option<RenderSettings> {
        if self.can_redo() {
            self.history_index += 1;
            Some(self.history[self.history_index].settings.clone())
        } else {
            None
        }
    }

    pub(super) fn can_undo(&self) -> bool {
        self.history_index > 0 && !self.history.is_empty()
    }

    pub(super) fn can_redo(&self) -> bool {
        !self.history.is_empty() && self.history_index < self.history.len() - 1
    }
}

/// Helper to compare authored render settings.
///
/// Uses pointer equality as a cheap short-circuit when the same
/// `RenderSettings` instance is compared against itself (e.g. the UI passes
/// the live `params.settings` reference both as the new value and as the
/// cached previous value during a no-op render). Two distinct instances are
/// always considered "changed" — this matches the pre-ARC-015 behavior and
/// only affects whether a redundant history entry is created, not
/// correctness.
fn settings_equal(a: &RenderSettings, b: &RenderSettings) -> bool {
    std::ptr::eq(a, b)
}
