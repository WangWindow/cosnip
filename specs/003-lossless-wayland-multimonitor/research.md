# Research: Lossless GNOME Wayland Capture and Multi-Monitor Selection

**Feature**: `003-lossless-wayland-multimonitor` | **Date**: 2026-10-29

## 1. Existing GNOME capture is lossy

Feature 002 used `org.gnome.Shell.Screencast` and decoded one frame from an H.264 MP4 with
`ffmpeg`. Measured command-to-overlay latency was 1.1–1.7 s, and the decoded pixels were not
guaranteed to match the compositor output.

Direct alternatives were tested on the target GNOME 49 session:

| API | Result |
|---|---|
| `org.gnome.Shell.Screenshot` | `AccessDenied: Screenshot is not allowed` |
| `org.freedesktop.portal.Screenshot` | Request created; backend calls Shell Screenshot with `permission_store_checked=true`; no `Response` |
| `org.gnome.Mutter.ScreenCast` | Works; returns a PipeWire node |
| `org.gnome.Shell.Screencast` | Works; H.264 MP4 |

The portal path is not usable for an instant snip because it is an interactive permission flow
and stalled in the automated session.

## 2. Mutter ScreenCast D-Bus sequence

`gdbus introspect` on the target showed `org.gnome.Mutter.ScreenCast` version 4 with
`CreateSession(a{sv}) -> o`. The working sequence is:

1. `CreateSession({})` on `/org/gnome/Mutter/ScreenCast`.
2. `RecordArea(x, y, width, height, {"is-recording": <false>, "cursor-mode": <uint32 0>})` on
   the returned session object.
3. Subscribe to `PipeWireStreamAdded(uint32)` on the returned stream object.
4. `Start()` on the session.
5. Read the node id from the signal.

Observed GJS result:

```text
session /org/gnome/Mutter/ScreenCast/Session/u39
stream  /org/gnome/Mutter/ScreenCast/Stream/u35
signal  PipeWireStreamAdded (uint32 94,)
```

`is-recording: false` avoids the persistent recording indicator. `cursor-mode: 0` hides the
cursor because Cosnip draws its own crosshair.

## 3. GJS + GStreamer oracle

A GJS script created the session and attached a GStreamer pipeline:

```text
pipewiresrc path=<node> ! videoconvert ! video/x-raw,format=RGBA ! filesink
```

It wrote 107,827,200 bytes in 500 ms: 13 frames at 1920x1080 RGBA (8,294,400 bytes each). The
first frame, rendered to PNG, matched the desktop. This proved that raw lossless frames were
available and that the issue was the Rust client, not Mutter.

## 4. Rust `pipewire` 0.10 client

The Rust prototype in `temp/pwtest` established these requirements:

- Create `MainLoopRc`, `ContextRc`, and `connect_rc`.
- Create a `StreamBox` with `MEDIA_TYPE=Video`, `MEDIA_CATEGORY=Capture`, `MEDIA_ROLE=Screen`.
- Register `param_changed` and `process` callbacks, then connect with
  `Direction::Input`, `Some(node)`,
  `StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS`.
- Do **not** set `TARGET_OBJECT`; doing so produced
  `state Paused -> Error("no target node available")` because the target is already passed to
  `connect`.
- Run the PipeWire main loop (`mainloop.run()`) while waiting. Blocking only on a channel kept
  the loop idle and timed out.
- `video/x-raw` negotiated as `VideoFormat::BGRx`, 1920x1080, stride 7680, 8,294,400 bytes.
- The process callback must read chunk metadata before taking a mutable data slice, or borrow
  checking fails.

The frame converter handles BGRx/BGRA/RGBx/RGBA, row stride padding, and validates the mapped
buffer length. DMA-BUF-only buffers are rejected with a clear error and fall back.

## 5. Product integration

The backend is enabled by default through the `cosnip-capture` `pipewire` feature. The measured
debug-build end-to-end runs on the target were:

```text
cosnip: captured 1920x1080 via gnome-pipewire in 142 ms
cosnip: captured 1920x1080 via gnome-pipewire in 179 ms
cosnip: captured 1920x1080 via gnome-pipewire in 261 ms
cosnip: captured 1920x1080 via gnome-pipewire in 300 ms
cosnip: captured 1920x1080 via gnome-pipewire in 337 ms
cosnip: captured 1920x1080 via gnome-pipewire in 412 ms
```

A full-screen PNG was saved and visually matched the desktop. No temporary media file and no
`ffmpeg` invocation occurred on this path.

Build requirements discovered on the target:

```text
libpipewire-0.3-dev, libspa-0.2-dev, libclang1-20, libllvm20
PKG_CONFIG_* and LIBCLANG_PATH must point at the development packages
BINDGEN_EXTRA_CLANG_ARGS must include the GCC include directory for stdbool.h
```

## 6. Multi-monitor geometry

RandR `GetMonitors` exposes one monitor per physical output on both Xorg and XWayland. Each
monitor has `(x, y, width, height)` in root coordinates.

Overlay options considered:

| Option | Result |
|---|---|
| One non-fullscreen window spanning the root | GNOME constrained it to the work area: `1920x1048+0+32`, so the top panel was not covered |
| One override-redirect overlay | No keyboard focus under XWayland |
| One fullscreen WM-managed viewport per monitor | Chosen; each monitor is fully covered and one viewport can take focus |

Global-coordinate design:

- The capture is the full root/desktop frame.
- A drag anchor and the selection are stored in frame pixels.
- Each viewport converts local pointer points to global points by adding its monitor origin.
- X11 implicit pointer grab delivers motion outside the originating window with out-of-window
  local coordinates, so a selection can continue onto another monitor.
- The viewport under the pointer requests focus; keyboard handling is gated to the focused
  overlay.
- The toolbar is drawn in the monitor containing the selection center; the magnifier and tips are
  drawn on the active monitor.

Monitor rectangles are normalized against the layout bounding box and scaled into the captured
frame. This also handles a frame larger than the logical layout (e.g. fractional scaling) as a
uniform scale. Mixed per-monitor DPI is not solved.

## 7. Texture limits

The first 3840x1080 Xvfb smoke run panicked inside egui:

```text
Texture "capture" has size 3840x1080, but the maximum texture side is 2048
```

The fix is to split the frame into tiles no larger than `min(max_texture_side, 2048)` and draw the
tiles intersecting each monitor. After the change the same 3840x1080 test passed and saved the
exact 500x300 `--area` crop.

## 8. Verification evidence

| Check | Result |
|---|---|
| GNOME 49 1920x1080 PipeWire capture | 142–412 ms, `gnome-pipewire` |
| Full-screen save | 1920x1080 PNG, visually correct |
| BGRx conversion unit test | pass |
| Short-buffer rejection | pass |
| Monitor layout unit tests | pass (empty, side-by-side, negative origin, scaled frame, center monitor) |
| Xvfb 3840x1080 with 2048 texture limit | pass, exact 500x300 crop |
| Simulated 960+960 monitors on Xvfb, drag 900,500 → 1100,550 | saved 200x50 across the 960 boundary |
| Escape in the second overlay viewport | closed the app under Xvfb focus |
| Single-monitor drag 300,300 → 600,450 | saved 300x150 |
| Pin from `--area 300 200 500 300` | 500x300 pin at 300,200; Escape closed it |
| 3840x1080 pin on the 2048-limit stack | pin window created without a texture panic |
| Real dual-monitor hardware | **not available; pending** |
| Mixed/fractional DPI | **not verified; deferred** |

## 9. Open gaps

1. Real multi-monitor focus transfer and window placement on GNOME must be validated when
   hardware is available.
2. Monitor layouts with the primary not at the origin should be checked against Mutter's
   `RecordArea` coordinate space.
3. Fractional scaling can make the raw stream larger than the RandR layout; the current uniform
   scale mapping is a best effort, not per-monitor DPI correction.
4. Portal ScreenCast remains an option for non-GNOME or future permission-based capture.
