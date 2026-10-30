//! Native pin transport for the optional Cosnip GNOME Shell extension.
//!
//! GNOME does not support `wlr-layer-shell` and does not expose an
//! always-on-top hint for `xdg_toplevel`, so a normal Wayland window cannot be
//! used for pins. The extension owns a Shell actor instead and exposes this
//! small D-Bus interface. The image is passed as RGBA bytes so the Shell can
//! upload the texture directly, without an XWayland surface or a resampling
//! pass.

use cosnip_core::Frame;

const SERVICE: &str = "org.cosnip.Pins";
const PATH: &str = "/org/cosnip/Pins";
const INTERFACE: &str = "org.cosnip.Pins";

#[derive(Debug, thiserror::Error)]
pub enum ShellPinError {
    #[error("cannot connect to the session bus: {0}")]
    Bus(String),
    #[error("Cosnip Pins extension is unavailable: {0}")]
    Call(String),
}

/// Whether the extension is currently exporting its D-Bus interface.
#[must_use]
pub fn available() -> bool {
    zbus::block_on(async {
        let connection = zbus::Connection::session().await.ok()?;
        let proxy = zbus::Proxy::new(&connection, SERVICE, PATH, INTERFACE)
            .await
            .ok()?;
        let version: String = proxy.call("Ping", &()).await.ok()?;
        Some(version.starts_with("cosnip-pins/"))
    })
    .unwrap_or(false)
}

/// Create a pin actor in GNOME Shell.
///
/// `x` and `y` are stage (logical) coordinates. `width` and `height` are the
/// logical display size. `image` supplies the native RGBA pixels.
pub fn pin(image: &Frame, x: f32, y: f32, width: f32, height: f32) -> Result<u32, ShellPinError> {
    let connection = zbus::block_on(zbus::Connection::session())
        .map_err(|error| ShellPinError::Bus(error.to_string()))?;
    zbus::block_on(async {
        let proxy = zbus::Proxy::new(&connection, SERVICE, PATH, INTERFACE)
            .await
            .map_err(|error| ShellPinError::Call(error.to_string()))?;
        let id: u32 = proxy
            .call(
                "Pin",
                &(
                    image.rgba(),
                    x.round() as i32,
                    y.round() as i32,
                    dimension(width),
                    dimension(height),
                    image.width() as i32,
                    image.height() as i32,
                ),
            )
            .await
            .map_err(|error| ShellPinError::Call(error.to_string()))?;
        Ok(id)
    })
}

fn dimension(value: f32) -> i32 {
    if value <= 0.0 {
        0
    } else {
        value.round().max(1.0) as i32
    }
}
