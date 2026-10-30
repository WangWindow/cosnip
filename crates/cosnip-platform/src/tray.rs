//! Lightweight StatusNotifierItem tray and XDG autostart helper.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use ksni::blocking::TrayMethods;
use ksni::menu::StandardItem;
use ksni::{Icon, MenuItem, ToolTip, Tray};

use crate::paths;

const TRAYS_DBUS_NAME: &str = "org.cosnip.Tray";

pub fn run(config: Option<PathBuf>) -> Result<(), String> {
    let _name = match acquire_name() {
        Ok(connection) => connection,
        Err(error) if error.is_empty() => return Ok(()),
        Err(error) => return Err(error),
    };
    let tray = CosnipTray {
        exe: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("cosnip")),
        config,
    };
    let handle = match tray.spawn() {
        Ok(handle) => handle,
        Err(_) => {
            // GNOME without the AppIndicator extension has no SNI host. This is not an error:
            // the tray icon simply stays hidden.
            return Ok(());
        }
    };
    while !handle.is_closed() {
        std::thread::sleep(Duration::from_millis(500));
    }
    Ok(())
}

fn acquire_name() -> Result<zbus::Connection, String> {
    let connection = zbus::block_on(zbus::Connection::session())
        .map_err(|error| format!("cannot connect to the session bus: {error}"))?;
    match zbus::block_on(
        connection
            .request_name_with_flags(TRAYS_DBUS_NAME, zbus::fdo::RequestNameFlags::DO_NOT_QUEUE),
    ) {
        Ok(_) => Ok(connection),
        Err(zbus::Error::NameTaken) => Err(String::new()),
        Err(error) => Err(format!("cannot acquire {TRAYS_DBUS_NAME}: {error}")),
    }
}

struct CosnipTray {
    exe: PathBuf,
    config: Option<PathBuf>,
}

impl Tray for CosnipTray {
    const MENU_ON_ACTIVATE: bool = true;

    fn id(&self) -> String {
        "cosnip".into()
    }

    fn title(&self) -> String {
        "Cosnip".into()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![tray_icon()]
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: "Cosnip".into(),
            description: "Snip or paste an image".into(),
            ..ToolTip::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            MenuItem::Standard(StandardItem {
                label: "Snip".into(),
                activate: Box::new(|tray| spawn(tray, "snip")),
                ..StandardItem::default()
            }),
            MenuItem::Standard(StandardItem {
                label: "Paste".into(),
                activate: Box::new(|tray| spawn(tray, "paste")),
                ..StandardItem::default()
            }),
            MenuItem::Standard(StandardItem {
                label: "Settings…".into(),
                activate: Box::new(|tray| spawn(tray, "settings")),
                ..StandardItem::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(StandardItem {
                label: "Quit".into(),
                activate: Box::new(|_| std::process::exit(0)),
                ..StandardItem::default()
            }),
        ]
    }
}

fn spawn(tray: &CosnipTray, subcommand: &str) {
    let mut command = Command::new(&tray.exe);
    if let Some(config) = &tray.config {
        command.arg("--config").arg(config);
    }
    if let Err(error) = command.arg(subcommand).spawn() {
        eprintln!("cosnip: tray action failed: {error}");
    }
}

fn tray_icon() -> Icon {
    const SIZE: i32 = 22;
    let mut data = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let index = ((y * SIZE + x) * 4) as usize;
            let dx = x as f32 - 11.0;
            let dy = y as f32 - 11.0;
            let circle = dx * dx + dy * dy <= 10.0 * 10.0;
            // A simple original scissors-like mark: a diagonal blade and two handles.
            let blade = (x + y - 21).abs() <= 1 && circle;
            let handle_left = (x - 7).pow(2) + (y - 8).pow(2) <= 4;
            let handle_right = (x - 15).pow(2) + (y - 8).pow(2) <= 4;
            let mark = blade || handle_left || handle_right;
            let (r, g, b, a) = if mark {
                (255, 255, 255, 255)
            } else if circle {
                (37, 99, 235, 255)
            } else {
                (0, 0, 0, 0)
            };
            // StatusNotifierItem uses ARGB32 network byte order.
            data[index..index + 4].copy_from_slice(&[a, r, g, b]);
        }
    }
    Icon {
        width: SIZE,
        height: SIZE,
        data,
    }
}

pub fn autostart_path() -> PathBuf {
    paths::autostart_dir().join("cosnip-tray.desktop")
}

#[must_use]
pub fn autostart_enabled() -> bool {
    autostart_path().is_file()
}

pub fn install_autostart(config: Option<&Path>) -> Result<PathBuf, String> {
    let path = autostart_path();
    let parent = path
        .parent()
        .ok_or_else(|| "autostart path has no directory".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    write_autostart(&path, config)?;
    Ok(path)
}

pub fn remove_autostart() -> Result<(), String> {
    let path = autostart_path();
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("cannot remove {}: {error}", path.display())),
    }
}

#[must_use]
pub fn host_available() -> bool {
    zbus::block_on(async {
        let Ok(connection) = zbus::Connection::session().await else {
            return false;
        };
        let Ok(reply) = connection
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "NameHasOwner",
                &("org.kde.StatusNotifierWatcher",),
            )
            .await
        else {
            return false;
        };
        reply.body().deserialize::<bool>().unwrap_or(false)
    })
}

fn write_autostart(path: &Path, config: Option<&Path>) -> Result<(), String> {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("cosnip"));
    let config_arg = config
        .map(|path| format!(" --config \"{}\"", path.display()))
        .unwrap_or_default();
    let content = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Cosnip Tray\n\
         Comment=Cosnip tray menu\n\
         Exec=\"{}\"{} tray\n\
         Terminal=false\n\
         X-GNOME-Autostart-enabled=true\n",
        exe.display(),
        config_arg
    );
    std::fs::write(path, content)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::write_autostart;

    #[test]
    fn writes_a_desktop_entry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cosnip-tray.desktop");
        write_autostart(&path, None).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("[Desktop Entry]"));
        assert!(text.contains(" tray"));
        assert!(text.contains("X-GNOME-Autostart-enabled=true"));
    }
}
