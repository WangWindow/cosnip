# Parity Matrix: Snipaste 2.11.3 Linux

**Reference**: Snipaste 2.11.3 Linux x86_64 AppImage `02a9af2118f502ce9a9e4be16026df05cda1920d`
**Target**: Cosnip, aarch64/x86_64 Linux
**Status legend**: `observed` = behavior documented; `partial` = evidence exists but a critical
path is unverified; `gap` = unresolved; `deferred` = knowingly out of MVP scope.
**Verification**: `unit`, `integration`, `golden`, `manual`.

| ID | Category | Reference behavior | Priority | Evidence | Status | Verification |
|---|---|---|---|---|---|---|
| BEH-001 | capture | X11 session uses a native X11 grabber | P1 | OBS-002, OBS-006, OBS-025 | observed | integration on Xorg |
| BEH-002 | capture | Wayland backend selection: KDE → KWin, GNOME ≤40 → Shell D-Bus, other → portal | P1 | OBS-002, OBS-003, OBS-004, OBS-005 | observed | unit + integration with mocked D-Bus |
| BEH-003 | capture | GNOME ≥41 uses the freedesktop portal Screenshot request | P1 | OBS-002, OBS-005 | partial | manual portal test; permission UX |
| BEH-004 | capture | KWin ScreenShot2 returns a PipeWire FD with cursor/native-resolution options | P2 | OBS-004 | observed | KDE integration test |
| BEH-005 | capture | Captured desktop includes cursor when enabled | P2 | OBS-003, OBS-004 | observed | golden/manual |
| BEH-006 | overlay | Snipping window covers the virtual desktop | P1 | OBS-007, OBS-009 | observed | integration |
| BEH-007 | overlay | Selection rectangle with eight handles and pixel size badge | P1 | OBS-009 | observed | golden/manual |
| BEH-008 | overlay | Magnifier with pixel grid, coordinates, RGB/HEX, C/Shift hints | P1 | OBS-010 | observed | golden |
| BEH-009 | overlay | Bottom-left keyboard tips: WASD, arrows, Tab, Ctrl+A, R/Shift+R, ,/., backtick/! | P1 | OBS-009 | observed | manual |
| BEH-010 | overlay | Annotation toolbar appears under the selection | P1 | OBS-009 | observed | golden |
| BEH-011 | overlay | WM-managed fullscreen gets keyboard focus on XWayland | P1 | OBS-016 | observed | spike integration |
| BEH-012 | overlay | Override-redirect overlay does not get focus on XWayland | P1 | OBS-016 | observed | spike/manual |
| BEH-013 | pin | Pins are override-redirect, frameless, always-on-top, no taskbar/pager | P1 | OBS-007, OBS-017 | observed | X11 property integration |
| BEH-014 | pin | Pins use an ARGB visual for transparency | P2 | OBS-018 | partial | needs EGL/alpha-config or ARGB adapter |
| BEH-015 | pin | Click-through uses an empty XFixes input region | P1 | OBS-007, OBS-017 | observed | shape integration test |
| BEH-016 | pin | Drag/wheel zoom/Ctrl+wheel opacity/rotate/flip/reset/close/destroy | P2 | OBS-014 | observed | manual interaction tests |
| BEH-017 | pin | Pin above native Wayland windows | P1 | OBS-023 | observed | native screencast; recheck KDE |
| BEH-018 | annotation | Item families: rectangle/ellipse, arrow/line, pencil, marker, text, mosaic, blur, magnifier, number, eraser | P1 | OBS-015 | observed | unit/golden per item |
| BEH-019 | annotation | Undo/redo across moves, rotations, and edits | P1 | OBS-014, OBS-015 | observed | unit/history tests |
| BEH-020 | annotation | Right click finishes the current shape; undo until disappearance then redo to re-edit | P2 | OBS-014 | observed | manual/unit |
| BEH-021 | annotation | Text supports CJK/IME and rotatable text boxes | P2 | OBS-014 | observed | manual/IME integration |
| BEH-022 | clipboard | Image copied as PNG through a dedicated clipboard service/helper | P1 | OBS-012 | observed | integration/clipboard bridge |
| BEH-023 | clipboard | Text, HTML, URI list, and color payloads supported | P2 | OBS-012 | observed | integration |
| BEH-024 | hotkey | F1 snip, F3 paste, Shift+F3 hide/show images, C color copy (defaults) | P1 | OBS-014 | observed | manual |
| BEH-025 | hotkey | X11 global shortcuts use XGrabKey-style grabs | P1 | OBS-020 | partial | delivery test on Xorg/XWayland |
| BEH-026 | hotkey | Wayland global shortcut path | P2 | OBS-022 | gap | portal GlobalShortcuts or DE shortcut |
| BEH-027 | cli | Commands only act on an already-running instance | P1 | OBS-011 | observed | integration (two-process test) |
| BEH-028 | cli | `snip` variants, `--hold`, `--area`, `--full`, `--last`, `--active-window`, `--active-screen`, `--delay`, `--block` | P1 | OBS-011 | observed | CLI integration |
| BEH-029 | cli | Output destinations: pin, clipboard, file, quick-save, file-dialog, printer, success, exec | P1 | OBS-011 | observed | integration per destination |
| BEH-030 | config | Linux config at `$HOME/.snipaste/config.ini`; history under `.snipaste/history` | P1 | OBS-013 | observed | unit/config |
| BEH-031 | config | Session restore of pasters and image groups | P2 | OBS-013 | observed | integration/persistence |
| BEH-032 | tray | StatusNotifierItem tray with menu | P2 | OBS-013, OBS-014 | partial | GNOME AppIndicator/manual |
| BEH-033 | output | PNG/JPEG/WebP output with quality and optional alpha removal | P1 | OBS-012, OBS-013 | observed | golden/codec tests |
| BEH-034 | overlay | `--hold` keeps annotation UI after area selection | P1 | OBS-011 | observed | integration |
| BEH-035 | overlay | Without `--hold`, area completion exits immediately (destination none observed) | P1 | OBS-011 | observed | integration |
| BEH-036 | overlay | Reuse/replay of last snip area and history | P2 | OBS-009 | observed | manual |
| BEH-037 | overlay | Window/element detection toggled with Tab | P2 | OBS-009 | observed | manual |
| BEH-038 | overlay | Captured cursor visibility toggled with backtick/! | P2 | OBS-009 | observed | manual |
| BEH-039 | pin | Middle click resets 100% scale and opacity | P2 | OBS-014 | observed | manual |
| BEH-040 | pin | Thumbnail mode via Shift+double-click | P3 | OBS-014 | deferred | manual |
| BEH-041 | capture | Multi-monitor and mixed-DPI coordinate mapping | P1 | OBS-022 | gap | multi-monitor test matrix |
| BEH-042 | capture | Wayland native screenshot/pin stacking on GNOME | P1 | OBS-022 | gap | interactive portal + visual test |
| BEH-043 | privacy | No telemetry, no cloud calls, local-only operation | P1 | constitution | observed | config/network audit |
| BEH-044 | output | Original iconography and UI assets | P1 | constitution | deferred | asset review |
| BEH-045 | pin | New pin includes a 12 px shadow margin; 500x300 selection produced a 524x324 window at (-12,-12) | P2 | OBS-023 | observed | golden/property test |

## Notes

- P1 rows are required for the first usable Cosnip milestone; P2/P3 rows may move.
- `partial` rows require a specific follow-up mentioned in the research gap register.
- A row is not “implemented” until it has a Cosnip test and parity evidence; this matrix tracks
  the reference baseline, not product progress.

## Cosnip Status (Feature 002 + 003 + 004)

This table tracks implementation progress, not reference evidence. `parity` means the behavior was
demonstrated; `partial` means a deliberately reduced subset; `deferred` means a later feature;
`difference` means an accepted difference.

| Behavior | Status | Notes |
|---|---|---|
| BEH-001 X11 capture | parity | Xvfb capture test; 12–35 ms at 1080p, ~0.5 s at 3840x1080 debug |
| BEH-002 backend selection | parity (MVP subset) | X11 + GNOME Mutter/PipeWire; H.264 fallback; KDE/portal error |
| BEH-003 GNOME portal capture | difference | Portal blocked/interactive; direct Mutter ScreenCast used instead |
| BEH-006/011 overlay/coverage/focus | parity | One fullscreen viewport per monitor; global selection coordinates |
| BEH-007 selection/badge/handles | parity | Drag, arrows, square, Ctrl+A, move, and eight functional resize handles |
| BEH-008 magnifier/color | partial | 11x11 magnifier, coordinates, RGB/HEX; no C/Shift color-copy shortcuts |
| BEH-009 keyboard tips | difference | persistent bottom-left tips intentionally absent; icon tooltips expose names/hints |
| BEH-010 toolbar | partial | compact single-row icon toolbar (original line icons), palette/widths/undo/redo/output actions |
| BEH-012 override overlay | difference | WM-managed fullscreen used for focus |
| BEH-013 pin flags | parity | override-redirect, depth 32, Utility, skip taskbar/pager |
| BEH-014 ARGB/shadow | partial | depth 32; opaque image-sized pin, no shadow/opacity |
| BEH-015 click-through | deferred | no toggle UI/hotkey yet |
| BEH-016 pin interactions | partial | drag, wheel zoom, close; no rotate/flip/opacity |
| BEH-017 native stacking | parity | native screencast shows pin above browser |
| BEH-018 annotation items | partial | rectangle/ellipse/arrow/line/pencil/marker/text/number/mosaic/blur/magnifier/eraser; no post-commit move/edit/rotate |
| BEH-019 undo/redo | partial | add and erase history; style/move/rotate edits do not exist yet |
| BEH-020 right-click finish/re-edit | partial | finishes a draft or returns to Select; no re-edit after commit |
| BEH-021 text/CJK/IME | partial | egui IME editor + cosmic-text output; system CJK fallback, no rotation |
| BEH-022 image clipboard | parity | 500x300 read by native Wayland GTK4 after exit; GNOME capture is now lossless |
| BEH-023 text/HTML/URI/color clipboard | deferred | later feature; image clipboard read is implemented |
| BEH-024..026 hotkeys | partial | `cosnip paste` implements the F3 image-paste behavior; F1/F3 global hotkeys still use a desktop shortcut |
| BEH-027..029 CLI | partial | `snip`, `--area`, `--full`, `--delay`, `--copy`, `--save`, `--pin`; other commands later |
| BEH-030/031 config/history | deferred | no config or history in MVP |
| BEH-032 tray | deferred | no tray in MVP |
| BEH-033 output | partial | PNG only; JPEG/WebP later |
| BEH-034/035 hold semantics | difference | MVP always shows overlay; no `--hold` |
| BEH-041 multi-monitor/DPI | parity | Mixed-scale dual-monitor capture via per-monitor `RecordMonitor` + XWayland-canvas composition; drags cross boundaries in both directions; pin geometry is point-correct at 2x. Native per-monitor output remains future work |
| BEH-042 portal/pure Wayland | difference | direct Mutter ScreenCast on GNOME; portal and pure-Wayland UI not implemented |
| BEH-045 pin shadow margin | difference | image-sized pin avoids black margin on depth-24 GL |

### Feature 003 Evidence

| Check | Evidence |
|---|---|
| Lossless GNOME capture | raw BGRx 1920x1080 PipeWire frame; `--doctor` reports `gnome-pipewire`; 142–412 ms |
| No lossy round trip | no `ffmpeg`, no temporary MP4 on the success path |
| Full-screen output | 1920x1080 PNG saved and visually compared |
| Tiled wide desktop | 3840x1080 Xvfb with 2048 texture limit; exact 500x300 `--area` crop |
| Tiled pin | 3840x1080 pin window created on the same 2048-limit stack |
| Cross-monitor drag | simulated 960+960 layout; 900→1100 drag saved 200x50 |
| Pin after multi-monitor changes | 500x300 pin at (300,200); Escape closed it |
| Real dual-monitor hardware | validated on DP-2 4096x2304 at (0,0) + eDP-1 2880x1800 at (4096,338); overlays cover both monitors and focus follows the pointer |
| Mixed/fractional DPI | validated: per-monitor frames 2560x1440 and 2880x1800 composed into a 6976x2304 XWayland canvas; documented common-canvas resampling |
| Cross-boundary drag both ways | 4300,1000 → 4000,1100 and 4000,1000 → 4300,1100 both saved 300x100 |
| HiDPI pin geometry | `--area 4000 1000 300 200 --pin` opened a 300x200 pin at (4000,1000) on the 2x XWayland desktop |

### Feature 004 Evidence

| Check | Evidence |
|---|---|
| Selection resize | Xvfb drag of the SE handle changed 700x450 to 800x500 |
| Selection move | drag inside the selection uses global offset, clamped to the frame |
| Rectangle/ellipse/arrow | Xvfb UI + saved PNG contain the rendered shapes |
| Text | egui editor accepted "Hello"; saved output contains rasterized text |
| CJK fallback | unit test renders `中文` through the system font database |
| Number | click places sequential numbered discs |
| Mosaic/blur | Xvfb over xmessage text produced pixelated and softened output |
| Magnifier | Xvfb output shows enlarged source text inside the circle |
| Eraser | unit test removes the topmost annotation and is undoable |
| Undo/redo | toolbar Undo removed the newest rectangle; Ctrl+Z works with a held modifier |
| Output isolation | saved PNG contains annotations but no selection border, handles, toolbar, or tips |
| Icon toolbar | Xvfb screenshot shows original line icons, blue active tool, hover fill/tooltip |
| Persistent tips | absent by design; README/--help carry the shortcut reference |
| Pin | 700x450 pin at (200,150) shows the annotated rectangle |
| IME on real GNOME | open; requires an interactive IME session |

### Feature 005 Evidence

| Check | Evidence |
|---|---|
| Clipboard image paste | `cosnip snip --area 300 200 500 300 --copy`, then `cosnip paste` opened a 500x300 pin at the pointer |
| PNG file paste | `cosnip paste --file area.png` opened a 500x300 pin at (1000,1000) on the 2x desktop |
| Pointer clamping | pointer at (6900,2000) with a 500x300 image produced a pin at (6476,2000) |
| Text-only clipboard | `wl-copy` text then `cosnip paste` exited 3 with "clipboard does not contain an image" and no window |
| Invalid file | missing path and `Cargo.toml` both exited 2 with actionable file/PNG messages |
| No capture path | paste logs show no ScreenCast/PipeWire capture and no temporary media |
| Pin rendering | grabbed pin contents matched the source image; tiling/close button present |
| Real-mouse pin interaction | open; synthetic XTest events do not reach non-fullscreen XWayland windows on the host |
