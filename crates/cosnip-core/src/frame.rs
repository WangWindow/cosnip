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

    /// Resize the frame with bilinear interpolation.
    pub fn scaled(&self, width: u32, height: u32) -> Result<Self, FrameError> {
        if width == 0 || height == 0 {
            return Err(FrameError::Empty);
        }
        if width == self.width && height == self.height {
            return Ok(self.clone());
        }

        let mut rgba = vec![0u8; width as usize * height as usize * 4];
        let x_scale = self.width as f64 / width as f64;
        let y_scale = self.height as f64 / height as f64;
        let max_x = self.width.saturating_sub(1) as f64;
        let max_y = self.height.saturating_sub(1) as f64;
        for y in 0..height as usize {
            let source_y = ((y as f64 + 0.5) * y_scale - 0.5).clamp(0.0, max_y);
            let y0 = source_y.floor() as usize;
            let y1 = (y0 + 1).min(self.height as usize - 1);
            let fy = source_y - y0 as f64;
            for x in 0..width as usize {
                let source_x = ((x as f64 + 0.5) * x_scale - 0.5).clamp(0.0, max_x);
                let x0 = source_x.floor() as usize;
                let x1 = (x0 + 1).min(self.width as usize - 1);
                let fx = source_x - x0 as f64;
                let target = (y * width as usize + x) * 4;
                for channel in 0..4 {
                    let top_left = self.pixel_at(x0, y0, channel);
                    let top_right = self.pixel_at(x1, y0, channel);
                    let bottom_left = self.pixel_at(x0, y1, channel);
                    let bottom_right = self.pixel_at(x1, y1, channel);
                    let top = f64::from(top_left) * (1.0 - fx) + f64::from(top_right) * fx;
                    let bottom = f64::from(bottom_left) * (1.0 - fx) + f64::from(bottom_right) * fx;
                    rgba[target + channel] =
                        (top * (1.0 - fy) + bottom * fy).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
        Self::new(width, height, rgba)
    }

    /// Copy `source` into this frame at `(x, y)`, clipping to the destination bounds.
    pub fn blit(&mut self, source: &Self, x: i32, y: i32) {
        let source_left = if x < 0 { x.unsigned_abs() } else { 0 };
        let source_top = if y < 0 { y.unsigned_abs() } else { 0 };
        if source_left >= source.width || source_top >= source.height {
            return;
        }
        let width = (source.width - source_left).min(self.width.saturating_sub(x.max(0) as u32));
        let height = (source.height - source_top).min(self.height.saturating_sub(y.max(0) as u32));
        if width == 0 || height == 0 {
            return;
        }
        let destination_x = x.max(0) as usize;
        let destination_y = y.max(0) as usize;
        let destination_stride = self.width as usize * 4;
        let source_stride = source.width as usize * 4;
        for row in 0..height as usize {
            let source_start =
                (source_top as usize + row) * source_stride + source_left as usize * 4;
            let destination_start = (destination_y + row) * destination_stride + destination_x * 4;
            let bytes = width as usize * 4;
            self.rgba[destination_start..destination_start + bytes]
                .copy_from_slice(&source.rgba[source_start..source_start + bytes]);
        }
    }

    fn pixel_at(&self, x: usize, y: usize, channel: usize) -> u8 {
        self.rgba[(y * self.width as usize + x) * 4 + channel]
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
