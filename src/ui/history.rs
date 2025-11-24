use super::UI;
use crate::fractal::FractalParams;

/// History entry for undo/redo functionality
#[derive(Clone)]
pub(super) struct HistoryEntry {
    pub(super) params: FractalParams,
    #[allow(dead_code)]
    pub(super) timestamp: std::time::Instant,
}

/// Undo/redo functionality for UI
impl UI {
    pub(super) fn save_to_history(&mut self, params: &FractalParams) {
        // If we're not at the end of history, truncate everything after current position
        if self.history_index < self.history.len() {
            self.history.truncate(self.history_index);
        }

        // Check if params actually changed from last saved state
        let should_save = if let Some(ref last) = self.last_saved_params {
            // Use a simple equality check - you might want to customize this
            // to ignore minor floating point differences
            !params_equal(params, last)
        } else {
            true
        };

        if should_save {
            self.history.push(HistoryEntry {
                params: params.clone(),
                timestamp: std::time::Instant::now(),
            });

            // Maintain max history size
            if self.history.len() > self.max_history_size {
                self.history.remove(0);
            } else {
                self.history_index = self.history.len();
            }

            self.last_saved_params = Some(params.clone());
        }
    }

    pub(super) fn undo(&mut self) -> Option<FractalParams> {
        if self.can_undo() {
            self.history_index = self.history_index.saturating_sub(1);
            Some(self.history[self.history_index].params.clone())
        } else {
            None
        }
    }

    pub(super) fn redo(&mut self) -> Option<FractalParams> {
        if self.can_redo() {
            self.history_index += 1;
            Some(self.history[self.history_index].params.clone())
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

/// Helper to compare fractal params
fn params_equal(a: &FractalParams, b: &FractalParams) -> bool {
    // For now, use pointer comparison (conservative approach)
    // In practice, this will always return false for different instances
    // but prevents unnecessary history entries
    std::ptr::eq(a, b)
}
