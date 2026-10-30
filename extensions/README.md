# Cosnip native GNOME extension

`cosnip@choco2233` is the primary Cosnip implementation. It captures, annotates,
copies, saves, and pins entirely inside GNOME Shell.

## Why an extension

GNOME does not support `wlr-layer-shell`, and a normal `xdg_toplevel` cannot be
made reliably always-on-top. The Shell already has the native screenshot API
used by GNOME's own screenshot UI:

- `Shell.Screenshot.screenshot_stage_to_content()`
- `Shell.Screenshot.screenshot_area()`
- `Shell.Screenshot.composite_to_stream()`

Using these from an extension avoids X11/XWayland, PipeWire, portals, and
an external binary. The same stage coordinate space handles mixed-DPI monitors,
global pointer movement, and always-on-top pin actors.

## Features

- F1 snip, F3 paste clipboard image as a pin (configurable)
- Selection with eight resize handles and a pixel-size badge
- Window detection with hover highlight and click-to-select (Tab toggles)
- Pixel magnifier with fixed grid and coordinate readout
- Toolbar: rectangle, line, arrow, pencil, highlighter, mosaic/blur, text,
  magnifier, eraser, cursor toggle, undo/redo, cancel, pin, save, copy
- Native symbolic icons and St widgets; Adwaita/GTK4 preferences
- Native pins with border texture, opacity, drag, wheel zoom, and right-click close
- Toolbar-only tool switching (no keyboard tool bindings)
- Color and line-width pickers in the toolbar
- Shortcut recorder in preferences
- GSettings-backed preferences
- No panel/tray icon; use the shortcut, preferences, or D-Bus
- D-Bus automation interface `org.cosnip.Native`
- No external process, PipeWire, X11/XWayland, or portal capture permission

## Install

```bash
./extensions/install.sh
```

Log out and back in so GNOME Shell scans a newly copied extension. An update to
an existing install also requires a re-login because the running Shell caches
extension modules and schemas. Then verify:

```bash
gnome-extensions info cosnip@choco2233
gnome-extensions prefs cosnip@choco2233
```

If it is not enabled automatically:

```bash
gnome-extensions enable cosnip@choco2233
```

## Shortcuts

Default global shortcuts are `F1` (snip) and `F3` (paste as pin). Change them
in the extension preferences or with `gsettings`:

```bash
gsettings set org.gnome.shell.extensions.cosnip snip-shortcut "['F1']"
gsettings set org.gnome.shell.extensions.cosnip paste-shortcut "['F3']"
```

## D-Bus

The extension exports `org.cosnip.Native` at `/org/cosnip/Native`:

```bash
gdbus call --session --dest org.cosnip.Native \
  --object-path /org/cosnip/Native \
  --method org.cosnip.Native.Snip
```

Other methods: `Paste`, `Ping`, `Count`, `CaptureArea`, `SnipArea`.

## Legacy standalone app

The Rust/eframe implementation is preserved at tag
`legacy-software-v0.1.0`. It remains useful for X11 and non-GNOME desktops, but
it is no longer the primary GNOME path.
