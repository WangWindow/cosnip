//! Clipboard image adapter.

use cosnip_core::Frame;

use crate::PlatformError;

pub fn clipboard_available() -> bool {
    arboard::Clipboard::new().is_ok()
}

/// Read the current clipboard image as a validated RGBA frame.
pub fn clipboard_image() -> Result<Frame, PlatformError> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| PlatformError::Clipboard(e.to_string()))?;
    let image = clipboard.get_image().map_err(|error| match error {
        arboard::Error::ContentNotAvailable => {
            PlatformError::Clipboard("clipboard does not contain an image".into())
        }
        other => PlatformError::Clipboard(other.to_string()),
    })?;
    frame_from_image_data(image)
}

fn frame_from_image_data(image: arboard::ImageData<'_>) -> Result<Frame, PlatformError> {
    let width = u32::try_from(image.width)
        .map_err(|_| PlatformError::Clipboard("clipboard image width is too large".into()))?;
    let height = u32::try_from(image.height)
        .map_err(|_| PlatformError::Clipboard("clipboard image height is too large".into()))?;
    Frame::new(width, height, image.bytes.into_owned())
        .map_err(|error| PlatformError::Clipboard(format!("invalid clipboard image: {error}")))
}

/// An owning clipboard handle. Keep it alive while the image must remain available.
pub struct ClipboardOwner {
    clipboard: arboard::Clipboard,
}

impl ClipboardOwner {
    pub fn new() -> Result<Self, PlatformError> {
        let clipboard =
            arboard::Clipboard::new().map_err(|e| PlatformError::Clipboard(e.to_string()))?;
        Ok(Self { clipboard })
    }

    pub fn set_image(&mut self, frame: &Frame) -> Result<(), PlatformError> {
        self.clipboard
            .set_image(arboard::ImageData {
                width: frame.width() as usize,
                height: frame.height() as usize,
                bytes: std::borrow::Cow::Borrowed(frame.rgba()),
            })
            .map_err(|e| PlatformError::Clipboard(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::frame_from_image_data;

    #[test]
    fn converts_clipboard_image_data() {
        let image = arboard::ImageData {
            width: 2,
            height: 1,
            bytes: Cow::Owned(vec![1, 2, 3, 255, 4, 5, 6, 128]),
        };
        let frame = frame_from_image_data(image).unwrap();
        assert_eq!(frame.width(), 2);
        assert_eq!(frame.height(), 1);
        assert_eq!(frame.pixel(0, 0), Some([1, 2, 3, 255]));
        assert_eq!(frame.pixel(1, 0), Some([4, 5, 6, 128]));
    }

    #[test]
    fn rejects_short_clipboard_image_data() {
        let image = arboard::ImageData {
            width: 2,
            height: 2,
            bytes: Cow::Owned(vec![0; 4]),
        };
        assert!(frame_from_image_data(image).is_err());
    }
}
