//! Native X11/Xorg capture.

use cosnip_core::Frame;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt as _, ImageFormat};

use crate::{CaptureBackend, CaptureBackendKind, CaptureError, session_is_wayland};

#[derive(Debug, Default)]
pub struct X11Backend;

impl CaptureBackend for X11Backend {
    fn kind(&self) -> CaptureBackendKind {
        CaptureBackendKind::X11
    }

    fn is_available(&self) -> Result<(), CaptureError> {
        if session_is_wayland() {
            return Err(CaptureError::Unsupported(
                "X11 capture cannot see native Wayland surfaces; use the GNOME backend".into(),
            ));
        }
        if std::env::var_os("DISPLAY").is_none() {
            return Err(CaptureError::NoDisplay);
        }
        Ok(())
    }

    fn capture(&self) -> Result<Frame, CaptureError> {
        self.is_available()?;
        let (conn, screen_num) =
            x11rb::connect(None).map_err(|e| CaptureError::X11(e.to_string()))?;
        let screen = &conn.setup().roots[screen_num];
        let width = screen.width_in_pixels;
        let height = screen.height_in_pixels;
        let reply = conn
            .get_image(ImageFormat::Z_PIXMAP, screen.root, 0, 0, width, height, !0)
            .map_err(|e| CaptureError::X11(e.to_string()))?
            .reply()
            .map_err(|e| CaptureError::X11(e.to_string()))?;

        let expected = width as usize * height as usize * 4;
        if reply.data.len() != expected {
            return Err(CaptureError::X11(format!(
                "unexpected image length: expected {expected}, got {}",
                reply.data.len()
            )));
        }
        let mut rgba = Vec::with_capacity(expected);
        for px in reply.data.as_chunks::<4>().0 {
            rgba.extend_from_slice(&[px[2], px[1], px[0], 255]);
        }
        let frame = Frame::new(u32::from(width), u32::from(height), rgba)
            .map_err(|e| CaptureError::InvalidFrame(e.to_string()))?;
        Ok(frame)
    }
}
