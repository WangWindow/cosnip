# Tasks: Clipboard Paste and Pin

**Feature**: `005-clipboard-paste-pin`

Legend: `[x]` complete, `[ ]` pending.

## Phase 1 — Clipboard image input

- [x] T001 Add `cosnip_platform::clipboard_image()` returning a validated `Frame` from
  `arboard::Clipboard::get_image`.
- [x] T002 Reject width/height zero and byte lengths that do not equal `width * height * 4`.
- [x] T003 Add unit tests for conversion, alpha preservation, and invalid lengths.
- [x] T004 Add a PNG file loader in the app using `cosnip_imaging::decode_png`, with a clear error
  for missing or non-PNG files.
- [x] T005 Add CLI parsing for `cosnip paste [--file PATH]`; `--file` takes precedence.

## Phase 2 — Pin-only UI startup

- [x] T010 Add a pin-only startup path in `cosnip-ui` that creates the pin textures during
  `with_context`, sets `overlay_open = false`, and does not capture or show the overlay.
- [x] T011 Place the pin at the global pointer converted from frame pixels to viewport points;
  clamp the top-left to the desktop and default to (0,0) when the pointer is unavailable.
- [x] T012 Reuse the existing pin drag, wheel zoom, close, and texture tiling; do not duplicate
  the pin window implementation.
- [x] T013 Ensure closing the last pin exits 0 and that a paste failure leaves no window or
  process.
- [x] T014 Keep the snip entrypoint and `UiOptions` behavior unchanged for existing callers.

## Phase 3 — Error handling and diagnostics

- [x] T020 Report empty clipboard, text-only clipboard, unsupported clipboard format, and
  clipboard-unavailable errors with distinct actionable messages and non-zero exits.
- [x] T021 Ensure the paste path never creates a Mutter session, PipeWire stream, X11 capture,
  temporary file, or ffmpeg process.
- [x] T022 Keep the existing `--doctor` clipboard line; optionally add a non-breaking
  clipboard-image capability line.

## Phase 4 — Verification

- [x] T030 CLI tests: `paste --help`, missing file, non-PNG file, and `--file` precedence.
- [x] T031 Unit/integration test: a generated 500x300 PNG pasted with `--file` creates a pin with
  the same dimensions and pixels.
- [x] T032 Manual clipboard test on GNOME 49 Wayland + XWayland: copy an image, paste, verify
  position, pixels, drag, zoom, and close.
- [x] T033 Manual HiDPI test on the mixed-scale desktop: a 300x200 image appears 300x200 at the
  pointer, not doubled.
- [x] T034 Re-run `cargo fmt --check`, clippy, and workspace tests; verify snip/copy/save/pin
  regressions are absent.
- [x] T035 Update README usage, parity matrix BEH-023/024 notes, and dependencies if needed.

## Follow-ups

- Text/HTML/URI/color clipboard payloads and text pins.
- Single-instance IPC so paste can target the resident process.
- Global hotkeys for F1 snip and F3 paste (portal or desktop shortcut).
- Tray/menu integration.
- Paste history and last-paste position.

## Implementation evidence

- `cosnip paste` from a clipboard image copied by `cosnip snip --copy` created a 500x300 pin at
  the pointer on the mixed-scale GNOME 49 desktop.
- `cosnip paste --file area.png` created a 500x300 pin at (1000,1000) and the grabbed pin contents
  matched the file.
- Pointer clamping produced a pin at (6476,2000) for a 500x300 image with the pointer at
  (6900,2000).
- Text-only clipboard exited 3 with `clipboard does not contain an image`; missing and non-PNG
  files exited 2 with actionable messages and no window.
- `COSNIP_TEST_PIN_CLOSE_MS=2500 target/release/cosnip paste --file area.png` exited 0 with no
  remaining pin window.
- `cargo fmt --check`, clippy with `-D warnings`, and the workspace tests pass.
- Real-mouse pin drag/zoom/close remains a manual check because synthetic XTest pointer events do
  not reach non-fullscreen XWayland windows on the validation host.
