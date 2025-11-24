#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

/// Helper function to compare two monitors for equality
pub(super) fn monitors_equal(
    a: &winit::monitor::MonitorHandle,
    b: &winit::monitor::MonitorHandle,
) -> bool {
    // Compare based on name and size
    let a_name = a.name();
    let b_name = b.name();
    let a_size = a.size();
    let b_size = b.size();

    a_name == b_name && a_size == b_size
}
