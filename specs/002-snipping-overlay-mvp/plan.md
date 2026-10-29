# Implementation Plan: Snipping Overlay MVP

**Branch**: `002-snipping-overlay-mvp` | **Date**: 2026-10-29 | **Spec**:
[spec.md](spec.md)

**Input**: Feature specification from `specs/002-snipping-overlay-mvp/spec.md`

## Summary

Build the first usable Cosnip binary: `cosnip snip` captures the desktop, shows a full-screen
selection overlay, and copies, saves, or pins the selected region. It supports native X11 capture
on Xorg and a GNOME 49 single-frame Screencast fallback on Wayland, using the architecture and
window strategy accepted in ADR 0001. Annotations, tray, settings, global hotkeys, multi-monitor,
and paste-from-clipboard are later features.

The MVP is deliberately a small vertical slice: one binary, one monitor, 1x scale, one capture
frame, one selection, three output destinations. It exists to prove the core loop and the hard
platform edges before adding the annotation model.

## Technical Context

**Language/Version**: Rust stable, edition 2024, MSRV **1.95** (required by eframe/egui 0.37).
Current toolchain 1.99.1. Build and test on `aarch64-unknown-linux-gnu` and
`x86_64-unknown-linux-gnu`.

**Primary Dependencies**:

- `eframe`/`egui` 0.37 (MIT OR Apache-2.0) with `glow`, `x11`, `wayland`, `default_fonts`,
  `system_fonts`; forced X11 backend.
- `winit` 0.30 (Apache-2.0) for X11 backend selection.
- `x11rb` 0.13 (MIT OR Apache-2.0) with `shm`, `randr`, `xfixes`, `shape` for capture and
  window/screen helpers.
- `zbus` 6.1 (MIT) with `zbus::block_on` for `org.gnome.Shell.Screencast`.
- `arboard` 3 (MIT OR Apache-2.0) for clipboard images.
- `image` 0.25 (MIT OR Apache-2.0) with PNG; crop/encode in a worker thread.
- `clap` 4 (MIT OR Apache-2.0), `tracing`/`tracing-subscriber` (MIT), `thiserror` 2,
  `anyhow` 1, `tempfile` 3 (MIT OR Apache-2.0).
- External runtime for the GNOME fallback: `ffmpeg` (GPL/LGPL build) used only to decode the
  short Screencast MP4. It is checked by `--doctor`; it is not bundled. This is an explicit MVP
  tradeoff to be replaced by a PipeWire/Mutter ScreenCast backend later.
- No `tiny-skia`/`cosmic-text` yet: the MVP has no annotation rendering; egui painter draws the
  overlay chrome, and `image` handles crop/encode.

**Storage**: No database. A small optional TOML config is not required for the MVP; CLI arguments
carry all options. Temporary recording/frame files live under `$XDG_RUNTIME_DIR/cosnip/` with
0700/0600 permissions and are cleaned on all paths.

**Testing**: Unit tests for geometry/selection/output naming/CLI; integration tests for X11
capture and image export under Xvfb; golden image crop tests; manual GNOME Screencast and pin
checklist; benchmarks for capture and selection responsiveness.

**Target Platform**: aarch64 Armbian/Ubuntu 26.04, GNOME 49 Wayland + XWayland, X11/Xorg, single
1920x1080 monitor at scale 1.0. x86_64 Linux is the secondary build/test target.

**Project Type**: Desktop application, Cargo workspace.

**Performance Goals**:

- X11: command to interactive overlay p95 ≤ 300 ms; confirmation to clipboard/save ≤ 150 ms.
- GNOME 49 Screencast: command to overlay p95 ≤ 2.0 s; failure reported ≤ 2.5 s.
- Selection/magnifier UI feedback ≤ 16 ms per frame at 1080p.
- Pin creation ≤ 150 ms; idle CPU < 1% with a pin; RSS < 300 MB at 1080p.
- No full-screen CPU re-composite per mouse move; reuse the captured texture and draw chrome.

**Constraints**:

- GNOME 49 denies `org.gnome.Shell.Screenshot` and the portal path is blocked; the accepted
  fallback is the Shell Screencast single-frame path.
- XWayland root capture is not supported; X11 capture is for Xorg only.
- Pins are image-sized override-redirect windows; alpha/shadow is an enhancement. On hardware GLX
  that yields depth 24, use an opaque image-sized window rather than a black margin.
- No multi-monitor/mixed-DPI in this MVP; detect and report clearly.
- No network, no telemetry, no reference assets.
- No `unsafe` in Cosnip code.

**Scale/Scope**: One binary, five crates, 1080p single monitor, one user, no daemon. Target ~6–8
weeks of incremental implementation; the first demonstrable slice is capture → overlay → copy.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **I. Personal, Lawful Use Only**: PASS. No reference code/assets; personal tool.
- **II. Behavioral Reference, Original Implementation**: PASS. Uses the sanitized M0 dossier;
  no decompiled code or reference resources.
- **III. Rust Workspace with Isolated Platform Edges**: PASS. Pure `cosnip-core`; capture backend
  trait; X11/GNOME adapters; one GUI stack (eframe/egui); no `unsafe`; dependencies justified;
  dual-architecture builds.
- **IV. Test-First and Evidence-Based Fidelity**: PASS. Unit/integration/golden tests, manual
  parity checklist, measured performance budgets. Tests are written with each slice.
- **V. Local-Only Privacy and Operational Restraint**: PASS. Offline, no telemetry, redacted
  logs, restrictive temp files, cleanup, idle budget.

No violations. Complexity is justified by the platform split (X11 capture vs GNOME Screencast) and
the pin window edge; both are isolated behind adapters.

## Project Structure

### Documentation (this feature)

```text
specs/002-snipping-overlay-mvp/
├── spec.md
├── plan.md
├── research.md                 # decisions/gaps specific to the MVP
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── cli.md
│   ├── capture-backend.md
│   └── output.md
├── checklists/
│   └── requirements.md
└── tasks.md
```

### Source Code (repository root)

```text
Cargo.toml                      # workspace
rust-toolchain.toml
rustfmt.toml
clippy.toml
deny.toml

crates/
  cosnip-core/                  # pure: geometry, selection, document, CLI value types, naming
    src/{lib,geometry,selection,document,output,error}.rs
  cosnip-imaging/               # image buffer, crop, PNG encode, magnifier sampling
    src/{lib,buffer,crop,encode,magnify}.rs
  cosnip-capture/               # CaptureBackend trait + X11 + GNOME Screencast
    src/{lib,backend,x11,gnome_screencast,decode,error}.rs
  cosnip-platform/              # clipboard adapter, screen info, temp artifacts
    src/{lib,clipboard,screen,temp}.rs
  cosnip-ui/                    # eframe app, overlay viewport, pin viewports, toolbar
    src/{lib,app,overlay,pin,toolbar,magnifier,theme}.rs
  cosnip-app/                   # binary, clap CLI, wiring, diagnostics
    src/{main,cli,doctor,wiring}.rs

tests/                          # workspace integration tests
  capture_x11.rs
  export.rs
  cli.rs
benches/                        # criterion benchmarks
  capture.rs
  selection.rs
  encode.rs
```

**Structure Decision**: Keep the domain and imaging pure; isolate X11, GNOME D-Bus, clipboard,
and eframe behind adapters. The binary is thin and only wires dependencies. The initial workspace
is intentionally smaller than the full roadmap; annotation/render crates are added by Feature 003.

## Phase Plan

### Phase 0 — Research and decisions

1. Confirm the GNOME Screencast single-frame sequence on the target: start, 300–700 ms, stop on
   the same connection, decode one frame, cleanup.
2. Choose X11 capture call (`GetImage` first, `ShmGetImage` if profiling requires it).
3. Confirm eframe viewport behavior with a 1x1 hidden/override root, fullscreen overlay, and
   image-sized pin.
4. Define exact CLI/output defaults and clipboard lifetime behavior.
5. Record findings in `research.md`.

### Phase 1 — Core and imaging

1. Create the workspace, lint/format/toolchain files, and crate skeletons.
2. Implement pure geometry, selection constraints, document/output naming, and errors.
3. Implement image buffer/crop/PNG encode and magnifier sampling with unit + golden tests.
4. Add CLI value types and validation.

### Phase 2 — Capture adapters

1. `CaptureBackend` trait with availability, capture, and error types.
2. X11 backend: connect, screen geometry, `GetImage`, cursor policy, test under Xvfb.
3. GNOME Screencast backend: zbus blocking call, persistent connection, record/stop, ffmpeg
   decode, temp cleanup, timeouts, test/mock.
4. Backend selection and `--doctor`.

### Phase 3 — Overlay and output

1. eframe app shell with X11 forced, 1x1 control root, overlay viewport.
2. Capture texture, dim, crosshair, selection rectangle/handles, size badge.
3. Magnifier and color readout sampling from the raw frame.
4. Keyboard: arrows, Shift, Ctrl+A, Enter/Ctrl+C, Esc; pointer drag.
5. Copy via arboard with clipboard lifetime handling; save atomically via worker; pin command.

### Phase 4 — Pin windows

1. Image-sized override-redirect viewport, no taskbar, always-on-top.
2. Move (OuterPosition), wheel zoom (10–500%), close (Esc/double-click).
3. Alpha-capable GL check; opaque fallback.
4. CLI `--pin` and process lifetime.

### Phase 5 — Verification and packaging

1. Xvfb integration tests, golden crop/save tests, clipboard smoke test.
2. GNOME 49 manual/automated screencast test; native Wayland pin screenshot.
3. Benchmarks and budget verification; `cargo fmt`, `clippy -D warnings`, `cargo test`.
4. `--doctor` diagnostics and release build (aarch64 native; x86_64 build check).
5. Document GNOME fallback/ffmpeg dependency and multi-monitor limitation.

## Risks and Mitigations

| Risk | Evidence | Mitigation |
|------|----------|------------|
| GNOME Screencast latency/recording indicator | M0 300–700 ms yielded 10 frames; ~1 s total | Use the shortest reliable recording, stop promptly, show a local busy state; document the limitation |
| ffmpeg missing on target | M0 used ffmpeg for frame extraction | `--doctor` checks; clear error; later replace with PipeWire crate |
| Screencast output MP4 fragmented/empty | M0 `StopScreencast` from another connection produced truncated MP4 | Keep one zbus connection; stop on the same connection; validate frame bytes |
| Clipboard lost on process exit | X11 clipboard ownership is process-bound | Stay alive as a hidden owner until replaced or a bounded timeout; test with GTK4 reader |
| Pin opacity depth 24 on hardware GLX | M0 hardware GLX depth 24, software GL depth 32 | MVP uses image-sized opaque pins; alpha/shadow enhancement later |
| Overlay focus on X11/XWayland | WM-managed fullscreen focus passed in M0; override did not | Use WM-managed fullscreen (accepted ADR) |
| Multi-monitor/DPI | Not tested | Detect >1 monitor or scale != 1 and report unsupported; document |
| Temp artifact leakage | Spec requires cleanup | RAII temp dir guard, tests for success/cancel/failure |
| X11 capture blocking UI | GetImage is synchronous | Capture in a worker thread before showing overlay; do not block the UI thread |

## Dependency and License Notes

All product dependencies are MIT/Apache-2.0/BSD-compatible. `ffmpeg` is an optional external
runtime for the GNOME fallback; its license depends on the distribution build. Record the exact
runtime requirement in `docs/dependencies.md` and `--doctor`. No copyleft code is linked.

## Constitution Re-check (post-design)

The design keeps the domain pure, isolates platform edges, uses one GUI stack, forbids `unsafe`,
commits only original code, and has measurable tests/budgets. The GNOME Screencast fallback is a
behavioral difference forced by the target compositor; it is documented as an MVP exception and
revisited for the PipeWire backend. No violation.

## Next Actions

1. Generate `tasks.md` and implement Phase 1 + Phase 2 first.
2. Validate the GNOME Screencast backend on the target before building the overlay UI.
3. Run `/speckit.analyze` after tasks are generated, then implement.
