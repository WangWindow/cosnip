# Feature Specification: Snipping Overlay MVP

**Feature Branch**: `002-snipping-overlay-mvp`

**Created**: 2026-10-29

**Status**: Implemented (MVP), verification in progress

**Input**: Capture the desktop, select a region, copy or save it, and pin the image on screen from
an ARM64 Linux desktop. This is the first usable Cosnip milestone; annotations, tray, global
hotkeys, OCR, and multi-monitor polish are later features.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Snip and Copy on X11/Xorg (Priority: P1)

As a Linux user on an X11/Xorg session, I can run `cosnip snip`, see a frozen full-screen overlay,
drag a rectangle, and press Enter or Ctrl+C to get that region on my clipboard.

**Why this priority**: This is the shortest end-to-end path that proves capture, overlay input,
selection geometry, rendering, and clipboard output.

**Independent Test**: On an Xvfb/Xorg session with a known pattern, run `cosnip snip --area ...`
and verify the clipboard image dimensions and pixels. Also perform a mouse drag selection.

**Acceptance Scenarios**:

1. **Given** an X11 session with a 1920x1080 screen, **When** the user runs `cosnip snip`, **Then**
   a full-screen overlay appears with the frozen desktop and a crosshair within 300 ms.
2. **Given** the overlay is visible, **When** the user drags from (100,100) to (600,400) and
   presses Enter, **Then** the clipboard contains a 500x300 image matching those pixels.
3. **Given** the overlay is visible, **When** the user presses Escape, **Then** the overlay closes
   without changing the clipboard or creating a file.
4. **Given** a selection exists, **When** the user presses arrow keys, **Then** the selection moves
   by one pixel; holding Shift keeps a square selection while dragging.

---

### User Story 2 - Snip on GNOME 49 Wayland (Priority: P1)

As a user on GNOME 49 Wayland where direct screenshot APIs are denied, I can still run
`cosnip snip` and select a region; Cosnip captures a single frame through the GNOME Shell
Screencast interface, then shows its own overlay.

**Why this priority**: The primary target device is GNOME Wayland. Without this path, the MVP
cannot be demonstrated on the target.

**Independent Test**: Run `cosnip snip` on the target GNOME 49 session, confirm the overlay shows
a recent desktop frame, copy a region, and verify no temporary recording files remain.

**Acceptance Scenarios**:

1. **Given** GNOME 49 Wayland and no portal permission, **When** the user runs `cosnip snip`,
   **Then** Cosnip starts a short Screencast recording, stops it on the same D-Bus connection,
   decodes a frame, and shows the overlay without a system dialog.
2. **Given** the Screencast capture path, **When** the snip completes, **Then** the total time from
   command to interactive overlay is at most 2.0 s on the target device.
3. **Given** a completed or cancelled snip, **When** the process exits, **Then** no recording,
   frame, or temporary image remains under the runtime directory.
4. **Given** the Screencast service is unavailable, **When** the user runs `cosnip snip`, **Then**
   the command exits with a clear error and does not leave a partial overlay or temp file.

---

### User Story 3 - Save the Selection (Priority: P2)

As a user, I can save the selected region as a PNG to an explicit path or to a default Pictures
location.

**Why this priority**: Saving is a core screenshot workflow and validates the imaging/export
pipeline independently of the clipboard and pin.

**Independent Test**: Snip a known pattern with `--save /tmp/test.png`; verify the PNG exists,
has the correct dimensions, and is a valid decodable image. Verify no partial file remains on
failure.

**Acceptance Scenarios**:

1. **Given** a selection, **When** the user runs `cosnip snip --save /tmp/cosnip-test.png`, **Then**
   a valid 500x300 PNG is written at that path.
2. **Given** an unwritable destination, **When** the save is attempted, **Then** the command exits
   non-zero with an actionable error and no zero-byte/partial file is left behind.
3. **Given** no explicit path, **When** the user saves, **Then** the file goes under the user's
   Pictures directory with a timestamped, collision-free name.

---

### User Story 4 - Pin the Selection (Priority: P2)

As a user, I can pin the selected region as a floating image window that stays above other
applications, can be moved, zoomed, and closed.

**Why this priority**: Pinning is the defining Snipaste behavior and validates the windowing
architecture from M0.

**Independent Test**: Pin a known image over a native Wayland application on GNOME 49; verify the
pin is visible above it, can be dragged and zoomed, and closes without affecting the clipboard.
Repeat on Xvfb/Xorg.

**Acceptance Scenarios**:

1. **Given** a selection, **When** the user chooses pin, **Then** a frameless override-redirect
   window appears at the selection position, above other windows, with no taskbar entry.
2. **Given** a pin, **When** the user drags it, **Then** it moves with the pointer and remains
   above the underlying application.
3. **Given** a pin, **When** the user scrolls the wheel, **Then** it zooms between 10% and 500%.
4. **Given** a pin, **When** the user presses Escape or double-clicks it, **Then** it closes; if it
   was the last window, the process exits.
5. **Given** a pin over a native Wayland window, **When** the desktop is inspected, **Then** the
   pin is still above the native surface (manual visual check).

---

### User Story 5 - Cancel and Failure Handling (Priority: P3)

As a user, I can cancel a snip or understand why capture failed without leaving stray windows,
files, or clipboard changes.

**Why this priority**: A screenshot tool must fail safely; partial state is more harmful than a
clear error.

**Independent Test**: Force each backend failure (no DISPLAY, Screencast unavailable, decode
failure, clipboard failure) and verify cleanup and exit codes.

**Acceptance Scenarios**:

1. **Given** no usable display, **When** the user runs `cosnip snip`, **Then** it exits non-zero
   with a message naming the missing capability.
2. **Given** a Screencast that starts but cannot produce a frame, **When** the timeout expires,
   **Then** the recording is stopped, temp files are deleted, and the command exits non-zero.
3. **Given** a clipboard write failure, **When** the image is copied, **Then** the user sees an
   error and any saved file remains valid.

### Edge Cases

- A 1x1 or zero-size selection: treat as cancelled; do not write an empty image.
- Selection dragged outside the monitor bounds: clamp to the desktop.
- Screen size changes between capture and overlay: use the captured frame's dimensions.
- Screencast produces a fragmented/empty MP4: treat as capture failure, not a black screenshot.
- `ffmpeg`/decoder is missing on GNOME: report the missing runtime dependency before starting the
  recording.
- Clipboard manager not running: keep the process alive long enough to serve the clipboard, or
  report failure.
- Pin created at a negative or off-screen position: clamp so at least a visible portion remains.
- Multi-monitor/mixed-DPI: not supported in this MVP; detect more than one monitor and fail with a
  clear "multi-monitor not yet supported" message, or capture only the primary monitor and state
  the limitation in `--help`.
- Session type changes between launches: choose the backend at runtime, not compile time.
- A second `cosnip snip` while a pin is open: start a new snip process; single-instance is later.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Cosnip MUST select a capture backend at runtime: native X11 on Xorg, GNOME 49
  Screencast single-frame on GNOME Wayland, and a clear unsupported error elsewhere in this MVP.
- **FR-002**: On Xorg, Cosnip MUST capture the desktop through X11 without shelling out to an
  external screenshot tool.
- **FR-003**: On GNOME 49 Wayland, Cosnip MUST use `org.gnome.Shell.Screencast` on one persistent
  D-Bus connection, record briefly, stop on that same connection, and decode exactly one frame.
- **FR-004**: The GNOME capture path MUST complete or fail within 2 seconds and MUST clean up all
  recording, frame, and temporary files on success, cancellation, and failure.
- **FR-005**: Cosnip MUST show a borderless full-screen overlay containing the captured frame,
  dimming outside the selection, a crosshair, a selection rectangle with handles, a size badge,
  and a magnifier/color readout.
- **FR-006**: Users MUST be able to select with the pointer, move the selection with arrow keys,
  constrain to a square with Shift, select the full screen with Ctrl+A, confirm with Enter or
  Ctrl+C, and cancel with Escape.
- **FR-007**: A confirmed selection MUST be copyable to the clipboard as an image, savable as a
  PNG, or pinnable. X11/Xorg output is pixel-exact; the GNOME Screencast fallback is decoded from
  a lossy H.264 recording and therefore may have small color/compression differences.
- **FR-008**: A clipboard copy MUST preserve the selected pixel dimensions and produce an image
  that at least one native Wayland application can read.
- **FR-009**: Saving MUST be atomic: write to a temporary file in the destination directory, fsync
  where practical, and rename into place; no partial file may remain on failure.
- **FR-010**: A pin MUST be a frameless, override-redirect, always-on-top, non-taskbar X11 window
  showing the selected image at 1:1; it MUST support move, wheel zoom (10–500%), and close
  (Escape/double-click).
- **FR-011**: When alpha-capable GL is unavailable, a pin MUST still be usable as an opaque
  image-sized window; shadows/rounded transparency are an enhancement, not a blocker.
- **FR-012**: Cosnip MUST expose a CLI with `snip`, `--area X Y W H`, `--full`, `--copy`, `--save
  PATH`, `--pin`, `--delay MS`, `--version`, and `--doctor`; the default output is clipboard.
- **FR-013**: Cosnip MUST exit non-zero with an actionable message when the display, capture
  backend, decoder, clipboard, or output path is unavailable.
- **FR-014**: Logs MUST NOT contain image content or full user file paths; temporary files MUST be
  created with restrictive permissions and cleaned up.
- **FR-015**: Cosnip MUST make no network requests and MUST work fully offline.
- **FR-016**: The capture, overlay, selection, document, output, and pin logic MUST be structured
  so that aarch64 and x86_64 builds share the same behavior; platform-specific code stays behind
  adapters.
- **FR-017**: The MVP MUST document and enforce a single-monitor/1x scope; multi-monitor and
  mixed-DPI support are deferred.
- **FR-018**: The product MUST keep the reference clean-room policy: no reference code, strings,
  screenshots, or assets are used.

### Key Entities

- **CaptureFrame**: A desktop image, dimensions, scale, and capture backend metadata.
- **Selection**: A rectangle in desktop coordinates with handles and constraint state.
- **SnipDocument**: The captured frame plus the current selection and output choice.
- **CaptureBackend**: X11 or GNOME Screencast, with availability and error reporting.
- **PinWindow**: An override-redirect image window with position, scale, and close state.
- **OutputDestination**: Clipboard, file path, or pin.
- **CliOptions**: Parsed command, capture area, delay, and output.
- **TempArtifact**: A runtime file with a restrictive permission and mandatory cleanup.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On Xorg/Xvfb at 1920x1080, 95% of snips from command to interactive overlay complete
  in under 300 ms; a 500x300 selection copies in under 100 ms after confirmation.
- **SC-002**: On the target GNOME 49 device, 95% of snips complete capture and show the overlay in
  under 2.0 s; failures are reported within 2.5 s.
- **SC-003**: Clipboard and saved PNG dimensions match the selection in automated tests. X11/Xorg
  output has zero unexpected pixel differences; GNOME fallback output is visually equivalent and
  dimension-exact, with lossy-codec differences documented.
- **SC-004**: A pinned image is visible above a native Wayland window and supports move, zoom, and
  close; no taskbar entry is present.
- **SC-005**: After 20 successful and 20 failed snips, no temp recording/frame files remain and no
  partial output files remain.
- **SC-006**: Idle CPU stays below 1% with a pin open; resident memory stays below 300 MB for a
  1080p session.
- **SC-007**: Both `aarch64-unknown-linux-gnu` and `x86_64-unknown-linux-gnu` builds pass the
  offline test suite and the CLI smoke test.
- **SC-008**: A repository audit finds no reference binaries, decompiled output, or reference
  assets in the product tree.

## Assumptions

- The first MVP targets a single 1920x1080 monitor at scale 1.0; the current device fits this.
- Xorg is available for the fast path; GNOME 49 Wayland is the fallback path.
- `ffmpeg` (or a decoder runtime) may be required for the GNOME Screencast fallback; it is
  documented as an optional runtime dependency and is not bundled. The fallback is lossy H.264;
  exact pixel parity is an Xorg/PipeWire requirement.
- The user runs the binary directly or via a desktop shortcut; tray and global hotkeys are later.
- Clipboard image exchange is verified with a GTK4 native Wayland reader; if the reverse direction
  is needed for paste, a native clipboard helper is a later feature.
- The MVP uses the accepted M0 architecture: WM-managed fullscreen overlay, override-redirect
  image-sized pins, X11 UI.

## Out of Scope

- Annotations, text editing, mosaic/blur, undo/redo.
- Tray icon, settings UI, global hotkeys, autostart, single-instance IPC.
- Paste-from-clipboard image windows, history, image groups, whiteboard.
- Multi-monitor and mixed-DPI correctness.
- KDE/KWin and generic portal capture backends.
- OCR, cloud APIs, Pro features.
- Windows/macOS.
