#![forbid(unsafe_code)]
//! Desktop capture backends.
//!
//! X11 capture is for Xorg sessions. GNOME Wayland uses Mutter ScreenCast + PipeWire raw frames
//! (lossless); if that path fails it falls back to a single-frame
//! `org.gnome.Shell.Screencast` recording because the Shell/portal screenshot APIs are denied.

pub mod gnome;
#[cfg(feature = "pipewire")]
pub mod mutter;
pub mod x11;

use std::path::PathBuf;

use cosnip_core::Frame;
use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as _;
use x11rb::protocol::xproto::ConnectionExt as _;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureBackendKind {
    X11,
    GnomeScreencast,
    #[cfg(feature = "pipewire")]
    GnomePipewire,
}

impl CaptureBackendKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::X11 => "x11",
            Self::GnomeScreencast => "gnome-screencast",
            #[cfg(feature = "pipewire")]
            Self::GnomePipewire => "gnome-pipewire",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScreenInfo {
    pub width: u32,
    pub height: u32,
    pub count: usize,
    pub scale: f64,
    pub name: String,
}

/// One monitor in captured-desktop (X11 root) coordinates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitorInfo {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
    pub name: String,
}

/// Enumerate active RandR monitors. X11 and XWayland both expose the desktop layout here.
pub fn monitors() -> Result<Vec<MonitorInfo>, CaptureError> {
    let (conn, screen_num) = x11rb::connect(None).map_err(|e| CaptureError::X11(e.to_string()))?;
    let screen = &conn.setup().roots[screen_num];
    let reply = conn
        .randr_get_monitors(screen.root, true)
        .map_err(|e| CaptureError::X11(e.to_string()))?
        .reply()
        .map_err(|e| CaptureError::X11(e.to_string()))?;
    let mut monitors = reply
        .monitors
        .iter()
        .enumerate()
        .map(|(index, monitor)| MonitorInfo {
            x: i32::from(monitor.x),
            y: i32::from(monitor.y),
            width: u32::from(monitor.width),
            height: u32::from(monitor.height),
            primary: monitor.primary,
            name: conn
                .get_atom_name(monitor.name)
                .ok()
                .and_then(|cookie| cookie.reply().ok())
                .map(|reply| String::from_utf8_lossy(&reply.name).into_owned())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| format!("monitor-{index}")),
        })
        .collect::<Vec<_>>();
    if monitors.is_empty() {
        monitors.push(MonitorInfo {
            x: 0,
            y: 0,
            width: u32::from(screen.width_in_pixels),
            height: u32::from(screen.height_in_pixels),
            primary: true,
            name: format!("{}x{}", screen.width_in_pixels, screen.height_in_pixels),
        });
    }
    Ok(monitors)
}

pub trait CaptureBackend: Send + Sync {
    fn kind(&self) -> CaptureBackendKind;
    fn is_available(&self) -> Result<(), CaptureError>;
    fn capture(&self) -> Result<Frame, CaptureError>;
}

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("no display session found")]
    NoDisplay,
    #[error("capture backend is not supported in this session: {0}")]
    Unsupported(String),
    #[error("capture backend is unavailable: {0}")]
    Unavailable(String),
    #[error("capture timed out")]
    Timeout,
    #[error("capture decode failed: {0}")]
    Decode(String),
    #[error("X11 capture failed: {0}")]
    X11(String),
    #[error("D-Bus capture failed: {0}")]
    Dbus(String),
    #[error("temporary capture cleanup failed: {0}")]
    Cleanup(String),
    #[error("capture produced an invalid frame: {0}")]
    InvalidFrame(String),
}

#[must_use]
pub fn session_is_wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v.eq_ignore_ascii_case("wayland"))
        || std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty())
}

#[must_use]
pub fn desktop_is_gnome() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|v| v.to_ascii_lowercase())
        .is_ok_and(|v| v.contains("gnome") || v.contains("unity"))
}

pub fn detect_backend() -> Result<CaptureBackendKind, CaptureError> {
    if session_is_wayland() {
        if desktop_is_gnome() {
            #[cfg(feature = "pipewire")]
            {
                Ok(CaptureBackendKind::GnomePipewire)
            }
            #[cfg(not(feature = "pipewire"))]
            {
                Ok(CaptureBackendKind::GnomeScreencast)
            }
        } else {
            Err(CaptureError::Unsupported(
                "Wayland desktop is not GNOME; only GNOME Screencast and Xorg are supported in this MVP"
                    .into(),
            ))
        }
    } else if std::env::var_os("DISPLAY").is_some() {
        Ok(CaptureBackendKind::X11)
    } else {
        Err(CaptureError::NoDisplay)
    }
}

#[must_use]
pub fn backend(kind: CaptureBackendKind) -> Box<dyn CaptureBackend> {
    match kind {
        CaptureBackendKind::X11 => Box::new(x11::X11Backend),
        CaptureBackendKind::GnomeScreencast => Box::new(gnome::GnomeScreencast),
        #[cfg(feature = "pipewire")]
        CaptureBackendKind::GnomePipewire => Box::new(mutter::MutterPipewire),
    }
}

pub fn screen_info() -> Result<ScreenInfo, CaptureError> {
    let (conn, screen_num) = x11rb::connect(None).map_err(|e| CaptureError::X11(e.to_string()))?;
    let setup = conn.setup();
    let screen = &setup.roots[screen_num];
    let count = conn
        .randr_get_monitors(screen.root, true)
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .map(|reply| reply.monitors.len().max(1))
        .unwrap_or(1);
    Ok(ScreenInfo {
        width: u32::from(screen.width_in_pixels),
        height: u32::from(screen.height_in_pixels),
        count,
        scale: 1.0,
        name: format!("{}x{}", screen.width_in_pixels, screen.height_in_pixels),
    })
}

#[must_use]
pub fn ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

pub(crate) fn runtime_dir() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(std::env::temp_dir)
}
