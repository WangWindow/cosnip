# ADR 0001: XWayland Windowing and Native Capture Architecture

- **Status**: Accepted (project-owner review, 2026-10-29)
- **Date**: 2026-10-29
- **Feature**: `001-reference-parity-foundation`
- **Deciders**: Project owner, implementation team

## Context

Cosnip targets Linux on aarch64 and x86_64. The primary device is GNOME 49 Wayland with
XWayland. Snipaste 2.11.3 is the behavioral reference and uses a Qt xcb UI with compositor/portal
capture paths. We need a window and capture architecture that preserves the core snip/pin
experience without copying the reference implementation.

Two platform facts constrain the design:

- Native Wayland clients cannot position top-level windows arbitrarily or request always-on-top
  in a portable way; GNOME does not support wlr-layer-shell.
- Pure X11 capture cannot see native Wayland surfaces; direct `GetImage` on the XWayland root
  fails with `BadMatch`.

The M0 spike compared a WM-managed fullscreen overlay, an override-redirect overlay, and
override-redirect ARGB pins on the target device. Evidence: OBS-016 through OBS-021.

## Decision

1. **Snip overlay: WM-managed fullscreen X11/XWayland window.**
   - It is created as a borderless fullscreen viewport on the target monitor.
   - It receives keyboard focus (`_NET_WM_STATE_FOCUSED`, active window) and is therefore suitable
     for Escape, shortcuts, and text input.
   - It must hide the system cursor and draw its own crosshair/selection UI.

2. **Pinned images: override-redirect ARGB X11 windows.**
   - Frameless, always-on-top, skip-taskbar, skip-pager; use a non-taskbar window type (spike: `UTILITY`; reference observed: override/normal).
   - Position is controlled by the application; the WM does not manage the window.
   - Click-through uses an empty X11 input shape (`XFixesSetWindowShapeRegion`/ShapeInput), not a
     boolean Qt flag.
   - Pins use a 32-bit ARGB visual. On this Mali/XWayland host, the default hardware GLX path
     produced depth 24; an alpha-capable configuration (software GL in the spike) produced depth
     32. The product must either find an EGL/alpha-capable config or use a dedicated ARGB pin
     renderer before promising shadows/opacity parity.
   - GNOME 49 was visually verified with a native composited screencast: the reference's
     524x324 pin (500x300 capture plus a 12 px shadow margin) and its fullscreen overlay both
     appear above native Wayland windows. The pin is depth 32, override-redirect, and hidden from
     the taskbar/pager.

3. **Capture backend is a platform adapter, not a windowing detail.**
   - Xorg session: native X11/XShm-style capture.
   - GNOME ≥41 Wayland: freedesktop portal Screenshot (blocked in the tested GNOME 49 session);
     the MVP uses GNOME Shell Screencast single-frame capture, which is lossy H.264. A
     PipeWire/Mutter ScreenCast backend is the planned lossless path.
   - GNOME ≤40: GNOME Shell Screenshot D-Bus (reference behavior).
   - KDE: KWin ScreenShot2 with PipeWire.
   - XWayland root capture is explicitly unsupported and must not be used as a fallback for
     Wayland capture.

4. **Global shortcuts are platform-specific.**
   - X11/XWayland UI: XGrabKey path (probe succeeded; delivery still needs a real-key test).
   - Wayland: freedesktop GlobalShortcuts portal or a desktop-environment shortcut that invokes
     the CLI. A missing portal is not a reason to block the core feature.

5. **Clipboard starts with the X11/XWayland bridge, with a native helper as a likely follow-up.**
   - A focused native Wayland GTK4 reader successfully read an image set by the X11 `arboard`
     spike (X11 → Wayland passes).
   - Native Wayland → X11 image read returned stale data in the smoke test; the reference ships a
     dedicated Wayland clipboard helper. Plan for that helper if paste-from-native-Wayland is
     required (it is for full parity).

## Alternatives Considered

| Alternative | Why rejected |
|---|---|
| Native Wayland UI + layer-shell | GNOME does not support layer-shell; exact pin position/always-on-top cannot be delivered on the primary target |
| Pure X11 application | Cannot capture native Wayland windows; fails the primary session |
| Override-redirect overlay | No keyboard focus under XWayland in the spike; keyboard-driven snip would break |
| Fullscreen WM-managed pins | WM constraints and taskbar/focus behavior do not match pinned-image behavior |
| eframe/wgpu renderer | Device reported `max_texture_dimension_3d = 512` while egui requested 2048; not viable on this host |
| Fully custom X11 renderer for the whole UI | Maximum control, but discards egui text/IME/layout and increases MVP risk; keep as a fallback for ARGB pins only |

## Consequences

**Positive**

- The primary target gets a focusable fullscreen overlay and precisely positioned pins.
- Capture is independent of the UI path, matching the reference architecture.
- The design is testable with X11 window properties, XShape queries, and window-targeted captures.
- No proprietary code or assets are required.

**Negative / Risks**

- Native Wayland stacking is verified on GNOME 49, but not yet on KDE or other compositors; their
  stacking policy may differ.
- Transparent pins depend on an alpha-capable GL configuration. Software GL works but may be a
  performance and packaging burden.
- GNOME 49 portal capture requires a user decision in this environment; silent capture is not
  proven.
- Tray on GNOME needs an AppIndicator extension; CLI/hotkey access must work without a tray.
- X11/XWayland global hotkey delivery is not proven; a desktop-shortcut fallback is required.

## Fallbacks

1. **Pin stacking fails on GNOME Wayland**: document Xorg as the high-parity session; keep pins
   best-effort on Wayland.
2. **Alpha pins impossible with the GL stack**: implement a dedicated ARGB X11 pin adapter with
   tiny-skia/softbuffer, or accept opaque pins with shaped rounded corners as a documented
   difference.
3. **Portal capture blocked**: use an Xorg session or a user-granted compositor-specific path;
   never fall back to XWayland root capture.
4. **Global shortcut delivery fails**: require a desktop shortcut that invokes `cosnip snip`.

## Evidence

- OBS-002..OBS-007 (backend/window/clipboard architecture)
- OBS-009..OBS-011 (overlay/CLI behavior)
- OBS-016..OBS-021 (spike results)
- OBS-022 (gap register)
- OBS-023 (native screencast: reference pin/overlay above native Wayland windows)

## Review Gate

This ADR is accepted only after the project owner reviews the M0 dossier. The next product plan
must include explicit acceptance tests for:

- Pin stacking above a native Wayland window.
- Pin transparency depth and opacity behavior.
- GNOME 49 portal capture with a real user decision.
- Global shortcut delivery, or the documented CLI fallback.
