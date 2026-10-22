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
    /// ENH-002: record one parameter change. `pre_change` is the settings
    /// snapshot taken BEFORE the mutation (the UI captures it at the top of
    /// `render`), `post` the mutated params.
    ///
    /// Two invariants make undo/redo actually work (both broken before —
    /// undo used to return the CURRENT state, i.e. a no-op):
    /// - the FIRST entry anchors the pre-change (initial) state, so there is
    ///   always an older state to undo into;
    /// - `history_index` anchors at the LAST entry (`len - 1`), not one past
    ///   it, so `undo`'s `index - 1` lands on the previous state.
    pub(crate) fn record_change(&mut self, pre_change: &RenderSettings, post: &FractalParams) {
        if self.history.is_empty() {
            self.push_snapshot(pre_change.clone());
        }
        self.push_snapshot(post.settings.clone());
    }

    fn push_snapshot(&mut self, settings: RenderSettings) {
        // If we've undone into history, truncate everything AFTER the current
        // position (index anchors AT the current state, so the redo tail
        // starts at index + 1).
        if self.history_index + 1 < self.history.len() {
            self.history.truncate(self.history_index + 1);
        }

        self.history.push_back(HistoryEntry {
            settings,
            timestamp: web_time::Instant::now(),
        });

        // Maintain max history size. ARC-019: VecDeque keeps the
        // Oldest entry at the front, so pop_front is O(1) (vs the
        // previous Vec::remove(0) which shifted every entry).
        if self.history.len() > self.max_history_size {
            self.history.pop_front();
        }
        // Index anchors at the just-pushed tail (the current state), NOT one
        // past it — see `record_change`.
        self.history_index = self.history.len() - 1;

        self.last_saved_settings = Some(self.history.back().unwrap().settings.clone());
    }

    /// Roll the user's authored render settings back one step.
    ///
    /// Returns the previous [`RenderSettings`] snapshot, or `None` if there is
    /// nothing to undo. The caller writes it back into `params.settings`,
    /// leaving `params.lod` and `params.accum` untouched (ARC-015).
    pub(crate) fn undo(&mut self) -> Option<RenderSettings> {
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
    pub(crate) fn redo(&mut self) -> Option<RenderSettings> {
        if self.can_redo() {
            self.history_index += 1;
            Some(self.history[self.history_index].settings.clone())
        } else {
            None
        }
    }

    pub(crate) fn can_undo(&self) -> bool {
        self.history_index > 0 && !self.history.is_empty()
    }

    pub(crate) fn can_redo(&self) -> bool {
        !self.history.is_empty() && self.history_index < self.history.len() - 1
    }
}
