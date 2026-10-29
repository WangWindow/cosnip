# Data Model: Lossless GNOME Wayland Capture and Multi-Monitor Selection

## MonitorInfo

| Field | Type | Notes |
|---|---|---|
| `x`, `y` | `i32` | RandR monitor origin in X11 root coordinates |
| `width`, `height` | `u32` | Monitor size in root coordinates |
| `primary` | `bool` | RandR primary flag |
| `name` | `String` | Connector/monitor name (for diagnostics and fallback labels) |

Produced by `cosnip_capture::monitors()`. An empty result falls back to one monitor covering the
root screen.

## OverlayMonitor

| Field | Type | Notes |
|---|---|---|
| `index` | `usize` | Position in the RandR list; also the winit fullscreen monitor index |
| `x`, `y` | `i32` | Origin after normalizing and scaling into captured-frame pixels |
| `width`, `height` | `u32` | Size in captured-frame pixels |

Derived by `fit_monitors` from `MonitorInfo` plus the captured frame dimensions. `viewport()` maps
the index to a stable egui `ViewportId`.

## FrameTile

| Field | Type | Notes |
|---|---|---|
| `texture` | `TextureHandle` | Uploaded slice |
| `x`, `y` | `i32` | Source origin in the captured frame |
| `width`, `height` | `u32` | Source size, at most `min(max_texture_side, 2048)` |

Tiles are created once in `CosnipApp::with_context` and drawn by every monitor viewport that
intersects them.

## Selection

Unchanged from Feature 002, but all coordinates are now global captured-frame pixels rather than
single-monitor pixels. The selection is clamped to the frame `(0, 0, frame.width, frame.height)`.

## MutterStream (internal)

| Field | Type | Notes |
|---|---|---|
| `connection` | `zbus::Connection` | Session bus connection kept alive until Stop |
| `path` | `String` | `/org/gnome/Mutter/ScreenCast/Session/...` |
| `node` | `u32` | PipeWire node id from `PipeWireStreamAdded` |

## State Transitions

```text
detect GNOME Wayland
        |
        v
  GnomePipewire backend
        |
        +--> Mutter session created
        |       |
        |       +--> RecordArea accepted
        |       +--> PipeWireStreamAdded received
        |       +--> PipeWire stream Streaming
        |       +--> first raw frame converted
        |       +--> session Stop
        |
        +--> any failure --> log warning --> GnomeScreencast fallback
```

Overlay states:

```text
capture complete
    -> N fullscreen viewports open (one per monitor)
    -> pointer drag updates global Selection
    -> confirm/cancel closes all viewports
    -> copy/save/pin uses the global Selection
```
