# Snipaste feature/settings parity

This matrix is based on the Snipaste 2.11.3 preferences window (General,
Interface, Snip, Pin, Output, Controls, Update, About) and the reference
toolbar/tips observations. Update/network and OS startup are intentionally out
of scope for the native GNOME extension.

## Implemented

| Area | Native extension |
|---|---|
| Capture | Native `Shell.Screenshot` stage capture; no portal/PipeWire |
| Selection | Drag, move, eight handles, size badge, Ctrl+A |
| Window detection | Hover highlight and click-to-select; Tab toggles |
| Display | Fixed two-tone selection frame, handles, dim mask, tips, magnifier with grid and coordinates; optional cursor capture |
| Snip behavior | Window auto-detection; Enter default output; double-click copy; middle-click pin; wheel changes tool width |
| Tools | Rectangle, line, arrow, pencil, highlighter, mosaic, blur, text, magnifier, eraser, select; toolbar click only |
| Eraser | Freehand annotation-pixel erase, original screenshot preserved |
| Mosaic/blur | Freehand brush strokes, shared preview/export mask |
| Undo/redo | Annotation add/erase history |
| Output | Copy PNG, save PNG, native pin, default output, directory, filename |
| Pins | Image-sized Shell actors, two-tone border texture, configurable opacity, drag, wheel zoom, right-click close |
| Settings | GSettings + Adwaita preferences; one button-recorded global shortcut per action; keyboard tool bindings removed |
| Automation | `org.cosnip.Native` D-Bus interface |

## Partial / next

| Area | Remaining work |
|---|---|
| UI element detection | Accessible-element detection under the pointer |
| Pin gestures | Ctrl+wheel opacity, rotate/flip/reset, double-click thumbnail |
| Output | Auto-save/quick-save rules, copy-as-file, image quality/background modes |
| Text | Better IME edge cases and text color/background controls |
| Interface | Palette size/ordering and color value format |
| Sounds | Optional capture/copy/save sounds |
| OCR / text extraction | Local engine integration |

## Excluded

- Run at login
- Update checking / network proxy / licenses
- Network OCR providers
- Panel/tray icon and its menu
- Circle/ellipse shape annotation (rectangle only)
- Pin groups
- Snip history replay (`/` and `.`)
- In-app shortcut remapping and mouse gesture remapping
- Keyboard shortcuts for switching toolbar tools
- Theme customization (theme color, UI font, palette editing)
- Number annotation tool (text covers the same use case)

## Reference evidence

Captured reference preferences pages:

- `temp/analysis/snipaste-prefs/`
- `temp/analysis/snipaste-prefs2/`

Reference toolbar and selection:

- `temp/analysis/ref-annot-rect3.png`
- `temp/analysis/cosnip-toolbar-final.png`
