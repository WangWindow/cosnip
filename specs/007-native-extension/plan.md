# Feature 007 plan

## Structure

```text
extensions/cosnip@cosnip/
  extension.js       entry point, keybindings, D-Bus, panel menu
  overlay.js         native selection and annotation UI
  annotations.js     tool model and Cairo preview/export
  toolbar.js         custom Cairo icons and actions
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
