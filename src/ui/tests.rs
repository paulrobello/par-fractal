use super::*;

#[test]
fn test_ui_creation() {
    let ui = UI::new();
    assert!(ui.show_ui);
}

#[test]
fn test_ui_default() {
    let ui = UI::default();
    assert!(ui.show_ui);
}

#[test]
fn test_ui_toggle() {
    let mut ui = UI::new();
    assert!(ui.show_ui);

    ui.show_ui = false;
    assert!(!ui.show_ui);

    ui.show_ui = true;
    assert!(ui.show_ui);
}

#[test]
fn test_ui_initial_state() {
    let ui = UI::new();
    assert!(ui.show_ui);
}

/// ENH-002: the history that backs the keyboard shortcuts round-trips a
/// parameter change. `record_change` anchors the pre-change snapshot, so the
/// FIRST undo already restores it (this used to be a no-op: the old
/// `history_index` anchored one past the tail and there was no baseline).
#[test]
fn undo_redo_roundtrip_restores_settings() {
    let mut ui = UI::new();
    let mut params = FractalParams::default();
    assert_eq!(params.settings.max_iterations, 80);

    // One change, recorded the way UI::render records it.
    let pre_change = params.settings.clone();
    params.settings.max_iterations = 500;
    ui.record_change(&pre_change, &params);

    assert!(ui.can_undo());
    let restored = ui.undo().expect("undo must return a snapshot");
    assert_eq!(
        restored.max_iterations, 80,
        "undo restores the pre-change state"
    );

    assert!(ui.can_redo());
    let redone = ui.redo().expect("redo must return a snapshot");
    assert_eq!(
        redone.max_iterations, 500,
        "redo reapplies the undone change"
    );

    // Undo past the start returns None.
    assert!(ui.undo().is_some());
    assert!(ui.undo().is_none());
}
