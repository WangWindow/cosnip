#![forbid(unsafe_code)]
//! Platform adapters: clipboard, output paths, atomic file writes.

pub mod clipboard;
pub mod config;
pub mod paths;
pub mod pointer;
pub mod shell_pin;
pub mod tray;

pub use clipboard::{ClipboardOwner, clipboard_available, clipboard_image};
pub use config::{
    Config, ConfigError, CursorMode, DefaultOutput, PinBackend, config_path, format_file_name,
};
pub use paths::{
    atomic_write, autostart_dir, cache_dir, config_dir, data_dir, default_save_path, pictures_dir,
    runtime_dir, save_path,
};
pub use pointer::{global_pointer, primary_button_down};
pub use tray::{
    autostart_enabled, host_available as tray_host_available, install_autostart, remove_autostart,
    run as run_tray,
};

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
