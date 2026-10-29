# Data Model: Clipboard Paste and Pin

## Entities

### ClipboardImage

A validated RGBA image read from the system clipboard.

| Field | Type | Rules |
|---|---|---|
| width | u32 | Non-zero; matches the source image |
| height | u32 | Non-zero; matches the source image |
| rgba | Vec<u8> | Exactly `width * height * 4` bytes; row-major, top-left origin |

`Frame::new` already enforces these rules. Clipboard `ImageData` is converted directly and
rejected if the byte length does not match.

### PinRequest

The input needed to open a pin-only session.

| Field | Type | Rules |
|---|---|---|
| image | Frame | From the clipboard or a decoded PNG |
| source | Clipboard or File(PathBuf) | `--file` takes precedence |
| pointer | Option<(i32, i32)> | Global X11/XWayland root coordinates; defaults to (0,0) |

### Pin

The existing pin window state, unchanged except for the startup source.

| Field | Type | Rules |
|---|---|---|
| frame | Frame | The pasted image |
| tiles | Vec<PinTile> | No tile larger than `min(max_texture_side, 2048)` per side |
| position | (f32, f32) | Viewport points, converted from the global pointer |
| scale | f32 | Starts at 1.0; wheel zoom clamps to 0.1–5.0 |
| viewport | ViewportId | Existing `cosnip-pin` viewport identity |

## Relationships

- `PinRequest` produces exactly one `Pin`.
- `ClipboardImage` and a decoded PNG are both `Frame` values; the pin path does not distinguish
  their origin.
- No clipboard data is persisted; the clipboard owner is only read.

## State Transitions

1. `Idle` → `Reading`: parse `paste`, choose `--file` or clipboard.
2. `Reading` → `Ready`: a validated frame is available.
3. `Reading` → `Failed`: empty/text clipboard, unavailable clipboard, or invalid file; exit
   non-zero with no window.
4. `Ready` → `Pinned`: the pin viewport is created at the pointer.
5. `Pinned` → `Closed`: the user closes the pin; the process exits 0.
