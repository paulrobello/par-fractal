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
