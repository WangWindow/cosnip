#![forbid(unsafe_code)]
//! Platform adapters: clipboard, output paths, atomic file writes.

pub mod clipboard;
pub mod paths;
pub mod pointer;

pub use clipboard::{ClipboardOwner, clipboard_available, clipboard_image};
pub use paths::{atomic_write, default_save_path, pictures_dir};
pub use pointer::{global_pointer, primary_button_down};

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("clipboard is unavailable: {0}")]
    Clipboard(String),
    #[error("cannot determine the Pictures directory")]
    NoPicturesDir,
    #[error("cannot create output directory: {0}")]
    CreateDir(String),
    #[error("cannot write output file: {0}")]
    Write(String),
}
