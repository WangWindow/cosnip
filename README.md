# Cosnip

**Native screenshot, annotation, and pin tool for GNOME Shell on Wayland.**

Cosnip captures the screen, annotates it, copies or saves the result, and keeps
images on top as pins — all inside GNOME Shell. It uses the compositor's own
screenshot API instead of X11/XWayland, portal capture, PipeWire, or an external
capture binary, so there is no permission prompt and images stay sharp on scaled
displays.

The previous Rust application remains available at tag
[`legacy-software-v0.1.0`](https://github.com/WangWindow/cosnip/releases/tag/legacy-software-v0.1.0).

## Features

- **Region capture** with eight resize handles, a live pixel-size badge, monitor
  edge snapping, and `Ctrl+A` to select the current monitor.
- **Window detection** with hover highlight and click-to-select.
- **Annotations**: rectangle, line, arrow, pencil, highlighter, mosaic, text, and
  a direct pixel eraser with undo/redo.
- **Multiline text** with GNOME input-method support. `Enter` inserts a newline,
  `Ctrl+Enter` commits, and clicking outside commits.
- **Pins**: paste clipboard images with `F3`, then drag, wheel-zoom, and
  right-click to close.
- **Flexible output**: copy, save as PNG or JPEG, or pin directly.
- **Scale-aware images**: output uses the selected monitor's native resolution
  with matching DPI metadata, including mixed-DPI desktops.
- **Non-blocking saves** with a notification that opens the containing folder.
- **Optional save-on-copy** to write the image while copying it.
- **No tray icon or panel menu.** Configure shortcuts and options in
  preferences, or drive the extension over D-Bus.

## Requirements

- GNOME Shell 50 or 51 on Wayland
- No external capture, encoder, or libvips dependency

## Install

```bash
git clone https://github.com/WangWindow/cosnip.git
cd cosnip
./extensions/install.sh
```

Log out and back in after installing or updating. GNOME Shell caches extension
modules and settings schemas. Then verify:

```bash
gnome-extensions info cosnip@choco2233
```

## Usage

Press `F1` to start a snip. Drag to select a region, use the toolbar to annotate,
then copy, save, or pin the result. Press `F3` to paste a clipboard image
directly as a pin. All shortcuts can be changed in the preferences:

```bash
gnome-extensions prefs cosnip@choco2233
```

### Controls

| Input | Action |
|---|---|
| Drag | Select an area |
| Drag inside / a handle | Move / resize the selection |
| `Shift` + drag | Square selection |
| Arrow keys | Nudge the selection |
| `Ctrl+A` | Select the monitor under the pointer |
| `Tab` | Toggle window detection |
| `Enter` | Run the default output action (copy by default) |
| `Ctrl+C` / `Ctrl+S` / `P` | Copy / save / pin |
| `Ctrl+Z` / `Ctrl+Shift+Z` | Undo / redo |
| Double-click selection | Copy |
| Middle-click selection | Pin |
| `Esc` | Cancel the active tool, then clear the selection, then close |

Choose annotation tools from the toolbar. Clicking the active tool again returns
to the selection tool. The color button cycles through preset colors, and
annotation width is fixed. The cursor button includes the pointer in the output.

The eraser removes annotation pixels only; the captured image underneath is
restored. Each click or drag can be undone.

## Output

PNG output is lossless; JPEG output uses quality 90. Pixel dimensions are the
logical selection size multiplied by the monitor scale, and PNG/JPEG metadata
records the matching logical DPI (`96 × monitor scale`: 96 at 100%, 120 at 125%,
192 at 200%).

Mixed-DPI desktops capture lower-scale monitors at their native resolution
instead of upscaling the stage texture. Saving and encoding run asynchronously,
so the Shell UI stays responsive. The save notification can open the folder
containing the image. Enable **Save image when copying** in preferences to write
the image to disk while copying it to the clipboard.

Defaults:

- Save directory: the user's `Pictures` folder (configurable)
- Filename: `Cosnip_%Y-%m-%d_%H-%M-%S.png`
- Format: PNG
- Save-on-copy: off

## D-Bus

```bash
gdbus call --session --dest org.cosnip.Native \
  --object-path /org/cosnip/Native \
  --method org.cosnip.Native.Snip
```

The interface also provides `Paste`, `Ping`, `Count`, `CaptureArea`, and
`SnipArea`.

## Development

Run the end-to-end tests in an isolated GNOME Shell and D-Bus session:

```bash
python3 extensions/tests/headless.py
python3 extensions/tests/headless.py --hidpi   # 200% scaling
```

The tests require GNOME Shell, GJS, and GTK4. They cover capture pixels, DPI
metadata, annotations, text input, pins, PNG/JPEG saving, and save retries.

## Documentation

- [Native extension details](extensions/README.md)
- [Snipaste parity matrix](docs/snipaste-parity.md)

## License

MIT OR Apache-2.0
