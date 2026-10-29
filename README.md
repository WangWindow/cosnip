# Cosnip

Cosnip is a personal, non-commercial Rust reimplementation of Snipaste-style snipping and pinning
for Linux. It targets **aarch64 and x86_64**, with aarch64 as the primary validation device.

The reference application is a behavioral oracle only. Cosnip uses original Rust code and original
assets; it does not ship or copy reference code, strings, or resources. See
`.specify/memory/constitution.md`.

## Status

**M0 reference analysis** is complete and accepted (Feature 001).

**MVP (Feature 002) is implemented**:

- `cosnip snip` captures the desktop, shows a full-screen selection overlay, and copies, saves, or
  pins the selected region.
- X11/Xorg capture uses native X11.
- GNOME 49 Wayland uses a short `org.gnome.Shell.Screencast` fallback because Shell/portal
  screenshot calls are denied on the target.
- Pins are frameless override-redirect image windows with move, wheel zoom, and close.
- Clipboard images survive process exit on GNOME via the shell clipboard manager (when it takes
  ownership).

Still out of scope: annotations, tray, global hotkeys, settings UI, single-instance IPC, history,
multi-monitor/mixed-DPI, KDE/portal capture, OCR, and paste-from-clipboard.

## Build

```bash
cargo build --release
cargo test
```

MSRV is Rust 1.95 (required by eframe/egui 0.37).

## Usage

```bash
# Snip and copy (default output)
target/release/cosnip snip

# Preselect a region and save it
target/release/cosnip snip --area 100 100 500 300 --save /tmp/cosnip.png

# Pin the selection
target/release/cosnip snip --area 100 100 500 300 --pin

# Full screen, copy
target/release/cosnip snip --full --copy

# Diagnostics
target/release/cosnip --doctor
```

Overlay keys: drag to select, arrows nudge, Shift keeps a square, Ctrl+A selects all,
Enter/Ctrl+C copies, Ctrl+S saves, P pins, Escape cancels.

## Platform support

| Session | Backend | Status |
|---|---|---|
| X11/Xorg | native X11 `GetImage` | supported |
| GNOME 49 Wayland + XWayland | GNOME Screencast single-frame fallback | supported, ~1.0–1.6 s capture |
| Other Wayland | — | unsupported in this MVP |
| XWayland root capture | — | not used; cannot see native Wayland surfaces |

The GNOME fallback requires `ffmpeg` on `PATH` and may briefly show the desktop recording
indicator. It is an explicit MVP tradeoff; a PipeWire/Mutter ScreenCast backend is the planned
replacement.

## Limitations

- Single monitor at scale 1.0 only; other layouts exit with a clear error.
- No annotations or text editing yet.
- No tray/global hotkeys yet; use a desktop shortcut that invokes `cosnip snip`.
- Clipboard persistence depends on a desktop clipboard manager; on a bare X11 session without one,
  the image may disappear when the process exits.
- Pins are opaque image-sized windows on GL configurations without alpha; shadows/rounded
  transparency are deferred.
- GNOME Shell/portal screenshot APIs are denied in the tested environment; the Screencast fallback
  is intentional.

## Repository layout

```text
crates/
  cosnip-core/       pure geometry, selection, frame, output types
  cosnip-imaging/    crop and PNG encode
  cosnip-capture/    X11 and GNOME Screencast backends
  cosnip-platform/   clipboard, paths, atomic writes, pointer queries
  cosnip-ui/         eframe overlay and pin viewports
  cosnip-app/        CLI and wiring
specs/               SDD feature specifications and research
docs/                ADR, parity matrix, dependency record
```

## Tests

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test

# Manual Xvfb integration (requires a running Xvfb):
DISPLAY=:99 XDG_SESSION_TYPE=x11 cargo test -p cosnip-capture --test x11_capture -- --ignored
DISPLAY=:99 XDG_SESSION_TYPE=x11 cargo test -p cosnip-app --test xvfb_smoke -- --ignored
```

Performance on the target (release): X11 capture ~12–35 ms; GNOME Screencast capture ~1.1–1.7 s;
overlay and pin are idle when not interacting.
