# Native GNOME Shell implementation

The primary Cosnip implementation is now `extensions/cosnip@choco2233`. It runs
entirely inside GNOME Shell.

## Capture

The extension uses the same internal API as GNOME's screenshot UI:

- `Shell.Screenshot.screenshot_stage_to_content()` for the stage.
- `Shell.Screenshot.composite_to_stream()` for a pixbuf of a selected region.
- `Main.wm.addKeybinding()` for global shortcuts.

Because the extension runs inside the Shell, these calls do not require the
portal screenshot permission, PipeWire, X11, or XWayland.

## Overlay and annotations

- The captured `Clutter.Content` is attached to a fullscreen `St.Widget`.
- Four lightweight `St.Widget` rectangles paint the mask outside the selection.
  The border and handles are widgets; a selection-sized `St.DrawingArea` paints
  annotations.
- Input is handled by the overlay actor inside a `Main.pushModal()` grab, so it
  gets global pointer and keyboard focus across monitors.
- Mosaic/blur/eraser use the same freehand stroke mask in preview and output.
- Text uses `St.Entry` while editing and Pango/Cairo for export.
- Copy/save/pin composite the screenshot pixbuf and the same annotation renderer
  into a Cairo surface at the selected monitor's native scale. PNG and JPEG saves carry matching DPI metadata (96 × monitor scale).
- Text editing raises the compositor input panel above the screenshot overlay so
  IME candidate windows remain visible.

## Pins

Pins are `St.ImageContent` actors added to the Shell chrome. They use native
RGBA textures, stay above normal windows, and never pass through X11/XWayland.

## Configuration

Settings live in `org.gnome.shell.extensions.cosnip` (GSettings). The
`gnome-extensions prefs cosnip@choco2233` window edits shortcuts, snip
behavior, output behavior, and pin opacity.

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
