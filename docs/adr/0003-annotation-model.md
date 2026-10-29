# ADR 0003: Annotation Model and Dual Preview/Output Rendering

- **Status**: Accepted
- **Date**: 2026-10-29
- **Feature**: `004-annotation-editing`
- **Deciders**: Project owner, implementation team

## Context

Feature 002 shipped only a Copy/Save/Pin/Cancel toolbar. Feature 003 added multi-monitor selection
but the eight selection handles were decorative and there was no way to annotate a snip.

The reference product's defining behavior is the annotation toolbar: shapes, freehand, text,
numbers, mosaic/blur, magnifier, eraser, and undo/redo. Cosnip needs this without copying reference
assets or implementation, and output images must contain the annotations rather than the overlay
chrome.

Two approaches were considered:

1. Render annotations only with egui and snapshot the viewport for output.
2. Keep a pure annotation model in the core, preview it with egui, and rasterize it separately for
   output.

## Decision

1. **Annotations are a pure core model.**
   - `cosnip-core::AnnotationKind` stores rectangle, ellipse, arrow, line, pencil, marker, text,
     number, mosaic, blur, and magnifier payloads in captured-frame coordinates.
   - `AnnotationDocument` owns committed items plus an undo/redo history for additions and eraser
     removals.
   - `Tool` describes the active toolbar mode; selection handle geometry lives in `Handle` and
     `Selection`.

2. **The overlay is a live egui preview.**
   - Shapes use egui painter primitives; text uses egui fonts/IME.
   - Mosaic/blur previews are block approximations; the magnifier preview samples the frozen frame.
   - Selection move/resize uses global coordinates and the same eight handles that are drawn.

3. **Output is rasterized by `cosnip-imaging`.**
   - `render(frame, selection, annotations)` crops the frozen frame and draws the same model with
     software primitives.
   - Text is shaped and rasterized through `cosmic-text`, which uses the system font database and
     fallback (including CJK). The output path is independent of egui's renderer.
   - Mosaic and blur are real pixel effects; the magnifier samples the base frame.
   - Copy, save, and pin all use this rendered image, so UI chrome never appears in output.

4. **The toolbar is one compact single-row icon panel.**
   - Tools: Select, Rect, Ellipse, Arrow, Line, Pencil, Marker, Text, No., Mosaic, Blur, Zoom,
     Eraser; palette, widths, Undo/Redo, Copy/Save/Pin/Cancel.
   - Icons are original line drawings implemented with egui primitives in `icons.rs`; no copied
     reference assets.
   - Active icons get a blue fill, hover gets a light fill, and every button has a tooltip.
   - Persistent bottom-left shortcut tips are removed by design; the reference lives in README and
     `--help`.
   - The panel is positioned in the monitor containing the selection center and clamped to the
     screen.

5. **Pointer routing is a small state machine.**
   - Select tool: handle → resize; inside selection → move; outside → new selection.
   - Draw tools: inside selection → draft annotation; outside → new selection.
   - Text/Number/Magnifier are click tools; Text opens an inline editor.
   - Right-click finishes a draft or returns to Select; Escape cancels the draft first, then the
     overlay.
   - Annotations are clipped to the selection in both preview and output.

6. **Text preview and output may use different fonts.**
   - The preview prioritizes IME and immediate feedback; output prioritizes deterministic
     rasterization and font fallback.
   - Pixel-equal text between preview and output is not a requirement; content and placement are.

## Alternatives Considered

| Alternative | Why rejected |
|---|---|
| Snapshot the egui viewport for output | Includes chrome; depends on GL readback; hard to crop reliably |
| Render text only with a built-in bitmap font | No CJK/IME; poor typography |
| Add tiny-skia for all vector rendering | Extra stack; software primitives are sufficient for the current tool set |
| Store annotations in selection-local coordinates | Breaks when the selection is moved/resized after annotation |
| Make every annotation editable after commit | Requires a document selection/transform model; deferred as a follow-up |

## Consequences

**Positive**

- The pure model is testable without a GPU and shared by preview and output.
- Copy/save/pin cannot accidentally include the selection border or toolbar.
- CJK text is supported on systems with a CJK font.
- Undo/redo and eraser are deterministic and unit-tested.
- Selection handles are functional before annotation starts.

**Negative / Risks**

- Preview and output font metrics differ; text may shift slightly.
- `cosmic-text` adds a sizeable shaping/rasterization dependency.
- Mosaic/blur previews are approximations.
- Annotations cannot be moved, rotated, or restyled after commit; undo/erase only.
- The inline editor's IME behavior is only validated manually on a real session.

## Evidence

- `specs/004-annotation-editing/research.md`.
- Unit tests: 19 core, 10 imaging, 5 UI.
- Xvfb UI: rectangle, ellipse, arrow, text, number, mosaic, blur, magnifier, resize, undo.
- CJK fallback test passes with the target's Noto CJK fonts.
- Saved output contains annotations and no overlay chrome.
