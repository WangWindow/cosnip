#![forbid(unsafe_code)]
//! Image operations for the snip document: crops, PNG encoding, and annotation rasterization.

use cosnip_core::{Frame, Rect};
use image::ImageEncoder;

mod annotate;

pub use annotate::render;

#[derive(Debug, thiserror::Error)]
pub enum ImagingError {
    #[error("selection is outside the captured frame")]
    OutsideFrame,
    #[error("invalid selection size")]
    EmptySelection,
    #[error("PNG encode failed: {0}")]
    Encode(String),
    #[error("PNG decode failed: {0}")]
    Decode(String),
}

/// Crop the exact rectangle from `frame`.
pub fn crop(frame: &Frame, rect: Rect) -> Result<Frame, ImagingError> {
    if rect.is_empty() {
        return Err(ImagingError::EmptySelection);
    }
    let bounds = Rect::new(0, 0, frame.width(), frame.height());
    let clamped = rect.clamp_to(bounds).ok_or(ImagingError::OutsideFrame)?;
    if clamped != rect {
        return Err(ImagingError::OutsideFrame);
    }

    let stride = frame.width() as usize * 4;
    let x = rect.x as usize * 4;
    let row_bytes = rect.width as usize * 4;
    let mut out = Vec::with_capacity(row_bytes * rect.height as usize);
    for row in 0..rect.height as usize {
        let start = (rect.y as usize + row) * stride + x;
        out.extend_from_slice(&frame.rgba()[start..start + row_bytes]);
    }
    Frame::new(rect.width, rect.height, out).map_err(|_| ImagingError::EmptySelection)
}

/// Encode a frame as a lossless RGBA PNG.
pub fn encode_png(frame: &Frame) -> Result<Vec<u8>, ImagingError> {
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(
            frame.rgba(),
            frame.width(),
            frame.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| ImagingError::Encode(e.to_string()))?;
    Ok(bytes)
}

/// Decode a PNG into a validated frame (used by tests and paste in later features).
pub fn decode_png(bytes: &[u8]) -> Result<Frame, ImagingError> {
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .map_err(|e| ImagingError::Decode(e.to_string()))?;
    let rgba = image.to_rgba8();
    Frame::new(rgba.width(), rgba.height(), rgba.into_raw())
        .map_err(|e| ImagingError::Decode(e.to_string()))
}

/// Sample a square grid of pixels around `(x, y)` for the magnifier.
#[must_use]
pub fn sample_square(frame: &Frame, x: i32, y: i32, radius: i32) -> Vec<[u8; 4]> {
    let mut out = Vec::with_capacity(((radius * 2 + 1) * (radius * 2 + 1)) as usize);
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            out.push(frame.pixel(x + dx, y + dy).unwrap_or([0, 0, 0, 255]));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use cosnip_core::{Frame, Rect};

    use super::{crop, decode_png, encode_png, sample_square};

    fn frame() -> Frame {
        let mut rgba = Vec::new();
        for y in 0..4u8 {
            for x in 0..4u8 {
                rgba.extend_from_slice(&[x * 10, y * 10, 7, 255]);
            }
        }
        Frame::new(4, 4, rgba).unwrap()
    }

    #[test]
    fn crops_exact_pixels() {
        let frame = frame();
        let cropped = crop(&frame, Rect::new(1, 1, 2, 2)).unwrap();
        assert_eq!(cropped.width(), 2);
        assert_eq!(cropped.height(), 2);
        assert_eq!(cropped.pixel(0, 0), Some([10, 10, 7, 255]));
        assert_eq!(cropped.pixel(1, 1), Some([20, 20, 7, 255]));
    }

    #[test]
    fn rejects_outside_selection() {
        assert!(crop(&frame(), Rect::new(3, 3, 2, 2)).is_err());
    }

    #[test]
    fn png_roundtrip_preserves_pixels() {
        let frame = frame();
        let png = encode_png(&frame).unwrap();
        let decoded = decode_png(&png).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn magnifier_samples_out_of_bounds_as_black() {
        let samples = sample_square(&frame(), 0, 0, 1);
        assert_eq!(samples.len(), 9);
        assert_eq!(samples[0], [0, 0, 0, 255]);
        assert_eq!(samples[4], [0, 0, 7, 255]);
    }
}
