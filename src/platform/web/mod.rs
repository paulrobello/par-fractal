//! Web platform implementations using browser APIs

mod capture;
mod file_dialog;
mod storage;

pub use capture::WebCapture;
pub use file_dialog::WebFileDialog;
pub use storage::WebStorage;
