# Data Model: Annotation Editing and Selection Adjustment

## Tool

`Select`, `Rectangle`, `Ellipse`, `Arrow`, `Line`, `Pencil`, `Marker`, `Text`, `Number`, `Mosaic`,
`Blur`, `Magnifier`, `Eraser`. Mosaic, blur, and eraser are continuous-brush tools.

`Tool::ALL` defines toolbar order. `is_draw_tool` excludes Select/Eraser; `is_rect_drag_tool`
covers rect/ellipse/mosaic/blur.

## Annotation

| Field | Type | Notes |
|---|---|---|
| `kind` | `AnnotationKind` | Payload, all coordinates in frame pixels |
| `color` | `[u8; 4]` | RGBA palette color |
| `width` | `u32` | Stroke width; effects use it to scale their block/radius |

`bounds()` returns a padded bounding rectangle for eraser hit testing; line/arrow/polyline hit
tests use point-to-segment distance.

## AnnotationKind

| Variant | Payload |
|---|---|
| Rectangle / Ellipse | `rect: Rect` |
| Arrow / Line | `start: Point`, `end: Point` |
| Pencil / Marker | `points: Vec<Point>` |
| Text | `position`, `text`, `size` |
| Number | `position`, `value` |
| Mosaic / Blur / Eraser | `points: Vec<Point>` |
| Magnifier | `center`, `radius`, `zoom` |

## AnnotationDocument

| Field | Type | Notes |
|---|---|---|
| `items` | `Vec<Annotation>` | Committed order is draw order |
| `undo` | `Vec<HistoryAction>` | Add/Remove actions |
| `redo` | `Vec<HistoryAction>` | Cleared on a new edit |

Operations: `add`, `remove_at(point)` (topmost hit), `undo`, `redo`, `clear`, `can_undo`,
`can_redo`.

## Handle

`NorthWest`, `North`, `NorthEast`, `East`, `SouthEast`, `South`, `SouthWest`, `West`.

`position(rect)` returns the handle center. `Selection::handle_at(point, tolerance)` returns the
closest handle; `resize` moves one edge/corner with a 4 px minimum; `move_from` clamps the delta.

## UI State

| Field | Notes |
|---|---|
| `annotations` | Committed document/history |
| `draft` | In-progress annotation |
| `drag_mode` | New selection / move / resize / annotation |
| `tool` | Active toolbar mode |
| `color`, `line_width` | New-annotation style |
| `next_number` | Next number stamp |
| `text_editor` | Monitor, position, buffer, focus state |

## Output

`render(frame, selection, annotations)` returns a cropped `Frame` containing the base pixels plus
annotations. It is used by copy, save, and pin; overlay chrome is never part of the model.
