# Feature 007 plan

## Structure

```text
extensions/cosnip@choco2233/
  extension.js       entry point, keybindings, D-Bus
  overlay.js         native selection and annotation UI
  annotations.js     tool model and Cairo preview/export
  toolbar.js         Lucide SVG icons and actions
  pins.js            Shell actor pins
  util.js            capture pixbuf, Cairo surfaces, clipboard, filenames
  prefs.js           GNOME extension preferences
  stylesheet.css     toolbar, badge, tips, text entry
  schemas/           GSettings schema
```

## Technical decisions

- Capture with `Shell.Screenshot.screenshot_stage_to_content()`.
- Store selection and annotations in stage logical coordinates.
- Render preview and output with the same `annotations.js` Cairo routines.
- Use fixed-position overlay children so the four masks surround the selection.
- Paint the frozen stage immediately above the panel, using nearest-neighbor
  texture sampling, and keep committed annotations separate from the draft.
- Limit mosaic work to blocks near the stroke segments.
- Use `Main.pushModal()` for global input during the overlay.
- Use `St.ImageContent`/`Cogl.PixelFormat.RGBA_8888` for pins.
- Use `St.Clipboard` for copy and Gio/Cairo for save.
- Use GSettings instead of TOML on the native path.
- Keep the legacy Rust app for X11/non-GNOME desktops.

## Verification

- Extension loads in a private headless GNOME Shell.
- `org.cosnip.Native.Ping` responds.
- `CaptureArea` produces a native-resolution PNG.
- `SnipArea` produces a saved selection with annotations.
- `PinSelection` creates a Shell pin and increments `Count`.
- D-Bus and headless tests pass; real-pointer interaction requires a live GNOME
  session.
- Compare checkerboard pixels in the selected preview and PNG at 100%, 125%,
  and 200%.
- Exercise annotation draft/commit/export and both smooth and wheel pin input
  in an isolated Shell, in addition to annotation geometry tests.
- Mixed-DPI multi-monitor fidelity and physical input hardware require live
  multi-monitor testing; stage capture is subject to Mutter's composite scale.
