# Quickstart: Lossless GNOME Wayland Capture and Multi-Monitor Selection

## Build prerequisites

Pipeline capture uses the Rust `pipewire` crate with bindgen, so the build host needs:

```bash
# Debian/Ubuntu names
sudo apt install libpipewire-0.3-dev libspa-0.2-dev libclang-dev libllvm-dev clang
```

If the headers are extracted into a sysroot instead of installed system-wide, export the same
variables used by the project:

```bash
export PKG_CONFIG_PATH="$PWD/temp/pipewire-sysroot/usr/lib/aarch64-linux-gnu/pkgconfig"
export PKG_CONFIG_SYSROOT_DIR="$PWD/temp/pipewire-sysroot"
export LIBCLANG_PATH="$PWD/temp/pipewire-sysroot/usr/lib/aarch64-linux-gnu"
export LD_LIBRARY_PATH="$PWD/temp/pipewire-sysroot/usr/lib/llvm-20/lib:$PWD/temp/pipewire-sysroot/usr/lib/aarch64-linux-gnu"
export BINDGEN_EXTRA_CLANG_ARGS="-I/usr/lib/gcc/aarch64-linux-gnu/15/include -I/usr/include"
```

## Verify the backend

```bash
cargo build
target/debug/cosnip --doctor
```

Expected on GNOME Wayland:

```text
session: wayland
desktop: GNOME
backend: gnome-pipewire
monitors: 1 (1920x1080, scale 1, name 1920x1080)
monitor 0: HDMI-1 1920x1080 at 0,0
verdict: ready
```

## Snip and save

```bash
target/debug/cosnip snip --full --save /tmp/full.png --no-copy
target/debug/cosnip snip --area 300 200 500 300 --save /tmp/area.png --no-copy
```

The area command must produce exactly a 500x300 PNG.

## Automated checks

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Xvfb integration:

```bash
# Start a 3840x1080 Xvfb on :99 (Xvfb is not installed system-wide on the dev host;
# the extracted binary can be used instead).
DISPLAY=:99 XDG_SESSION_TYPE=x11 WAYLAND_DISPLAY= \
  cargo test -p cosnip-app --test xvfb_smoke -- --ignored
```

Simulated dual-monitor interaction (debug build; the env override is debug-only):

```bash
COSNIP_TEST_MONITORS='960x1080+0+0,960x1080+960+0' \
COSNIP_TEST_WINDOWED_OVERLAY=1 \
COSNIP_TEST_AUTOCONFIRM_MS=8000 \
target/debug/cosnip snip --save /tmp/drag.png --no-copy
```

Drag from (900,500) to (1100,550) with an XTest tool while the overlay is open; `/tmp/drag.png`
must be 200x50.

## Manual multi-monitor checklist

- [ ] Each monitor is fully covered, including panels, with no gap or duplicate image.
- [ ] A drag crosses the monitor boundary and continues smoothly.
- [ ] The magnifier, size badge, and toolbar appear on the expected monitor.
- [ ] Keyboard shortcuts work after moving the pointer to another monitor.
- [ ] The saved/clipboard image contains the continuous region.
- [ ] Mixed-DPI is treated as a known limitation.
