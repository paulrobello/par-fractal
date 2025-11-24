//! Native platform implementations using filesystem and native dialogs

mod capture;
mod file_dialog;
mod storage;

pub use capture::NativeCapture;
pub use file_dialog::NativeFileDialog;
pub use storage::NativeStorage;
