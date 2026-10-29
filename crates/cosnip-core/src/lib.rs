#![forbid(unsafe_code)]
//! Pure domain types for Cosnip.
//!
//! This crate has no OS, UI, or windowing dependencies.

pub mod frame;
pub mod geometry;
pub mod output;
pub mod selection;

pub use frame::{Frame, FrameError};
pub use geometry::{Point, Rect, Size};
pub use output::OutputRequest;
pub use selection::Selection;
