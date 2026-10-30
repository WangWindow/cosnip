# Native GNOME Shell implementation

The primary Cosnip implementation is now `extensions/cosnip@cosnip`. It runs
entirely inside GNOME Shell.

## Capture

The extension uses the same internal API as GNOME's screenshot UI:

- `Shell.Screenshot.screenshot_stage_to_content()` for the stage and cursor.
- `Shell.Screenshot.composite_to_stream()` for a pixbuf of a selected region.
- `Main.wm.addKeybinding()` for global shortcuts.

Because the extension runs inside the Shell, these calls do not require the
portal screenshot permission, PipeWire, or XWayland.

## Overlay and annotations

- The captured `Clutter.Content` is attached to a fullscreen `St.Widget`.
- A `St.DrawingArea` paints the dim mask, selection border, handles, and
  annotations with Cairo.
- Input is handled by the overlay actor inside a `Main.pushModal()` grab, so it
  gets global pointer and keyboard focus across monitors.
- Mosaic/blur/eraser use the same freehand stroke mask in preview and output.
- Text uses `St.Entry` while editing and Pango/Cairo for export.
- Copy/save/pin composite the screenshot pixbuf and the same annotation renderer
  into a Cairo surface.

## Pins

Pins are `St.ImageContent` actors added to the Shell chrome. They use native
RGBA textures, stay above normal windows, and never pass through XWayland.

## Configuration

Settings live in `org.gnome.shell.extensions.cosnip` (GSettings). The
`gnome-extensions prefs cosnip@cosnip` window edits shortcuts, annotation
defaults, output behavior, and pin zoom.

## Automation

`org.cosnip.Native` at `/org/cosnip/Native` exposes `Snip`, `Paste`, `Ping`,
`Count`, and test/automation helpers for area capture, selection, annotations,
and saving.

## Boundaries

- GNOME only; X11 and other desktops use the legacy standalone app.
- Annotation rendering is implemented in GJS/Cairo rather than the Rust imaging
  crate.
- A Shell extension runs in the compositor process, so errors must be contained
  with normal exception handling; the legacy app remains available as a fallback.
