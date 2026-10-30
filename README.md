# Cosnip

Cosnip is a lightweight Linux snipping and pinning tool written in Rust. Select a region,
annotate it if needed, then copy it, save it as PNG, or pin it on screen. The workflow is
inspired by Snipaste, but Cosnip uses original code and assets and is not affiliated with it.

Supports X11 and GNOME Wayland, on aarch64 and x86_64.

## Features

- Full-screen selection overlay with multi-monitor and HiDPI support.
- Compact Snipaste-style toolbar: shapes, arrow, pencil, highlighter, mosaic, text, eraser,
  undo/redo, cancel, pin, save, and copy. Right-click a drawing tool for color and width.
- Annotations: rectangle, ellipse, arrow, line, pencil, highlighter, text, number, mosaic,
  blur, magnifier, and eraser, with undo/redo. Mosaic, blur, and eraser are freehand brushes;
  the eraser removes annotation pixels and never alters the underlying screenshot.
- Copy to clipboard, save to PNG, or pin the selection.
- Paste a clipboard image or a PNG file as a pin. Pins can be moved, zoomed, and closed.
- Native-resolution save/pin for selections on scaled monitors, avoiding upscaling blur.
- Optional GNOME Shell pin backend for sharp, always-on-top pins without XWayland.
- Snipaste-style tabbed preferences dialog (General, Interface, Snip, Pin, Output, Controls,
  About) with automatic saving, and a TOML configuration file.
- Lightweight StatusNotifierItem tray.

## Requirements

- Linux with one of:
  - X11/Xorg
  - GNOME 50+ Wayland with PipeWire
- Rust 1.95 or newer, plus a C toolchain, to build from source.
- PipeWire development headers and LLVM/Clang at build time.
- Optional: `ffmpeg` for the H.264 fallback on GNOME. Not needed for the normal lossless path.
- Optional: `xdg-desktop-portal` for the settings folder picker.
- Optional: a StatusNotifierItem host for the tray. On GNOME, enable the AppIndicator/KStatusNotifier
  extension.

Build dependencies on Debian/Ubuntu:

```bash
sudo apt install build-essential pkg-config libpipewire-0.3-dev libspa-0.2-dev \
  libclang-dev libllvm-dev clang
```

Package names vary by distribution. On Fedora, use `gcc pkgconf pipewire-devel clang llvm`;
on Arch, use `base-devel pkgconf pipewire clang llvm`.

## Install

Build and install into `~/.cargo/bin`:

```bash
git clone https://github.com/WangWindow/cosnip.git
cd cosnip
cargo install --path crates/cosnip-app --release
```

Make sure `~/.cargo/bin` is in your `PATH`, then check the installation:

```bash
cosnip --doctor
```

Alternatively, build and copy the binary anywhere you like:

```bash
cargo build --release
install -Dm755 target/release/cosnip ~/.local/bin/cosnip
```

## Quick start

```bash
cosnip snip     # drag a region, then press Enter to copy it
cosnip paste    # pin the current clipboard image
```

## Usage

```text
cosnip snip [OPTIONS]      Capture the desktop and select a region
cosnip paste [--file PATH] Pin a clipboard image or a PNG file
cosnip settings            Open the settings window
cosnip tray                Run the system tray
cosnip --doctor            Print diagnostics and exit
```

`snip` options:

| Option | Description |
|---|---|
| `--area X Y WIDTH HEIGHT` | Preselect a region. Coordinates are captured-frame pixels. |
| `--full` | Select the whole captured desktop. |
| `--delay MS` | Wait before capturing. |
| `--copy` | Copy the selection. This is the default output. |
| `--no-copy` | Do not copy the selection. |
| `--save PATH` | Save the selection as PNG. |
| `--pin` | Pin the selection on screen. |

Global options:

| Option | Description |
|---|---|
| `--config PATH` | Use a custom TOML configuration file. |
| `--doctor` | Print session, backend, monitor, clipboard, and tray diagnostics. |

Examples:

```bash
# Full screen, copy only
cosnip snip --full --copy

# Preselected region, save only
cosnip snip --area 100 100 500 300 --save ~/Pictures/snip.png

# Select and pin
cosnip snip --pin

# Pin a PNG file at the pointer
cosnip paste --file ~/Pictures/snip.png
```

On GNOME HiDPI sessions, `--area` uses the XWayland desktop coordinate space, not compositor
logical points.

## Controls

Snip overlay:

| Input | Action |
|---|---|
| R / C / A / B / H / M / T / E / V | Rectangle, ellipse, arrow, pencil, highlighter, mosaic, text, eraser, select |
| Drag | Select a region |
| Drag inside the selection | Move it |
| Drag a handle | Resize it |
| Arrow keys | Nudge the selection |
| Shift while dragging | Keep a square |
| Ctrl+A | Select the whole desktop |
| Enter or Ctrl+C | Copy and close |
| Ctrl+S | Save |
| P | Pin |
| Ctrl+Z / Ctrl+Shift+Z | Undo / redo |
| Escape | Cancel the current tool, then the overlay |
| Right-click while annotating | Finish or leave the current tool |
| Right-click a drawing tool | Choose color and line width |

Pin:

| Input | Action |
|---|---|
| Drag | Move the pin |
| Mouse wheel | Zoom |
| Right-click | Close the pin (both backends) |
| Double-click, Escape, or the × button | Close the window pin |

## Configuration

The configuration file is TOML at `$XDG_CONFIG_HOME/cosnip/config.toml`, which defaults to
`~/.config/cosnip/config.toml`. Use `cosnip settings` to edit it, or edit it directly.

```toml
[general]
default_output = "copy"        # copy | save | pin
save_dir = ""                  # empty = Pictures
filename = "Cosnip_%Y-%m-%d_%H-%M-%S.png"
delay_ms = 0
native_output = true

[overlay]
cursor_mode = "auto"           # auto | always | hidden
magnifier = true
show_tips = true

[annotations]
color = "#FF4040"
line_width = 3
font_size = 20
text_shadow = true

[pin]
zoom_step = 0.002
backend = "auto"               # auto | shell | window
```

Command-line flags override the configuration. Missing or invalid values fall back to defaults
with a warning. Changes apply to new `snip` and `paste` processes.

With `native_output = true`, a selection contained within a single monitor whose compositor scale
differs from the XWayland scale is saved or pinned at that monitor's native pixel size. For
example, an 800x600 XWayland selection on a 1.25-scale monitor becomes a 500x375 PNG. The pin
still covers the original on-screen selection rectangle, so it does not shrink visually.

`pin.backend` selects how pins are displayed:

- `auto` uses the Cosnip Pins GNOME Shell extension when it is running, otherwise the window pin.
- `shell` requires the extension.
- `window` always uses the XWayland pin window.

The extension is optional. It avoids XWayland resampling and keeps pins above normal windows.
See [`extensions/README.md`](extensions/README.md). A full extension-only capture/overlay
prototype is analyzed in [`docs/gnome-native-extension.md`](docs/gnome-native-extension.md).

## Desktop integration

Cosnip does not grab global shortcuts itself. Bind these commands in your desktop's keyboard
settings:

```text
cosnip snip
cosnip paste
```

On GNOME, open Settings → Keyboard → Custom Shortcuts. Use the full path to `cosnip` if it is not
in the graphical session's `PATH`.

To start the tray automatically, add a desktop entry to `~/.config/autostart/` that runs
`cosnip tray` with the full path to the binary.

For native pins on GNOME, install the bundled Shell extension:

```bash
./extensions/install.sh
```

New extensions may only load after logging out and back in on Wayland. Then verify with
`gnome-extensions info cosnip-pins@cosnip`.

## Platform support

| Session | Capture backend | Status |
|---|---|---|
| X11/Xorg | Native X11 | Supported, pixel-exact |
| GNOME 49 Wayland | Mutter ScreenCast + PipeWire raw frames | Supported, lossless |
| GNOME Wayland without PipeWire | GNOME Screencast H.264 | Automatic fallback, lossy; requires `ffmpeg` |
| Other Wayland desktops | — | Not supported |

Mixed-scale multi-monitor layouts are composed into the XWayland desktop coordinate space. In that
case monitors with a lower compositor scale are resampled into the canvas. Set `native_output =
false` to always use the composed canvas size. The live overlay still goes through XWayland's
global 2x buffer and the compositor downscale on those monitors; single-monitor output avoids
that path. On GNOME, the optional Shell pin backend displays the native RGBA image directly in
the compositor, so pins stay sharp on scaled displays.

## Troubleshooting

- Run `cosnip --doctor` to see the detected session, capture backend, monitors, clipboard support,
  and tray availability.
- If capture reports an unsupported session, use X11 or GNOME Wayland with PipeWire.
- If the tray icon is missing on GNOME, enable the AppIndicator/KStatusNotifier extension.
- If GNOME falls back to H.264 or asks for `ffmpeg`, check that PipeWire is available and that
  `cosnip --doctor` reports the PipeWire backend.
- Clipboard paste supports images only, and `--file` accepts PNG only. If a copied image disappears
  after the process exits, install or enable a clipboard manager.
- The settings folder picker requires `xdg-desktop-portal`.
- Text annotations use installed system fonts. Install a CJK font if CJK text renders as blanks.

## Development

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Feature specifications live in `specs/`; architecture decisions and the parity record live in
`docs/`.

## License

MIT OR Apache-2.0.
