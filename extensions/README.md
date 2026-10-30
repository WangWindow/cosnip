# Cosnip native GNOME extension

`cosnip@cosnip` is the primary Cosnip implementation. It captures, annotates,
copies, saves, and pins entirely inside GNOME Shell.

## Why an extension

GNOME does not support `wlr-layer-shell`, and a normal `xdg_toplevel` cannot be
made reliably always-on-top. The Shell already has the native screenshot API
used by GNOME's own screenshot UI:

- `Shell.Screenshot.screenshot_stage_to_content()`
- `Shell.Screenshot.screenshot_area()`
- `Shell.Screenshot.composite_to_stream()`

Using these from an extension avoids XWayland, PipeWire, portals, and an
external binary. The same stage coordinate space handles mixed-DPI monitors,
global pointer movement, and always-on-top pin actors.

## Features

- F1 snip, F3 paste clipboard image as a pin (configurable)
- Selection with eight resize handles and a pixel-size badge
- Window detection with hover highlight and click-to-select (Tab toggles)
- Pixel magnifier, selection border/mask/anchor settings
- Toolbar: rectangle/ellipse/line group, arrow, pencil, highlighter,
  mosaic/blur, text, eraser, undo/redo, cancel, pin, save, copy
- Number and magnifier annotations via N/G
- Native pins with border texture, opacity, drag, wheel zoom, and right-click close
- Shortcut recorder in preferences
- GSettings-backed preferences
- D-Bus automation interface `org.cosnip.Native`
- No external process, PipeWire, XWayland, or portal capture permission

## Install

```bash
./extensions/install.sh
```

Log out and back in so GNOME Shell scans a newly copied extension, then verify:

```bash
gnome-extensions info cosnip@cosnip
gnome-extensions prefs cosnip@cosnip
```

If it is not enabled automatically:

```bash
gnome-extensions enable cosnip@cosnip
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
