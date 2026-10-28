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
| `tiny-skia` | planned | BSD-3-Clause | CPU rasterization for document/annotations/pins |
| `cosmic-text` | planned | MIT OR Apache-2.0 | Text shaping and system-font fallback |
| `kurbo` | planned | MIT OR Apache-2.0 | Pure geometry for the domain model |
| `zbus` / `ashpd` | planned | MIT | D-Bus/portal capture, GlobalShortcuts, single instance |
| `ksni` | planned | MIT | StatusNotifierItem tray; GNOME may require an extension |
| `clap` | planned | MIT OR Apache-2.0 | CLI surface |
| `serde` / `toml` | planned | MIT OR Apache-2.0 | Config |
| `tracing` / `tracing-subscriber` | planned | MIT | Redacted local logs |
| `thiserror` / `anyhow` | planned | MIT OR Apache-2.0 | Error boundaries |
| `rayon` | planned | MIT OR Apache-2.0 | Parallel image processing; measure on ARM |
| `criterion` / `proptest` / `insta` / `tempfile` | dev | MIT/Apache-2.0 | Benchmarks, property tests, snapshots, temp files |

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

The analysis tools are development-time instruments. They must not become runtime, build, or test
dependencies (Constitution II).

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
