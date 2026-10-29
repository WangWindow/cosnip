# Tasks: Lossless GNOME Wayland Capture and Multi-Monitor Selection

**Feature**: `003-lossless-wayland-multimonitor`

Legend: `[x]` complete, `[ ]` pending. Hardware and mixed-DPI items are explicitly left open.

## Phase 1 — Lossless capture research

- [x] T001 Record the GNOME 49 screenshot/portal denial and ScreenCast success evidence.
- [x] T002 Prove raw frames with GJS + GStreamer (`PipeWireStreamAdded` → `pipewiresrc`).
- [x] T003 Prove a Rust `pipewire` 0.10 client can read BGRx 1920x1080 at stride 7680.
- [x] T004 Identify the `TARGET_OBJECT` and main-loop pitfalls and record them in research.md.
- [x] T005 Measure capture-to-frame latency on the target.

## Phase 2 — Lossless capture implementation

- [x] T010 Add optional `pipewire`, `futures-lite`, and `async-io` dependencies and a default
  `pipewire` feature to `cosnip-capture`.
- [x] T011 Add `CaptureBackendKind::GnomePipewire` and backend selection.
- [x] T012 Implement `CreateSession`, `RecordArea`, signal subscription, `Start`, and `Stop`.
- [x] T013 Implement PipeWire stream creation, raw format negotiation, and first-frame capture.
- [x] T014 Convert BGRx/BGRA/RGBx/RGBA to RGBA with stride and length validation.
- [x] T015 Add a 3 s signal/stream timeout and H.264 fallback on failure.
- [x] T016 Add unit tests for conversion, stride, and short buffers.
- [x] T017 Report the PipeWire backend in `--doctor`.

## Phase 3 — Multi-monitor capture and overlay

- [x] T020 Add `MonitorInfo` and `monitors()` via RandR.
- [x] T021 Capture the full virtual desktop root rather than a single monitor.
- [x] T022 Remove the `check_single_monitor` rejection.
- [x] T023 Add `fit_monitors` normalization/scaling into frame pixels.
- [x] T024 Create one borderless fullscreen overlay viewport per monitor.
- [x] T025 Convert local pointer positions to global frame coordinates per viewport.
- [x] T026 Keep the drag anchor/selection global across monitor boundaries.
- [x] T027 Route focus, magnifier, tips, size badge, and toolbar to the correct monitor.
- [x] T028 Close every overlay viewport on confirm/cancel/pin/error.
- [x] T029 Pass the monitor layout from the app into the UI.
- [x] T030 Add debug-only `COSNIP_TEST_MONITORS` layout injection for automated tests.

## Phase 4 — Large desktop rendering

- [x] T040 Detect `max_texture_side` and split the frame into tiles.
- [x] T041 Draw only intersecting tiles per monitor with correct UVs.
- [x] T042 Reproduce and fix the 2048-limit panics on a 3840x1080 Xvfb display.
- [x] T043 Tile pinned images with the same limit; verify a 3840x1080 pin is created.

## Phase 5 — Verification

- [x] T050 Unit tests: conversion, monitor normalization, negative origin, scaled frame, toolbar
  monitor selection.
- [x] T051 Xvfb 3840x1080 `--area` save exactness with a 2048 texture limit.
- [x] T052 Single-monitor drag 300,300 → 600,450 saves 300x150.
- [x] T053 Simulated dual-monitor drag 900,500 → 1100,550 saves 200x50.
- [x] T054 Pin a 500x300 `--area` selection; verify geometry and Escape close.
- [x] T054b Pin a 3840x1080 full-screen selection on the 2048-limit Xvfb display; verify it opens
  without a texture panic.
- [x] T055 Run `cargo fmt`, `cargo clippy -- -D warnings`, and the workspace tests.
- [x] T056 Update README, dependency record, parity matrix, ADR, and Feature 002 supersession note.
- [ ] T057 Validate on real dual-monitor GNOME hardware (focus, placement, cross-screen drag).
- [ ] T058 Validate a layout where the primary monitor is not at (0,0) against Mutter's
  `RecordArea` coordinates.
- [ ] T059 Validate fractional/mixed-DPI behavior or add a clear runtime limitation message.
- [ ] T060 Validate x86_64 release build with PipeWire development packages.

## Open follow-ups

- Portal ScreenCast with restore tokens for non-GNOME sessions.
- DMA-BUF import to avoid CPU copies on high-resolution desktops.
- Monitor hot-plug handling while the overlay is open.
- Per-monitor scaling/transform correction.
