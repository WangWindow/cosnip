# Quickstart: Clipboard Paste and Pin

## Prerequisites

- A built `cosnip` binary (`cargo build --release`).
- A desktop session with a system clipboard (X11/Xorg or GNOME 49 Wayland + XWayland).
- An image copied to the clipboard, or a PNG file.

## Verify the clipboard path

1. Copy a known 500x300 image in any application.
2. Run:

   ```bash
   target/release/cosnip paste
   ```

3. A frameless pin of the image appears at the pointer. It must be 500x300 pixels and show the
   copied pixels exactly.
4. Drag the pin, scroll to zoom, and close it. The command should exit successfully.

No overlay, capture log, temporary media, or ffmpeg process should appear.

## Verify the file path

```bash
target/release/cosnip paste --file /tmp/known.png
```

The pin dimensions must match the PNG. A missing or non-PNG file must exit non-zero with an
actionable message and no window.

## Verify failures

```bash
# Copy text instead of an image, then:
target/release/cosnip paste

# Expected: non-zero exit and a message that no clipboard image is available.
```

## HiDPI check

On the mixed-scale validation desktop (XWayland 6976x2304, 2x global scale), paste a 300x200 image
at the pointer. The pin window must be 300x200 physical pixels at the pointer, not 600x400 and not
at double coordinates.

## Automated checks

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## Manual checklist

- [x] Clipboard image creates a pin at the pointer.
- [x] Pin pixels and dimensions match the source.
- [ ] Drag, wheel zoom, and close work (real-mouse check pending; geometry/render verified).
- [x] `--file` wins over the clipboard.
- [x] Text-only/empty clipboard and invalid file fail cleanly.
- [x] No capture backend, overlay, or temporary media on the paste path.
- [x] HiDPI position and size are correct.
- [x] Existing snip/copy/save/pin tests still pass.
