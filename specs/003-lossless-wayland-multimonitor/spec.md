# Feature Specification: Lossless GNOME Wayland Capture and Multi-Monitor Selection

**Feature Branch**: `003-lossless-wayland-multimonitor`

**Created**: 2026-10-29

**Status**: Implemented; validated on mixed-scale dual-monitor GNOME 49 hardware

**Input**: Replace the lossy GNOME Shell Screencast H.264 fallback with a lossless Mutter
ScreenCast + PipeWire path, and make region selection span the whole virtual desktop the way
Snipaste does on Windows.

## Context

Feature 002 delivered the snip/copy/save/pin loop, but on GNOME 49 Wayland it captured one frame
from `org.gnome.Shell.Screencast`, which encodes H.264. The decoded image is visually close but
not pixel-exact. Feature 002 also rejected every layout with more than one monitor.

The reference product selects across all monitors in one continuous frozen desktop. Cosnip must
match that behavior on X11/Xorg and on GNOME Wayland + XWayland, including mixed compositor scales.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Lossless Capture on GNOME Wayland (Priority: P1)

As a GNOME Wayland user, I can run `cosnip snip` and get a frozen desktop image whose pixels match
the compositor output, without an H.264 encode/decode round trip.

**Why this priority**: Pixel fidelity is the core promise of a snipping tool. On the primary
target, the existing fallback is the only lossy path.

**Independent Test**: On GNOME Wayland, run `cosnip snip --full --save /tmp/full.png`; compare the
PNG against an X11 or compositor reference frame, and verify the log reports
`gnome-pipewire`, not `gnome-screencast`.

**Acceptance Scenarios**:

1. **Given** a GNOME Wayland session with Mutter ScreenCast available, **When** Cosnip captures,
   **Then** it creates a Mutter session, records the desktop area, starts the stream, reads a raw
   PipeWire frame, and closes the session on every exit path.
2. **Given** a 1920x1080 desktop, **When** the user runs `cosnip snip`, **Then** capture to an
   interactive overlay completes in at most 600 ms on the target device.
3. **Given** a captured raw BGRx/BGRA/RGBx/RGBA frame, **When** the frame is converted, **Then**
   each output pixel preserves the source channel values and alpha is forced opaque where the
   source has no alpha.
4. **Given** the happy path, **When** the capture completes, **Then** no MP4, image, or temporary
   file is written and `ffmpeg` is not invoked.

---

### User Story 2 - Select Across Multiple Monitors (Priority: P1)

As a user with two or more monitors, I can start a selection on one monitor and drag it onto
another; the saved image contains the continuous desktop region.

**Why this priority**: Snipaste's multi-monitor selection is a defining desktop behavior. A
screenshot tool that only sees one monitor is not usable on a multi-monitor desk.

**Independent Test**: With a virtual desktop split into two 1920x1080 monitors, drag from
(900,500) on the left monitor to (1100,550) on the right, confirm, and verify the output is
200x50 and contains pixels from both monitors.

**Acceptance Scenarios**:

1. **Given** two side-by-side monitors, **When** Cosnip starts, **Then** it enumerates the RandR
   monitor layout and captures the full virtual desktop.
2. **Given** the overlay is open, **When** the desktop is inspected, **Then** each monitor has one
   fullscreen overlay viewport with no decorations, showing its own slice of the frozen frame.
3. **Given** a drag that crosses a monitor boundary, **When** the pointer moves over the second
   monitor, **Then** the selection continues in global desktop coordinates instead of stopping at
   the boundary.
4. **Given** a selection spanning monitors, **When** the user presses Enter/Ctrl+C, **Then** the
   clipboard or PNG contains exactly the selected global rectangle.
5. **Given** a monitor with negative or non-zero layout coordinates, **When** the layout is
   normalized, **Then** the captured frame still maps each monitor slice without gaps or overlaps.

---

### User Story 3 - Safe Fallback and Clear Diagnostics (Priority: P2)

As a user on a system where PipeWire capture is unavailable, I can still snip through the H.264
fallback and understand that the result is lossy.

**Why this priority**: The lossless path is newer and depends on PipeWire; failure must not make
the tool unusable.

**Independent Test**: Force the Mutter session or PipeWire stream to fail and verify Cosnip logs
the failure, falls back to `org.gnome.Shell.Screencast`, and still saves a valid PNG.

**Acceptance Scenarios**:

1. **Given** Mutter ScreenCast fails, **When** Cosnip captures on GNOME, **Then** it logs a
   fallback warning and attempts the Screencast backend.
2. **Given** both paths fail, **When** the command exits, **Then** it exits non-zero with an
   actionable error and leaves no session, stream, or temporary file behind.
3. **Given** `cosnip --doctor` on GNOME, **Then** it reports `backend: gnome-pipewire`, the
   desktop size, and each monitor's name, size, and origin.

---

### User Story 4 - Large Desktops and Texture Limits (Priority: P2)

As a user with a wide or 4K multi-monitor desktop, I can still see the frozen frame even when the
GPU cannot hold the whole desktop in one texture.

**Why this priority**: Multi-monitor desktops routinely exceed the 2048-pixel texture limit of
software or older GL stacks.

**Independent Test**: Run the overlay on a 3840x1080 Xvfb display whose GL stack reports a
2048-pixel maximum texture side; verify the selection UI renders and `--area` saves the exact
crop.

**Acceptance Scenarios**:

1. **Given** a frame wider than the maximum texture side, **When** textures are uploaded, **Then**
   the frame is split into tiles no larger than the limit.
2. **Given** tiled textures, **When** each monitor viewport draws, **Then** only intersecting
   tiles are drawn with correct source UVs and no visible seams.
3. **Given** a pinned selection larger than the texture limit, **When** the pin is shown, **Then**
   its image is tiled and the pin window still has the selection's dimensions.

### Edge Cases

- A monitor is hot-plugged or resized between capture and overlay: use the captured frame and the
  layout snapshot from the same run.
- A monitor layout has gaps: selection is still clamped to the captured frame; toolbar and
  magnifier choose the monitor containing the relevant point.
- PipeWire offers a DMA-BUF-only buffer: report a decode error and fall back; do not read invalid
  memory.
- The PipeWire signal arrives after the timeout: close the session and fall back.
- A frame has stride padding: use the stride for row addressing and validate the mapped length.
- A frame is larger than the logical monitor layout: scale each monitor frame into the XWayland
  desktop rectangle; mixed per-monitor native-resolution output remains a known limitation.
- Maximum texture size below monitor size: tile within the monitor.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: On GNOME Wayland, Cosnip MUST prefer a lossless Mutter ScreenCast + PipeWire
  backend and MUST report it as `gnome-pipewire`.
- **FR-002**: The backend MUST call `CreateSession`, `RecordArea`, and `Start`, then wait for the
  `PipeWireStreamAdded` signal on the stream object.
- **FR-003**: The PipeWire stream MUST connect to the advertised node with `AUTOCONNECT |
  MAP_BUFFERS` and MUST run the PipeWire main loop while waiting for a frame.
- **FR-004**: The converter MUST accept BGRx, BGRA, RGBx, and RGBA, honor row stride, and reject
  short or non-mappable buffers.
- **FR-005**: A frame MUST arrive or the backend MUST time out within 3 seconds; on failure the
  Mutter session MUST be stopped and the H.264 Screencast fallback attempted.
- **FR-006**: The successful PipeWire path MUST NOT invoke `ffmpeg` or write temporary media files.
- **FR-007**: Cosnip MUST enumerate active monitors through RandR and capture the full virtual
  desktop bounding box.
- **FR-008**: Cosnip MUST create one borderless fullscreen overlay viewport per monitor, using the
  monitor index from the layout snapshot.
- **FR-009**: Pointer, selection, magnifier, size badge, toolbar, and keyboard actions MUST use
  global captured-frame coordinates; each viewport converts to and from its monitor origin.
- **FR-010**: A drag beginning on one monitor MUST continue when the pointer enters another
  monitor, and confirmation MUST crop the global selection.
- **FR-011**: The overlay MUST remove the single-monitor rejection and MUST not fail merely because
  RandR reports more than one monitor.
- **FR-012**: Large frames MUST be uploaded as textures no larger than
  `InputState::max_texture_side`, capped at 2048 pixels per side. Pinned images MUST use the same
  tiling rule.
- **FR-013**: The fallback path MUST remain available, log that it is lossy, and preserve all
  Feature 002 outputs.
- **FR-014**: `--doctor` MUST list the detected backend and every monitor geometry.
- **FR-015**: Mixed per-monitor scale layouts MUST be supported by capturing each monitor
  separately and composing into a common XWayland desktop canvas. The composition MAY resample
  monitors whose compositor scale is below the global XWayland scale; true per-monitor
  native-resolution output is out of scope.
- **FR-016**: No `unsafe` code, new network access, telemetry, or reference assets may be added.

### Key Entities

- **MonitorInfo**: RandR monitor name, origin, size, and primary flag in X11 root coordinates.
- **OverlayMonitor**: A MonitorInfo normalized into captured-frame pixels plus a fullscreen
  viewport identity.
- **FrameTile**: A texture-sized slice of the captured frame with source origin and size.
- **MutterStream**: A Mutter session path, stream path, and PipeWire node id.
- **Selection**: A rectangle in global captured-frame coordinates.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: GNOME Wayland 1920x1080 capture-to-overlay p95 ≤ 600 ms (measured 142–412 ms in
  development builds; release expected lower).
- **SC-002**: A full-screen GNOME capture is pixel-identical to an X11 capture of the same static
  desktop content (manual/automated comparison).
- **SC-003**: A 200x50 drag crossing a simulated monitor boundary saves exactly 200x50.
- **SC-004**: A 3840x1080 desktop renders through 2048-pixel tiles without a texture panic.
- **SC-005**: Existing unit, CLI, X11, clipboard, save, and pin checks continue to pass.

## Out of Scope

- Per-monitor native-resolution output (the current composition uses a common XWayland-scale
  canvas).
- Per-monitor transform correction beyond what Mutter reports for the logical layout.
- The xdg-desktop-portal ScreenCast permission flow and restore tokens.
- KDE/KWin capture.
- Monitor hot-plug handling while the overlay is open.
- Annotations, history, tray, and global hotkeys.
