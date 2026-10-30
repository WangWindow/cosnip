# Cosnip GNOME Shell extension

`cosnip-pins@cosnip` provides native, always-on-top pins on GNOME.

## Why an extension is required

GNOME does not support the `wlr-layer-shell` protocol, and an ordinary
`xdg_toplevel` has no portable always-on-top hint on Wayland. Drawing the pin
inside the Shell is the only way to get all of:

- native-resolution display with no XWayland resampling,
- always-on-top stacking,
- a free-floating position across monitors,
- drag and scroll-zoom input.

The extension owns a `St.ImageContent` actor and exposes a small D-Bus service
at `org.cosnip.Pins`. Cosnip sends the rendered RGBA image directly; no
temporary PNG or X11 window is involved.

## Install

```bash
./extensions/install.sh
```

If GNOME does not load it immediately, log out and back in, then enable it:

```bash
gnome-extensions enable cosnip-pins@cosnip
```

On GNOME, the AppIndicator extension is unrelated; this extension is separate.

## Use

Choose the backend in `~/.config/cosnip/config.toml`:

```toml
[pin]
backend = "auto"    # auto | shell | window
```

- `auto` uses the extension when its D-Bus name is present, otherwise falls
  back to the XWayland pin window.
- `shell` requires the extension and reports an error if it is unavailable.
- `window` always uses the XWayland pin window.

Pins support:

- drag with the left mouse button,
- zoom with the scroll wheel,
- right-click to close.

Pins stay above normal windows and below GNOME Shell chrome. They are hidden
in the overview.
