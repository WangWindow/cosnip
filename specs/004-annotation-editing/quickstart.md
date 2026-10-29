# Quickstart: Annotation Editing and Selection Adjustment

## Run

```bash
cargo build
target/debug/cosnip snip --area 200 150 700 450 --save /tmp/annotated.png --no-copy
```

Use the icon toolbar under the selection (hover an icon for its name and hint):

- **Select** — move the selection or drag one of eight handles to resize.
- **Rect / Ellipse / Arrow / Line** — drag inside the selection.
- **Pencil / Marker** — freehand; marker is translucent and thicker.
- **Text** — click, type in the inline editor, Enter to place, Escape to cancel.
- **No.** — click to place the next number.
- **Mosaic / Blur** — drag a region.
- **Zoom** — click to place a magnifier.
- **Eraser** — click an annotation to remove it.
- Palette and `1px/3px/6px` set the style of new annotations.
- **Undo/Redo** or Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y.
- **Copy / Save / Pin** render annotations into the output.

Right-click finishes the current draft or returns to Select. Escape cancels a draft/editor first,
then closes the overlay.

## Verify

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Xvfb annotation check:

```bash
# :100 is a 1920x1080 Xvfb display.
DISPLAY=:100 XDG_SESSION_TYPE=x11 WAYLAND_DISPLAY= \
COSNIP_TEST_WINDOWED_OVERLAY=1 COSNIP_TEST_AUTOCONFIRM_MS=10000 \
target/debug/cosnip snip --area 200 150 700 450 --save /tmp/annotated.png --no-copy &
```

With a running app, click Rect in the toolbar and drag inside the selection; the saved PNG must
contain a red rectangle and no selection border/toolbar.

Resize check: drag the south-east handle from (900,600) to (1000,650); output must be 800x500.

Text check: select Text, click in the selection, type `Hello`, press Enter, then save. The output
must contain the text. CJK requires a system CJK font (for example Noto Sans CJK).

## Known limitations

- Annotations cannot be moved/rotated/restyled after commit; use undo or the eraser.
- Preview text uses egui fonts; output uses cosmic-text/system fonts and can differ slightly.
- Mosaic/blur previews are approximations; output is exact.
- IME needs an interactive session check.
