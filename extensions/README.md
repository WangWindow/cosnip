# Cosnip native GNOME extension

`cosnip@choco2233` is the primary Cosnip implementation. It captures, annotates,
copies, and pins inside GNOME Shell.

## Why an extension

GNOME does not support `wlr-layer-shell`, and a normal `xdg_toplevel` cannot be
made reliably always-on-top. The Shell already has the native screenshot API
used by GNOME's own screenshot UI:

- `Shell.Screenshot.screenshot_stage_to_content()`
- `Shell.Screenshot.screenshot_area()`
- `Shell.Screenshot.composite_to_stream()`

Using these from an extension avoids X11/XWayland, PipeWire, portals, and
an external capture binary. The same stage coordinate space handles monitor
geometry, global pointer movement, and always-on-top pin actors. Saved output is
resized to the selected monitor's native scale and carries matching DPI
metadata, so mixed-DPI desktops do not force a 200% image for a 125% display.

## Features

- F1 snip, F3 paste clipboard image as a pin (configurable)
- Selection with eight resize handles and a pixel-size badge
- Window detection with hover highlight and click-to-select (Tab toggles)
- Cursor pixel magnifier with fixed grid and coordinate readout
- Toolbar: rectangle, line, arrow, pencil, highlighter, mosaic, text,
  eraser, cursor toggle, cancel, pin, save, copy
- Lucide icons and St widgets; Adwaita/GTK4 preferences
- Native pins with border texture, opacity, drag, wheel zoom, and right-click close
- Toolbar-only tool switching (no keyboard tool bindings)
- Preset color cycling with a fixed annotation width
- Direct pixel eraser with undo/redo
- Shortcut recorder in preferences
- GSettings-backed preferences
- No panel/tray icon; use the shortcut, preferences, or D-Bus
- D-Bus automation interface `org.cosnip.Native`
- No external capture process, PipeWire, X11/XWayland, or portal capture permission

The toolbar uses unmodified SVGs from [Lucide](https://lucide.dev/), revision
`a630cec77eac6568b989e87e4947a8bfe45caacd`. See
`cosnip@choco2233/icons/LICENSE-lucide.txt` for the upstream license and
contributor notices.

The toolbar cursor button controls whether the pointer captured when snipping
is composited into the saved image. The text tool uses the compositor input
method; the IME candidate panel is temporarily raised above the screenshot
overlay while the text entry is active.

## Output formats

The Output preferences offer PNG (lossless) and JPEG (smaller, fixed quality
90). Both formats use the selected monitor's native scale and matching DPI
metadata. No external encoder or libvips install is required. **Save image when
copying** optionally writes the image while copying it to the clipboard. Save
notifications can open the destination folder.

## Annotation controls

The eraser removes only the annotation pixels covered by a narrow brush; the
captured screenshot underneath is restored. A click makes a single dab, and
each drag or click can be undone with Ctrl+Z or redone with Ctrl+Shift+Z.

The color button cycles through the preset palette. Annotation width is fixed
for predictable drawing and lower toolbar complexity. Text is multiline:
Enter inserts a line break, Ctrl+Enter commits, and clicking outside the entry
commits it.

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

## Tests

Run `python3 extensions/tests/headless.py` to exercise capture pixels at 100%
and 125%, PNG/JPEG output, DPI metadata, annotation previews, the direct pixel
eraser, multiline text input, and pin scrolling in a private GNOME Shell and
D-Bus session. Add `--hidpi` to test 200% scaling. The tests require GNOME Shell,
GJS and GTK4.

## Legacy standalone app

The Rust/eframe implementation is preserved at tag
`legacy-software-v0.1.0`. It remains useful for X11 and non-GNOME desktops, but
it is no longer the primary GNOME path.
