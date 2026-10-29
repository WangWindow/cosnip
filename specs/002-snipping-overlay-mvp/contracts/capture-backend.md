# Capture Backend Contract

```rust
pub trait CaptureBackend: Send + Sync {
    fn kind(&self) -> CaptureBackendKind;
    fn is_available(&self) -> Result<(), CaptureError>;
    fn capture(&self, request: &CaptureRequest) -> Result<CaptureFrame, CaptureError>;
}
```

## General Rules

- `capture` captures the full desktop in this MVP. The overlay applies `CaptureRequest.area`.
- Frames are RGBA8, row-major, top-left origin; `rgba.len() == width * height * 4`.
- Capture MUST NOT block the UI thread; callers run it on a worker.
- A timeout produces `CaptureError::Timeout`; no partial frame is returned.
- The backend MUST clean its own temporary artifacts before returning, including on error.
- Backend selection is runtime-based; architecture does not select a backend.

## X11 Backend (Xorg only)

- Availability requires a usable `DISPLAY` and a session that is not Wayland.
- Use `x11rb` `GetImage` on the root window; XShm is a later optimization.
- Return the exact screen pixels and dimensions; do not scale.
- On XWayland or a failed root read, return `Unsupported`/`CaptureError::Unavailable`; do not
  return a black frame as success.
- Capture must complete within 300 ms for 1920x1080 on the target.

## GNOME Screencast Backend (GNOME 49 Wayland)

- Availability requires:
  - a Wayland session with GNOME,
  - `org.gnome.Shell.Screencast` usable,
  - `ffmpeg` on PATH,
  - exactly one monitor at scale 1.0 (MVP).
- Use one persistent blocking D-Bus connection per capture:
  1. create a unique `$XDG_RUNTIME_DIR/cosnip-<pid>/` directory (0700),
  2. call `Screencast(file_template, {draw-cursor, framerate})`,
  3. wait 300–700 ms,
  4. call `StopScreencast` on the same connection,
  5. decode exactly one frame with ffmpeg to raw RGBA,
  6. validate `len == width * height * 4`,
  7. remove the MP4 and raw frame.
- Do not call `StopScreencast` from a second connection; this produces a truncated MP4.
- Total time MUST be ≤ 2.0 s and failures MUST be reported within 2.5 s.
- The recording indicator is an accepted MVP limitation.
- If the service or decoder is unavailable, return `Unavailable`/`Decode`; never fall back to
  XWayland root capture.

## Backend Selection

| Session | Desktop | Backend |
|---|---|---|
| X11 | any | X11 |
| Wayland | GNOME | GNOME Screencast |
| Wayland | other | Unsupported (MVP) |

## Test Interface

- Unit tests use a fake backend returning a deterministic 64x48 RGBA pattern.
- X11 integration tests run under Xvfb and compare captured pixels to a drawn pattern.
- GNOME integration is a manual/automated target test; mocked zbus tests cover error mapping and
  cleanup.
