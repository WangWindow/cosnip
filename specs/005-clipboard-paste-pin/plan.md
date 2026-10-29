# Implementation Plan: Clipboard Paste and Pin

**Branch**: `005-clipboard-paste-pin` | **Date**: 2026-10-29 | **Status**: Implemented | **Spec**:
[spec.md](spec.md)

**Input**: Feature specification from `/specs/005-clipboard-paste-pin/spec.md`

## Summary

Add a `cosnip paste` command that reads an image from the system clipboard (or a PNG via
`--file`) and opens the existing frameless pin window at the global pointer. The paste path does
not start a capture backend, show the snip overlay, copy, or save. The existing pin viewport,
texture tiling, drag/zoom/close handling, and HiDPI point conversion are reused.

This is the reverse of the current snip-to-clipboard flow and the next step toward Snipaste's
F1/F3 workflow without committing to a resident daemon. Global hotkeys, single-instance IPC, tray,
and text/HTML/URI clipboard payloads remain follow-up features.

## Technical Context

**Language/Version**: Rust stable, edition 2024, MSRV 1.95.

**Primary Dependencies**:

- `arboard` 3 (MIT OR Apache-2.0) for clipboard image reads (already used for writes).
- `image` 0.25 with PNG (already used by `cosnip-imaging`).
- `eframe`/`egui` 0.37, `winit` 0.30, `x11rb` 0.13 for the existing pin viewport and pointer.
- `clap` 4 for the new `paste` subcommand.

No new dependencies are required.

**Storage**: None. The pasted frame lives in memory. `--file` reads an existing PNG; no temporary
media is written.

**Testing**: Unit tests for clipboard `ImageData` conversion and PNG decode; CLI parsing and error
tests; the existing pin/texture tests; Xvfb smoke test where available; manual GNOME Wayland and
Xorg checks.

**Target Platform**: Linux X11/Xorg and GNOME 49 Wayland + XWayland on aarch64 and x86_64, using
the existing windowing and capture coordinate model.

**Project Type**: Desktop application, Cargo workspace.

**Performance Goals**:

- 1920x1080 clipboard image to visible pin p95 ≤ 300 ms on the validation desktop.
- No capture backend, PipeWire session, overlay viewport, or ffmpeg process on the paste path.
- Pin creation and tiling follow the existing 2048-pixel texture cap.

**Constraints**:

- Image payloads only; text/HTML/URI/color paste is out of scope.
- PNG only for `--file`; other formats are future work.
- No single-instance daemon or global hotkey in this feature.
- No `unsafe`; no network; no telemetry; no reference assets.
- Existing snip behavior and tests must stay green.

**Scale/Scope**: One new CLI command and one pin-only UI entry path, reusing the existing pin
implementation. The first usable slice is `cosnip paste --file` + `cosnip paste` from the
clipboard, followed by failure handling and validation.

## Constitution Check

- **I. Personal, Lawful Use Only**: PASS. Local personal tool; no reference assets or code.
- **II. Behavioral Reference, Original Implementation**: PASS. The F3 paste behavior is a
  documented observation from the M0 dossier; implementation is original Rust.
- **III. Rust Workspace with Isolated Platform Edges**: PASS. Clipboard access stays in
  `cosnip-platform`; the pin UI stays in `cosnip-ui`; the binary only wires them. No new GUI
  stack, no `unsafe`, no new dependencies.
- **IV. Test-First and Evidence-Based Fidelity**: PASS. Unit/CLI tests plus manual clipboard and
  HiDPI checks; success criteria are measurable.
- **V. Local-Only Privacy and Operational Restraint**: PASS. No network, telemetry, or temporary
  capture artifacts; clipboard data stays in memory.

No violations. Complexity is limited to a new command and a pin-only startup mode.

## Project Structure

### Documentation (this feature)

```text
specs/005-clipboard-paste-pin/
├── spec.md
├── plan.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── cli.md
└── tasks.md
```

### Source Code (repository root)

```text
crates/
  cosnip-platform/
    src/clipboard.rs        # read an image into a validated Frame
  cosnip-imaging/
    src/lib.rs              # reuse decode_png for --file
  cosnip-ui/
    src/app.rs              # pin-only startup, pointer placement, shared pin lifecycle
  cosnip-app/
    src/main.rs             # paste subcommand and wiring
  cosnip-core/
    src/frame.rs            # existing Frame validation
```

**Structure Decision**: Keep clipboard I/O in the platform adapter, keep PNG decoding in the
imaging crate, and keep the pin lifecycle in the existing UI app. The CLI remains thin. No new
crate or service is introduced.

## Phase Plan

### Phase 0 — Research

1. Confirm `arboard::Clipboard::get_image` behavior on GNOME 49 Wayland + XWayland and Xorg,
   including RGBA byte order and alpha.
2. Confirm how the current eframe root control window and pin viewport behave when the overlay is
   never opened, and identify the minimal pin-only startup state.
3. Confirm the global pointer coordinate space on mixed-DPI XWayland and the clamping rule for
   image-sized pins.

### Phase 1 — Design

1. Define `CosnipApp::new_pin(frame, position)` (or an equivalent `UiMode::Pin`) that creates the
   pin textures in `with_context`, sets `overlay_open = false`, and keeps the root window alive.
2. Define `cosnip paste` argument parsing: optional `--file PATH`; `--file` wins.
3. Define clipboard-to-frame validation: width/height/byte length and opaque/alpha handling.
4. Define pin placement: top-left at the global pointer, converted to viewport points, clamped to
   the desktop bounds; fall back to (0,0) if no pointer is available.

### Phase 2 — Implementation

1. Add `clipboard_image()` to `cosnip-platform` returning a `Frame`.
2. Add the `paste` subcommand and `--file` handling in `cosnip-app`.
3. Add the pin-only UI path in `cosnip-ui`; reuse `Pin`, tiles, zoom, drag, and close.
4. Add error messages and exit codes for empty/text clipboard, invalid file, and clipboard
   failure.
5. Keep the snip path unchanged.

### Phase 3 — Verification

1. Unit tests for clipboard image conversion and PNG decode.
2. CLI tests for `paste --help`, missing file, non-PNG file, and conflicting arguments.
3. Manual tests: copy a 500x300 image and paste; verify pixels, position, drag, zoom, close.
4. HiDPI test on the mixed-scale desktop: verify the pin is not doubled in position or size.
5. Re-run `cargo fmt --check`, clippy, and workspace tests.

## Complexity Tracking

No constitution violations. The only added complexity is a second UI startup mode; it reuses the
existing pin viewport rather than introducing a second windowing implementation.
