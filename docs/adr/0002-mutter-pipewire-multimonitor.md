# ADR 0002: Mutter ScreenCast + PipeWire Capture and Per-Monitor Overlays

- **Status**: Accepted (validated on mixed-scale dual-monitor GNOME 49 hardware)
- **Date**: 2026-10-29
- **Feature**: `003-lossless-wayland-multimonitor`
- **Deciders**: Project owner, implementation team
- **Supersedes in part**: ADR 0001 §3 (GNOME capture) and the single-monitor scope of Feature 002

## Context

ADR 0001 selected a WM-managed fullscreen XWayland overlay and native X11 capture, with a planned
PipeWire/Mutter ScreenCast backend for GNOME Wayland. Feature 002 shipped with an H.264
`org.gnome.Shell.Screencast` fallback because the Shell and portal screenshot APIs were denied.

Two user-visible limitations followed:

1. GNOME Wayland captures were lossy and took 1.1–1.7 s.
2. Cosnip rejected multi-monitor layouts, while the reference product selects across the virtual
   desktop.

Mutter ScreenCast is available in the same session and exposes raw PipeWire frames. The question
was how to consume it without a subprocess, and how to extend the overlay to multiple monitors
without losing keyboard focus.

## Decision

1. **Use `org.gnome.Mutter.ScreenCast` directly on GNOME Wayland.**
   - `CreateSession` → `RecordArea(0,0,virtual_width,virtual_height)` → `Start` →
     `PipeWireStreamAdded`.
   - Record with `is-recording: false` and `cursor-mode: 0`.
   - Read raw `video/x-raw` frames from the node through the Rust `pipewire` 0.10 crate.
   - Convert BGRx/BGRA/RGBx/RGBA to RGBA in-process; no ffmpeg, no temporary media.
   - Run the PipeWire main loop while waiting for the first frame.

2. **Connect to the node by passing it to `Stream::connect(Some(node), ...)`.**
   - Do not set `TARGET_OBJECT`; doing both produces `no target node available`.
   - Use `AUTOCONNECT | MAP_BUFFERS` and accept CPU-mappable buffers only.

3. **Keep the H.264 Screencast fallback.**
   - If the Mutter session, signal, stream, or conversion fails, log a lossy-fallback warning and
     run the existing `org.gnome.Shell.Screencast` path.
   - The fallback is never silent.

4. **Create one fullscreen overlay viewport per RandR monitor.**
   - A single window spanning the root is clamped by GNOME to the work area
     (`1920x1048+0+32` in the geometry test), leaving the panel uncovered.
   - An override-redirect overlay does not receive keyboard focus on XWayland.
   - Per-monitor WM-managed fullscreen windows give full coverage and focus; the overlay under the
     pointer requests focus.

5. **Store selection and pointer state in global captured-frame pixels.**
   - Each viewport maps local pointer positions to global coordinates using its monitor origin.
   - The X11 implicit pointer grab continues a drag outside the originating window, so one
     selection can span monitors.
   - The toolbar follows the monitor containing the selection center; the magnifier/tips follow the
     active monitor.

6. **Tile the frozen frame for GL texture limits.**
   - Tiles are at most `min(max_texture_side, 2048)` pixels per side.
   - This avoids `Texture "capture" has size 3840x1080, but the maximum texture side is 2048` on
     software/older GL stacks.

7. **Start with uniform scale 1.0 (superseded by the mixed-scale addendum below).**
   - Layout rectangles are normalized and uniformly scaled into the captured frame, which covers a
     frame larger than the logical layout.
   - Mixed per-monitor DPI, transform correction, and monitor hot-plug remain open.

## Addendum: Mixed-Scale Capture (2026-10-29)

Hardware validation on a two-monitor layout exposed a Mutter limitation not visible in the
simulated 1x tests:

- The desktop mixed a 4096x2304 monitor at compositor scale 1.25 with a 2880x1800 monitor at scale
  2.0. XWayland exposed a 6976x2304 root (logical stage x 2).
- `RecordArea(0, 0, logical_width, logical_height)` returned a 6976x2304 frame, but only the
  primary stage view contained pixels; the second monitor was black. The area stream paints with
  the maximum overlapping view scale and does not composite a mixed-scale area correctly.
- Passing the XWayland root size to `RecordArea` was also wrong: Mutter expects logical stage
  coordinates, so the returned frame was 13952x4608 with the desktop packed into the top-left
  quarter.

The implementation now:

1. Reads the RandR monitor connectors and records each monitor with `RecordMonitor` in one Mutter
   session, collecting all PipeWire nodes before `Start`.
2. Scales each monitor frame into its XWayland monitor rectangle and composes one virtual-desktop
   frame. Uniform-scale layouts hit the identity path; mixed-scale monitors are bilinearly
   resampled into the common canvas.
3. Keeps the full-stage `RecordArea` as a fallback, using logical stage bounds from
   `org.gnome.Mutter.DisplayConfig.GetCurrentState`, then falls back to H.264.
4. Uses the global X11 pointer during overlay drags so a selection continues across monitor
   boundaries in either direction, and guards against a synthetic press in the destination
   viewport restarting the selection.
5. Converts pin coordinates, texture rectangles, zoom size, and drag deltas from frame pixels to
   viewport points, fixing pins that previously opened at double coordinates on 2x XWayland.

The common-canvas composition is a deliberate tradeoff: a monitor below the global XWayland scale
is upsampled, so saved pixels are not native for that monitor. True per-monitor native-resolution
composition remains future work.

## Alternatives Considered

| Alternative | Why rejected |
|---|---|
| GStreamer subprocess/pipeline | Adds a runtime dependency, parsing, and process lifecycle; Rust PipeWire is direct |
| Portal ScreenCast | Requires an interactive user decision and restore tokens; not suitable for instant snip |
| Keep H.264 only | Fails pixel fidelity and latency goals |
| Single spanning overlay window | GNOME clamps it to the work area; cannot cover the panel |
| Override-redirect spanning overlay | No keyboard focus under XWayland |
| One window per monitor without fullscreen | Panels/work areas differ per monitor and coverage is not guaranteed |
| One giant texture | Fails on 2048-limit GL stacks |
| `TARGET_OBJECT` plus target argument | Mutter/pipewire reports no target node |

## Consequences

**Positive**

- GNOME Wayland capture is lossless and measured at 142–412 ms in debug builds.
- Multi-monitor selection works in global coordinates; simulated dual-monitor drag was exact.
- Wide desktops render on weak GL stacks through tiled textures.
- The capture path no longer writes temporary media or invokes ffmpeg on success.
- The fallback preserves the Feature 002 behavior on restricted systems.

**Negative / Risks**

- Direct Mutter ScreenCast is GNOME-specific and may change; the adapter and fallback contain the
  risk.
- Multiple fullscreen viewports and focus transfer are validated on the mixed-scale test desktop;
  other compositor/WM combinations remain to be checked.
- Mixed-scale layouts use per-monitor streams and a common XWayland-scale canvas. A monitor below
  the global XWayland scale is upsampled, so native per-monitor output is still future work.
- PipeWire development headers, libclang, and libLLVM are required at build time.
- Multi-monitor layouts whose primary monitor has a non-zero/negative origin still need a separate
  hardware configuration; the validated desktop has its primary at (0,0) and a non-zero secondary
  origin.

## Fallbacks

1. **PipeWire unavailable** → H.264 Screencast fallback with a warning.
2. **Both GNOME paths unavailable** → actionable error; use Xorg for pixel-exact capture.
3. **Focus transfer fails on real hardware** → fall back to a single active-monitor overlay and
   document the limitation.
4. **Mixed-scale area-stream misalignment** → per-monitor `RecordMonitor` streams composed in
   XWayland coordinates; a common-canvas resampling limitation is documented.

## Evidence

- `specs/003-lossless-wayland-multimonitor/research.md` sections 1–9.
- GJS oracle: 13 raw RGBA frames in 500 ms at 1920x1080.
- Rust prototype: BGRx 1920x1080, stride 7680, 8,294,400 bytes.
- Product: 142–412 ms, full-screen PNG visually correct, `--doctor` reports `gnome-pipewire`.
- Xvfb: 3840x1080 with 2048 texture limit, exact `--area` crop.
- Simulated dual monitor: drag 900,500 → 1100,550 saved 200x50.
- Real mixed-scale dual monitor: DP-2 4096x2304 at (0,0), eDP-1 2880x1800 at (4096,338),
  XWayland desktop 6976x2304. Full capture saved 6976x2304 with both slices; 300x100 drags
  across the boundary saved in both directions; pin at (4000,1000) opened 300x200 at the right
  position. Capture-to-frame was 344–520 ms in release builds.
