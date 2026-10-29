//! A validated RGBA8 frame.

use std::fmt;

/// An RGBA8, row-major, top-left-origin image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameError {
    Empty,
    InvalidLength { expected: usize, actual: usize },
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "frame dimensions must be non-zero"),
            Self::InvalidLength { expected, actual } => {
                write!(f, "invalid frame length: expected {expected}, got {actual}")
            }
        }
    }
}

impl std::error::Error for FrameError {}

impl Frame {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, FrameError> {
        if width == 0 || height == 0 {
            return Err(FrameError::Empty);
        }
        let expected = width as usize * height as usize * 4;
        if rgba.len() != expected {
            return Err(FrameError::InvalidLength {
                expected,
                actual: rgba.len(),
            });
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    #[must_use]
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    #[must_use]
    pub fn into_rgba(self) -> Vec<u8> {
        self.rgba
    }

    /// Return the RGBA pixel at `(x, y)`, or `None` when out of bounds.
    #[must_use]
    pub fn pixel(&self, x: i32, y: i32) -> Option<[u8; 4]> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        let index = (y as usize * self.width as usize + x as usize) * 4;
        Some([
            self.rgba[index],
            self.rgba[index + 1],
            self.rgba[index + 2],
            self.rgba[index + 3],
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::{Frame, FrameError};

    #[test]
    fn validates_length() {
        assert_eq!(
            Frame::new(2, 2, vec![0; 15]),
            Err(FrameError::InvalidLength {
                expected: 16,
                actual: 15
            })
        );
        assert!(Frame::new(2, 2, vec![0; 16]).is_ok());
    }

    #[test]
    fn reads_pixels() {
        let mut data = vec![0u8; 2 * 2 * 4];
        data[0..4].copy_from_slice(&[1, 2, 3, 4]);
        let frame = Frame::new(2, 2, data).unwrap();
        assert_eq!(frame.pixel(0, 0), Some([1, 2, 3, 4]));
        assert_eq!(frame.pixel(-1, 0), None);
    }
}
