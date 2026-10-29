# Implementation Plan: Annotation Editing and Selection Adjustment

**Branch**: `004-annotation-editing` | **Date**: 2026-10-29 | **Spec**:
[spec.md](spec.md)

## Summary

Add a pure annotation model to `cosnip-core`, functional selection handles, a compact annotation
toolbar, an egui preview, and a software rasterizer in `cosnip-imaging` used by copy/save/pin.
Text output uses `cosmic-text` 0.19 (font shaping/rasterization + system fallback).

## Technical Context

**Language**: Rust stable, edition 2024, MSRV 1.95.

**Dependencies**:

- New: `cosmic-text` 0.19 (MIT OR Apache-2.0) for text shaping/rasterization and CJK fallback.
- Existing: `eframe`/`egui` 0.37 (preview), `image` 0.25 (PNG), `cosnip-core`.
- No new system libraries; `cosmic-text` discovers fonts via fontdb.

**Model**:

```text
Annotation { kind, color, width }
AnnotationKind = Rectangle | Ellipse | Arrow | Line | Pencil | Marker
               | Text | Number | Mosaic | Blur | Magnifier
AnnotationDocument { items, undo, redo }
Tool = Select | Rectangle | Ellipse | Arrow | Line | Pencil | Marker
     | Text | Number | Mosaic | Blur | Magnifier | Eraser
Handle = NW | N | NE | E | SE | S | SW | W
```

All coordinates are captured-frame pixels. Text positions are top-left. Selection move/resize is
clamped to the frame with a minimum 4 px size.

**UI state machine**:

- Select: handle → resize; inside → move; outside → new selection.
- Draw tool: inside selection → draft; outside → new selection.
- Text: click opens `TextEditor`; Enter commits, Escape cancels.
- Number/Magnifier: click commits immediately.
- Eraser: click removes the topmost hit annotation.
- Drafts are committed on pointer release when geometrically valid.
- Right-click finishes a draft or returns a draw tool to Select.
- Escape cancels a draft/editor first, then closes the overlay.
- Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y edit the document.

**Preview**:

- egui painter draws shapes, text, numbers, block mosaics/blurs, and a cell-based magnifier.
- Preview is clipped to the selection; overlay chrome is never part of the model.
- The toolbar is one Area in the monitor containing the selection center, clamped to the screen.
- Pointer input over the toolbar/editor is excluded with `Context::is_pointer_over_egui`.

**Output**:

- `cosnip-imaging::render(frame, selection, annotations) -> Frame`.
- Crop first, then draw in global coordinates through a translating canvas.
- Raster primitives: thick line/disc stamping, rect/ellipse strokes, arrow head, polyline,
  translucent marker, disc numbers, text via cosmic-text.
- Mosaic: block averages; blur: box blur; magnifier: nearest-neighbor sample from the base crop.
- Copy/save/pin share the rendered image; pin re-tiles it for GL limits.

**Testing**:

- Core: 19 tests including annotations, undo/redo, erase, handle hit-test, resize, move.
- Imaging: 10 tests including BGRx, rectangle clipping, mosaic, blur, text, CJK fallback,
  magnifier.
- UI: 5 layout tests; manual Xvfb annotation flows.
- Xvfb: annotated save and pin smoke.

## Constitution Check

- **I. Personal/lawful**: PASS. Original code/assets.
- **II. Behavioral reference**: PASS. Behavior only; no reference assets or code.
- **III. Isolated platform edges**: PASS. Pure model in core; egui preview; imaging rasterizer;
  new dependency justified and recorded.
- **IV. Test-first/evidence**: PASS. Unit tests for model/effects, Xvfb visual checks, parity
  evidence, measured cleanup.
- **V. Local-only privacy**: PASS. No network/telemetry; images stay local.

No violations.

## Risks

| Risk | Mitigation |
|---|---|
| Preview text differs from output | Document; output is authoritative |
| Missing CJK font | System-font fallback; documented requirement |
| Mosaic/blur preview cost | Block cap (4096 cells) |
| Right-click/ctrl detection under Xvfb | Toolbar Undo tested; key undo tested with held modifier |
| Large text rendering cost | FontSystem per render; only on output, not per frame |

## Verification

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- Xvfb annotated save and pin flows.
