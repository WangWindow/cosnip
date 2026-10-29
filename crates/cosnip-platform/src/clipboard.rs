//! Clipboard image adapter.

use cosnip_core::Frame;

use crate::PlatformError;

pub fn clipboard_available() -> bool {
    arboard::Clipboard::new().is_ok()
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
