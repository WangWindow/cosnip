# Feature 007: Native GNOME Shell extension

## Goal

Replace the standalone Rust overlay/pin path on GNOME with a Shell extension
that captures, annotates, copies, saves, and pins natively.

## Requirements

- GNOME Shell 50+, Wayland
- No XWayland overlay or pins
- No PipeWire or portal screenshot dependency
- Native stage capture through `Shell.Screenshot`
- Fullscreen Shell overlay with selection, handles, toolbar, and annotations
- Copy/save/pin through Cairo and `St.ImageContent`
- GSettings configuration and an extension preferences window
- D-Bus automation interface
- Legacy standalone app remains available at tag `legacy-software-v0.1.0`
- Use original SVGs from a consistent open-source icon library and retain its license.
- Keep the frozen selection pixel-aligned and display its physical output size.
- Cover the Shell panel with the frozen image without exposing an intermediate frame.
- Keep annotation previews responsive as existing annotations and strokes grow.
- Zoom pins around the pointer using both mouse wheels and continuous scrolling.

## Acceptance

- Extension loads in GNOME Shell without errors.
- F1 opens a native fullscreen overlay; F3 pins clipboard images.
- Selection, move, resize, and annotation tools work with a real pointer.
- Copy, save, and pin produce the selected region with annotations.
- Pins are image-sized Shell actors with drag, zoom, and right-click close.
- Preferences open with `gnome-extensions prefs cosnip@choco2233`.
- No XWayland window is created for the overlay or pins.
- On a single scaled monitor, selected preview and exported pixels match the
  captured stage; magnifier samples adjacent physical pixels.
- Annotation previews, committed annotations, and exported results agree.
- Pin scrolling works on the normal desktop and ignores hidden pins.

## Non-goals

- X11 support in the extension
- XWayland fallback inside the extension
- Reusing the Rust annotation renderer
