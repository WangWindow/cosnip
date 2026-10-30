# Tasks: Desktop Shell

Legend: `[x]` complete.

## Phase 1 — Config and XDG

- [x] T001 Add `serde` + `toml` dependencies.
- [x] T002 Add `Config` with general/overlay/annotations/pin sections and defaults.
- [x] T003 Add XDG config/data/cache/runtime/autostart paths.
- [x] T004 Add load/save/validation and unit tests.
- [x] T005 Add global `--config` and CLI-over-config precedence.

## Phase 2 — Settings

- [x] T010 Add `cosnip settings` and an eframe settings window.
- [x] T011 Add General, Overlay, Annotations, Shortcuts, and Tray sections.
- [x] T012 Add rfd folder picker on a worker thread.
- [x] T013 Add Save/Reload/Open config folder.
- [x] T014 Add shortcut command display and GNOME Keyboard settings button.
- [x] T015 Add autostart toggle.

## Phase 3 — Tray

- [x] T020 Add `cosnip tray` with ksni `async-io` + `blocking`.
- [x] T021 Add Snip/Paste/Settings/Quit menu actions.
- [x] T022 Add a generated ARGB tray icon.
- [x] T023 Add D-Bus single-instance guard; duplicate exits 0.
- [x] T024 Treat a missing SNI host as hidden, not an error.
- [x] T025 Add `--doctor` tray host status.

## Phase 4 — Overlay polish and memory

- [x] T030 Transparent single-line text editor with shadow and configured style.
- [x] T031 Cursor visibility state machine; restore on commit/close.
- [x] T032 Release capture frame/textures after rendering output.
- [x] T033 Drop pin CPU pixels after texture creation.
- [x] T034 Stream PNG encoding.
- [x] T035 Canvas owns the crop; reuse FontSystem per render.
- [x] T036 Avoid composition clones for identity-size monitors.
- [x] T037 Verify full-screen pin PSS (260 -> 156 MB; private anonymous ~106 MB).
- [x] T038 Add native single-monitor output from compressed monitor frames and keep the pin display size equal to the selection.
- [x] T039 Scale annotation previews, text, and hit boxes by the viewport pixel ratio; verify the eraser.
- [ ] T040 Shadow margins were prototyped and removed; pins stay image-sized to avoid transparent
  override-redirect compositing artifacts.
- [x] T041 Align the overlay toolbar with the reference item set/order and compact light styling.
- [x] T042 Add right-click color/width style popups and keep style controls out of the main row.
- [x] T043 Add the optional GNOME Shell pin backend (`org.cosnip.Pins` + `St.ImageContent`).
- [x] T044 Add `pin.backend = auto|shell|window`, `--doctor` shell-pin detection, and automatic
  window fallback.

## Verification

- [x] Config tests, workspace tests, fmt, clippy.
- [x] Manual settings window open; tray registration; duplicate tray.
- [x] Full-screen pin memory measurement.
- [x] Transparent text editor and typing verified in the fullscreen overlay via XTest, including the
  system IME candidate window.
- [ ] Real-mouse pin drag/zoom/close and the cursor after text commit still need a physical
  keyboard/mouse check; synthetic XTest does not reach non-fullscreen XWayland windows on the
  validation host.
- [x] Shell extension loaded in a headless GNOME Shell 51: `Ping` returned `cosnip-pins/1`,
  `Pin`/`Count`/`Clear` succeeded for 1x1 and 2x2 RGBA images.
- [x] Rust transport: `cosnip paste --file 10x10.png` under the headless Shell exited 0 and
  `org.cosnip.Pins.Count` returned 1; `cosnip --doctor` reported `shell pins: available`.
- [ ] Shell pin drag, wheel zoom, right-click close, overview hiding, and visual crispness still
  need a real GNOME session check (the validation session was locked).

## Implementation evidence

- `cosnip --config <file>` loaded a TOML config; `default_output = "save"` and a custom
  `save_dir` produced a 6976x2304 PNG in that directory.
- Invalid TOML printed a parse warning, used defaults, and still completed the snip.
- `cosnip settings` now follows the reference preferences layout: top tabs
  (General/Interface/Snip/Pin/Output/Controls/About), optional sub-tabs, bordered group pages,
  help/restore footer, automatic saving, and a compact 560x440 window; `rfd` uses the available
  XDG FileChooser portal v4.
- `cosnip tray` registered `org.cosnip.Tray` and a StatusNotifierItem with the running
  AppIndicator host; idle PSS was 6.7 MB; a second launch exited 0.
- `cosnip --doctor` reports the config path and tray host availability.
- The autostart desktop-entry writer has a unit test.
- The text editor was visually verified as transparent with the configured color/size and no black
  input box; typing `hi` worked and the system IME candidate window appeared. The editor now
  re-requests focus if the canvas steals it. Preview font size and stroke widths are converted to
  viewport points, so pinned text matches the editor.
- The eraser was verified on a drawn rectangle after aligning preview scale and hit testing.
- A native single-monitor pin was verified: an 800x600 global selection produced a 500x375 native
  PNG displayed in an 800x600 image-sized pin.
- Full-screen pin PSS dropped from 260 MB to 145 MB (private anonymous memory ~106 MB; the extra
  file-backed pages include the newly linked tray/rfd dependencies). Overlay PSS is ~194 MB on
  the 6976x2304 desktop.
- The toolbar was visually compared against `ref-annot-rect3.png` before the session locked:
  reference order, 13 items, light background, dark line icons, red-dot shape group, and centered
  placement. Final icon polish and the right-click popup still need a live screenshot check.
