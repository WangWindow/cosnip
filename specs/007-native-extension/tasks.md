# Feature 007 tasks

- [x] Add extension metadata, GSettings schema, stylesheet, and install script.
- [x] Add native stage capture via `Shell.Screenshot`.
- [x] Add fullscreen Shell overlay with selection, handles, and size badge.
- [x] Add toolbar icons and output actions.
- [x] Add rectangle, ellipse, arrow, pencil, highlighter, text, mosaic, blur,
      and eraser annotations.
- [x] Share the Cairo annotation renderer between preview and export.
- [x] Add copy, save, and native pin output.
- [x] Add Shell pin manager with drag, zoom, and right-click close.
- [x] Add GSettings and `gnome-extensions prefs` page.
- [x] Add `org.cosnip.Native` D-Bus automation methods.
- [x] Verify capture, save, annotation export, and pin creation in a headless
      GNOME Shell with a virtual monitor.
- [ ] Verify real-pointer selection, toolbar clicks, text/IME editing, and pin
      drag/zoom in a live GNOME session.
- [ ] Verify clipboard copy in a live session.
- [ ] Connect the cursor toggle to cursor capture and matching preview.

## Overlay and pin quality

- [x] Replace mixed toolbar icons with original Lucide SVGs and its license.
- [x] Correct mask placement, panel stacking, pixel sampling, and the size badge.
- [x] Separate draft and committed annotation redraws and optimize mosaic strokes.
- [x] Fix smooth-scroll direction and zoom pins around the pointer.
- [x] Run isolated pixel, annotation, and wheel regression tests.
- [ ] Validate pointer, perceived latency, and mixed-DPI monitors in a live session.
