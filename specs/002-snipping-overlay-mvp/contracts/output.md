# Output Contract: Clipboard, Save, Pin

## Common

- The output image is the exact crop of the captured frame; width/height MUST equal the confirmed
  selection.
- Default format: PNG, RGBA8, no metadata, no scaling.
- All output work runs off the UI thread where practical; failures are reported with an exit code
  and do not leave partial state.

## Clipboard

- Use `arboard::Clipboard::set_image` with RGBA bytes.
- The clipboard image MUST be readable by a native Wayland GTK4 app when the process remains alive
  and the image is set on an X11/XWayland session.
- The MVP keeps an owning clipboard handle alive and waits 800 ms after `set_image` before exit so
  a desktop clipboard manager can take ownership. On GNOME this was verified with a 500x300 image;
  on a bare X11 session without a clipboard manager, persistence is not guaranteed.
- A clipboard failure maps to exit code 5.

## Save

- Path from `--save PATH`; if absent, use `$XDG_PICTURES_DIR/Cosnip_<YYYY-MM-DD_HH-MM-SS>.png`,
  falling back to `$HOME/Pictures`.
- Create the parent? No: an explicit parent must exist; the default Pictures directory is created
  if missing with 0700.
- Atomic write: create `.<name>.tmp-<pid>` in the destination directory with 0600, write the PNG,
  flush/sync, rename over the target. On error, remove the temp file.
- Collision-free default name: append `-1`, `-2`, … if the target exists.
- A successful save leaves exactly one valid PNG with the selection dimensions.
- A save failure maps to exit code 5 and leaves no partial file.

## Pin

- Create an image-sized window at the selection's top-left; no shadow margin in the MVP.
- X11 window attributes: override-redirect, no decorations, `UTILITY` type, skip-taskbar/pager,
  always-on-top.
- If the GL config supports alpha, a future version may add a shadow; the MVP MUST NOT show a
  black margin around the image.
- Interactions:
  - left drag moves the window,
  - wheel zoom in the range 10%–500%, anchored at the top-left,
  - Escape or double-click closes,
  - closing the last pin exits the process.
- Pins must be visible above native Wayland windows on GNOME 49 (M0 evidence).
- Pin failure maps to exit code 5.

## Determinism

- Clipboard/save image bytes are deterministic for the same frame and selection.
- Golden tests compare dimensions exactly and pixels with zero tolerance for lossless output.
- The overlay chrome is not part of the output.
