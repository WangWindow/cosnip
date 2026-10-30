# Native GNOME Shell path

This note records the feasibility of replacing the external Cosnip process with a
GNOME Shell extension on GNOME Wayland.

## Feasibility

The Shell already contains the required primitives. In GNOME Shell 51,
`/org/gnome/shell/ui/screenshot.js` uses:

- `Shell.Screenshot.screenshot_stage_to_content()` to capture the whole stage,
  including the cursor content and scale.
- `Shell.Screenshot.screenshot_area()` / `screenshot_window()` for focused
  captures.
- `Shell.Screenshot.composite_to_stream()` to encode the result.
- `Main.wm.addKeybinding()` to receive a desktop shortcut.
- `Shell.Screenshot` is callable from an extension because it runs inside the
  Shell process, so it does not need the portal's screenshot permission.

A pure extension can therefore:

1. Capture the stage natively on a keybinding.
2. Put the captured `Clutter.Content` into a fullscreen `St.Widget` overlay.
3. Draw the selection, dim mask, toolbar, and annotations with Clutter/Cairo.
4. Export with `composite_to_stream`, copy with `St.Clipboard`, or create a pin
   actor with `St.ImageContent` (the existing `cosnip-pins` mechanism).
5. Register its own shortcuts through a small GSettings schema.

## Problems this solves

- No XWayland surface for the overlay or pins, so no resampling blur.
- One stage-coordinate space handles pointer movement across monitors and mixed
  DPI layouts.
- Pins are native Shell actors and are always above normal windows.
- No PipeWire, portal, X11, or external helper process is required.
- No GNOME Settings custom-shortcut setup; the extension owns its keybindings.

## Costs and remaining hard problems

- **Annotation rendering**: the current Rust renderer (cosmic-text, pixel
  effects, mosaics) must be reimplemented in Cairo/GJS. Text input and CJK/IME
  are possible with `St.Entry` and Pango, but this is a significant port.
- **Export compositing**: annotations live in Shell actors; exporting must
  render the same scene into a Cairo image surface instead of grabbing the
  screen, or the toolbar and dim mask would leak into the output.
- **Shell main-loop cost**: full-stage textures and per-pixel effects must not
  block the compositor. Large annotations need async/chunked work.
- **Stability**: a JavaScript error or allocation spike can degrade or crash the
  entire GNOME Shell, not just a helper process.
- **Compatibility**: Shell APIs and styles change between GNOME releases; the
  extension must track them.
- **Platform scope**: GNOME-only. X11 and other Wayland desktops would still
  need the external app.

## Recommended path

Keep the current hybrid for now:

- Rust app: capture, annotation editing, output, X11 fallback.
- `cosnip-pins@cosnip`: native pins on GNOME, with automatic window fallback.

If a fully native experience is required, build a separate
`cosnip-native@cosnip` prototype in stages:

1. Keybinding + `screenshot_stage_to_content` + native selection overlay.
2. Copy/save/pin of the selection without annotations.
3. Rectangle/arrow/text annotations and Cairo export.
4. Mosaic/blur/eraser and the remaining annotation tools.
5. Extension preferences UI using `ExtensionPreferences`.

This keeps the existing app usable while the native path is validated.
