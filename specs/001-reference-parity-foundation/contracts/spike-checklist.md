# Spike Checklist: XWayland Windowing and Capture

**Environment**: aarch64 Armbian/Ubuntu 26.04, GNOME 49 Wayland, XWayland, 1920x1080, single
monitor, eframe 0.37, glow backend, forced X11.
**Date**: 2026-10-29
**Spike**: `temp/spike` (disposable; not a product crate)

A case is `pass` only when the stated observation is reproduced on the target. `blocked` means the
environment prevented a valid test; it is not a pass. `fail` means the behavior did not work.

| ID | Case | Expected | Result | Measurement / Notes |
|---|---|---|---|---|
| SPIKE-001 | WM-managed fullscreen overlay | Covers screen, receives focus, Escape closes | **pass** | 1920x1080; `_NET_WM_STATE_FULLSCREEN`, `ABOVE`, `FOCUSED`; `_NET_ACTIVE_WINDOW` points to the overlay; first frame 5–11 ms |
| SPIKE-002 | Override-redirect fullscreen overlay | Covers screen; compare focus behavior | **pass with limitation** | 1920x1080, depth 32, override-redirect; **focused=false**, not active. Use the WM-managed variant for keyboard interaction |
| SPIKE-003 | Override-redirect pin window | Positioned, transparent, topmost, taskbar-hidden | **pass** | 360x220 at (240,220); override-redirect; `UTILITY`; skip-taskbar/pager; depth 32 when root transparency + software GL; first frame 6–33 ms |
| SPIKE-004 | Pin transparency with default hardware GLX | Depth-32 ARGB | **fail** | Depth 24 with the default GLX path; no alpha. Software GL produced depth 32. Needs EGL/alpha-config or an ARGB pin adapter |
| SPIKE-005 | Pin click-through | Empty input shape; bounding/clip retained | **pass** | Input region count 0; bounding and clip regions full |
| SPIKE-006 | X11 root capture under XWayland | Full desktop image | **fail** | `GetImage` → `BadMatch`; XWayland root is not a usable native capture source. Use compositor/portal on Wayland |
| SPIKE-007 | Clipboard image set | Image published to the clipboard | **partial** | `arboard::set_image` path exists; cross-application paste was not automated on this session. The reference uses a dedicated D-Bus/Wayland helper |
| SPIKE-008 | XGrabKey probe | Grab/ungrab without protocol error | **pass (probe only)** | Grab/ungrab succeeded on the XWayland root; actual global delivery is not proven |
| SPIKE-009 | Pin above a native Wayland window | Visually above | **pass** | GNOME Shell Screencast recording shows the reference pin over the native Zed/terminal; the reference pin is 524x324 at (88,88), depth 32, override-redirect |
| SPIKE-010 | GNOME 49 silent screenshot | Image without a user dialog | **blocked** | GNOME Shell D-Bus `AccessDenied`; portal returns a request handle but no response while a user decision is pending |
| SPIKE-017 | Reference overlay above native Wayland | Covers composited desktop; UI visible | **pass** | Native screencast shows the 1920x1080 reference overlay above native windows; X11 capture background is black, UI is visible; log shows focus in |
| SPIKE-018 | Native screencast oracle | Record and frame-extract composited desktop | **pass** | GJS keeps the `org.gnome.Shell.Screencast` connection alive for 6–8 s, calls `StopScreencast` on the same connection, then `ffmpeg` extracts a PNG |
| SPIKE-011 | Overlay/pin creation latency | Record values | **pass** | Overlay 5–11 ms; pin 6–33 ms (software GL, XWayland) |
| SPIKE-012 | Idle CPU after overlay/pin settles | Near-idle | **fail (spike artifact)** | Spike calls `request_repaint()` every frame; observed ~367% CPU. Product must be event-driven; remeasure with a no-repaint build |
| SPIKE-013 | Memory footprint | Record RSS | **partial** | Spike RSS ~178 MB with control window + overlay/pin under software GL; not a product budget |
| SPIKE-014 | GNOME Shell screenshot D-Bus from unprivileged process | Success | **fail** | `org.gnome.Shell.Screenshot` returns `AccessDenied: Screenshot is not allowed` |
| SPIKE-015 | Reference overlay dynamic observation | Overlay/UI behavior visible | **pass** | X11-forced reference overlay captured: 1920x1080, selection badge, toolbar, tips, magnifier; 32-bit ARGB |
| SPIKE-016 | Reference single-instance CLI | Second instance drives first | **pass** | First instance background + second `snip` produced `Second instance` and `Snipper: ready`; `--hold` keeps annotation UI |

## Commands Used

```bash
# Spike, full sequence
LIBGL_ALWAYS_SOFTWARE=1 SPIKE_ROOT_TRANSPARENT=1 SPIKE_AUTORUN=all \
  DISPLAY=:0 XAUTHORITY=/run/user/1000/.mutter-Xwaylandauth.OGJ6X3 \
  temp/spike/target/release/cosnip-spike

# Window inspection
xwininfo -root -tree | grep -iE 'Cosnip (Overlay|Pin)'
xprop -id <id> _NET_WM_STATE _NET_WM_WINDOW_TYPE
xvfb-run ... # not used; a real XWayland session was used

# Input shape
python3 - <<'PY'
from Xlib import display
from Xlib.ext import shape
d=display.Display(); w=d.create_resource_object('window', 0x...)
print(len(shape.get_rectangles(w, 2).rectangles))  # 0 == click-through
PY
```

## Native Screencast Oracle

```javascript
// gjs: keep the connection alive so the MP4 is finalized
const {Gio, GLib} = imports.gi;
const bus = Gio.bus_get_sync(Gio.BusType.SESSION, null);
const opts = {
  'draw-cursor': new GLib.Variant('b', true),
  'framerate': new GLib.Variant('i', 30),
};
const args = new GLib.Variant('(sa{sv})', ['/tmp/cosnip-cast-%d.mp4', opts]);
bus.call_sync('org.gnome.Shell.Screencast', '/org/gnome/Shell/Screencast',
  'org.gnome.Shell.Screencast', 'Screencast', args, null, Gio.DBusCallFlags.NONE, -1, null);
GLib.usleep(6000000);
bus.call_sync('org.gnome.Shell.Screencast', '/org/gnome/Shell/Screencast',
  'org.gnome.Shell.Screencast', 'StopScreencast', null, null, Gio.DBusCallFlags.NONE, -1, null);
```

```bash
ffmpeg -loglevel error -ss 2 -i /tmp/cosnip-cast-*.mp4 -frames:v 1 -y frame.png
```

Do not call `StopScreencast` from a separate `gdbus` process; the caller connection must stay
alive or the file will lack a `moov` atom.

## Interpretation

- The **overlay should be WM-managed fullscreen**, not override-redirect, because focus is the
  gating behavior for keyboard shortcuts and text entry.
- The **pin should be override-redirect ARGB**. The default GLX path on this device cannot provide
  alpha; software GL works, but the product needs an EGL/alpha-config investigation or a dedicated
  ARGB pin renderer before pin shadows/opacity are promised.
- `MousePassthrough` correctly produces an empty input region.
- XWayland is not a capture source; Wayland capture must use the compositor or portal.
- Native Wayland stacking is confirmed for GNOME 49: the reference's X11 override-redirect pin and
  overlay are above native Wayland surfaces. Recheck on KDE and other compositors.
- The GNOME 49 portal permission path is a product-level UX blocker and needs an interactive test
  before the MVP plan freezes the capture backend.
