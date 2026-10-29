# CLI Contract: Snipping Overlay MVP

## Commands

```text
cosnip snip [OPTIONS]
cosnip --doctor
cosnip --version
cosnip --help
```

## `snip` options

| Option | Meaning |
|---|---|
| `--area X Y W H` | Preselect a rectangle and show the overlay for adjustment |
| `--full` | Preselect the entire captured desktop |
| `--delay MS` | Wait before capture; `0` or absent means no delay |
| `--copy` | Copy the confirmed selection as an image (default if no output is given) |
| `--save PATH` | Save the confirmed selection as PNG to PATH |
| `--pin` | Pin the confirmed selection; keep the process alive while the pin is open |
| `--no-copy` | Suppress the default clipboard output when `--save`/`--pin` is used |

`--area` and `--full` are mutually exclusive. `--save` and `--pin` may be combined; `--copy` is
independent.

## Examples

```bash
cosnip snip
cosnip snip --full --copy
cosnip snip --area 100 100 500 300 --save /tmp/cosnip.png
cosnip snip --pin
cosnip snip --delay 1500 --copy
```

## Exit Codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 2 | Invalid usage/arguments |
| 3 | Display/backend unavailable |
| 4 | Capture or frame-decode failure |
| 5 | Output failure (clipboard/save/pin) |
| 6 | User cancelled (Escape/close) |

## Behavior

- The command is one-shot for copy/save: it exits after the output attempt. A desktop clipboard
  manager is required for the image to survive exit; without one, the image may disappear.
- `--pin` keeps the process alive until all pins are closed.
- `--delay` applies before the capture, not before the overlay.
- Logs go to stderr and MUST NOT contain image content or full user file paths.
- `--doctor` prints: session type, selected backend, backend availability, monitor count/size/
  scale, ffmpeg availability, clipboard backend, and a one-line verdict. Exit 0 if the selected
  backend is usable, 3 otherwise.
- `--version` prints `cosnip <semver>`.

## Errors

Errors must name the failed capability and a next action, for example:

- no DISPLAY and no Wayland session → "No display session found"
- GNOME Screencast unavailable → "GNOME Screencast service is unavailable"
- ffmpeg missing → "ffmpeg is required for the GNOME capture fallback"
- multi-monitor → "Multi-monitor is not supported in this MVP"
- unwritable save path → "Cannot write to <path>: <reason>"
