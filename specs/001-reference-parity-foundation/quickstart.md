# Quickstart: Reference Analysis and XWayland Spike

All reference artifacts stay under the gitignored `temp/` directory. Do not commit an AppImage,
extracted tree, decompiled output, string dump, reference screenshot, or rootfs.

## Prerequisites

- Linux aarch64 host (primary) or x86_64 host.
- `curl`, `sha1sum`, `file`, `readelf`, `strings`, `dpkg-deb`, `tar`, `python3`.
- For dynamic reference execution on aarch64: an x86_64 userland (rootfs), `qemu-x86_64`, and
  `proot`; no sudo is required if packages are unpacked under `temp/`.
- Rust stable 1.99.1 for the disposable spike. The product MSRV is 1.95 because eframe 0.37
  requires it.

## 1. Acquire and Verify the Reference

```bash
mkdir -p temp/reference temp/tools temp/analysis temp/spike
curl -fL -o temp/reference/Snipaste-2.11.3-x86_64.AppImage \
  https://download.snipaste.com/archives/Snipaste-2.11.3-x86_64.AppImage
sha1sum temp/reference/Snipaste-2.11.3-x86_64.AppImage
# expected: 02a9af2118f502ce9a9e4be16026df05cda1920d
```

The AppImage is 37,450,944 bytes. The squashfs image starts at offset `0x2e4c0` for this build;
verify the `hsqs` superblock before extracting.

```bash
tail -c +$((0x2e4c0 + 1)) temp/reference/Snipaste-2.11.3-x86_64.AppImage \
  > temp/reference/snipaste.squashfs
# Obtain unsquashfs without sudo:
(cd temp/tools && apt-get download squashfs-tools && dpkg-deb -x ./*.deb root)
temp/tools/root/usr/bin/unsquashfs -d temp/reference/squashfs-root \
  temp/reference/snipaste.squashfs
```

Inspect without running:

```bash
file temp/reference/squashfs-root/usr/bin/Snipaste
readelf -d temp/reference/squashfs-root/usr/bin/Snipaste | grep NEEDED
strings -a temp/reference/squashfs-root/usr/bin/Snipaste > temp/analysis/strings-main.txt
```

## 2. Static Analysis with rea/kuna

```bash
export REA_KUNA_COMMAND=/home/orange/.local/bin/kuna
rea doctor
kuna functions temp/reference/squashfs-root/usr/bin/Snipaste --summary --json
kuna strings temp/reference/squashfs-root/usr/bin/Snipaste \
  --filter 'org\.gnome\.Shell\.Screenshot|org\.kde\.KWin|org\.freedesktop\.portal\.Screenshot|XGrabKey' \
  --json > temp/analysis/backends.json
```

Use targeted decompilation only; do not bulk-import decompiled output into the repository.
Refer to the research dossier for the backend map and the window/annotation findings.

## 3. Dynamic Reference Harness (aarch64, no sudo)

```bash
cd temp/tools
apt-get download qemu-user proot
for d in qemu-user*.deb proot*.deb; do dpkg-deb -x "$d" root; done
cd ../..
curl -fL -o temp/qemu/ubuntu-base-26.04-amd64.tar.gz \
  https://cdimage.ubuntu.com/ubuntu-base/releases/26.04/release/ubuntu-base-26.04-base-amd64.tar.gz
mkdir -p temp/qemu/rootfs
tar -xzf temp/qemu/ubuntu-base-26.04-amd64.tar.gz -C temp/qemu/rootfs
# install amd64 X11/GL libs once (fake root inside proot, no host sudo):
PROOT_NO_SECCOMP=1 temp/tools/root/usr/bin/proot -0 \
  -q "$PWD/temp/tools/root/usr/bin/qemu-x86_64" -r "$PWD/temp/qemu/rootfs" \
  -b /proc -b /sys -b /dev -b /etc/resolv.conf -w / \
  /bin/bash -lc 'apt-get update -qq && apt-get install -y -qq --no-install-recommends \
    libx11-6 libxext6 libxfixes3 libxrender1 libxrandr2 libxi6 libxcursor1 libxinerama1 \
    libxkbcommon0 libxkbcommon-x11-0 libgl1 libglx0 libopengl0 libegl1 libfontconfig1 \
    libfreetype6 libdbus-1-3 libglib2.0-0 libpng16-16 libxcb1 libx11-xcb1 libsm6 libice6 \
    libxau6 libxdmcp6 libbsd0 libmd0 libxcb-cursor0 libxcb-icccm4 libxcb-image0 \
    libxcb-keysyms1 libxcb-randr0 libxcb-render-util0 libxcb-render0 libxcb-shape0 \
    libxcb-shm0 libxcb-sync1 libxcb-util1 libxcb-xfixes0 libxcb-xkb1 libxcb-glx0 \
    libgl1-mesa-dri'
```

Run the reference with X11 forced for overlay observation:

```bash
export DISPLAY=:0
export XAUTHORITY=/run/user/1000/.mutter-Xwaylandauth.OGJ6X3
export XDG_RUNTIME_DIR=/run/user/1000
export QT_PLUGIN_PATH=$PWD/temp/reference/squashfs-root/usr/plugins
export LD_LIBRARY_PATH=$PWD/temp/reference/squashfs-root/usr/lib:$PWD/temp/reference/squashfs-root/checkrt/cxx
export HOME=/tmp
temp/tools/root/usr/bin/qemu-x86_64 -L temp/qemu/rootfs \
  temp/reference/squashfs-root/usr/bin/Snipaste
```

Start the first instance in the background; a second instance sends a command:

```bash
temp/tools/root/usr/bin/qemu-x86_64 -L temp/qemu/rootfs \
  temp/reference/squashfs-root/usr/bin/Snipaste snip --hold --area 100 100 500 300
```

Observe `$HOME/.snipaste/splog.txt` and X11 windows with `xwininfo -root -tree`. On GNOME 49,
the portal screenshot path requires an interactive decision; this is a documented gap. For
overlay observation, force X11 (`XDG_SESSION_TYPE=x11`, unset `WAYLAND_DISPLAY`).

## 4. Build and Run the Disposable Spike

```bash
cd temp/spike
cargo build --release
# Full-screen overlay + pin + click-through + measurements:
LIBGL_ALWAYS_SOFTWARE=1 SPIKE_ROOT_TRANSPARENT=1 SPIKE_AUTORUN=all \
  DISPLAY=:0 XAUTHORITY=/run/user/1000/.mutter-Xwaylandauth.OGJ6X3 \
  ./target/release/cosnip-spike
```

Variants:

```bash
SPIKE_AUTORUN=overlay-normal       # WM-managed fullscreen overlay
SPIKE_AUTORUN=overlay-override     # override-redirect overlay (no focus)
SPIKE_AUTORUN=pin                  # override-redirect ARGB pin
SPIKE_AUTORUN=pin-pass             # pin with empty input shape
SPIKE_PIN_NORMAL=1                 # WM-managed pin (for comparison)
SPIKE_ROOT_TRANSPARENT=1           # alpha-capable GL config for depth-32 pins
LIBGL_ALWAYS_SOFTWARE=1            # required for depth-32 on this Mali/XWayland host
```

Inspect window properties while the spike is running:

```bash
xwininfo -root -tree | grep -iE 'Cosnip (Overlay|Pin)'
xprop -id <window-id> _NET_WM_STATE _NET_WM_WINDOW_TYPE
xwininfo -id <window-id> | grep -E 'Depth|Map State|Override'
```

Capture a specific X11 window (not the root) with Python-Xlib if visual evidence is needed; root
capture under XWayland is black or rejected.

## 5. Cleanup

```bash
# Stop the spike/reference processes, then leave artifacts in temp/ for local review.
# Do not add temp/ to git.
git status --short
```

## Expected M0 Results

- Fullscreen WM-managed overlay: focusable, 1920x1080, 5–11 ms first frame.
- Override-redirect pin: 360x220 at 240,220, depth 32 with software GL and transparent root;
  depth 24 with the default hardware GLX path.
- Click-through: empty X11 input shape.
- X11 root capture under XWayland: `BadMatch`; use a Wayland compositor/portal backend instead.
- XGrabKey probe: succeeds on the XWayland root; actual delivery still needs a manual keypress
  test.
