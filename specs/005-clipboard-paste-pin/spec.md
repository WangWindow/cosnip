# Feature Specification: Clipboard Paste and Pin

**Feature Branch**: `005-clipboard-paste-pin`

**Created**: 2026-10-29

**Status**: Implemented; real-mouse pin interaction verification pending

**Input**: User description: "Plan the next feature: paste an image from the system clipboard as a
floating pin, matching Snipaste's F3 workflow. The user copies an image in any application, runs
`cosnip paste` (or binds it to a desktop shortcut), and gets a pin at the pointer without opening
the capture overlay. Also support pinning an image file for testing and scripting."

## Context

Features 002–004 delivered capture, selection, annotation, clipboard output, and pins. The
remaining daily-use gap in the reference workflow is the reverse direction: taking an image from
the clipboard and pinning it. The reference binds this to F3. Cosnip does not yet provide a
paste command, and the current pin creation path exists only after a snip.

This feature adds the paste-to-pin path without introducing a resident service or global hotkey.
Binding `cosnip paste` to a desktop shortcut is the documented workaround, consistent with the
current `cosnip snip` shortcut. Single-instance IPC and built-in global hotkeys remain follow-up
features.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Paste a Clipboard Image as a Pin (Priority: P1)

As a user who has copied an image in a browser, chat, or image editor, I can run `cosnip paste`
and immediately get a frameless pin of that image at my pointer, without a capture overlay.

**Why this priority**: This completes the copy/paste pin loop that defines the reference product
and makes Cosnip useful for comparing or referencing images without taking a new screenshot.

**Independent Test**: Put a known 500x300 PNG on the clipboard, run `cosnip paste`, and verify
that a pin window of exactly 500x300 appears at the pointer with the same pixels. Move, zoom, and
close it, then verify the process exits.

**Acceptance Scenarios**:

1. **Given** the clipboard contains an image and the pointer is on the desktop, **When** the user
   runs `cosnip paste`, **Then** a frameless pin showing the clipboard image appears at the
   pointer without opening the snip overlay or starting a capture backend.
2. **Given** a pasted pin, **When** the user drags it, scrolls over it, or closes it, **Then** it
   moves, zooms, and closes like a pin created from a snip.
3. **Given** a pasted image larger than the GL texture limit, **When** the pin is created, **Then**
   it is tiled and opens without a texture-size panic.
4. **Given** a pasted pin is the only window, **When** it is closed, **Then** the command exits
   successfully.

---

### User Story 2 - Pin an Image File (Priority: P2)

As a user or script, I can pin an existing PNG file with `cosnip paste --file PATH` for testing,
automation, or showing a saved snip again.

**Why this priority**: It makes the paste path verifiable without a clipboard session and supports
the reference's file-paste behavior with a small, contained extension.

**Independent Test**: Run `cosnip paste --file /tmp/known.png` on a desktop and verify the pin
matches the file dimensions and pixels.

**Acceptance Scenarios**:

1. **Given** a valid PNG file, **When** the user runs `cosnip paste --file PATH`, **Then** the pin
   shows the decoded image at the pointer.
2. **Given** a missing, unreadable, or non-PNG file, **When** the command runs, **Then** it exits
   non-zero with an actionable message and creates no window.
3. **Given** both a clipboard image and `--file`, **When** the command runs, **Then** `--file`
   takes precedence and the clipboard is not read.

---

### User Story 3 - Clear Failure When No Image Is Available (Priority: P2)

As a user, I can understand why paste did not produce a pin and can correct the situation without
leaving stray windows or processes.

**Why this priority**: Paste is often invoked by a shortcut; silent failure or a blank window is
worse than a clear message.

**Independent Test**: Clear the clipboard (or copy text only) and run `cosnip paste`; verify a
non-zero exit, a message naming the missing image, and no pin window.

**Acceptance Scenarios**:

1. **Given** the clipboard contains only text, **When** the user runs `cosnip paste`, **Then** the
   command reports that no image is available and exits non-zero.
2. **Given** the clipboard is unavailable or the display has no clipboard owner, **When** the
   command runs, **Then** it reports the clipboard error and exits non-zero.
3. **Given** a paste failure, **When** the command exits, **Then** no pin, overlay, capture
   session, or temporary file remains.

---

### User Story 4 - Keep the Existing Snip Workflow Unchanged (Priority: P3)

As an existing user, I can continue to use `cosnip snip` exactly as before while the new paste
command is added.

**Why this priority**: Paste must not regress capture, selection, annotation, output, or pins.

**Independent Test**: Run the existing snip/copy/save/pin checks after the paste implementation;
verify identical results and exit behavior.

**Acceptance Scenarios**:

1. **Given** the existing snip commands, **When** the workspace test suite runs, **Then** all
   prior capture, selection, annotation, clipboard, save, and pin checks pass.
2. **Given** a `cosnip paste` failure, **When** the command exits, **Then** an unrelated running
   snip or pin is not affected.

### Edge Cases

- The pointer is outside the captured desktop or the desktop layout changes between paste
  invocations: clamp the pin position to the visible desktop.
- The clipboard image has an alpha channel: preserve it; pins may still render opaque on GL
  configurations without alpha, and that limitation is documented.
- The image is larger than the desktop: place it at the pointer, clamped so its top-left is
  visible; the rest may extend off-screen and can be moved.
- The clipboard image is extremely large: tile textures and avoid allocating a second full copy
  beyond the decoded frame.
- The clipboard changes while the image is being read: use the snapshot returned by the read.
- Two `cosnip paste` commands run concurrently: each may create its own pin or forward to a
  single instance later; this feature does not require single-instance behavior.
- The desktop uses a global XWayland scale: position and size the pin in viewport points so it
  appears at the pointer, not at double coordinates.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Cosnip MUST provide a `paste` command that reads an image from the system clipboard.
- **FR-002**: On success, the command MUST create a pin at the global pointer without opening a
  capture overlay, starting a capture backend, or copying/saving a new image.
- **FR-003**: The pin image MUST preserve the clipboard image dimensions and RGBA channel values.
- **FR-004**: Pasted pins MUST use the existing pin window behavior: frameless, always-on-top,
  draggable, wheel-zoomable, closable, and texture-tiled.
- **FR-005**: Pin position and size MUST be expressed in the desktop's viewport coordinate space,
  with the pin placed at the pointer and clamped to the desktop bounds.
- **FR-006**: The command MUST support `--file PATH` to pin a PNG file, with `--file` taking
  precedence over the clipboard.
- **FR-007**: An empty clipboard, text-only clipboard, unsupported clipboard format, or
  unreadable file MUST produce a non-zero exit and an actionable message, with no window left
  behind.
- **FR-008**: The paste path MUST NOT create a Mutter ScreenCast session, PipeWire stream, X11
  root capture, temporary media file, or ffmpeg invocation.
- **FR-009**: `cosnip --doctor` MUST continue to report clipboard availability; paste-specific
  diagnostics MAY be added without changing the existing fields.
- **FR-010**: Existing `snip` commands and outputs MUST remain compatible.
- **FR-011**: No `unsafe` code, new network access, telemetry, or reference assets may be added.

### Key Entities

- **ClipboardImage**: A validated RGBA frame read from the system clipboard, with width, height,
  and pixel data.
- **PinRequest**: The image, global pointer position, and optional source file used to create a
  pin.
- **Pin**: The existing frameless image window with position, scale, and tiled textures.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A 1920x1080 clipboard image appears as a pin within 300 ms of command start on the
  validation desktop.
- **SC-002**: The pinned image is pixel-identical to the clipboard source for all tested sizes and
  alpha values.
- **SC-003**: No capture backend, overlay, temporary media, or ffmpeg process is used on the paste
  path.
- **SC-004**: Empty/text-only clipboard and invalid-file cases exit non-zero with no remaining
  pin or overlay in 100% of tests.
- **SC-005**: Existing workspace tests, fmt, and clippy checks remain green.

## Assumptions

- The system clipboard exposes images through the existing `arboard` adapter on X11 and GNOME
  Wayland; text/HTML/URI/color paste is out of scope for this feature.
- The pin window behavior and texture tiling from Features 002–004 are reused; this feature only
  adds a new way to obtain the image and start the pin.
- Global hotkeys and single-instance IPC remain follow-up features; users bind `cosnip paste` to
  a desktop shortcut, as they do for `cosnip snip` today.
- PNG is the supported file format for `--file`; other image formats are future work.
- The desktop coordinate space and HiDPI conversion already established by Feature 003 apply to
  pasted pins.
