# Implementation Plan: Desktop Shell

**Branch**: `006-desktop-shell` | **Date**: 2026-10-29 | **Status**: Implemented

## Summary

Add an XDG TOML config module, an egui settings window, a lightweight ksni tray, and overlay
text/cursor/memory polish. Keep the tray free of eframe and leave global shortcuts to GNOME.

## Technical Context

- Rust 2024, MSRV 1.95.
- `serde` + `toml` for config; `rfd` 0.17 (xdg-portal) for folder selection; `ksni` 0.3 with
  `async-io` + `blocking` for SNI; existing `zbus` for tray single-instance.
- Config path: `$XDG_CONFIG_HOME/cosnip/config.toml`; autostart:
  `$XDG_CONFIG_HOME/autostart/cosnip-tray.desktop`.
- `cosnip tray` spawns the current executable for Snip/Paste/Settings; it does not own pins.
- `cosnip settings` is an on-demand eframe window.
- Text editor: transparent TextEdit, configured color/size, optional shadow.
- Memory: release capture frame/tiles before output; pin stores dimensions only; stream PNG;
  Canvas owns the crop; reuse one FontSystem per render.
- Pins have two backends: the existing XWayland window and an optional GNOME Shell actor.
  `auto` prefers the extension when `org.cosnip.Pins` is running and otherwise falls back to a
  window. The extension accepts raw RGBA over D-Bus and uploads it with `St.ImageContent`, so the
  compositor draws the pin at native resolution without XWayland resampling.

## Constitution Check

- Platform edges stay in `cosnip-platform`; UI in `cosnip-ui`; binary wiring in `cosnip-app`.
- No unsafe, no network, no telemetry, no reference assets.
- New dependencies: `serde`/`toml` (MIT/Apache), `rfd` (MIT), `ksni` (Unlicense); reviewed in
  `docs/dependencies.md`.

## Structure

```text
crates/cosnip-platform/src/{config,paths,shell_pin,tray}.rs
crates/cosnip-ui/src/{app,settings}.rs
crates/cosnip-app/src/main.rs
extensions/cosnip-pins@cosnip/
specs/006-desktop-shell/
```

## Verification

- Config roundtrip/missing/invalid/clamp tests.
- CLI help and paste error tests.
- Manual settings window, tray registration, SNI host, duplicate tray, autostart write.
- Manual text editor/cursor check; memory PSS before/after.
