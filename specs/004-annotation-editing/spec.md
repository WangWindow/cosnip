# Feature Specification: Annotation Editing and Selection Adjustment

**Feature Branch**: `004-annotation-editing`

**Created**: 2026-10-29

**Status**: Implemented; real-session IME validation pending

**Input**: Make the selection handles functional and add the annotation toolbar that defines
Snipaste-style snipping: shapes, freehand, text, numbers, mosaic/blur, magnifier, eraser, and
undo/redo, with annotations included in copy/save/pin output.

## Context

Feature 002 shipped the capture → selection → output/pin loop with a Copy/Save/Pin/Cancel toolbar.
Feature 003 added multi-monitor capture and drew eight handles, but they did not resize anything.
The reference product's core value is annotation: the user selects a region, marks it up, then
copies, saves, or pins it.

This feature adds a pure annotation model, a live overlay preview, a software output rasterizer,
and a compact single-row icon toolbar with hover tooltips. Persistent keyboard tips are removed by
design; the shortcut reference stays in the README and `--help`.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Adjust the Selection (Priority: P1)

As a user, I can move and resize my selection before annotating, instead of starting over.

**Why this priority**: Functional handles are a basic expectation and determine whether the
annotation tools can be used on the intended region.

**Independent Test**: Start with `--area 200 150 700 450`, drag the south-east handle to
(1000,650), and verify the saved output is 800x500. Drag inside the selection and verify it moves.

**Acceptance Scenarios**:

1. **Given** a selection, **When** the pointer is within 8 px of a corner/edge handle, **Then** the
   handle is hit and dragging resizes that edge/corner while the opposite side stays fixed.
2. **Given** a selection, **When** the user drags inside it with the Select tool, **Then** the
   selection moves without changing size and stays clamped to the desktop.
3. **Given** no selection, **When** the user drags, **Then** a new selection is created.
4. **Given** the Select tool, **When** the user drags outside the selection, **Then** a new
   selection replaces the old one.

---

### User Story 2 - Draw Common Annotations (Priority: P1)

As a user, I can draw rectangles, ellipses, arrows, lines, freehand strokes, and highlighter
strokes inside the selection.

**Why this priority**: These are the most frequently used markup tools and validate the toolbar,
input state machine, preview, and output renderer.

**Independent Test**: Draw one of each on Xvfb, save, and verify the output contains the marks in
the correct positions but no overlay chrome.

**Acceptance Scenarios**:

1. **Given** the Rect/Ellipse tool, **When** the user drags, **Then** a shape preview follows the
   pointer and a valid shape is committed on release.
2. **Given** the Arrow/Line tool, **When** the user drags, **Then** the line ends at the pointer and
   the arrow gets a head.
3. **Given** the Pencil/Marker tool, **When** the user drags, **Then** a polyline is collected and
   committed; the marker is translucent and thicker.
4. **Given** Shift while dragging a rectangle/ellipse, **Then** the shape is square.
5. **Given** copied/saved/pinned output, **Then** annotations are rasterized and clipped to the
   selection.

---

### User Story 3 - Add Text, Numbers, Effects, and Erase (Priority: P2)

As a user, I can place text, automatic numbers, mosaic/blur regions, and a magnifier, and erase a
mistake.

**Why this priority**: These tools complete the Snipaste annotation surface and exercise the
software effects/text pipeline.

**Independent Test**: Place text, two numbers, a mosaic region over text, a blur region over text,
and a magnifier; save and inspect the result.

**Acceptance Scenarios**:

1. **Given** the Text tool, **When** the user clicks, **Then** an inline editor opens at that point;
   Enter commits and Escape cancels. The output contains the text.
2. **Given** the Text tool and CJK input, **When** a CJK font is installed, **Then** the output uses
   font fallback and renders the characters.
3. **Given** the Number tool, **When** the user clicks, **Then** a numbered disc is placed and the
   next click uses the next number.
4. **Given** Mosaic/Blur, **When** the user drags a brush stroke, **Then** the saved output contains a pixelated /
   blurred region and the preview shows a block approximation.
5. **Given** the Magnifier tool, **When** the user clicks, **Then** a circular zoomed view of the
   frozen frame is committed.
6. **Given** the Eraser, **When** the user drags over an annotation, **Then** the covered pixels are
   removed and the removal is undoable.

---

### User Story 4 - Undo, Redo, and Finish Tools (Priority: P1)

As a user, I can undo/redo and finish a stroke without leaving the overlay.

**Why this priority**: Annotation mistakes are common; undo/redo is essential for a usable tool.

**Independent Test**: Add two rectangles, click Undo, and verify only one remains; Redo restores
it. Draw a pencil stroke and right-click to finish it.

**Acceptance Scenarios**:

1. **Given** annotations, **When** the user presses Ctrl+Z or clicks Undo, **Then** the last add or
   erase is reverted.
2. **Given** an undone action, **When** the user presses Ctrl+Shift+Z/Ctrl+Y or clicks Redo, **Then**
   it is reapplied.
3. **Given** a draft stroke, **When** the user right-clicks, **Then** the draft is finished.
4. **Given** no draft, **When** the user right-clicks, **Then** the active draw tool returns to
   Select.
5. **Given** Escape, **Then** the current draft is cancelled first; a second Escape closes the
   overlay.

---

### User Story 5 - Output Contains Annotations Only (Priority: P1)

As a user, copied, saved, and pinned images contain my annotations but not the selection border,
handles, toolbar, magnifier HUD, or tips.

**Why this priority**: Overlay chrome in output would make the feature unusable.

**Independent Test**: Save an annotated selection and inspect the PNG; verify the annotation is
present and the blue selection chrome is absent. Pin the same region and inspect the pin image.

**Acceptance Scenarios**:

1. **Given** annotations, **When** the user copies, saves, or pins, **Then** the output is
   `render(frame, selection, annotations)`.
2. **Given** annotations outside the selection, **Then** they are clipped.
3. **Given** no annotations, **Then** output is pixel-identical to the previous crop behavior.

### Edge Cases

- A zero-size shape or a click with a drag tool is discarded.
- A selection resize cannot cross the opposite edge (minimum 4 px).
- Text with an empty string is discarded.
- Text output uses the system font database; missing glyphs render as fallback/blank.
- Mosaic/blur previews cap blocks to avoid excessive draw calls; output is exact.
- Annotations survive selection move/resize in desktop coordinates; output clips to the new
  selection.
- Undo/redo history is session-local and not persisted.
- The text editor swallows Escape/Enter before global shortcuts.
- Right-click on the toolbar does not alter the tool.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The selection MUST expose eight resize handles and move/resize operations in the pure
  core.
- **FR-002**: Annotation kinds MUST be rectangle, ellipse, arrow, line, pencil, marker, text,
  number, mosaic, blur, and magnifier.
- **FR-003**: `AnnotationDocument` MUST support add, erase-at-point, undo, redo, and clear.
- **FR-004**: The overlay MUST preview annotations with egui and clip them to the selection.
- **FR-005**: Copy, save, and pin MUST render annotations with `cosnip-imaging::render`, not with
  viewport capture.
- **FR-006**: Text output MUST use system-font shaping/rasterization with fallback, including CJK
  when a CJK font is installed.
- **FR-007**: Mosaic and blur MUST alter output pixels; magnifier MUST sample the frozen frame.
- **FR-008**: The toolbar MUST provide all tools, palette, widths, undo/redo, and output actions as
  a compact icon toolbar with hover tooltips.
- **FR-009**: Pointer routing MUST distinguish handle resize, selection move, new selection, draft
  annotation, text editor, and eraser.
- **FR-010**: Ctrl+Z, Ctrl+Shift+Z/Ctrl+Y, Escape, right-click, Enter, and existing output
  shortcuts MUST remain functional.
- **FR-011**: Output MUST NOT contain selection borders, handles, toolbars, tips, or HUDs.
- **FR-012**: Annotations MUST be stored in captured-frame coordinates and clipped at render time.
- **FR-013**: No new `unsafe`, network access, telemetry, or reference assets may be introduced.
- **FR-014**: The annotation model MUST remain in the pure core; UI and imaging consume it.
- **FR-015**: Persistent bottom-left shortcut tips MUST be absent by default; hover tooltips MUST
  identify icon buttons.

### Key Entities

- **Tool**: The active toolbar mode.
- **Annotation**: Kind + color + width.
- **AnnotationDocument**: Committed annotations plus undo/redo history.
- **Handle**: One of eight selection resize anchors.
- **TextEditor**: Inline text placement state (monitor, position, buffer).
- **FrameTile**: GL texture slice for overlay display.

## Success Criteria *(mandatory)*

- **SC-001**: A 700x450 selection resized from its SE handle to (1000,650) saves 800x500.
- **SC-002**: One of each shape/effect tool produces visible output in the expected region.
- **SC-003**: Text "Hello" and CJK "中文" render in output tests.
- **SC-004**: Undo removes the newest add/erase; redo restores it.
- **SC-005**: Saved annotated output contains no overlay chrome.
- **SC-006**: All workspace tests, fmt, and clippy pass.

## Out of Scope

- Post-commit annotation selection, move, rotate, restyle, or re-edit.
- Text rotation and rich text.
- Per-item z-order controls.
- Arrow head geometry matching the reference exactly.
- Automated IME testing; manual validation required.
- Annotations in history/session restore.
