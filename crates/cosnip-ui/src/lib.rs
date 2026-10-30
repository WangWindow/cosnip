#![forbid(unsafe_code)]
//! eframe-based overlay and pin UI.

mod app;
mod icons;
mod settings;

pub use app::{Outcome, UiOptions, UiStyle, run, run_pin};
pub use settings::{SettingsOutcome, run_settings};
