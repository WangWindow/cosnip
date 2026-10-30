# Cosnip

Cosnip is a screenshot, annotation, and pin tool for GNOME. The primary
implementation is now a **native GNOME Shell extension**: capture, overlay,
annotations, clipboard, saving, and pins all run inside the compositor. There is
no external process, XWayland overlay, PipeWire capture, or portal screenshot
permission on GNOME.

The previous standalone Rust/eframe application is preserved at tag
[`legacy-software-v0.1.0`](https://github.com/WangWindow/cosnip/releases/tag/legacy-software-v0.1.0).

## Features

- Native stage capture through `Shell.Screenshot`
- Selection with eight resize handles and a size badge
- Toolbar: rectangle/ellipse group, arrow, pencil, highlighter, mosaic/blur,
  text, eraser, undo/redo, cancel, pin, save, copy
- Freehand mosaic/blur and annotation-pixel eraser
- Native pins: image-sized, always on top, drag, wheel zoom, right-click close
- GNOME-native preferences via `gnome-extensions prefs`
- GSettings configuration and a D-Bus automation interface
- No XWayland resampling, so overlays and pins stay sharp on scaled displays

## Requirements

- GNOME Shell 48–51 on Wayland
- `glib-compile-schemas` (usually provided by GLib) to install
- No Rust toolchain, PipeWire headers, portal, or X11 session

## Install

```bash
git clone https://github.com/WangWindow/cosnip.git
cd cosnip
./extensions/install.sh
```

Log out and back in so GNOME Shell scans a newly copied extension, then verify:

```bash
gnome-extensions info cosnip@cosnip
```

If needed:

```bash
gnome-extensions enable cosnip@cosnip
```

Defaults: `F1` snip, `F3` paste clipboard image as a pin.

## Use

Open the preferences with:

```bash
gnome-extensions prefs cosnip@cosnip
```

Or change GSettings directly:

```bash
# Global shortcuts
gsettings set org.gnome.shell.extensions.cosnip snip-shortcut "['F1']"
gsettings set org.gnome.shell.extensions.cosnip paste-shortcut "['F3']"

# Default output: copy | save | pin
gsettings set org.gnome.shell.extensions.cosnip default-output 'copy'

# Save directory and file name pattern
gsettings set org.gnome.shell.extensions.cosnip save-dir "$HOME/Pictures"
gsettings set org.gnome.shell.extensions.cosnip filename 'Cosnip_%Y-%m-%d_%H-%M-%S.png'

# Annotation defaults
gsettings set org.gnome.shell.extensions.cosnip annotation-color '#FF4040'
gsettings set org.gnome.shell.extensions.cosnip annotation-width 3
gsettings set org.gnome.shell.extensions.cosnip font-size 20
```

The extension also adds a camera icon to the top bar with
**Snip / Paste / Close all pins / Preferences**.

## Snip controls

| Input | Action |
|---|---|
| Drag | Create a selection |
| Drag inside the selection | Move it |
| Drag a handle | Resize it |
| Shift while dragging | Keep a square |
| Arrow keys | Nudge the selection |
| Ctrl+A | Select the whole desktop |
| R / C / A / B / H / M / T / E / V | Rectangle, ellipse, arrow, pencil, highlighter, mosaic, text, eraser, select |
| Right-click while annotating | Finish or leave the current tool |
| Enter | Run the configured default output (copy by default) |
| Ctrl+C | Copy |
| Ctrl+S | Save |
| P | Pin |
| Ctrl+Z / Ctrl+Shift+Z | Undo / redo |
| Escape | Cancel the current tool, then the overlay |

Pins support drag, wheel zoom, and right-click close.

## D-Bus automation

The extension exports `org.cosnip.Native` at `/org/cosnip/Native`:

```bash
gdbus call --session --dest org.cosnip.Native \
  --object-path /org/cosnip/Native \
  --method org.cosnip.Native.Snip

gdbus call --session --dest org.cosnip.Native \
  --object-path /org/cosnip/Native \
  --method org.cosnip.Native.Paste
```

Methods: `Snip`, `Paste`, `Ping`, `Count`, `CaptureArea`, `SnipArea`,
`SetSelection`, `AddRect`, `AddMosaic`, `AddEraser`, `SaveSelection`,
`PinSelection`, `CloseOverlay`.

## How it works

- `Shell.Screenshot.screenshot_stage_to_content()` captures the stage and cursor
  natively.
- The overlay is a Shell actor with Cairo rendering, so the preview never goes
  through XWayland.
- The selected region is composited into a Cairo image surface for copy/save/pin.
  The same annotation renderer is used for the preview and the output.
- Pins are `St.ImageContent` actors in the Shell UI layer.
- Configuration lives in the extension GSettings schema.

See [`extensions/README.md`](extensions/README.md) and
[`docs/gnome-native-extension.md`](docs/gnome-native-extension.md).

## Legacy standalone app

The previous implementation and its optional pin extension are preserved at tag
`legacy-software-v0.1.0`. Use it when you need:

- X11/Xorg support
- non-GNOME Wayland desktops
- the external `cosnip` CLI
- the previous TOML settings window

Build from that tag:

```bash
git checkout legacy-software-v0.1.0
cargo build --release -p cosnip-app
```

## License

MIT OR Apache-2.0
