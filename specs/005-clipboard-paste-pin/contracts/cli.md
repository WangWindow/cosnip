# CLI Contract: Clipboard Paste and Pin

## `cosnip paste`

Reads an image from the system clipboard and opens a pin at the global pointer.

```text
cosnip paste [--file PATH]
```

### Options

| Option | Type | Behavior |
|---|---|---|
| `--file PATH` | Path | Decode a PNG and pin it. Takes precedence over the clipboard. |

### Exit codes

| Code | Meaning |
|---|---|
| 0 | The pin was created and later closed successfully |
| 2 | Invalid arguments, missing file, unreadable file, or non-PNG file |
| 3 | No display, clipboard unavailable, or unsupported session |
| 4 | UI or pin creation failure |
| 5 | Internal image conversion or texture failure |

### Observable behavior

- Success: one frameless pin window at the pointer; no snip overlay; no capture backend; no
  temporary media.
- Failure: a single actionable message on stderr; no pin, overlay, capture session, or temporary
  file.
- `--file` with a missing path or a non-PNG file fails before the UI starts.
- The existing `cosnip snip` command is unchanged.

## `cosnip --doctor`

The existing `clipboard` line remains. A future addition may report whether the clipboard
currently contains an image, but this is not required for the first implementation.

## Examples

```bash
# Copy an image in another application, then pin it at the pointer.
cosnip paste

# Pin a saved PNG for testing or scripting.
cosnip paste --file /tmp/known.png
```
