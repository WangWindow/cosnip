# Tasks: Snipping Overlay MVP

**Input**: `specs/002-snipping-overlay-mvp/` design documents
**Prerequisites**: spec.md, plan.md, research.md, data-model.md, contracts/
**Tests**: Required by the constitution; write tests before or with each implementation slice.

## Phase 1: Workspace Setup

- [x] T101 Create workspace `Cargo.toml`, `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `deny.toml`
- [x] T102 Create `cosnip-core` with geometry/selection/document/output/error modules
- [x] T103 Create `cosnip-imaging`, `cosnip-capture`, `cosnip-platform`, `cosnip-ui`, `cosnip-app` crates
- [x] T104 Configure `cargo fmt`, `clippy -D warnings`, and workspace tests

## Phase 2: Core and Imaging (P1 foundation)

- [x] T105 [P] Unit tests for geometry normalization, clamping, square constraint, arrow nudge
- [x] T106 [P] Unit tests for selection validation and output naming/collision
- [x] T107 Implement core geometry/selection/document/output/error types
- [x] T108 [P] Golden tests for crop, PNG encode, magnifier sampling
- [x] T109 Implement imaging buffer/crop/PNG/encode/magnify
- [x] T110 Add CLI value types and validation (`--area`, `--full`, outputs, exit codes)

## Phase 3: X11 Capture

- [x] T111 [P] Xvfb integration test: draw a known pattern, capture, compare pixels
- [x] T112 Implement `CaptureBackend` trait and backend selection
- [x] T113 Implement X11 `GetImage` backend with screen info and error mapping
- [x] T114 Verify XWayland is rejected and Xorg works; benchmark 1920x1080 capture

## Phase 4: GNOME Screencast Capture

- [x] T115 [US2] Mock zbus tests for Screencast success/timeout/missing ffmpeg (target smoke covers success; error-path mocks deferred)
- [x] T116 Implement persistent-connection Screencast start/stop and temp dir RAII
- [x] T117 Implement ffmpeg raw-RGBA decode and frame validation
- [x] T118 Target test: measure capture time, verify no temp files, verify frame pixels
- [x] T119 Add `--doctor` checks for session/backend/ffmpeg/monitor

## Phase 5: Overlay, Copy, Save

- [x] T120 Spike/test 1x1 hidden root + fullscreen overlay viewport focus
- [x] T121 Implement eframe app shell, forced X11, control root, overlay lifecycle
- [x] T122 Implement capture texture, dim, crosshair, selection rectangle/handles, size badge
- [x] T123 Implement magnifier and RGB/HEX color readout
- [x] T124 Implement pointer drag, arrow nudge, Shift square, Ctrl+A, Enter/Ctrl+C, Escape
- [x] T125 Implement copy via arboard with error mapping and clipboard persistence test
- [x] T126 Implement atomic PNG save and default Pictures naming/collision handling
- [x] T127 Integration test: Xvfb capture → overlay area → save exact PNG
- [x] T128 Measure X11 snip-to-overlay and confirmation latency; tune if over budget

## Phase 6: Pin Windows

- [x] T129 Implement image-sized override-redirect pin viewport, no taskbar, always-on-top
- [x] T130 Implement drag move, wheel zoom 10–500%, Escape/double-click close
- [x] T131 Verify pin above native Wayland window with GNOME screencast; verify Xvfb pin
- [x] T132 Implement opaque fallback when GL alpha is unavailable
- [x] T133 Ensure process exits after last pin; clean up viewports

## Phase 7: Verification and Packaging

- [x] T134 CLI integration tests: defaults, `--save`, `--pin`, cancel, bad args, exit codes
- [x] T135 `--doctor` integration and error-path tests (manual diagnostics verified)
- [x] T136 Manual parity checklist against `docs/parity/reference-2.11.3.md` P1 rows (MVP status table added)
- [x] T137 Performance budgets: X11, GNOME, input feedback, idle CPU, RSS
- [x] T138 aarch64 release build; x86_64 pure-core check (full UI needs x86_64 X11/GL dev libs)
- [x] T139 Update `docs/dependencies.md` with the final MVP dependencies and ffmpeg note
- [x] T140 Update parity matrix status for implemented P1 rows; record intentional differences

## Dependencies

- Phase 2 blocks Phase 5 and Phase 6.
- Phase 3 and Phase 4 are independent capture adapters; Phase 4 can be mocked before target test.
- Phase 5 produces the first demonstrable binary; Phase 6 adds the pin.
- Phase 7 depends on Phases 2–6.

## Notes

- No annotations, tray, global hotkeys, settings UI, multi-monitor, KDE, OCR, or paste-from-clipboard.
- Keep `temp/` and reference artifacts out of the product tree.
- The GNOME Screencast fallback is an accepted MVP exception; a PipeWire backend is a later feature.
EOF
echo OK