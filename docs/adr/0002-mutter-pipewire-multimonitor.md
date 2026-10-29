# ADR 0002: Mutter ScreenCast + PipeWire Capture and Per-Monitor Overlays

- **Status**: Accepted (implementation verified on single monitor; hardware multi-monitor
  validation pending)
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

7. **Scope this feature to uniform scale 1.0.**
   - Layout rectangles are normalized and uniformly scaled into the captured frame, which covers a
     frame larger than the logical layout.
   - Mixed per-monitor DPI, transform correction, and monitor hot-plug remain open.

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
- Multiple fullscreen viewports and focus transfer have not been validated on real dual-monitor
  hardware.
- Mixed/fractional DPI can misalign the raw frame and overlay coordinates.
- PipeWire development headers, libclang, and libLLVM are required at build time.
- Multi-monitor layouts whose primary monitor has a non-zero/negative origin need verification
  against Mutter's stage coordinates.

## Fallbacks

1. **PipeWire unavailable** → H.264 Screencast fallback with a warning.
2. **Both GNOME paths unavailable** → actionable error; use Xorg for pixel-exact capture.
3. **Focus transfer fails on real hardware** → fall back to a single active-monitor overlay and
   document the limitation.
4. **Mixed-DPI misalignment** → constrain the feature to 1x until a per-monitor correction lands.

## Evidence

- `specs/003-lossless-wayland-multimonitor/research.md` sections 1–9.
- GJS oracle: 13 raw RGBA frames in 500 ms at 1920x1080.
- Rust prototype: BGRx 1920x1080, stride 7680, 8,294,400 bytes.
- Product: 142–412 ms, full-screen PNG visually correct, `--doctor` reports `gnome-pipewire`.
- Xvfb: 3840x1080 with 2048 texture limit, exact `--area` crop.
- Simulated dual monitor: drag 900,500 → 1100,550 saved 200x50.
