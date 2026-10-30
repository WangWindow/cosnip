# Feature Specification: Desktop Shell (Tray, Settings, and TOML Config)

**Feature Branch**: `006-desktop-shell`

**Created**: 2026-10-29

**Status**: Implemented

**Input**: Add a Snipaste-style system tray and settings UI, store user preferences in an XDG
TOML file, use rfd for folder selection, keep global shortcuts managed by GNOME, and improve
overlay text/cursor behavior and memory use.

## User Scenarios & Testing

### User Story 1 - Tray Menu (Priority: P1)

As a desktop user, I can start one lightweight tray process and reach Snip, Paste, Settings, and
Quit without opening a terminal.

**Independent Test**: Run `cosnip tray`, verify the StatusNotifierItem appears with the
AppIndicator host, invoke each menu action, and verify a second `cosnip tray` exits 0 without a
second icon.

### User Story 2 - Settings and TOML (Priority: P1)

As a user, I can edit defaults in a settings window or the TOML file and have new snip/paste
commands honor them.

**Independent Test**: Set `default_output = "save"` and a custom `save_dir`, run `cosnip snip`,
and verify the PNG goes to the configured directory.

### User Story 3 - Desktop Shortcuts (Priority: P1)

As a GNOME user, I can bind `cosnip snip` and `cosnip paste` in GNOME Settings without the
application grabbing keys itself.

**Independent Test**: The settings window shows absolute commands and opens GNOME Keyboard
settings; binding the command to F1/F3 invokes the same CLI path.

### User Story 4 - Text and Cursor Polish (Priority: P1)

As a user, the text annotation editor is transparent and the pointer remains visible after
committing text.

**Independent Test**: Select Text, place text, press Enter, and verify the pointer is visible and
the editor has no black box.

### User Story 5 - Lower Pin Memory (Priority: P2)

As a user, a full-screen pin does not retain the capture frame, overlay textures, and a second
CPU image.

**Independent Test**: Measure PSS for a full-screen pin; target ~120–150 MB on the 6976x2304
validation desktop.

## Functional Requirements

- FR-001: `cosnip tray` MUST provide Snip, Paste, Settings, and Quit through StatusNotifierItem.
- FR-002: A missing SNI/AppIndicator host MUST NOT produce an error; the icon is simply hidden.
- FR-003: A second tray launch MUST exit successfully without a second icon.
- FR-004: `cosnip settings` MUST edit config, use rfd for folder selection, and save atomically.
- FR-005: Config MUST live at `$XDG_CONFIG_HOME/cosnip/config.toml` and respect `--config PATH`.
- FR-006: CLI flags MUST override config; invalid config MUST fall back with a warning.
- FR-007: Global shortcuts MUST remain desktop-managed; the settings UI provides command guidance.
- FR-008: Text editing MUST use the configured color and size with a transparent background and
  optional shadow.
- FR-009: Cursor visibility MUST be restored after text commit and on overlay close.
- FR-010: Pin output MUST release capture resources and avoid duplicate full-frame copies.
- FR-011: No `unsafe`, network, telemetry, or reference assets may be added.
- FR-012: A selection wholly inside a scaled monitor MUST be rendered from that monitor's native
  frame when native output is enabled; the pin display size MUST still match the selection.
- FR-013: Annotation preview position, stroke width, text size, and eraser hit boxes MUST use the
  same frame-pixel scale so previews and pinned text match.
- FR-014: Pins MUST remain image-sized. A shadow margin is not used because transparent
  override-redirect compositing can render incorrectly on some GL stacks.

## Success Criteria

- SC-001: Tray idle PSS under 20 MB and duplicate launch exits in under 1 s.
- SC-002: Settings opens within 2 s and config save is atomic.
- SC-003: New snip/paste commands honor the saved config.
- SC-004: Full-screen pin PSS ≤ 160 MB on the validation desktop (156 MB measured).
- SC-005: Existing snip, paste, annotation, save, and pin tests remain green.

## Assumptions

- GNOME 49 with the AppIndicator extension; the validation host has it enabled.
- XDG FileChooser portal v4 is available for rfd.
- Global hotkeys/single-instance pin registry are follow-up features.
- Text editing remains single-line.
