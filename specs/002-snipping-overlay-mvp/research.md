# Research: Snipping Overlay MVP

**Feature**: `002-snipping-overlay-mvp`
**Date**: 2026-10-29
**Depends on**: `001-reference-parity-foundation` (OBS-001..OBS-025)
**Status**: Decisions made; implementation research open items listed at the end

This document records only what the MVP needs beyond the M0 dossier. The reference behavior and
clean-room evidence live in `specs/001-reference-parity-foundation/research.md`.

## Decisions

### D-001 — Capture backend split

- Xorg/X11: native X11 `GetImage` (XShm later if profiling requires it).
- GNOME 49 Wayland: `org.gnome.Shell.Screencast` single-frame fallback.
- Other Wayland desktops: unsupported in this MVP with an actionable error.
- XWayland root capture is not a backend.

Rationale: GNOME 49 denies Shell Screenshot and the portal backend's Shell call is denied; the
Screencast service is the only public path that worked on the target. X11 capture worked on
Xvfb/Xorg but not on XWayland.

### D-002 — Screencast single-frame sequence

Measured on the target:

| Recording window | Video duration | Frames | Total process time |
|---|---|---|---|
| 300 ms | 0.41 s | 11 | ~1.03 s |
| 700 ms | 0.82 s | 10 | ~1.42 s |

The capture must keep one D-Bus connection alive: call `Screencast`, sleep briefly, call
`StopScreencast` on the same connection. Stopping from another `gdbus` process produced an MP4
without a `moov` atom. Use the default H.264 pipeline and decode one frame with `ffmpeg` for the
MVP. Use a unique runtime directory and clean it on every exit path.

The GNOME Screencast output is H.264 MP4, 1920x1080, 30 fps in the tested environment. The
recording indicator may appear briefly; this is an accepted MVP limitation.

### D-003 — Overlay window

Use a WM-managed fullscreen X11 window (eframe forced to X11). The M0 spike showed this receives
keyboard focus (`_NET_WM_STATE_FULLSCREEN`, `ABOVE`, `FOCUSED`), while an override-redirect
fullscreen window did not. The overlay captures a frame first, then shows; it must not block the
UI thread while capturing.

### D-004 — Pin window

Use an image-sized override-redirect X11 window with `UTILITY` type, no decorations, no taskbar,
and always-on-top. Image-size avoids a black shadow margin when the GL config lacks alpha.
Alpha/shadow/opacity are enhancements for a later feature; on this device the hardware GLX path
produced depth 24, while software GL produced depth 32.

### D-005 — Clipboard lifetime

`arboard` images persist after the owning X11 process exits on the target GNOME session: an
80x60 image owned by a short-lived X11 process was read by a native Wayland GTK4 app after the
owner exited. On a bare X11 session without a clipboard manager, persistence is not guaranteed;
the MVP copies and exits, and documents that a clipboard manager is recommended. If this proves
unreliable, add a bounded clipboard-holder mode in a later feature.

### D-006 — UI scope

No annotation engine, text editing, toolbar customization, tray, global hotkeys, settings UI,
history, or multi-monitor. The overlay has a fixed toolbar with Copy, Save, Pin, and Cancel.
Magnifier and color readout are included because they are part of region selection, not
annotation.

### D-007 — Defaults

- Default output: clipboard.
- Default save path: `$XDG_PICTURES_DIR/Cosnip_<timestamp>.png` (fallback `$HOME/Pictures`).
- `--full` selects the whole captured desktop.
- `--area X Y W H` preselects and shows the overlay.
- `--delay MS` waits before capture.
- Exit codes: 0 success; 2 usage; 3 display/backend unavailable; 4 capture failed; 5 output
  failed; 6 cancelled by user.

## Open Items for Implementation

1. Verify a 1x1 override-redirect/transparent root window keeps eframe's viewport loop alive and
   hidden from the taskbar.
2. Verify `ffmpeg` raw RGBA decode invocation and frame-size validation on the target.
3. Verify clipboard persistence with a 1920x1080 PNG and a native Wayland app.
4. Verify wheel-zoom window resizing and drag positioning for override-redirect pins under
   XWayland.
5. Multi-monitor and mixed-DPI are explicitly out of scope; detect and report.
6. Portal/KDE are out of scope; the backend error must say which backend is unsupported.

## Evidence to Reuse

- OBS-016..OBS-021: overlay/pin/click-through/ARGB/performance.
- OBS-023: native Wayland stacking with reference pin/overlay.
- OBS-024: X11 → native Wayland image clipboard.
- OBS-025: X11/Xvfb capture and reference overlay.

## Risks

- Screencast capture may fail if the GStreamer pipeline blocklist contains all encoders, or if
  the recording service is not available. Report a clear error; do not fall back to a black X11
  capture.
- The recording indicator is a visible difference from a silent screenshot; document it.
- `ffmpeg` is an external runtime; `--doctor` must detect it and the packaging docs must list it.
- The target's hardware GLX may not give alpha pins; the MVP's image-sized opaque pins are still
  functional but lack shadows/rounded corners.

## Implementation Results (2026-10-29)

- **X11 release capture**: 12–35 ms for 1920x1080 under Xvfb; the overlay is 1920x1080 and the
  debug smoke test captured/saved a 500x300 PNG.
- **GNOME Screencast release capture**: 1.07–1.56 s for 1920x1080; a decoded frame showed real
  native desktop content; the temporary capture directory was removed.
- **Overlay on GNOME**: 1920x1080 depth 32; `_NET_WM_STATE_FULLSCREEN`, `ABOVE`, `FOCUSED`;
  window-targeted and composited-screencast captures show the real desktop, selection, toolbar,
  magnifier, size badge, and keyboard tips. A mouse drag from (100,100) to (600,400) produced an
  exact 500x300 output; the canvas anchors at the pointer-press frame, not at egui's later drag
  threshold. Toolbar Copy was exercised with a real synthetic click and exited successfully.
- **Clipboard**: keeping `ClipboardOwner` alive plus an 800 ms handoff delay let a focused native
  Wayland GTK4 reader read the exact 500x300 image after process exit. Dropping the clipboard owner
  immediately after `set_image` caused a “no compatible transfer format” failure.
- **Pin**: 500x300 at (100,100), depth 32, override-redirect, `UTILITY`, skip-taskbar/pager;
  native composited screencast shows it above the browser; drag moved exactly to (200,200); wheel
  zoom reached 945x567; the close button exited the process; idle CPU was 0% over a 2 s sample.
- **Tests**: 10 core + 4 imaging + 2 platform + 4 CLI tests pass; ignored Xvfb capture and
  save-smoke tests pass when run with `DISPLAY=:99`; `cargo fmt` and
  `cargo clippy --all-targets --all-features -- -D warnings` pass.
- **Portability**: core/imaging/capture/platform check for `x86_64-unknown-linux-gnu`; UI/app were
  built and tested on aarch64. A full x86_64 UI build needs x86_64 X11/GL development libraries.
- **Known gap**: the GNOME fallback records for ~500 ms and may show a recording indicator; a
  failed zbus start can block without an outer timeout. Multi-monitor is rejected by the MVP guard.
