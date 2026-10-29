# Cosnip

Cosnip is a personal, non-commercial Rust reimplementation of Snipaste-style snipping and pinning
for Linux. It targets **aarch64 and x86_64**, with aarch64 as the primary validation device.

The reference application is a behavioral oracle only. Cosnip uses original Rust code and original
assets; it does not ship or copy reference code, strings, or resources. See
`.specify/memory/constitution.md`.

## Status

**M0 reference analysis** is complete and accepted (Feature 001).

**MVP (Feature 002) is implemented**, **Feature 003 adds lossless GNOME capture and
multi-monitor selection at 1x**, and **Feature 004 adds selection editing and annotations**:

- `cosnip snip` captures the desktop, shows a full-screen selection overlay, and copies, saves, or
  pins the selected region.
- X11/Xorg capture uses native X11.
- GNOME 49 Wayland uses **Mutter ScreenCast + PipeWire raw BGRx frames** (lossless). The old
  `org.gnome.Shell.Screencast` H.264 path remains as an automatic, logged fallback.
- Multi-monitor desktops are captured as one virtual desktop. Each monitor gets a fullscreen
  overlay, and a selection can cross monitor boundaries in global coordinates.
- The selection has functional resize handles and can be moved before annotating.
- Annotation tools: rectangle, ellipse, arrow, line, pencil, highlighter, text, number, mosaic,
  blur, magnifier, and eraser, in a compact Snipaste-style icon toolbar with hover tooltips.
  Undo/redo covers adding and erasing annotations.
- Annotations are rasterized into copied, saved, and pinned output, including CJK text through
  system-font fallback.
- Large desktops and large pinned images are split into GL textures no larger than the driver limit
  (capped at 2048 per side), so 3840-pixel-wide layouts work on software/older GL stacks.
- Pins are frameless override-redirect image windows with move, wheel zoom, and close.
- Clipboard images survive process exit on GNOME via the shell clipboard manager (when it takes
  ownership).

Still out of scope: post-commit annotation editing/moving/rotation, tray, global hotkeys, settings
UI, single-instance IPC, history, mixed/fractional per-monitor DPI, KDE/portal capture, OCR, and
paste-from-clipboard.

## Build

```bash
# Debian/Ubuntu build dependencies for the PipeWire backend
sudo apt install libpipewire-0.3-dev libspa-0.2-dev libclang-dev libllvm-dev clang

cargo build --release
cargo test
```

The default build enables the lossless PipeWire backend. PipeWire is part of GNOME, so no extra
runtime package is normally needed. `ffmpeg` is only required if the H.264 fallback is used.
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

Overlay keys: drag to select, drag inside to move, drag a handle to resize, arrows nudge, Shift
keeps a square, Ctrl+A selects all, Ctrl+Z/Ctrl+Shift+Z undo/redo, Enter/Ctrl+C copies, Ctrl+S
saves, P pins, Escape cancels the current tool or the overlay. The toolbar is icon-only; hover an
icon for its name and hint. Persistent keyboard tips are intentionally not shown. Pick an annotation
tool and draw inside the selection; right-click finishes/leaves the current tool.

## Platform support

| Session | Backend | Status |
|---|---|---|
| X11/Xorg | native X11 `GetImage` | supported; pixel-exact |
| GNOME 49 Wayland + XWayland | Mutter ScreenCast + PipeWire raw frames | supported; lossless, ~0.26–0.42 s |
| GNOME Wayland without PipeWire | GNOME Shell Screencast H.264 | automatic fallback; lossy, ~1.1–1.7 s |
| Other Wayland | — | unsupported in this MVP |
| XWayland root capture | — | not used; cannot see native Wayland surfaces |

`--doctor` reports the selected backend and every detected monitor. On GNOME the lossless path
uses the same compositor stream as GNOME's recorder but reads raw `video/x-raw` buffers instead of
encoding H.264; no temporary video is written.

Multi-monitor is supported for a uniform scale of 1.0. Each monitor is covered by its own
fullscreen overlay; selection, magnifier, toolbar, and output use global desktop coordinates.
Real dual-monitor hardware validation is still pending (the implementation was verified with a
simulated 960+960 layout).

## Limitations

- Multi-monitor is supported at uniform scale 1.0. Mixed/fractional per-monitor DPI and monitor
  transform correction are not yet implemented; a raw frame larger than the logical layout is
  mapped with a uniform scale as a best effort.
- Real dual-monitor hardware has not been available for validation; the per-monitor overlay,
  focus transfer, and cross-screen drag were verified with a simulated layout.
- Direct Mutter ScreenCast is GNOME-specific. Other Wayland desktops need the portal ScreenCast
  path, which is future work.
- PipeWire development headers, libclang, and libLLVM are required at build time.
- Annotations cannot be selected, moved, rotated, or re-edited after they are committed; use
  undo/redo or the eraser. Text rotation and per-item style editing are deferred.
- Text output is shaped with `cosmic-text` and the system font database. CJK text requires a CJK
  font installed on the system; the overlay preview uses egui fonts and may differ slightly from
  the rasterized output.
- Mosaic/blur previews are block approximations; the saved/copied/pinned output uses the full
  software effect.
- The overlay does not show persistent shortcut tips; shortcuts are documented here and in tooltips.
- No tray/global hotkeys yet; use a desktop shortcut that invokes `cosnip snip`.
- Clipboard persistence depends on a desktop clipboard manager; on a bare X11 session without one,
  the image may disappear when the process exits.
- Pins are opaque image-sized windows on GL configurations without alpha; shadows/rounded
  transparency are deferred.
- The H.264 fallback may briefly show the desktop recording indicator and is used only when the
  lossless path fails.

## Repository layout

```text
crates/
  cosnip-core/       pure geometry, selection handles, annotation model/history, output types
  cosnip-imaging/    crop, PNG encode, software annotation rasterizer (cosmic-text)
  cosnip-capture/    X11, Mutter ScreenCast + PipeWire, GNOME Screencast fallback
  cosnip-platform/   clipboard, paths, atomic writes, pointer queries
  cosnip-ui/         eframe overlay, icon toolbar, pin viewports
  cosnip-app/        CLI and wiring
specs/               SDD feature specifications and research
docs/                ADR, parity matrix, dependency record
```

## Tests

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

# Manual Xvfb integration (requires a running Xvfb):
DISPLAY=:99 XDG_SESSION_TYPE=x11 cargo test -p cosnip-capture --test x11_capture -- --ignored
DISPLAY=:99 XDG_SESSION_TYPE=x11 cargo test -p cosnip-app --test xvfb_smoke -- --ignored
```

The Xvfb smoke test is also the 2048-texture-limit check on a 3840x1080 display.

Debug-only simulated multi-monitor layout (two 960x1080 halves):

```bash
COSNIP_TEST_MONITORS='960x1080+0+0,960x1080+960+0' \
COSNIP_TEST_WINDOWED_OVERLAY=1 \
COSNIP_TEST_AUTOCONFIRM_MS=8000 \
target/debug/cosnip snip --save /tmp/drag.png --no-copy
```

Drag across the 960-pixel boundary with an XTest tool; a 900→1100 horizontal drag saves 200 px.

Performance on the target (debug build): X11 capture ~0.5 s at 3840x1080, ~12–35 ms at 1080p;
GNOME Mutter/PipeWire capture-to-overlay 142–412 ms at 1920x1080; H.264 fallback ~1.1–1.7 s.
Overlay and pin are idle when not interacting.
