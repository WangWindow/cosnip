# Quickstart: Desktop Shell

## Settings

```bash
target/release/cosnip settings
```

The config file is `$XDG_CONFIG_HOME/cosnip/config.toml` (default
`~/.config/cosnip/config.toml`). Use `--config PATH` to override it.

```toml
[general]
default_output = "copy"        # copy | save | pin
save_dir = ""                  # empty = Pictures
filename = "Cosnip_%Y-%m-%d_%H-%M-%S.png"
native_output = true           # native pixels on mixed-DPI layouts

[overlay]
cursor_mode = "auto"           # auto | always | hidden
magnifier = true

[annotations]
color = "#FF4040"
line_width = 3
font_size = 20
text_shadow = true

[pin]
zoom_step = 0.002
```

## GNOME shortcuts

Open GNOME Settings → Keyboard → Custom Shortcuts and bind:

```text
Snip:  /path/to/cosnip snip   F1
Paste: /path/to/cosnip paste  F3
```

The settings window shows the absolute commands and has a button to open the GNOME panel.

## Tray

```bash
target/release/cosnip tray
```

- Menu: Snip, Paste, Settings, Quit.
- Requires StatusNotifierItem; on GNOME enable the AppIndicator extension.
- Without a host, the process stays silent and no icon is shown.
- Enable "Start the tray at login" in Settings to write
  `$XDG_CONFIG_HOME/autostart/cosnip-tray.desktop`.

## Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
target/release/cosnip --doctor
```

`--doctor` reports the config path and tray host availability.
