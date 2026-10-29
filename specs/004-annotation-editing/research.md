# Research: Annotation Editing and Selection Adjustment

**Feature**: `004-annotation-editing` | **Date**: 2026-10-29

## 1. Rendering strategy

The overlay already uses eframe/egui with GL textures. Rendering annotations **only** in egui would
make copy/save/pin require a viewport readback, and would risk including selection chrome.

Decision: a pure annotation model plus two renderers:

- egui preview for latency and IME.
- software rasterizer for output.

This mirrors the reference architecture (document model + renderer) without using its code.

## 2. Selection handles

The existing overlay drew eight dots but had no hit testing. The core now exposes `Handle`,
`Selection::handle_at`, `resize`, and `move_from`.

- Handle hit tolerance: 8 px, closest handle wins.
- Resize keeps the opposite edge fixed, clamps to the desktop, and enforces a 4 px minimum.
- Move clamps the delta so the selection never leaves the frame.
- Handle dots are drawn only when the Select tool is active.

Verified on Xvfb: resizing a 700x450 selection from (900,600) to (1000,650) saved 800x500.

## 3. Text and CJK

Options considered:

| Option | Result |
|---|---|
| egui text for output | Requires viewport capture; font metrics differ; no direct pixel access |
| Built-in bitmap font | No CJK/IME |
| `fontdue`/`ab_glyph` + a single font | TTC/CJK fallback and shaping are manual |
| `cosmic-text` 0.19 | Font database, fallback, shaping, swash rasterization |

`cosmic-text` was already listed in the dependency plan. The renderer creates a `FontSystem`, a
`Buffer`, and a `SwashCache` per text annotation, composites callback coverage into the canvas, and
clips to the canvas bounds. A unit test renders `中文` and finds changed pixels on the target
(with Noto CJK installed).

The overlay editor uses egui's `TextEdit`, which supports IME through winit. Preview fonts may
differ from output fonts; content/placement is the compatibility requirement, not pixel identity.

## 4. Effects

- **Mosaic**: block average over the current canvas, block = `max(8, width*4)`.
- **Blur**: box blur over the current canvas, radius = `max(2, width*2)`.
- **Magnifier**: nearest-neighbor sample from the **base crop**, zoom 2, radius 48.
- Preview approximations use center-sampled blocks capped at 4096 cells; output is exact.

Verified on Xvfb over an `xmessage` text window: mosaic pixelated the text, blur softened it, and
the magnifier showed enlarged source text.

## 5. Input state machine

- Select tool: handle → resize, inside → move, outside → new selection.
- Draw tools: inside selection → draft; outside → new selection.
- Text/Number/Magnifier: click actions; text opens an inline editor.
- Drafts are committed on release if valid; zero-size shapes are discarded.
- Right-click finishes a draft/returns to Select.
- Escape cancels draft/editor before closing.
- Ctrl+Z/Ctrl+Shift+Z/Ctrl+Y operate on `AnnotationDocument`.
- `Context::is_pointer_over_egui` prevents toolbar/editor clicks from starting canvas actions.

## 6. Output isolation

Copy/save/pin call `render(frame, selection, annotations)`. The renderer starts from a crop of the
frozen frame and draws only model annotations. Selection borders, handles, toolbar, tips, and HUDs
are UI-only.

Verified: saved PNG contains the red rectangle but no blue selection border/handles.

## 7. Icon toolbar and tips

The text-label toolbar was replaced with a compact single-row icon panel:

- Original 24-unit line icons are drawn with egui primitives (`icons.rs`); no reference assets.
- Active tool: blue fill/white icon; hover: subtle light fill; disabled undo/redo: dimmed.
- Palette uses rounded swatches with a white selection ring; stroke widths use line glyphs.
- Every icon has a two-line tooltip (name + hint).
- Persistent bottom-left tips were removed by design; README/`--help` remain the shortcut
  reference.

Verified on Xvfb: the 760-pixel panel fits below a 700-pixel selection, the Rect tooltip appears on
hover, the Rect icon selects the tool, and the Save icon writes the annotated PNG.

## 8. Evidence

| Check | Result |
|---|---|
| Core tests | 19 pass |
| Imaging tests | 10 pass |
| UI tests | 6 pass (layout + all icons draw) |
| Rectangle | UI + output correct |
| Ellipse + arrow | UI + output correct |
| Text "Hello" | inline editor + output text |
| CJK "中文" | unit test renders pixels |
| Number | sequential disc |
| Mosaic | text pixelated |
| Blur | text softened |
| Magnifier | enlarged text in circle |
| Resize | 700x450 → 800x500 |
| Undo | toolbar removes newest rectangle |
| Key undo | works with held Ctrl; Xvfb chord timing is flaky |
| Output chrome | absent |
| Real-session IME | pending manual check |

## 8. Open gaps

1. Post-commit annotation selection/move/rotate/restyle is not implemented.
2. Text rotation and rich text are not implemented.
3. Preview/output text metrics differ.
4. IME needs a real GNOME session with ibus/fcitx.
5. Pin with annotations was visually checked on Xvfb: a 700x450 pin at (200,150) contains the red rectangle.
