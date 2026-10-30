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
| Display | Border width/color, mask color, anchors, anchor color, tips, magnifier with grid/coordinates |
| Snip behavior | Hide 1x1 windows, capture cursor, Enter/double/middle actions, recent-area reuse (R/Shift+R), disable wheel width changes, cross lines |
| Tools | Rectangle, ellipse, line, arrow, pencil, highlighter, mosaic, blur, text, number, magnifier, eraser, select |
| Eraser | Freehand annotation-pixel erase, original screenshot preserved |
| Mosaic/blur | Freehand brush strokes, shared preview/export mask |
| Undo/redo | Annotation add/erase history |
| Output | Copy PNG, save PNG, native pin, default output, directory, filename |
| Pins | Image-sized Shell actors, two-tone border texture, border width/color, opacity, drag, wheel zoom, right-click close, zoom-at-pointer |
| Settings | GSettings + GNOME preferences; button-recorded global shortcuts |
| Automation | `org.cosnip.Native` D-Bus interface |

## Partial / next

| Area | Remaining work |
|---|---|
| UI element detection | Accessible-element detection under the pointer |
| Snip history | Replay (`,`/`.`), history count/loop, persisted history |
| Pin groups | Create/switch/hide groups, richer context menu, thumbnail mode |
| Pin gestures | Ctrl+wheel opacity, rotate/flip/reset, double-click thumbnail |
| Output | Auto-save/quick-save rules, copy-as-file, image quality/background modes |
| Text | Better IME edge cases and text color/background controls |
| Interface | Theme color, UI font, palette size/ordering, color value format |
| Controls | In-app shortcut remapping and mouse gesture remapping |
| Sounds | Optional capture/copy/save sounds |
| OCR / text extraction | Local engine integration |

## Excluded

- Run at login
- Update checking / network proxy / licenses
- Network OCR providers

## Reference evidence

Captured reference preferences pages:

- `temp/analysis/snipaste-prefs/`
- `temp/analysis/snipaste-prefs2/`

Reference toolbar and selection:

- `temp/analysis/ref-annot-rect3.png`
- `temp/analysis/cosnip-toolbar-final.png`
