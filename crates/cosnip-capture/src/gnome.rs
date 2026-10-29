//! GNOME 49 single-frame Screencast fallback.
//!
//! The Shell/portal screenshot APIs are denied on the target. The Screencast service is allowed;
//! we keep one D-Bus connection alive, record briefly, stop, and decode one frame.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use cosnip_core::Frame;
use zbus::block_on;

use crate::{
    CaptureBackend, CaptureBackendKind, CaptureError, desktop_is_gnome, ffmpeg_available,
    runtime_dir, session_is_wayland,
};

#[derive(Debug, Default)]
pub struct GnomeScreencast;

impl CaptureBackend for GnomeScreencast {
    fn kind(&self) -> CaptureBackendKind {
        CaptureBackendKind::GnomeScreencast
    }

    fn is_available(&self) -> Result<(), CaptureError> {
        if !session_is_wayland() || !desktop_is_gnome() {
            return Err(CaptureError::Unsupported(
                "GNOME Screencast requires a GNOME Wayland session".into(),
            ));
        }
        if std::env::var_os("DISPLAY").is_none() {
            return Err(CaptureError::Unavailable(
                "XWayland is required for the overlay; DISPLAY is not set".into(),
            ));
        }
        if !ffmpeg_available() {
            return Err(CaptureError::Unavailable(
                "ffmpeg is required for the GNOME capture fallback".into(),
            ));
        }
        Ok(())
    }

    fn capture(&self) -> Result<Frame, CaptureError> {
        self.is_available()?;
        let temp = tempfile::Builder::new()
            .prefix("cosnip-capture-")
            .tempdir_in(runtime_dir())
            .map_err(|e| CaptureError::Unavailable(format!("cannot create temp dir: {e}")))?;
        let stem = temp.path().join("capture");
        let stem_str = stem
            .to_str()
            .ok_or_else(|| CaptureError::Unavailable("runtime path is not UTF-8".into()))?;

        let conn =
            block_on(zbus::Connection::session()).map_err(|e| CaptureError::Dbus(e.to_string()))?;

        let options: HashMap<String, zbus::Variant<'_>> = HashMap::from([
            ("draw-cursor".into(), zbus::Variant::from(false)),
            ("framerate".into(), zbus::Variant::from(30i32)),
        ]);
        let body = (stem_str, options);
        let reply = block_on(conn.call_method(
            Some("org.gnome.Shell.Screencast"),
            "/org/gnome/Shell/Screencast",
            Some("org.gnome.Shell.Screencast"),
            "Screencast",
            &body,
        ))
        .map_err(|e| CaptureError::Dbus(e.to_string()))?;
        let (ok, file): (bool, String) = reply
            .body()
            .deserialize()
            .map_err(|e| CaptureError::Dbus(e.to_string()))?;
        if !ok {
            return Err(CaptureError::Dbus(
                "GNOME Screencast refused to start".into(),
            ));
        }

        std::thread::sleep(Duration::from_millis(500));

        block_on(conn.call_method(
            Some("org.gnome.Shell.Screencast"),
            "/org/gnome/Shell/Screencast",
            Some("org.gnome.Shell.Screencast"),
            "StopScreencast",
            &(),
        ))
        .map_err(|e| CaptureError::Dbus(format!("stop failed: {e}")))?;

        let file = if file.is_empty() {
            stem.with_extension("mp4")
        } else {
            std::path::PathBuf::from(file)
        };
        if !file.is_file() {
            return Err(CaptureError::Decode(format!(
                "recording was not created: {}",
                file.display()
            )));
        }

        let (width, height) = screen_dimensions(&file)?;
        let output = std::process::Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-i",
                file.to_str().unwrap_or_default(),
                "-frames:v",
                "1",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgba",
                "-",
            ])
            .output()
            .map_err(|e| CaptureError::Decode(format!("cannot run ffmpeg: {e}")))?;
        if !output.status.success() {
            return Err(CaptureError::Decode(
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }
        let frame = Frame::new(width, height, output.stdout)
            .map_err(|e| CaptureError::InvalidFrame(e.to_string()))?;
        Ok(frame)
    }
}

fn screen_dimensions(path: &Path) -> Result<(u32, u32), CaptureError> {
    if let Ok(info) = crate::screen_info()
        && info.width > 0
        && info.height > 0
    {
        return Ok((info.width, info.height));
    }
    let output = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0:s=x",
            path.to_str().unwrap_or_default(),
        ])
        .output()
        .map_err(|e| CaptureError::Decode(format!("cannot run ffprobe: {e}")))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let (w, h) = text
        .trim()
        .split_once('x')
        .ok_or_else(|| CaptureError::Decode("ffprobe did not return dimensions".into()))?;
    let width = w
        .parse()
        .map_err(|_| CaptureError::Decode("invalid ffprobe width".into()))?;
    let height = h
        .parse()
        .map_err(|_| CaptureError::Decode("invalid ffprobe height".into()))?;
    Ok((width, height))
}
