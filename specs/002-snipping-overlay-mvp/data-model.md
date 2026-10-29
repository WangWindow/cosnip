# Data Model: Snipping Overlay MVP

## Geometry

All desktop coordinates are integers in captured-frame pixels for this MVP (single monitor, 1x).

| Type | Fields | Rules |
|---|---|---|
| `Point` | `x: i32`, `y: i32` | May be negative only during drag; clamp before output |
| `Size` | `w: u32`, `h: u32` | Both > 0 for output |
| `Rect` | `x`, `y`, `w`, `h` | Normalized; `right = x + w`, `bottom = y + h` |
| `ScreenInfo` | `width`, `height`, `scale`, `name`, `count` | MVP requires count = 1 and scale = 1.0 |

## Capture

| Type | Fields | Notes |
|---|---|---|
| `CaptureRequest` | `area: Option<Rect>`, `include_cursor: bool`, `timeout` | Area is applied by the overlay in MVP; the backend captures the full desktop |
| `CaptureFrame` | `width`, `height`, `stride`, `rgba: Vec<u8>`, `backend`, `captured_at` | RGBA8, row-major, top-left origin; `Arc`-shared with UI |
| `CaptureBackendKind` | `X11`, `GnomeScreencast` | Selected at runtime |
| `CaptureError` | `Unsupported`, `Unavailable`, `Timeout`, `Decode`, `Cleanup` | User-facing categories |
| `CaptureBackend` | `kind()`, `is_available()`, `capture() -> Result<CaptureFrame>` | Implemented by X11 and GNOME adapters |

## Selection and Document

| Type | Fields | Rules |
|---|---|---|
| `Selection` | `rect: Option<Rect>`, `dragging: bool`, `anchor: Point` | Empty selection is invalid for output |
| `SnipDocument` | `frame: Arc<CaptureFrame>`, `selection: Selection`, `output: OutputRequest` | Immutable frame; selection is UI state |
| `OutputRequest` | `copy: bool`, `save: Option<PathBuf>`, `pin: bool` | At least one destination; default copy |
| `CliOptions` | `command`, `area`, `full`, `delay_ms`, `output`, `doctor` | Parsed by clap |

## Output

| Type | Fields | Notes |
|---|---|---|
| `Crop` | `rect`, `pixels: Vec<u8>` | Exact frame pixels; no scaling |
| `PngOutput` | `width`, `height`, `path` | Atomic write |
| `ClipboardImage` | `width`, `height`, `rgba` | `arboard::ImageData` |
| `OutputResult` | `copied`, `saved_path`, `pinned` | Reported to CLI/log |

## Pin

| Type | Fields | Notes |
|---|---|---|
| `PinId` | `u64` | Monotonic |
| `PinWindow` | `id`, `image: Arc<CaptureFrame>`, `position`, `scale`, `viewport_id` | Image-sized at scale 1.0 |
| `PinState` | `pins: Vec<PinWindow>`, `dragging: Option<PinId>`, `last_pointer` | UI-owned |

## Temp and Diagnostics

| Type | Fields | Notes |
|---|---|---|
| `RuntimeTemp` | `dir: PathBuf` | `$XDG_RUNTIME_DIR/cosnip-<pid>/`; 0700; RAII cleanup |
| `DoctorReport` | `session`, `backend`, `ffmpeg`, `monitor`, `scale`, `clipboard` | Printed by `--doctor` |
| `AppError` | `kind`, `message`, `source` | Exit code mapping |

## Validation

- `Selection` output requires width > 0 and height > 0 after clamping.
- Saved PNG must decode and have exactly the selection dimensions.
- A pin must have width/height > 0 and a position that leaves at least 1 pixel visible.
- Temp files must be deleted on success, cancel, and error; tests assert the directory is empty.
- A capture frame must have `rgba.len() == width * height * 4`.
- Backend selection must be based on runtime session variables, not compile-time architecture.
