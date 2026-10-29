# Implementation Plan: Lossless GNOME Wayland Capture and Multi-Monitor Selection

**Branch**: `003-lossless-wayland-multimonitor` | **Date**: 2026-10-29 | **Spec**:
[spec.md](spec.md)

## Summary

Add a direct `org.gnome.Mutter.ScreenCast` backend that reads raw PipeWire frames, and extend the
overlay from one fullscreen viewport to one viewport per RandR monitor with global selection
coordinates. Keep the Feature 002 H.264 Screencast fallback as a safety net. Split the captured
frame into GL-texture-sized tiles so wide desktops work on stacks with a 2048-pixel texture limit.

This is an incremental feature: the CLI, selection model, clipboard, save, and pin paths are
unchanged. The risk is concentrated in platform capture, PipeWire lifetime/negotiation, and
multi-viewport input mapping.

## Technical Context

**Language/Version**: Rust stable, edition 2024, MSRV 1.95; validated on aarch64 and expected to
build on x86_64.

**Primary Dependencies**:

- `pipewire` 0.10.1 (MIT) with feature `v0_3_44`, plus transitive `libspa`/`pipewire-sys`.
  Build-time system requirements: `libpipewire-0.3-dev`, `libspa-0.2-dev`, libclang, libLLVM.
  Runtime: PipeWire and a GNOME/Mutter session.
- `futures-lite` 2.6 (Apache-2.0 OR MIT) for signal-stream polling, `async-io` 2.6 for the
  D-Bus timeout.
- Existing `zbus` 6.1 (MIT) for Mutter D-Bus calls and signals.
- Existing `eframe`/`egui` 0.37, `winit` 0.30, `x11rb` 0.13, `image` 0.25, `arboard` 3.
- `ffmpeg` remains an optional external runtime for the fallback only.

**Platform**: GNOME 49 Wayland + XWayland on aarch64; X11/Xorg; Xvfb for automated layout tests.
Hardware for this iteration: one 1920x1080 monitor. Multi-monitor is validated with a simulated
960+960 layout across two overlay windows; real dual-monitor hardware validation remains open.

**Capture flow**:

1. Detect GNOME Wayland in `detect_backend`; choose `GnomePipewire`.
2. `MutterSession::start` opens a session, records `(0,0,root_width,root_height)`, subscribes to
   `PipeWireStreamAdded`, starts the session, and reads the node id with a 3 s timeout.
3. `pipewire_capture` creates a PipeWire main loop/context/core and a capture stream.
4. Negotiate `video/x-raw` with BGRx/BGRA/RGBx/RGBA and the desktop size.
5. Process the first buffer, convert to RGBA, quit the loop.
6. Stop the Mutter session; on any failure print a warning and run the H.264 fallback.

**Overlay flow**:

1. Enumerate RandR monitors with `x11rb` and normalize them into frame pixels via `fit_monitors`.
2. Create one fullscreen egui viewport per monitor (`with_fullscreen(true).with_monitor(i)`).
3. Each viewport draws its monitor's slice of the frozen frame and converts local pointer positions
   to global frame coordinates.
4. The viewport under the pointer requests keyboard focus; the drag anchor is global, so a
   pointer grab that leaves the originating window continues the selection.
5. A toolbar appears in the monitor containing the selection center; the magnifier and tips appear
   on the active monitor.
6. All overlay viewports close together on confirm, cancel, or error.

**Texture strategy**: `with_context` splits the frame into tiles of at most
`min(max_texture_side, 2048)` pixels per side. Each viewport draws tiles intersecting its monitor
with per-tile UVs. A 3840x1080 frame therefore uses two 2048x1080 tiles on a 2048-limit stack.

**Testing**:

- Unit tests for BGRx conversion, short-buffer rejection, monitor normalization, scaling, negative
  origins, and selection-center monitor choice.
- Xvfb smoke with a 3840x1080 screen and a 2048 texture limit verifying exact `--area` output.
- Simulated dual-monitor test with 960x1080 overlays and `xdotool` drag across the boundary.
- Manual GNOME test for capture latency and pixel fidelity; real dual-monitor checklist pending.

**Constraints**:

- Direct Mutter ScreenCast is a GNOME-session API, not a portal; it may change between GNOME
  releases and is not a cross-desktop standard.
- No DMA-BUF import yet; memory-mapped buffers only.
- Uniform scale 1.0 baseline; mixed DPI is documented as approximate.
- No `unsafe`; the PipeWire crate owns FFI.
- No network, no telemetry, no reference assets.

## Constitution Check

*GATE: Must pass before implementation and re-check after design.*

- **I. Personal, Lawful Use Only**: PASS. No reference code or assets; behavior only.
- **II. Behavioral Reference, Original Implementation**: PASS. Observation-driven API sequence;
  all code original Rust.
- **III. Rust Workspace with Isolated Platform Edges**: PASS. Capture stays in `cosnip-capture`;
  the UI consumes a monitor-layout value type; no `unsafe`; new dependencies are MIT/Apache-2.0
  and recorded in `docs/dependencies.md`.
- **IV. Test-First and Evidence-Based Fidelity**: PASS. Conversion/layout unit tests, Xvfb tiled
  render test, simulated cross-monitor drag, measured latency, parity matrix updates.
- **V. Local-Only Privacy and Operational Restraint**: PASS. Raw frames stay in memory; no temp
  media; no network; session/stream closed on all paths.

No violations. Additional build dependencies (PipeWire headers, libclang) are justified by the
lossless requirement and are build-time only.

## Project Structure

```text
specs/003-lossless-wayland-multimonitor/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
└── tasks.md

crates/cosnip-capture/src/
├── lib.rs          # backend enum, detection, monitor enumeration
├── mutter.rs       # Mutter ScreenCast + PipeWire backend
└── gnome.rs        # H.264 fallback (unchanged)

crates/cosnip-ui/src/app.rs        # multi-monitor overlay, tiles, global coordinates
crates/cosnip-app/src/main.rs      # no single-monitor guard, doctor monitor list
docs/adr/0002-...                  # capture and overlay decision record
```

**Structure Decision**: Keep the PipeWire adapter inside the existing capture crate and behind the
existing `CaptureBackend` trait. The monitor layout remains a plain data type so the UI and
diagnostics share it without depending on PipeWire.

## Phase Plan

### Phase 0 — Research

1. Confirm Mutter ScreenCast D-Bus sequence on GNOME 49.
2. Prove raw PipeWire frames with GJS/GStreamer.
3. Prove the same stream from Rust `pipewire` 0.10, including target selection and main-loop
   requirements.
4. Measure end-to-end latency and identify texture-size limits.

### Phase 1 — Lossless backend

1. Add optional PipeWire dependencies and the `gnome-pipewire` backend variant.
2. Implement session start/signal/stop, stream negotiation, frame conversion, and timeout.
3. Enable by default; retain the Screencast fallback.
4. Add conversion unit tests and doctor output.

### Phase 2 — Multi-monitor

1. Add RandR monitor enumeration and virtual-bounds screen info.
2. Remove the single-monitor guard; pass the layout into the UI.
3. Normalize monitor rectangles into frame pixels.
4. Create one fullscreen overlay per monitor.
5. Convert pointer/selection coordinates per monitor; route toolbar, magnifier, tips, and focus.
6. Tile the frozen frame for texture limits.

### Phase 3 — Verification and documentation

1. Unit tests, Xvfb smoke, simulated dual-monitor drag.
2. `cargo fmt`, `clippy -D warnings`, full tests.
3. Update README, dependencies, parity matrix, ADR, and Feature 002 supersession note.
4. Record the remaining hardware/fractional-DPI gap.

## Complexity Tracking

| Decision | Why needed | Simpler alternative rejected |
|---|---|---|
| Direct Mutter ScreenCast | Only lossless, low-latency path available on GNOME 49 | Portal Screenshot is blocked/needs interaction; Shell Screencast is H.264 |
| Rust PipeWire client | No external process, raw frames, exact control | GStreamer subprocess adds runtime dependency and parsing overhead |
| One viewport per monitor | WM-managed fullscreen is the only focusable XWayland overlay; a single non-fullscreen window is clamped below the GNOME panel | Single spanning window failed geometry test (1920x1048+0+32) |
| Tiled textures | GL max texture side can be 2048 on software/older stacks | One texture panics on wide desktops |
| H.264 fallback retained | PipeWire may be unavailable or restricted | Removing fallback would regress Feature 002 |

## Risks

- **Mutter API stability**: Direct ScreenCast is GNOME-specific. Mitigation: isolate in one
  adapter and keep fallback.
- **Focus transfer between fullscreen viewports**: verified logic on simulated windows; real
  multi-monitor WM behavior is unverified. Mitigation: pointer monitor requests focus and keyboard
  handling is gated to the focused overlay.
- **Mixed DPI**: frame dimensions and RandR logical coordinates may diverge. Mitigation: layout is
  scaled into the frame; per-monitor DPI remains documented as unsupported.
- **Build dependency weight**: bindgen/libclang needed at build time. Mitigation: record in
  dependencies and document installation.
