# Dependency Record

This record covers the M0 research spike and the planned MVP stack. A dependency enters the
product workspace only after the MVP plan justifies it against capability, maintenance status,
license, and attack surface (Constitution III).

## Product Toolchain

| Item | Version / MSRV | License | Notes |
|---|---|---|---|
| Rust stable | 1.99.1 (current); MSRV 1.95 | MIT OR Apache-2.0 | eframe/egui 0.37 declare rust-version 1.95 |
| `eframe` / `egui` | 0.37 | MIT OR Apache-2.0 | Primary GUI stack for overlay, toolbar, settings, text |
| `egui_glow` / `glow` | 0.37 / 0.17 | MIT OR Apache-2.0 | GLES/OpenGL renderer; chosen over wgpu on the target |
| `winit` | 0.30 | Apache-2.0 | X11 backend selection, viewport/window plumbing |
| `x11rb` | 0.13 | MIT OR Apache-2.0 | X11 capture, XShape, XFixes, XGrabKey, screen info |
| `arboard` | 3 | MIT OR Apache-2.0 | Clipboard bridge; native Wayland helper may be needed later |
| `image` | 0.25 | MIT OR Apache-2.0 | PNG/JPEG/WebP output; enable only needed codecs |
| `tiny-skia` | planned for annotations | BSD-3-Clause | CPU rasterization; not used by the MVP |
| `cosmic-text` | planned for annotations | MIT OR Apache-2.0 | Text shaping and system-font fallback |
| `kurbo` | not used (MVP uses integer core geometry) | MIT OR Apache-2.0 | Available for later vector paths |
| `zbus` | 6.1 | MIT | Mutter ScreenCast D-Bus session/signal; `zbus::block_on` keeps one connection alive |
| `pipewire` | 0.10.1 | MIT | Lossless Mutter ScreenCast raw-frame capture; feature `v0_3_44`; target node passed to `Stream::connect` |
| `libspa` / `libspa-sys` / `pipewire-sys` | 0.10.1 | MIT | Transitive PipeWire/SPA bindings for format negotiation and buffer mapping |
| `futures-lite` | 2.6 | Apache-2.0 OR MIT | Polling the `PipeWireStreamAdded` signal with a timeout |
| `async-io` | 2.6 | Apache-2.0 OR MIT | Timeout timer for D-Bus signal/stream startup |
| `ashpd` | planned | MIT | Portal ScreenCast/GlobalShortcuts for a later backend |
| `ksni` | planned for tray | MIT | StatusNotifierItem tray; GNOME may require an extension |
| `clap` | 4.6 | MIT OR Apache-2.0 | CLI surface |
| `jiff` | 0.2 | MIT | Timestamped default output names |
| `tracing` / `tracing-subscriber` | 0.1 | MIT | Available for redacted logs; MVP uses stderr messages |
| `thiserror` | 2 | MIT OR Apache-2.0 | Typed adapter errors |
| `rayon` | planned | MIT OR Apache-2.0 | Parallel image processing; measure on ARM |
| `tempfile` | 3 | MIT OR Apache-2.0 | Runtime temp dirs and test fixtures |
| `criterion` / `proptest` / `insta` | dev/planned | MIT/Apache-2.0 | Benchmarks, property tests, snapshots |

## Analysis-Only Instruments (Never Linked)

| Tool | Version | License / Distribution | Role |
|---|---|---|---|
| `rea` | 6.8.0 | Local tool distribution | Overview, evidence, targeted analysis orchestration |
| `kuna` | 1.984 | Local tool distribution | Ghidra-based decompilation/inventory on x86-64 |
| QEMU user (`qemu-x86_64`) | 10.2.1 | GPL-2.0 (tool) | Dynamic reference execution; not linked into Cosnip |
| `proot` | 5.4.0 | GPL-2.0 (tool) | User-space rootfs harness; not linked |
| Ubuntu base amd64 rootfs | 26.04 | Ubuntu component licenses | Reference runtime libraries only |
| `squashfs-tools` | 4.7.5 | GPL-2.0 (tool) | AppImage extraction |
| Snipaste 2.11.3 AppImage | 2.11.3 | Proprietary freeware | Behavioral reference; not redistributed |
| `ffmpeg` | system | GPL/LGPL build-dependent | Optional runtime decoder for the GNOME Screencast fallback; not bundled |

## Build-Time System Dependencies

| Item | Role |
|---|---|
| `libpipewire-0.3-dev`, `libspa-0.2-dev` | Headers and pkg-config files for `pipewire-sys`/`libspa-sys` |
| `libclang` / `libLLVM` | bindgen at build time for the PipeWire/SPA bindings |
| C compiler and GCC include directory | `BINDGEN_EXTRA_CLANG_ARGS` must find `stdbool.h` when using a sysroot |

These are build-time only. The resulting binary links `libpipewire-0.3.so.0`, which is present on
GNOME/PipeWire systems.

The analysis tools are development-time instruments. They must not become runtime, build, or test
dependencies (Constitution II).

`ffmpeg` is an external runtime dependency only for the GNOME Screencast H.264 fallback. `--doctor`
detects it; the binary does not bundle it. The default GNOME Wayland path is now lossless Mutter
ScreenCast + PipeWire and does not invoke `ffmpeg` or write temporary media. The fallback remains
for systems where PipeWire is unavailable or the Mutter stream fails.

## Rejected for This Device

| Candidate | Reason |
|---|---|
| `wgpu` 30 | Vulkan device limits on the target (`max_texture_dimension_3d = 512` vs requested 2048) |
| GTK4/gtk-rs | Heavier dependency set; overlay/pin input-shape control is weaker for this design |
| Slint | License/commercial considerations; not needed |
| Qt/cxx-qt | Would introduce C++/LGPL and defeat the original-Rust goal |

## License Policy

- Prefer MIT/Apache-2.0/BSD-3-Clause dependencies.
- A copyleft dependency requires explicit justification in the plan.
- Record any new dependency in this file when it enters the product workspace.
- Run `cargo deny` (license + advisory) and `cargo audit` once the product workspace is created.
