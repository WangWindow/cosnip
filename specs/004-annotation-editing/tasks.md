# Tasks: Annotation Editing and Selection Adjustment

**Feature**: `004-annotation-editing`

Legend: `[x]` complete, `[ ]` pending.

## Phase 1 — Selection

- [x] T001 Add `Handle`, handle positions, hit testing, resize, and move to `cosnip-core`.
- [x] T002 Keep selection clamped to the frame and enforce a minimum size.
- [x] T003 Draw handles only in Select mode and make them draggable.
- [x] T004 Add core tests for hit testing, resize, clamping, and move.
- [x] T005 Verify a 700x450 selection resized to 800x500 on Xvfb.

## Phase 2 — Model

- [x] T010 Add `Tool`, `AnnotationKind`, `Annotation`, and bounds/hit testing.
- [x] T011 Add `AnnotationDocument` with add, erase, undo, redo, and can_undo/can_redo.
- [x] T012 Add core tests for undo/redo, erase-topmost, line hit testing, tool metadata.
- [x] T013 Add `Rect::expand`.

## Phase 3 — Output rasterizer

- [x] T020 Add `cosmic-text` 0.19 and record the dependency.
- [x] T021 Add `cosnip-imaging::render` with translating canvas.
- [x] T022 Draw rect/ellipse/arrow/line/pencil/marker.
- [x] T023 Draw text/number with cosmic-text; add CJK fallback test.
- [x] T024 Add mosaic, box blur, and base-frame magnifier.
- [x] T025 Clip annotations to the selection.
- [x] T026 Add imaging tests for text, mosaic, blur, magnifier, clipping.
- [x] T027 Route copy/save/pin through the rasterized image.

## Phase 4 — Overlay UI

- [x] T030 Add toolbar tools, palette, widths, undo/redo, and output actions.
- [x] T031 Add the pointer state machine (handle/move/new/draft/eraser).
- [x] T032 Add text editor with Enter/Escape and IME-capable TextEdit.
- [x] T033 Add right-click finish/return-to-Select.
- [x] T034 Add Ctrl+Z/Ctrl+Shift+Z/Ctrl+Y.
- [x] T035 Clip preview annotations to the selection.
- [x] T036 Exclude toolbar/editor clicks from canvas actions.
- [x] T037 Replace persistent tips with tooltips; keyboard reference stays in README.
- [x] T038 Original line icon set, single-row icon toolbar, active/hover/tooltip states.
- [x] T039 Verify output contains no overlay chrome.

## Phase 5 — Verification and docs

- [x] T040 Xvfb rectangle, ellipse, arrow, text, number, mosaic, blur, magnifier.
- [x] T041 Xvfb resize and undo.
- [x] T042 `cargo fmt`, clippy, workspace tests.
- [x] T043 Update README, dependencies, parity matrix, ADR, specs.
- [ ] T044 Real-session IME (ibus/fcitx) check.
- [x] T045 Pin with annotations visual check.
- [ ] T046 Post-commit annotation move/rotate/restyle (future feature).
- [ ] T047 Automated golden tests for text/effects across platforms.

## Follow-ups

- Annotation object selection and transform handles.
- Text rotation and rich text.
- Per-item z-order and style editing.
- Persist annotations in history? (probably not; snips are ephemeral)
