# Research: Snipaste 2.11.3 Linux Reference Behavior

**Feature**: `001-reference-parity-foundation`
**Date**: 2026-10-29
**Reference**: Snipaste 2.11.3 Linux x86_64 AppImage
**SHA-1**: `02a9af2118f502ce9a9e4be16026df05cda1920d`
**Size**: 37,450,944 bytes
**Primary target**: aarch64, Ubuntu 26.04, GNOME 49 Wayland + XWayland, 1920x1080, single monitor
**Status**: M0 observations complete; explicit gaps are recorded at the end

This document records behavior, architecture, and measurements. It does not reproduce source
code, decompiled bodies, string tables, or reference assets. Raw analysis artifacts remain in
`temp/`.

## 1. Reference Build Shape

| Property | Observation |
|---|---|
| Package | Official AppImage, verified SHA-1 |
| App version | 2.11.3 (2026-01-18) |
| UI toolkit | Qt 6.2.4, patched Qt tree |
| Main binary | x86-64 ELF, stripped, ~9.98 MB |
| Compiler fingerprints | GCC 14.2.0; GCC 9.4.0 objects |
| X11 libraries | libX11, libXext, libXfixes |
| Qt modules | Core, Gui, Widgets, DBus, Network, OpenGL, Multimedia, PrintSupport, Svg, Xml, Core5Compat |
| Platform plugins | xcb, Wayland, xcb-glx, xcb-egl, ibus, fcitx5, qt6ct |
| Image plugins | PNG, JPEG, WebP, TIFF, SVG, GIF, ICO, TGA, WBMP |
| Desktop integration | `Snipaste.desktop`, D-Bus service name `com.snipaste.Snipaste`, KDE restricted interface `org.kde.KWin.ScreenShot2` |
| Bundled dependencies | Qt, OpenSSL 1.1, GnuTLS, FFmpeg/Multimedia, PulseAudio, cups, XCB helpers |

Evidence: OBS-001, OBS-002.

The main binary imports X11 symbols for window management, cursor capture, and input shape
(`XMoveResizeWindow`, `XTranslateCoordinates`, `XFixesGetCursorImage`,
`XFixesSetWindowShapeRegion`, `XGrabKey`) and exposes Qt RTTI/metaobject names for the application
model. This is enough to identify architecture without reconstructing source.

## 2. Capture Backend Selection

Static cross-reference analysis of the capture factory shows the following decision tree:

1. Determine whether the session is Wayland:
   - `XDG_SESSION_TYPE == "wayland"`, or
   - `WAYLAND_DISPLAY` contains `wayland`.
2. If not Wayland: use the X11 screen grabber.
3. If Wayland and `XDG_CURRENT_DESKTOP` contains `kde`: use KWin ScreenShot2.
4. If Wayland and the desktop contains `gnome` or `unity`:
   - Run `gnome-shell --version`.
   - On GNOME Shell version 40 or lower: use the GNOME Shell Screenshot D-Bus service.
   - Otherwise: use the freedesktop portal Screenshot service.
5. Any other Wayland desktop: use the freedesktop portal Screenshot service.

The runtime log confirms the chosen backend with a line of the form
`Screenshot source: <backend>`. The observed values were `x11`,
`org.gnome.Shell.Screenshot`, `org.kde.KWin.ScreenShot2`, and
`org.freedesktop.portal.Desktop`. Evidence: OBS-002, OBS-003, OBS-004, OBS-006.

### 2.1 GNOME Shell Screenshot (legacy path)

The GNOME path calls the session-bus service `org.gnome.Shell.Screenshot` at
`/org/gnome/Shell/Screenshot`, method `Screenshot`. Arguments observed from the call setup:

- include cursor: false
- flash: false
- output file: a temporary PNG under `/tmp` with a timestamped name

The method is synchronous from the application's point of view and writes a file. Evidence:
OBS-003.

### 2.2 KDE KWin ScreenShot2

The KDE path uses `org.kde.KWin.ScreenShot2` at `/org/kde/KWin/ScreenShot2`:

- `CaptureArea(x, y, width, height, options, pipe_fd)` for a rectangle
- `CaptureWorkspace(options, pipe_fd)` for the full workspace

Options include `include-cursor` and `native-resolution`; the result is delivered through a
PipeWire file descriptor. Evidence: OBS-004.

### 2.3 Freedesktop portal Screenshot

The portal path uses `org.freedesktop.portal.Desktop` at
`/org/freedesktop/portal/desktop`, interface `org.freedesktop.portal.Screenshot`, method
`Screenshot`:

- parent window: `x11:` plus the X11 window identifier
- options: `interactive = false`, `handle_token = <unique token>`
- response: `org.freedesktop.portal.Request.Response`

The portal is asynchronous; the application subscribes to the request response. Evidence:
OBS-005.

### 2.4 X11 screen grabber

The X11 path logs `Screenshot source: x11`, enumerates the virtual desktop, and captures the
framebuffer. On a native Xorg session this is the expected fast path. Under GNOME XWayland, a
direct `GetImage` on the root window fails with `BadMatch` in the spike, and the reference's X11
capture content was black because native Wayland surfaces are outside the XWayland root. X11
capture must therefore be treated as an **Xorg-only** backend; it is not a valid Wayland capture
strategy. Evidence: OBS-006, OBS-019.

## 3. Overlay and Pin Window Strategy

### 3.1 Reference window flags

Static analysis of the snipping-window and image-window constructors shows a common Qt window
flag set:

- `Window`
- `BypassWindowManagerHint` (override-redirect)
- `FramelessWindowHint`
- `WindowStaysOnTopHint`
- `NoDropShadowWindowHint`

The image-window constructor additionally uses X11 window-management helpers and sets
`_NET_WM_STATE_SKIP_TASKBAR`, `_NET_WM_STATE_SKIP_PAGER`, `_NET_WM_STATE_ABOVE` /
`_NET_WM_STATE_STAYS_ON_TOP`, and `_NET_WM_WINDOW_TYPE_UTILITY`. The snipping window is a
32-bit ARGB override-redirect window covering the virtual desktop; the magnifier and toolbars are
separate override-redirect popup windows. Evidence: OBS-007, OBS-008, OBS-009.

### 3.2 Click-through

Click-through is implemented with the XFixes/Shape input region, not with a Qt
`WindowTransparentForInput` flag. A target rectangle is subtracted from the window rectangle and
the result is applied as the input shape. An empty input region makes the whole window
non-interactive. Evidence: OBS-007, OBS-017.

### 3.3 Target-device spike

A disposable `eframe` 0.37 spike forced the winit X11 backend and tested two overlay variants and
one pin variant.

| Case | Result |
|---|---|
| WM-managed fullscreen overlay | 1920x1080, `_NET_WM_STATE_FULLSCREEN`, `ABOVE`, `FOCUSED`; active X11 window; first frame 5–11 ms |
| Override-redirect overlay | 1920x1080, depth 32; **no focus**; not the active window |
| Override-redirect pin, default hardware GLX | 360x220 at 240,220; `UTILITY`, skip-taskbar/pager; **depth 24**, no alpha |
| Override-redirect pin, software GL + transparent root viewport | depth 32; rounded transparent pin rendered correctly; first frame 6–33 ms |
| Pin click-through | input shape region empty; bounding/clip regions remain full |
| X11 root capture under XWayland | `GetImage` fails with `BadMatch` |
| XGrabKey probe | grab+ungrab succeeded on the XWayland root |

Evidence: OBS-016, OBS-017, OBS-018, OBS-019, OBS-020, OBS-021.

### 3.4 Reference stacking on native Wayland (visual oracle)

GNOME Shell's `org.gnome.Shell.Screencast` D-Bus service records the composited desktop,
including native Wayland surfaces. A 5–8 second recording was kept alive on one D-Bus connection
and stopped explicitly; the resulting MP4 was frame-extracted with `ffmpeg`. This provides a
visual oracle that the direct screenshot APIs denied.

Observed with the reference running under X11-forced mode:

- A reference pin created by `snip -o pin --area 100 100 500 300` appeared at (88,88) with a
  524x324 window. The extra 12 px around the 500x300 capture is the pin's shadow/border margin.
  The window was depth 32 and override-redirect, with `SKIP_TASKBAR`, `SKIP_PAGER`,
  `_KDE_NET_WM_WINDOW_TYPE_OVERRIDE`, and `_NET_WM_WINDOW_TYPE_NORMAL`.
- The pin was visibly **above** native Wayland windows (the Zed editor and terminal in the
  recording). This resolves the main stacking risk for X11 override-redirect pins on GNOME
  Wayland.
- A reference snipping overlay triggered with `snip --hold --area 100 100 500 300` covered the
  entire composited desktop and appeared above native Wayland windows. Its screenshot background
  was black because the reference was forced to use X11 capture, but its selection, toolbar, and
  tips were visible and it logged `Snipper: focus in`.

Evidence: OBS-023.

**Decision implication**: use a WM-managed fullscreen window for the interactive snip overlay
(keyboard focus works), and use override-redirect ARGB windows for pins. Transparent pins require
an alpha-capable GL configuration; on this Mali/XWayland device the hardware GLX path produced
depth 24, while software GL produced depth 32. This must be resolved with EGL or a dedicated ARGB
pin adapter before parity for pin shadows/opacity is claimed. Reference pins are topmost above
native Wayland windows on GNOME 49.

## 4. Overlay Interaction Model

Observed with `snip --hold --area 100 100 500 300`:

- Full-screen frozen overlay with a dimmed background.
- Selection rectangle with eight handles and a size badge (`500 x 300 px`).
- A horizontal annotation toolbar appears under the selection.
- A bottom-left keyboard-tip panel lists:
  - `W/A/S/D` move the cursor by one pixel.
  - Arrow keys move the snipping area by one pixel.
  - `Tab` toggles window/element detection.
  - `Ctrl+A` selects the active screen or the fullscreen.
  - `R` / `Shift+R` reuse the last snipping area(s).
  - `,` / `.` replay snipping history.
  - `` ` `` / `!` show or hide the captured mouse cursor.
- A magnifier popup shows a checkerboard transparency backdrop, a zoomed pixel grid, cursor
  coordinates, RGB/HEX values, and the hints “press C to copy color” and “press Shift to switch
  RGB/HEX.” The observed popup was 179x237 pixels at the cursor-adjacent position.
- The annotation toolbar was approximately 414x30 pixels under the selection and contained
  tool buttons (rectangle, ellipse, line/arrow, marker, text, mosaic, blur, eraser) plus
  undo/redo, close, pin, and output actions. The exact icon-to-command mapping needs a follow-up
  observation before it becomes a P1 requirement.

Evidence: OBS-008, OBS-009, OBS-010, OBS-011.

### 4.1 CLI and single-instance behavior

Runtime logs show that a second process connects to the first and parses the command line; the
first process owns the UI. The command line is ignored when the application is not already
running. This matches the embedded help text.

Observed command behavior:

- `snip` triggers a snip request.
- `snip --area X Y W H` preselects the rectangle.
- `--hold` keeps the annotation UI open after the area is chosen.
- Without `--hold`, a completed area is finalized immediately; the log recorded
  `Screenshot (x, y, w, h) destination: none`.
- The application logs a `Snipper: ready` timing and focus transitions.

The embedded help also documents `--size`, `--full`, `--last`, `--active-window`,
`--active-screen`, `--custom`, `--delay`, `--shadow`/`--no-shadow`, output destinations
(`pin`, `clipboard`, file path, quick-save, file-dialog, printer, success, exec), `--block`,
`paste`, `pick-color`, `toggle-images`, `show-images`, `hide-images`, click-through commands,
group commands, whiteboard, hotkey toggles, and `--config`. These are behavior references, not
requirements for the first MVP. Evidence: OBS-011.

## 5. Clipboard Architecture

The reference uses a D-Bus clipboard service named `com.Snipaste.ClipboardService`:

- `mimeData` retrieves the current payload.
- `setMimeData` publishes a payload.
- A `snipaste/id` field identifies the owner/serial and avoids self-paste loops.
- Image data is transferred as PNG (`snipaste/png`).
- Other supported payloads include `text/uri-list`, `text/html`, `text/plain`, and
  `application/x-color`.

On Wayland it starts a helper process next to the application binary with a Wayland platform
argument and redirects its stderr to `wlclip.txt`. This is consistent with the release notes about
KDE Plasma 6.5 clipboard compatibility and with the presence of both `LinuxClipboard` and
`WaylandClipboard` classes.

Evidence: OBS-012.

## 6. Configuration, History, Tray, and Logs

- Linux config path observed at runtime: `$HOME/.snipaste/config.ini` (in the analysis harness,
  `/tmp/.snipaste/config.ini`).
- History directory: `$HOME/.snipaste/history`, with a versioned header (observed version 201)
  and image groups/pasters. The reference restores pasters at launch.
- Log file: `$HOME/.snipaste/splog.txt`; log levels and rotation are configurable.
- The reference creates a 1x1 tray/helper window and uses a StatusNotifierItem-style tray. In the
  target GNOME session, `org.kde.StatusNotifierWatcher` was not available, so no visible tray is
  expected without an AppIndicator extension.
- Embedded help documents F1 as the default snip hotkey, F3 as paste, Shift+F3 as hide/show all
  images, C for color copy, and many in-app annotation shortcuts.

Evidence: OBS-013, OBS-014.

## 7. Annotation Model (class-level)

Recovered Qt RTTI/metaobject names identify the model and view families:

- Items: `PaintItem`, `RectEllipsePaintItem`, `ArrowPaintItem`, `LinePaintItem`,
  `LinestripPaintItem`, `PencilPaintItem`, `HightlightPaintItem`, `MosaicPaintItem`,
  `MagnifyPaintItem`, `NumberPaintItem`, `TextPaintItem`, `EraserPaintItem`, `SimplePaintItem`,
  `MaskItem`, `TooltipItem`, `ButtonContainerItem`.
- Views/controllers: `PaintScene`, `PaintView`, `PaintBar`, `Snipper`, `PasterController`,
  `SnipOptions`, `SnipType`.
- Window/monitor helpers: `ScreenGrabber_x11`, `ScreenGrabber_gnomewayland`,
  `ScreenGrabber_kwin`, `ScreenGrabber_wayland`, `MagneticWindow`, `WindowDetector`,
  `ThreadedWindowDetector`.
- Desktop helpers: `VirtualDesktop_linux`, `LinuxClipboard`, `WaylandClipboard`.

This is an architectural map, not a source reconstruction. It tells the Rust design which
concepts need independent equivalents (scene, commands/history, item kinds, toolbar, window
detection, clipboard adapter, virtual-desktop adapter).

Evidence: OBS-015.

## 8. Pin Interactions (documented behavior)

The embedded help and menu strings describe the following pin behavior:

- Pins are topmost floating image windows; mouse pass-through is off by default.
- Drag moves; wheel zooms; Ctrl+wheel adjusts translucency; middle click resets scale/opacity;
  1/2 rotate; 3/4 flip; Shift+double-click enters thumbnail mode; Esc/double-click closes;
  Ctrl+W closes; Shift+Esc destroys; Space starts annotation.
- The context menu exposes shadow, opacity, click-through, image operations, and close/destroy.
- Pins can be hidden/shown as a group; closed pins can be recovered by repeated paste until the
  closed-image limit is reached.
- Pins can be placed on all virtual desktops on supported platforms.

Evidence: OBS-014, OBS-007.

## 9. Measurements

### Reference under qemu/proot (emulated; not a native baseline)

- Process launch to “Launch time”: 6.0–6.5 s.
- `Snipper: ready`: ~625 ms first observed run; ~468 ms later run.
- `Screenshot source: x11` → background size: ~0.43 s.
- X11 capture of the XWayland root produced black content; the overlay UI itself is a 32-bit
  ARGB window and was captured successfully with a window-targeted X11 capture.
- Reference performance values are **emulated** and must not be used as acceptance thresholds.

### Native aarch64 spike (eframe/glow, XWayland)

- WM-managed fullscreen overlay first frame: 5–11 ms.
- Override-redirect pin first frame: 6–33 ms.
- X11 root capture: failed with `BadMatch`.
- Software GL pin transparency: depth 32, rounded transparent corners; default hardware GLX pin:
  depth 24, no alpha.
- Continuous repaint in the spike drove CPU to ~367%; this is a spike artifact and confirms that
  the product must be event-driven and idle without repaints.

Evidence: OBS-016..OBS-021.

## 10. Reproduction Harness

- Reference AppImage extracted with `unsquashfs` at the correct squashfs offset; the runtime
  binary runs under `qemu-x86_64` with an Ubuntu 26.04 amd64 rootfs.
- `proot -q qemu-x86_64` with `PROOT_NO_SECCOMP=1` provides the user-space rootfs; `-0` is needed
  only for package installation inside the rootfs.
- `DISPLAY=:0`, XWayland auth file, XDG runtime dir, and the X11 socket are bound into the rootfs.
- The first reference instance runs in the background; a second instance with `snip ...` triggers
  the UI. This matches the reference's single-instance design.
- Forced X11 mode (`XDG_SESSION_TYPE=x11`, unset `WAYLAND_DISPLAY`) was used to observe the
  overlay without the GNOME 49 portal permission dialog.
- Window-targeted captures were taken with Python-Xlib `GetImage`; root capture is black or
  rejected on XWayland.
- `gnome-shell --version` inside the qemu rootfs is not available, which changes the reference's
  backend selection; this was controlled by a temporary fake shell version only for experimentation
  and is not a supported product configuration.
- Native composited screenshots are obtained with `org.gnome.Shell.Screencast` from a GJS process
  that keeps the D-Bus connection alive, sleeps for the capture window, and calls
  `StopScreencast` on the same connection. Stopping from a different `gdbus` process produced a
  truncated MP4 without a `moov` atom. The finalized MP4 is frame-extracted with `ffmpeg`.
- Window-targeted X11 captures remain useful for UI details; the native screencast is used for
  stacking, coverage, and native-Wayland relationships.

Evidence: OBS-006, OBS-008, OBS-009, OBS-010, OBS-019, OBS-023.

## 11. Gaps and Confidence

| Gap | Impact | Confidence | Follow-up |
|---|---|---|---|
| GNOME 49 direct screenshot is denied; portal request waits for user action | Native Wayland capture path unverified | Medium (static + call behavior) | Interactive session test, permission-store inspection, or Xorg fallback |
| Native Wayland stacking | Resolved for GNOME 49 | High (native screencast) | Recheck on KDE and other compositors |
| Multi-monitor and mixed-DPI | Coordinate mapping unknown | Low | Add a monitor/scale test matrix in MVP |
| KDE/KWin ScreenShot2 | Backend unverified | Medium (static) | Test on a KDE session |
| Tray on GNOME | Tray may be invisible | High (no StatusNotifierWatcher) | Document AppIndicator requirement; CLI fallback |
| Global hotkey delivery on Wayland | Shortcut path uncertain | Medium (XGrabKey probe succeeded, delivery not proven) | Real keypress test; portal GlobalShortcuts test |
| Clipboard cross-app paste | End-to-end image exchange unverified | Medium (D-Bus protocol + arboard smoke test) | Paste into a native Wayland app; reverse direction |
| `paste --files` command | Parsed but produced no visible image window in the emulated harness | Low | Retest with a relative path and an x86_64 reference run; check whether this path requires Pro |
| Annotation input/focus | Automated interaction was unreliable under emulation | Low | Manual interaction or an x86_64 reference machine |
| Reference performance | Emulation skew | Low | Measure on an x86_64 machine |

## 12. Evidence Index

| ID | Evidence |
|---|---|
| OBS-001 | AppImage size/SHA-1; ELF and Qt inventory |
| OBS-002 | Backend factory decision tree; runtime source log |
| OBS-003 | GNOME Shell Screenshot D-Bus call shape |
| OBS-004 | KWin ScreenShot2 method/options/FD |
| OBS-005 | Portal Screenshot request behavior on GNOME 49 |
| OBS-006 | X11 grabber source and XWayland limitations |
| OBS-007 | Window flags, X11 atoms, XFixes input-shape logic |
| OBS-008 | Snipper runtime log sequence and CLI trigger |
| OBS-009 | Overlay screenshot: selection, toolbar, tips |
| OBS-010 | Magnifier/color popup screenshot |
| OBS-011 | Single-instance/CLI behavior and `--hold` |
| OBS-012 | `com.Snipaste.ClipboardService` protocol and Wayland helper |
| OBS-013 | Config/history/log paths and restoration |
| OBS-014 | Documented hotkeys and pin interactions |
| OBS-015 | RTTI/metaobject architecture map |
| OBS-016 | Spike fullscreen overlay focus/properties |
| OBS-017 | Spike pin properties and click-through input shape |
| OBS-018 | Spike ARGB depth with software GL + transparent root |
| OBS-019 | Spike X11 `GetImage` failure under XWayland |
| OBS-020 | Spike XGrabKey probe |
| OBS-021 | Spike latency and CPU observations |
| OBS-022 | Gap register (this section) |
| OBS-023 | GNOME Shell Screencast visual oracle; reference pin/overlay above native Wayland windows; reference pin 524x324 depth 32 |

## 13. Parity Matrix

The living matrix is in [`docs/parity/reference-2.11.3.md`](../../docs/parity/reference-2.11.3.md).
It is the input to the MVP specification and must be updated with every new observation.
