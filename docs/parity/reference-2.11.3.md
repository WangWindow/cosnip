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
