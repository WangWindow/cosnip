# Cosnip

A native GNOME Shell screenshot, annotation, and pin tool by **choco2233**.
It runs entirely inside GNOME Shell with no X11/XWayland dependency.

The previous Rust application is available at tag
[`legacy-software-v0.1.0`](https://github.com/WangWindow/cosnip/releases/tag/legacy-software-v0.1.0).

## Requirements

- GNOME Shell 48–51 on Wayland

## Install

```bash
git clone https://github.com/WangWindow/cosnip.git
cd cosnip
./extensions/install.sh
```

Log out and back in after installing or updating. GNOME Shell caches extension
modules and settings schemas. Then check:

```bash
gnome-extensions info cosnip@choco2233
```

Defaults: `F1` snip, `F3` paste an image as a pin. Configure shortcuts and
options with:

```bash
gnome-extensions prefs cosnip@choco2233
```

## Controls

| Input | Action |
|---|---|
| Drag | Select an area |
| Drag inside / a handle | Move / resize the selection |
| `Shift` + drag | Square selection |
| Arrow keys, `Ctrl+A` | Nudge, select the whole screen |
| `Tab` | Toggle window detection |
| `Enter` | Default output (copy by default) |
| `Ctrl+C` / `Ctrl+S` / `P` | Copy / save / pin |
| `Ctrl+Z` / `Ctrl+Shift+Z` | Undo / redo |
| `Esc` | Cancel tool, then close |

Switch annotation tools by clicking the toolbar; tool keys are intentionally not
bound. Click the color or width button for a picker, and use the mouse button in
the toolbar to include or exclude the captured cursor. Pins support drag, wheel
zoom, and right-click close. There is no panel/tray menu; use the shortcut,
preferences, or D-Bus.

## D-Bus

```bash
gdbus call --session --dest org.cosnip.Native \
  --object-path /org/cosnip/Native \
  --method org.cosnip.Native.Snip
```

Other methods include `Paste`, `Ping`, `CaptureArea`, and `SnipArea`.

## Documentation

- [Native extension details](extensions/README.md)
- [Snipaste parity matrix](docs/snipaste-parity.md)

## License

MIT OR Apache-2.0
