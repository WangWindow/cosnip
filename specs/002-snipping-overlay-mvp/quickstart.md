# Quickstart: Snipping Overlay MVP

## Build

```bash
cargo build --release
cargo test
```

The binary is `target/release/cosnip`.

## Diagnostics

```bash
./target/release/cosnip --doctor
```

Expected fields: session type, backend, monitor count/size/scale, ffmpeg availability, clipboard
backend, and a verdict. On the target GNOME 49 session, the backend should be
`gnome-screencast`; on Xorg, `x11`.

## X11/Xorg Test

A nested Xvfb server is useful and requires no root:

```bash
Xvfb :99 -screen 0 1920x1080x24 -ac -noreset -nolisten tcp &
DISPLAY=:99 XDG_SESSION_TYPE=x11 XAUTHORITY= COSNIP_TEST_WINDOWED_OVERLAY=1 \
  COSNIP_TEST_AUTOCONFIRM_MS=800 ./target/release/cosnip snip --area 100 100 500 300 --save /tmp/cosnip.png
```

A window manager is not required for capture, but it is useful for focus/stacking tests. Verify
the PNG dimensions with `file`/`identify` or a small test.

## GNOME 49 Wayland Test

```bash
COSNIP_TEST_AUTOCONFIRM_MS=1200 \
  DISPLAY=:0 XAUTHORITY=/run/user/1000/.mutter-Xwaylandauth.OGJ6X3 \
  ./target/release/cosnip snip --area 100 100 500 300 --copy
```

Expected sequence:

1. a short GNOME Screencast recording (300–700 ms),
2. one frame decoded with ffmpeg,
3. the overlay appears with the frozen desktop,
4. Enter/Ctrl+C copies the selected region,
5. the temporary recording/frame is removed.

If the command reports that the Screencast service or ffmpeg is unavailable, run `--doctor` and
follow the error. The recording indicator may flash; this is an accepted MVP limitation.

## Pin Test

```bash
# On Xorg/Xvfb
DISPLAY=:99 ./target/release/cosnip snip --area 100 100 500 300 --pin

# On GNOME 49, with a native Wayland app behind the pin
DISPLAY=:0 XAUTHORITY=/run/user/1000/.mutter-Xwaylandauth.OGJ6X3 \
  ./target/release/cosnip snip --area 100 100 500 300 --pin
```

Verify: the pin is at (100,100), above the underlying window, has no taskbar entry, moves with the
pointer, zooms with the wheel, and closes with Escape/double-click.

## Clipboard Verification

Use a focused native Wayland GTK4 reader:

```bash
gjs -c 'imports.gi.versions.Gtk="4.0"; const G=imports.gi.Gtk, D=imports.gi.Gdk, L=imports.gi.GLib; G.init(); const w=new G.Window({title:"clip-reader"}); w.present(); const c=D.Display.get_default().get_clipboard(); const loop=new L.MainLoop(null,false); L.timeout_add(500,()=>{c.read_texture_async(null,(s,r)=>{try{const t=c.read_texture_finish(r); t.save_to_png("/tmp/clip.png"); print(t.get_width()+"x"+t.get_height());}catch(e){print(e.message);} loop.quit();}); return false;}); loop.run();'
```

The saved image dimensions must match the selected region.

## Tests and Budgets

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo bench
```

Targets: X11 snip-to-overlay p95 ≤ 300 ms; GNOME p95 ≤ 2.0 s; selection feedback ≤ 16 ms; idle
CPU < 1% with a pin; RSS < 300 MB at 1080p.

## Cleanup

No temp files should remain in `$XDG_RUNTIME_DIR/cosnip-*` after success, cancel, or error.
