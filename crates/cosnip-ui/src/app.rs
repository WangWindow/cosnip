//! The overlay and pin viewports.
//!
#![allow(deprecated)]

//! eframe is forced to the X11 backend. A 1x1 override-redirect control window keeps the event
//! loop alive; the snip overlay is a WM-managed fullscreen viewport, and pins are image-sized
//! override-redirect viewports.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cosnip_capture::{CaptureBackendKind, MonitorInfo};
use cosnip_core::{
    Annotation, AnnotationDocument, AnnotationKind, Frame, Handle, OutputRequest, Point, Rect,
    Selection, Tool,
};
use cosnip_imaging::{crop, encode_png, render};
use cosnip_platform::{
    ClipboardOwner, atomic_write, default_save_path, global_pointer, primary_button_down,
};
use eframe::egui;
use egui::viewport::{ViewportCommand, ViewportId, X11WindowType};
use egui::{
    Color32, ColorImage, Id, Key, Order, Sense, Stroke, StrokeKind, TextureHandle, TextureOptions,
};
use winit::platform::x11::EventLoopBuilderExtX11;

const PIN_ID: &str = "cosnip-pin";

#[derive(Clone, Debug)]
pub struct UiOptions {
    pub backend: CaptureBackendKind,
    pub initial_selection: Option<Rect>,
    pub output: OutputRequest,
    pub save_path: Option<PathBuf>,
    pub monitors: Vec<MonitorInfo>,
}

/// A monitor placed in captured-frame pixels and backed by one fullscreen overlay viewport.
#[derive(Clone, Debug, PartialEq, Eq)]
struct OverlayMonitor {
    index: usize,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl OverlayMonitor {
    fn viewport(&self) -> ViewportId {
        ViewportId::from_hash_of(("cosnip-overlay", self.index))
    }

    fn rect(&self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }

    fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x < self.x.saturating_add(self.width as i32)
            && point.y < self.y.saturating_add(self.height as i32)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    All,
    Copy,
    Save,
    Pin,
}

#[derive(Clone, Copy, Debug)]
enum DragMode {
    NewSelection,
    MoveSelection { start: Rect },
    ResizeSelection { handle: Handle },
    Annotation,
}

struct TextEditor {
    monitor: usize,
    position: Point,
    text: String,
    focus_requested: bool,
}

const PALETTE: [[u8; 4]; 7] = [
    [255, 64, 64, 255],
    [255, 196, 0, 255],
    [80, 220, 100, 255],
    [80, 160, 255, 255],
    [200, 120, 255, 255],
    [255, 255, 255, 255],
    [0, 0, 0, 255],
];
const WIDTHS: [u32; 3] = [1, 3, 6];

#[derive(Clone, Copy, Debug, Default)]
pub struct Outcome {
    pub code: i32,
    pub copied: bool,
    pub saved: bool,
    pub pinned: bool,
}

pub fn run(frame: Frame, options: UiOptions) -> Outcome {
    let outcome = Arc::new(Mutex::new(Outcome {
        code: 4,
        ..Outcome::default()
    }));
    let app = CosnipApp::new(frame, options, Arc::clone(&outcome));
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Cosnip")
            .with_inner_size([1.0, 1.0])
            .with_position([0.0, 0.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_override_redirect(true)
            .with_window_type(X11WindowType::Utility)
            .with_has_shadow(false),
        renderer: eframe::Renderer::Glow,
        event_loop_builder: Some(Box::new(|builder| {
            builder.with_x11();
        })),
        ..Default::default()
    };
    let result = eframe::run_native(
        "Cosnip",
        native_options,
        Box::new(move |cc| Ok(Box::new(app.with_context(cc)))),
    );
    if let Err(error) = result {
        eprintln!("cosnip: UI failed: {error}");
        return Outcome {
            code: 4,
            ..Outcome::default()
        };
    }
    *outcome.lock().unwrap()
}

struct CosnipApp {
    frame: Arc<Frame>,
    tiles: Vec<FrameTile>,
    selection: Selection,
    output: OutputRequest,
    save_path: Option<PathBuf>,
    overlay_open: bool,
    monitors: Vec<OverlayMonitor>,
    focused_monitor: Option<usize>,
    cursor_hidden: Vec<bool>,
    pointer_monitor: Option<usize>,
    drag_anchor: Option<Point>,
    drag_monitor: Option<usize>,
    drag_mode: Option<DragMode>,
    annotations: AnnotationDocument,
    draft: Option<Annotation>,
    tool: Tool,
    color: [u8; 4],
    line_width: u32,
    next_number: u32,
    text_editor: Option<TextEditor>,
    pin: Option<Pin>,
    clipboard: Option<ClipboardOwner>,
    outcome: Arc<Mutex<Outcome>>,
    done: bool,
    auto_confirm_at: Option<std::time::Instant>,
    exit_at: Option<std::time::Instant>,
}

struct Pin {
    frame: Frame,
    tiles: Vec<PinTile>,
    position: (f32, f32),
    scale: f32,
    viewport: ViewportId,
    drag_start_global: Option<(i32, i32)>,
    drag_start_position: (f32, f32),
    dragging: bool,
}

struct PinTile {
    texture: TextureHandle,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

/// A slice of the captured frame small enough for the GL texture-size limit.
struct FrameTile {
    texture: TextureHandle,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl FrameTile {
    fn rect(&self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }
}

impl CosnipApp {
    fn new(frame: Frame, options: UiOptions, outcome: Arc<Mutex<Outcome>>) -> Self {
        let layout = monitor_override().unwrap_or(options.monitors);
        let monitors = fit_monitors(&layout, frame.width(), frame.height());
        let cursor_hidden = vec![false; monitors.len()];
        Self {
            frame: Arc::new(frame),
            tiles: Vec::new(),
            selection: Selection::new(options.initial_selection),
            output: options.output,
            save_path: options.save_path,
            overlay_open: true,
            monitors,
            focused_monitor: None,
            cursor_hidden,
            pointer_monitor: None,
            drag_anchor: None,
            drag_monitor: None,
            drag_mode: None,
            annotations: AnnotationDocument::default(),
            draft: None,
            tool: Tool::Select,
            color: PALETTE[0],
            line_width: 3,
            next_number: 1,
            text_editor: None,
            pin: None,
            clipboard: None,
            outcome,
            done: false,
            auto_confirm_at: std::env::var("COSNIP_TEST_AUTOCONFIRM_MS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .map(|ms| std::time::Instant::now() + std::time::Duration::from_millis(ms)),
            exit_at: None,
        }
    }

    fn with_context(mut self, cc: &eframe::CreationContext<'_>) -> Self {
        let max_side = texture_tile_size(&cc.egui_ctx);
        for y in (0..self.frame.height()).step_by(max_side as usize) {
            for x in (0..self.frame.width()).step_by(max_side as usize) {
                let width = max_side.min(self.frame.width() - x);
                let height = max_side.min(self.frame.height() - y);
                let rect = Rect::new(x as i32, y as i32, width, height);
                let Ok(tile) = crop(&self.frame, rect) else {
                    continue;
                };
                let size = [tile.width() as usize, tile.height() as usize];
                let image = ColorImage::from_rgba_unmultiplied(size, tile.rgba());
                let texture = cc.egui_ctx.load_texture(
                    format!("capture-{x}-{y}"),
                    image,
                    TextureOptions::LINEAR,
                );
                self.tiles.push(FrameTile {
                    texture,
                    x: rect.x,
                    y: rect.y,
                    width,
                    height,
                });
            }
        }
        self
    }

    fn finish(&mut self, ctx: &egui::Context, code: i32) {
        self.finish_delayed(ctx, code, std::time::Duration::ZERO);
    }

    fn finish_delayed(&mut self, ctx: &egui::Context, code: i32, delay: std::time::Duration) {
        self.done = true;
        self.overlay_open = false;
        self.outcome.lock().unwrap().code = code;
        for monitor in &self.monitors {
            ctx.send_viewport_cmd_to(monitor.viewport(), ViewportCommand::Close);
        }
        self.exit_at = Some(std::time::Instant::now() + delay);
        if !delay.is_zero() {
            ctx.request_repaint_after(delay);
        }
    }

    fn close_overlay(&mut self, ctx: &egui::Context) {
        self.overlay_open = false;
        for monitor in &self.monitors {
            ctx.send_viewport_cmd_to(monitor.viewport(), ViewportCommand::Close);
        }
    }

    fn selected_rect(&self) -> Option<Rect> {
        self.selection.rect().filter(|r| !r.is_empty())
    }

    fn rendered_selection(&self) -> Result<(Frame, Rect), String> {
        let rect = self.selected_rect().ok_or("no selection")?;
        let image =
            render(&self.frame, rect, self.annotations.items()).map_err(|e| e.to_string())?;
        Ok((image, rect))
    }

    fn copy_image(&mut self, image: &Frame) -> Result<(), String> {
        let mut owner = ClipboardOwner::new().map_err(|e| e.to_string())?;
        owner.set_image(image).map_err(|e| e.to_string())?;
        self.clipboard = Some(owner);
        self.outcome.lock().unwrap().copied = true;
        eprintln!("cosnip: copied selection");
        Ok(())
    }

    fn save_image(&mut self, image: &Frame, explicit: Option<PathBuf>) -> Result<PathBuf, String> {
        let bytes = encode_png(image).map_err(|e| e.to_string())?;
        let path = match explicit {
            Some(path) => path,
            None => {
                default_save_path(&jiff::Zoned::now().strftime("%Y-%m-%d_%H-%M-%S").to_string())
                    .map_err(|e| e.to_string())?
            }
        };
        atomic_write(&path, &bytes).map_err(|e| e.to_string())?;
        self.outcome.lock().unwrap().saved = true;
        Ok(path)
    }

    fn pin_image(&mut self, ctx: &egui::Context, image: Frame, rect: Rect) -> Result<(), String> {
        let mut tiles = Vec::new();
        let max_side = texture_tile_size(ctx);
        for y in (0..image.height()).step_by(max_side as usize) {
            for x in (0..image.width()).step_by(max_side as usize) {
                let width = max_side.min(image.width() - x);
                let height = max_side.min(image.height() - y);
                let tile_rect = Rect::new(x as i32, y as i32, width, height);
                let Ok(tile) = crop(&image, tile_rect) else {
                    continue;
                };
                let size = [tile.width() as usize, tile.height() as usize];
                let color = ColorImage::from_rgba_unmultiplied(size, tile.rgba());
                let texture =
                    ctx.load_texture(format!("pin-{x}-{y}"), color, TextureOptions::LINEAR);
                tiles.push(PinTile {
                    texture,
                    x: tile_rect.x,
                    y: tile_rect.y,
                    width,
                    height,
                });
            }
        }
        self.pin = Some(Pin {
            frame: image,
            tiles,
            position: (rect.x as f32, rect.y as f32),
            scale: 1.0,
            viewport: ViewportId::from_hash_of(PIN_ID),
            drag_start_global: None,
            drag_start_position: (rect.x as f32, rect.y as f32),
            dragging: false,
        });
        self.outcome.lock().unwrap().pinned = true;
        self.close_overlay(ctx);
        ctx.request_repaint();
        Ok(())
    }

    fn confirm(&mut self, ctx: &egui::Context, action: Action) {
        self.commit_text_editor();
        let (want_copy, want_save, want_pin) = match action {
            Action::All => (
                self.output.copy,
                self.output.save.is_some(),
                self.output.pin,
            ),
            Action::Copy => (true, false, false),
            Action::Save => (false, true, false),
            Action::Pin => (false, false, true),
        };
        if !want_copy && !want_save && !want_pin {
            return;
        }

        let (image, rect) = match self.rendered_selection() {
            Ok(result) => result,
            Err(error) => {
                eprintln!("cosnip: {error}");
                self.finish(ctx, 5);
                return;
            }
        };
        if want_copy && let Err(error) = self.copy_image(&image) {
            eprintln!("cosnip: clipboard failed: {error}");
            self.finish(ctx, 5);
            return;
        }
        if want_save {
            let path = self.save_path.clone();
            match self.save_image(&image, path) {
                Ok(path) => eprintln!("cosnip: saved {}", path.display()),
                Err(error) => {
                    eprintln!("cosnip: save failed: {error}");
                    self.finish(ctx, 5);
                    return;
                }
            }
        }
        if want_pin {
            if let Err(error) = self.pin_image(ctx, image, rect) {
                eprintln!("cosnip: pin failed: {error}");
                self.finish(ctx, 5);
            }
        } else {
            let delay = if want_copy {
                std::time::Duration::from_millis(800)
            } else {
                std::time::Duration::ZERO
            };
            self.finish_delayed(ctx, 0, delay);
        }
    }

    fn show_overlay(&mut self, ctx: &egui::Context) {
        for monitor in self.monitors.clone() {
            let mut builder = egui::ViewportBuilder::default()
                .with_title(format!("Cosnip Snipper {}", monitor.index + 1))
                .with_decorations(false)
                .with_always_on_top()
                .with_has_shadow(false);
            if std::env::var_os("COSNIP_TEST_WINDOWED_OVERLAY").is_some() {
                builder = builder
                    .with_position([monitor.x as f32, monitor.y as f32])
                    .with_inner_size([monitor.width as f32, monitor.height as f32]);
            } else {
                builder = builder.with_fullscreen(true).with_monitor(monitor.index);
            }
            ctx.show_viewport_immediate(monitor.viewport(), builder, |ui, _class| {
                self.overlay_ui(ui, &monitor);
            });
        }
    }

    fn overlay_ui(&mut self, ui: &mut egui::Ui, monitor: &OverlayMonitor) {
        let ctx = ui.ctx().clone();
        let screen = ui.max_rect();
        let pixels_per_point = ctx.pixels_per_point();
        let bounds = Rect::new(0, 0, self.frame.width(), self.frame.height());
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(false));
        if focused {
            self.focused_monitor = Some(monitor.index);
        }
        let hover = ctx.input(|i| i.pointer.hover_pos());
        let pointer_global = hover.map(|p| self.local_to_global(p, monitor, pixels_per_point));
        let pointer_inside = pointer_global.is_some_and(|p| monitor.contains(p));
        if pointer_inside {
            self.pointer_monitor = Some(monitor.index);
        }

        let needs_focus = (pointer_inside && self.focused_monitor != Some(monitor.index))
            || (self.focused_monitor.is_none() && monitor.index == 0);
        if needs_focus && !focused {
            ctx.send_viewport_cmd(ViewportCommand::Focus);
            self.focused_monitor = Some(monitor.index);
        }
        if !self.cursor_hidden[monitor.index] {
            ctx.send_viewport_cmd(ViewportCommand::CursorVisible(false));
            self.cursor_hidden[monitor.index] = true;
        }

        let response = ui.interact(
            screen,
            Id::new(("overlay-canvas", monitor.index)),
            Sense::click_and_drag(),
        );
        let text_editing = self.text_editor.is_some();
        let over_chrome = ctx.is_pointer_over_egui();
        if ctx.input(|i| i.pointer.primary_pressed())
            && response.hovered()
            && let Some(p) = pointer_global
        {
            if self.text_editor.is_some() {
                if !over_chrome {
                    self.commit_text_editor();
                }
            } else if !over_chrome {
                self.begin_pointer_action(p, monitor.index);
            }
        }
        if response.drag_started()
            && self.drag_mode.is_none()
            && self.text_editor.is_none()
            && !over_chrome
            && let Some(p) = pointer_global
        {
            self.begin_pointer_action(p, monitor.index);
        }
        let is_drag_monitor = self.drag_monitor == Some(monitor.index);
        if let Some(current) = pointer_global
            && is_drag_monitor
            && (response.dragged() || ctx.input(|i| i.pointer.primary_down()))
        {
            let square = ctx.input(|i| i.modifiers.shift);
            self.update_drag(current, square, bounds);
        }
        if is_drag_monitor && (response.drag_stopped() || ctx.input(|i| i.pointer.any_released())) {
            self.finish_drag();
        }
        if ctx.input(|i| i.pointer.secondary_pressed()) && response.hovered() && !over_chrome {
            if self.drag_mode.is_some() {
                self.finish_drag();
            } else if self.text_editor.is_some() {
                self.commit_text_editor();
            } else if self.tool != Tool::Select {
                self.set_tool(Tool::Select);
            }
        }

        // Draw this monitor's slice of the frozen frame, tile by tile.
        for tile in &self.tiles {
            let Some(visible) = tile.rect().intersect(monitor.rect()) else {
                continue;
            };
            let target = self.global_to_local(visible, monitor, pixels_per_point);
            let uv = egui::Rect::from_min_max(
                egui::pos2(
                    (visible.x - tile.x) as f32 / tile.width as f32,
                    (visible.y - tile.y) as f32 / tile.height as f32,
                ),
                egui::pos2(
                    (visible.x - tile.x + visible.width as i32) as f32 / tile.width as f32,
                    (visible.y - tile.y + visible.height as i32) as f32 / tile.height as f32,
                ),
            );
            ui.painter()
                .image(tile.texture.id(), target, uv, Color32::WHITE);
        }
        let selection = self.selected_rect();
        let local_selection = selection
            .map(|selection| self.global_to_local(selection, monitor, pixels_per_point))
            .filter(|selection| selection.intersects(screen))
            .map(|selection| selection.intersect(screen));
        draw_dim(ui, screen, local_selection);
        if let Some(selection) = selection {
            let local = self.global_to_local(selection, monitor, pixels_per_point);
            draw_selection(ui, screen, local, self.tool == Tool::Select);
            let clip = local.intersect(screen);
            for annotation in self.annotations.items() {
                self.draw_annotation_preview(ui, monitor, pixels_per_point, annotation, clip);
            }
            if let Some(draft) = &self.draft {
                self.draw_annotation_preview(ui, monitor, pixels_per_point, draft, clip);
            }
        }
        if let Some(local_pointer) = hover
            && pointer_inside
            && !over_chrome
        {
            draw_crosshair(ui, screen, local_pointer);
            self.draw_magnifier(ui, screen, local_pointer, pointer_global.unwrap());
        }
        if let Some(selection) = selection {
            if monitor.contains(Point::new(selection.x, selection.y)) {
                let local = self.global_to_local(selection, monitor, pixels_per_point);
                draw_size_badge(ui, local);
            }
            if selection_center_monitor(&self.monitors, selection) == Some(monitor.index) {
                self.show_toolbar(&ctx, monitor, selection, screen, pixels_per_point);
            }
        }
        self.show_text_editor(ui, monitor, pixels_per_point);
        let tips_monitor = self.pointer_monitor.or(self.focused_monitor).unwrap_or(0);
        if tips_monitor == monitor.index {
            draw_tips(ui, screen);
        }
        if self.focused_monitor == Some(monitor.index) && !text_editing {
            self.handle_keys(&ctx, bounds);
        }
    }

    fn local_to_global(
        &self,
        point: egui::Pos2,
        monitor: &OverlayMonitor,
        pixels_per_point: f32,
    ) -> Point {
        Point::new(
            monitor.x + (point.x * pixels_per_point).round() as i32,
            monitor.y + (point.y * pixels_per_point).round() as i32,
        )
    }

    fn global_to_local(
        &self,
        rect: Rect,
        monitor: &OverlayMonitor,
        pixels_per_point: f32,
    ) -> egui::Rect {
        let scale = pixels_per_point.max(f32::EPSILON);
        egui::Rect::from_min_max(
            egui::pos2(
                (rect.x - monitor.x) as f32 / scale,
                (rect.y - monitor.y) as f32 / scale,
            ),
            egui::pos2(
                (rect.right() - monitor.x) as f32 / scale,
                (rect.bottom() - monitor.y) as f32 / scale,
            ),
        )
    }

    fn point_to_local(
        &self,
        point: Point,
        monitor: &OverlayMonitor,
        pixels_per_point: f32,
    ) -> egui::Pos2 {
        let scale = pixels_per_point.max(f32::EPSILON);
        egui::pos2(
            (point.x - monitor.x) as f32 / scale,
            (point.y - monitor.y) as f32 / scale,
        )
    }

    fn begin_pointer_action(&mut self, point: Point, monitor_index: usize) {
        let selection = self.selected_rect();
        let inside_selection = selection.is_some_and(|rect| rect.contains(point));
        self.drag_anchor = Some(point);
        self.drag_monitor = Some(monitor_index);

        if self.tool == Tool::Select {
            if let Some(rect) = selection {
                if let Some(handle) = self.selection.handle_at(point, 8) {
                    self.drag_mode = Some(DragMode::ResizeSelection { handle });
                    return;
                }
                if rect.contains(point) {
                    self.drag_mode = Some(DragMode::MoveSelection { start: rect });
                    return;
                }
            }
            self.selection.begin(point);
            self.drag_mode = Some(DragMode::NewSelection);
            return;
        }

        if !inside_selection {
            self.selection.begin(point);
            self.drag_mode = Some(DragMode::NewSelection);
            return;
        }

        match self.tool {
            Tool::Eraser => {
                self.annotations.remove_at(point);
                self.drag_anchor = None;
                self.drag_monitor = None;
                self.drag_mode = None;
            }
            Tool::Text => {
                self.text_editor = Some(TextEditor {
                    monitor: monitor_index,
                    position: point,
                    text: String::new(),
                    focus_requested: false,
                });
                self.drag_anchor = None;
                self.drag_monitor = None;
                self.drag_mode = None;
            }
            Tool::Number => {
                let value = self.next_number;
                self.next_number = self.next_number.saturating_add(1);
                self.annotations.add(Annotation::new(
                    AnnotationKind::Number {
                        position: point,
                        value,
                    },
                    self.color,
                    self.line_width,
                ));
                self.drag_anchor = None;
                self.drag_monitor = None;
                self.drag_mode = None;
            }
            Tool::Magnifier => {
                self.annotations.add(Annotation::new(
                    AnnotationKind::Magnifier {
                        center: point,
                        radius: 48,
                        zoom: 2,
                    },
                    self.color,
                    self.line_width,
                ));
                self.drag_anchor = None;
                self.drag_monitor = None;
                self.drag_mode = None;
            }
            tool if tool.is_draw_tool() => {
                self.draft = self.new_draft(point);
                self.drag_mode = Some(DragMode::Annotation);
            }
            _ => {
                self.drag_anchor = None;
                self.drag_monitor = None;
                self.drag_mode = None;
            }
        }
    }

    fn new_draft(&self, point: Point) -> Option<Annotation> {
        let kind = match self.tool {
            Tool::Rectangle => AnnotationKind::Rectangle {
                rect: Rect::new(point.x, point.y, 0, 0),
            },
            Tool::Ellipse => AnnotationKind::Ellipse {
                rect: Rect::new(point.x, point.y, 0, 0),
            },
            Tool::Arrow => AnnotationKind::Arrow {
                start: point,
                end: point,
            },
            Tool::Line => AnnotationKind::Line {
                start: point,
                end: point,
            },
            Tool::Pencil => AnnotationKind::Pencil {
                points: vec![point],
            },
            Tool::Marker => AnnotationKind::Marker {
                points: vec![point],
            },
            Tool::Mosaic => AnnotationKind::Mosaic {
                rect: Rect::new(point.x, point.y, 0, 0),
            },
            Tool::Blur => AnnotationKind::Blur {
                rect: Rect::new(point.x, point.y, 0, 0),
            },
            _ => return None,
        };
        Some(Annotation::new(kind, self.color, self.line_width))
    }

    fn update_drag(&mut self, current: Point, square: bool, bounds: Rect) {
        match self.drag_mode {
            Some(DragMode::NewSelection) => {
                if let Some(anchor) = self.drag_anchor {
                    self.selection.drag_to(anchor, current, bounds, square);
                }
            }
            Some(DragMode::MoveSelection { start }) => {
                if let Some(anchor) = self.drag_anchor {
                    self.selection.move_from(
                        start,
                        current.x - anchor.x,
                        current.y - anchor.y,
                        bounds,
                    );
                }
            }
            Some(DragMode::ResizeSelection { handle }) => {
                self.selection.resize(handle, current, bounds, 4);
            }
            Some(DragMode::Annotation) => {
                let Some(anchor) = self.drag_anchor else {
                    return;
                };
                let Some(draft) = self.draft.as_mut() else {
                    return;
                };
                match &mut draft.kind {
                    AnnotationKind::Rectangle { rect }
                    | AnnotationKind::Ellipse { rect }
                    | AnnotationKind::Mosaic { rect }
                    | AnnotationKind::Blur { rect } => {
                        *rect = if square {
                            Rect::square_from_points(anchor, current)
                        } else {
                            Rect::from_points(anchor, current)
                        }
                        .clamp_to(bounds)
                        .unwrap_or(Rect::new(anchor.x, anchor.y, 1, 1));
                    }
                    AnnotationKind::Arrow { end, .. } | AnnotationKind::Line { end, .. } => {
                        *end = current;
                    }
                    AnnotationKind::Pencil { points } | AnnotationKind::Marker { points }
                        if points.last().is_none_or(|last| *last != current) =>
                    {
                        points.push(current);
                    }
                    _ => {}
                }
            }
            None => {}
        }
    }

    fn finish_drag(&mut self) {
        if matches!(self.drag_mode, Some(DragMode::Annotation))
            && let Some(draft) = self.draft.take()
            && draft_is_valid(&draft)
        {
            self.annotations.add(draft);
        }
        self.draft = None;
        self.drag_anchor = None;
        self.drag_monitor = None;
        self.drag_mode = None;
        if self.selection.rect().is_some_and(|rect| rect.is_empty()) {
            self.selection.clear();
        }
    }

    fn cancel_drag(&mut self) -> bool {
        if self.drag_mode.is_none() {
            return false;
        }
        self.draft = None;
        self.drag_anchor = None;
        self.drag_monitor = None;
        self.drag_mode = None;
        if self.selection.rect().is_some_and(|rect| rect.is_empty()) {
            self.selection.clear();
        }
        true
    }

    fn commit_text_editor(&mut self) {
        let Some(editor) = self.text_editor.take() else {
            return;
        };
        let text = editor.text.trim();
        if text.is_empty() {
            return;
        }
        self.annotations.add(Annotation::new(
            AnnotationKind::Text {
                position: editor.position,
                text: text.to_string(),
                size: 20,
            },
            self.color,
            self.line_width,
        ));
    }

    fn set_tool(&mut self, tool: Tool) {
        if self.tool == tool {
            return;
        }
        self.cancel_drag();
        self.commit_text_editor();
        self.tool = tool;
    }

    fn show_text_editor(
        &mut self,
        ui: &mut egui::Ui,
        monitor: &OverlayMonitor,
        pixels_per_point: f32,
    ) {
        let Some(editor_info) = self
            .text_editor
            .as_ref()
            .map(|editor| (editor.monitor, editor.position))
        else {
            return;
        };
        if editor_info.0 != monitor.index {
            return;
        }
        let position = self.point_to_local(editor_info.1, monitor, pixels_per_point);
        let Some(editor) = self.text_editor.as_mut() else {
            return;
        };
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::CursorVisible(true));
        self.cursor_hidden[monitor.index] = false;
        let id = Id::new("cosnip-text-editor");
        let mut commit = false;
        let mut cancel = false;
        egui::Area::new(id)
            .fixed_pos(position)
            .order(Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::NONE
                    .fill(Color32::from_black_alpha(220))
                    .inner_margin(egui::Margin::same(4))
                    .corner_radius(egui::CornerRadius::same(4))
                    .show(ui, |ui| {
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut editor.text)
                                .desired_width(220.0)
                                .hint_text("Text"),
                        );
                        if !editor.focus_requested {
                            response.request_focus();
                            editor.focus_requested = true;
                        }
                        ui.label(
                            egui::RichText::new("Enter place · Esc cancel")
                                .size(10.0)
                                .color(Color32::from_gray(190)),
                        );
                    });
            });
        if ui.ctx().input(|i| i.key_pressed(Key::Enter)) {
            commit = true;
        }
        if ui.ctx().input(|i| i.key_pressed(Key::Escape)) {
            cancel = true;
        }
        if commit {
            self.commit_text_editor();
        } else if cancel {
            self.text_editor = None;
        }
    }

    fn draw_annotation_preview(
        &self,
        ui: &egui::Ui,
        monitor: &OverlayMonitor,
        pixels_per_point: f32,
        annotation: &Annotation,
        clip: egui::Rect,
    ) {
        let painter = ui.painter().with_clip_rect(clip);
        let color = Color32::from_rgba_unmultiplied(
            annotation.color[0],
            annotation.color[1],
            annotation.color[2],
            annotation.color[3],
        );
        let width = annotation.width.max(1) as f32;
        let stroke = Stroke::new(width, color);
        match &annotation.kind {
            AnnotationKind::Rectangle { rect } => {
                let local = self.global_to_local(*rect, monitor, pixels_per_point);
                painter.rect_stroke(local, 0.0, stroke, StrokeKind::Inside);
            }
            AnnotationKind::Ellipse { rect } => {
                let local = self.global_to_local(*rect, monitor, pixels_per_point);
                painter.add(egui::Shape::ellipse_stroke(
                    local.center(),
                    local.size() / 2.0,
                    stroke,
                ));
            }
            AnnotationKind::Arrow { start, end } => {
                let start = self.point_to_local(*start, monitor, pixels_per_point);
                let end = self.point_to_local(*end, monitor, pixels_per_point);
                painter.line_segment([start, end], stroke);
                let angle = (end.y - start.y).atan2(end.x - start.x);
                let length = (12.0 + width * 2.0).min(40.0);
                for spread in [-0.5, 0.5] {
                    let tip = end
                        - egui::vec2(
                            length * (angle + spread).cos(),
                            length * (angle + spread).sin(),
                        );
                    painter.line_segment([end, tip], stroke);
                }
            }
            AnnotationKind::Line { start, end } => {
                painter.line_segment(
                    [
                        self.point_to_local(*start, monitor, pixels_per_point),
                        self.point_to_local(*end, monitor, pixels_per_point),
                    ],
                    stroke,
                );
            }
            AnnotationKind::Pencil { points } => {
                draw_polyline_preview(&painter, points, monitor, pixels_per_point, stroke);
            }
            AnnotationKind::Marker { points } => {
                let marker = Stroke::new(
                    (width * 4.0).max(4.0),
                    Color32::from_rgba_unmultiplied(
                        annotation.color[0],
                        annotation.color[1],
                        annotation.color[2],
                        96,
                    ),
                );
                draw_polyline_preview(&painter, points, monitor, pixels_per_point, marker);
            }
            AnnotationKind::Text {
                position,
                text,
                size,
            } => {
                let position = self.point_to_local(*position, monitor, pixels_per_point);
                painter.text(
                    position,
                    egui::Align2::LEFT_TOP,
                    text,
                    egui::FontId::proportional(*size as f32),
                    color,
                );
            }
            AnnotationKind::Number { position, value } => {
                let position = self.point_to_local(*position, monitor, pixels_per_point);
                let radius = (14.0 + width).clamp(12.0, 24.0);
                painter.circle_filled(position, radius, color);
                painter.text(
                    position,
                    egui::Align2::CENTER_CENTER,
                    value.to_string(),
                    egui::FontId::proportional(radius * 1.4),
                    Color32::WHITE,
                );
            }
            AnnotationKind::Mosaic { rect } => {
                self.draw_block_effect_preview(
                    &painter,
                    monitor,
                    pixels_per_point,
                    *rect,
                    (annotation.width * 4).max(8),
                );
            }
            AnnotationKind::Blur { rect } => {
                self.draw_block_effect_preview(
                    &painter,
                    monitor,
                    pixels_per_point,
                    *rect,
                    (annotation.width * 2).max(3),
                );
            }
            AnnotationKind::Magnifier { .. } => {
                self.draw_magnifier_annotation_preview(
                    &painter,
                    monitor,
                    pixels_per_point,
                    annotation,
                );
            }
        }
    }

    fn draw_block_effect_preview(
        &self,
        painter: &egui::Painter,
        monitor: &OverlayMonitor,
        pixels_per_point: f32,
        rect: Rect,
        requested_block: u32,
    ) {
        let mut block = requested_block.max(2);
        let mut columns = rect.width / block + 1;
        let mut rows = rect.height / block + 1;
        while columns.saturating_mul(rows) > 4096 {
            block = block.saturating_mul(2);
            columns = rect.width / block + 1;
            rows = rect.height / block + 1;
        }
        let mut y = rect.y;
        while y < rect.bottom() {
            let mut x = rect.x;
            let height = block.min((rect.bottom() - y).max(1) as u32);
            while x < rect.right() {
                let width = block.min((rect.right() - x).max(1) as u32);
                let sample_x = x + width as i32 / 2;
                let sample_y = y + height as i32 / 2;
                let color = self
                    .frame
                    .pixel(sample_x, sample_y)
                    .map_or(Color32::BLACK, |p| Color32::from_rgb(p[0], p[1], p[2]));
                let cell = Rect::new(x, y, width, height);
                let local = self.global_to_local(cell, monitor, pixels_per_point);
                painter.rect_filled(local, 0.0, color);
                x += block as i32;
            }
            y += block as i32;
        }
    }

    fn draw_magnifier_annotation_preview(
        &self,
        painter: &egui::Painter,
        monitor: &OverlayMonitor,
        pixels_per_point: f32,
        annotation: &Annotation,
    ) {
        let AnnotationKind::Magnifier {
            center,
            radius,
            zoom,
        } = &annotation.kind
        else {
            return;
        };
        let center = *center;
        let radius = *radius;
        let zoom = (*zoom).max(1) as f32;
        let color = Color32::from_rgba_unmultiplied(
            annotation.color[0],
            annotation.color[1],
            annotation.color[2],
            annotation.color[3],
        );
        let width = annotation.width.max(1) as f32;
        let scale = pixels_per_point.max(f32::EPSILON);
        let center_local = self.point_to_local(center, monitor, pixels_per_point);
        let radius_local = radius as f32 / scale;
        let cell = 4.0_f32;
        let mut y = -radius_local;
        while y < radius_local {
            let mut x = -radius_local;
            while x < radius_local {
                if x * x + y * y <= radius_local * radius_local {
                    let source = Point::new(
                        center.x + (x * scale / zoom).round() as i32,
                        center.y + (y * scale / zoom).round() as i32,
                    );
                    let sample = self
                        .frame
                        .pixel(source.x, source.y)
                        .map_or(Color32::BLACK, |p| Color32::from_rgb(p[0], p[1], p[2]));
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            center_local + egui::vec2(x, y),
                            egui::vec2(
                                cell.min(radius_local - x).max(0.5),
                                cell.min(radius_local - y).max(0.5),
                            ),
                        ),
                        0.0,
                        sample,
                    );
                }
                x += cell;
            }
            y += cell;
        }
        painter.circle_stroke(
            center_local,
            radius_local,
            Stroke::new(width.max(2.0), color),
        );
    }

    fn handle_keys(&mut self, ctx: &egui::Context, bounds: Rect) {
        let (escape, enter, ctrl_c, ctrl_s, ctrl_a, pin_key, undo, redo, left, right, up, down) =
            ctx.input(|i| {
                (
                    i.key_pressed(Key::Escape),
                    i.key_pressed(Key::Enter),
                    i.modifiers.command && i.key_pressed(Key::C),
                    i.modifiers.command && i.key_pressed(Key::S),
                    i.modifiers.command && i.key_pressed(Key::A),
                    i.key_pressed(Key::P),
                    i.modifiers.command && i.key_pressed(Key::Z) && !i.modifiers.shift,
                    i.modifiers.command
                        && (i.key_pressed(Key::Y) || (i.modifiers.shift && i.key_pressed(Key::Z))),
                    i.key_pressed(Key::ArrowLeft),
                    i.key_pressed(Key::ArrowRight),
                    i.key_pressed(Key::ArrowUp),
                    i.key_pressed(Key::ArrowDown),
                )
            });
        if escape {
            if self.cancel_drag() {
                return;
            }
            self.finish(ctx, 6);
            return;
        }
        if undo {
            self.annotations.undo();
        }
        if redo {
            self.annotations.redo();
        }
        if ctrl_a {
            self.selection.full(bounds);
        }
        if left || right || up || down {
            let dx = i32::from(right) - i32::from(left);
            let dy = i32::from(down) - i32::from(up);
            self.selection.nudge(dx, dy, bounds);
        }
        if enter {
            self.confirm(ctx, Action::All);
        } else if ctrl_c {
            self.confirm(ctx, Action::Copy);
        } else if ctrl_s {
            self.confirm(ctx, Action::Save);
        } else if pin_key {
            self.confirm(ctx, Action::Pin);
        }
    }

    fn show_toolbar(
        &mut self,
        ctx: &egui::Context,
        monitor: &OverlayMonitor,
        selection: Rect,
        screen: egui::Rect,
        pixels_per_point: f32,
    ) {
        let local = self.global_to_local(selection, monitor, pixels_per_point);
        let width = 680.0;
        let x = local.left().clamp(
            screen.left() + 4.0,
            (screen.right() - width - 4.0).max(screen.left() + 4.0),
        );
        let mut y = local.bottom() + 8.0;
        if y + 86.0 > screen.bottom() {
            y = (local.top() - 94.0).max(screen.top() + 4.0);
        }
        egui::Area::new(Id::new(("cosnip-toolbar", monitor.index)))
            .fixed_pos(egui::pos2(x, y))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(Color32::from_rgb(28, 28, 32))
                    .inner_margin(egui::Margin::same(6))
                    .corner_radius(egui::CornerRadius::same(6))
                    .show(ui, |ui| {
                        ui.set_max_width(width);
                        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                        ui.horizontal_wrapped(|ui| {
                            for tool in Tool::ALL {
                                if ui
                                    .selectable_label(self.tool == tool, tool.label())
                                    .on_hover_text(tool_hint(tool))
                                    .clicked()
                                {
                                    self.set_tool(tool);
                                }
                            }
                        });
                        ui.separator();
                        ui.horizontal_wrapped(|ui| {
                            for color in PALETTE {
                                let (rect, response) =
                                    ui.allocate_exact_size(egui::vec2(16.0, 16.0), Sense::click());
                                ui.painter().rect_filled(
                                    rect,
                                    3.0,
                                    Color32::from_rgba_unmultiplied(
                                        color[0], color[1], color[2], color[3],
                                    ),
                                );
                                if self.color == color {
                                    ui.painter().rect_stroke(
                                        rect.expand(1.0),
                                        3.0,
                                        Stroke::new(1.0, Color32::WHITE),
                                        StrokeKind::Outside,
                                    );
                                }
                                if response.clicked() {
                                    self.color = color;
                                }
                            }
                            ui.separator();
                            for width in WIDTHS {
                                if ui
                                    .selectable_label(
                                        self.line_width == width,
                                        format!("{width}px"),
                                    )
                                    .clicked()
                                {
                                    self.line_width = width;
                                }
                            }
                            ui.separator();
                            if ui
                                .add_enabled(self.annotations.can_undo(), egui::Button::new("Undo"))
                                .clicked()
                            {
                                self.annotations.undo();
                            }
                            if ui
                                .add_enabled(self.annotations.can_redo(), egui::Button::new("Redo"))
                                .clicked()
                            {
                                self.annotations.redo();
                            }
                            ui.separator();
                            if ui.button("Copy").clicked() {
                                self.confirm(ui.ctx(), Action::Copy);
                            }
                            if ui.button("Save").clicked() {
                                self.confirm(ui.ctx(), Action::Save);
                            }
                            if ui.button("Pin").clicked() {
                                self.confirm(ui.ctx(), Action::Pin);
                            }
                            if ui.button("Cancel").clicked() {
                                self.finish(ui.ctx(), 6);
                            }
                        });
                    });
            });
    }

    fn draw_magnifier(
        &self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        pointer: egui::Pos2,
        center: Point,
    ) {
        let radius = 5;
        let cell = 8.0;
        let box_size = egui::vec2(
            (radius * 2 + 1) as f32 * cell,
            (radius * 2 + 1) as f32 * cell,
        );
        let mut origin = egui::pos2(pointer.x + 20.0, pointer.y + 20.0);
        if origin.x + box_size.x > rect.right() {
            origin.x = pointer.x - box_size.x - 20.0;
        }
        if origin.y + box_size.y + 26.0 > rect.bottom() {
            origin.y = pointer.y - box_size.y - 46.0;
        }
        ui.painter().rect_filled(
            egui::Rect::from_min_size(origin, box_size).expand(4.0),
            4.0,
            Color32::from_black_alpha(220),
        );
        for gy in 0..(radius * 2 + 1) {
            for gx in 0..(radius * 2 + 1) {
                let px = center.x - radius + gx;
                let py = center.y - radius + gy;
                let color = self
                    .frame
                    .pixel(px, py)
                    .map_or(Color32::BLACK, |p| Color32::from_rgb(p[0], p[1], p[2]));
                let cell_rect = egui::Rect::from_min_size(
                    origin + egui::vec2(gx as f32 * cell, gy as f32 * cell),
                    egui::vec2(cell, cell),
                );
                ui.painter().rect_filled(cell_rect, 0.0, color);
            }
        }
        ui.painter().rect_stroke(
            egui::Rect::from_min_size(origin, box_size),
            0.0,
            Stroke::new(1.0, Color32::WHITE),
            StrokeKind::Inside,
        );
        let color = self
            .frame
            .pixel(center.x, center.y)
            .unwrap_or([0, 0, 0, 255]);
        ui.painter().text(
            origin + egui::vec2(0.0, box_size.y + 8.0),
            egui::Align2::LEFT_TOP,
            format!(
                "{}, {}  #{:02X}{:02X}{:02X}",
                center.x, center.y, color[0], color[1], color[2]
            ),
            egui::FontId::monospace(12.0),
            Color32::WHITE,
        );
    }

    fn show_pin(&mut self, ctx: &egui::Context) {
        let Some(pin) = self.pin.as_mut() else { return };
        let base = egui::vec2(pin.frame.width() as f32, pin.frame.height() as f32);
        let size = base * pin.scale;
        let builder = egui::ViewportBuilder::default()
            .with_title("Cosnip Pin")
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_override_redirect(true)
            .with_window_type(X11WindowType::Utility)
            .with_position([pin.position.0, pin.position.1])
            .with_inner_size([size.x, size.y])
            .with_has_shadow(false);
        let viewport = pin.viewport;
        let tiles = pin
            .tiles
            .iter()
            .map(|tile| {
                let min = egui::pos2(tile.x as f32 * pin.scale, tile.y as f32 * pin.scale);
                let max = egui::pos2(
                    (tile.x + tile.width as i32) as f32 * pin.scale,
                    (tile.y + tile.height as i32) as f32 * pin.scale,
                );
                (tile.texture.id(), egui::Rect::from_min_max(min, max))
            })
            .collect::<Vec<_>>();
        let base_size = base;
        let mut new_position = pin.position;
        let mut new_scale = pin.scale;
        let mut close = false;
        ctx.show_viewport_immediate(viewport, builder, |ui, _class| {
            let ctx = ui.ctx().clone();
            let rect = ui.max_rect();
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            for (texture, target) in &tiles {
                ui.painter().image(
                    *texture,
                    target.translate(rect.min.to_vec2()),
                    uv,
                    Color32::WHITE,
                );
            }
            let response = ui.interact(rect, Id::new("pin-canvas"), Sense::click_and_drag());
            if response.drag_started()
                && let Some(current) = global_pointer()
            {
                let delta = ctx.input(|i| i.pointer.delta());
                let start = (
                    current.0 - delta.x.round() as i32,
                    current.1 - delta.y.round() as i32,
                );
                pin.drag_start_global = Some(start);
                pin.drag_start_position = new_position;
                pin.dragging = true;
            }
            if pin.dragging {
                if let (Some(start), Some(current)) = (pin.drag_start_global, global_pointer()) {
                    new_position = (
                        pin.drag_start_position.0 + (current.0 - start.0) as f32,
                        pin.drag_start_position.1 + (current.1 - start.1) as f32,
                    );
                    ctx.send_viewport_cmd(ViewportCommand::OuterPosition(egui::pos2(
                        new_position.0,
                        new_position.1,
                    )));
                }
                if !primary_button_down() {
                    pin.dragging = false;
                    pin.drag_start_global = None;
                } else {
                    ctx.request_repaint_after(std::time::Duration::from_millis(8));
                }
            }
            if response.drag_stopped() {
                pin.dragging = false;
                pin.drag_start_global = None;
            }
            if response.double_clicked() || response.secondary_clicked() {
                close = true;
            }
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                close = true;
            }
            let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.1 {
                new_scale = (new_scale * (1.0 + scroll * 0.002)).clamp(0.1, 5.0);
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(base_size * new_scale));
            }
            // A visible close affordance for windows without keyboard focus.
            let close_rect = egui::Rect::from_min_size(
                rect.right_top() + egui::vec2(-22.0, 4.0),
                egui::vec2(18.0, 18.0),
            );
            ui.painter()
                .circle_filled(close_rect.center(), 9.0, Color32::from_black_alpha(170));
            ui.painter().text(
                close_rect.center(),
                egui::Align2::CENTER_CENTER,
                "×",
                egui::FontId::proportional(14.0),
                Color32::WHITE,
            );
            if ui
                .interact(close_rect, Id::new("pin-close"), Sense::click())
                .clicked()
            {
                close = true;
            }
        });
        if close {
            ctx.send_viewport_cmd_to(viewport, ViewportCommand::Close);
            self.pin = None;
            self.finish(ctx, 0);
        } else if let Some(pin) = self.pin.as_mut() {
            pin.position = new_position;
            pin.scale = new_scale;
        }
    }
}

impl eframe::App for CosnipApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(at) = self.auto_confirm_at {
            let now = std::time::Instant::now();
            if self.overlay_open && now >= at {
                self.auto_confirm_at = None;
                self.confirm(ctx, Action::All);
            } else {
                ctx.request_repaint_after(at.saturating_duration_since(now));
            }
        }
        if self.overlay_open {
            self.show_overlay(ctx);
            if self.drag_mode.is_some() && !primary_button_down() {
                self.finish_drag();
            }
        }
        if self.pin.is_some() {
            self.show_pin(ctx);
        }
        if self.done {
            if self
                .exit_at
                .is_none_or(|at| std::time::Instant::now() >= at)
            {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            } else if let Some(at) = self.exit_at {
                ctx.request_repaint_after(at.saturating_duration_since(std::time::Instant::now()));
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The root viewport is a 1x1 control window. Overlay and pin content live in viewports.
        let _ = ui;
    }
}

fn texture_tile_size(ctx: &egui::Context) -> u32 {
    ctx.input(|i| i.max_texture_side).clamp(64, 2048) as u32
}

#[cfg(debug_assertions)]
fn monitor_override() -> Option<Vec<MonitorInfo>> {
    let value = std::env::var("COSNIP_TEST_MONITORS").ok()?;
    let mut monitors = Vec::new();
    for (index, part) in value.split(',').enumerate() {
        let (size, position) = part.trim().split_once('+')?;
        let (width, height) = size.split_once('x')?;
        let (x, y) = position.split_once('+')?;
        monitors.push(MonitorInfo {
            x: x.parse().ok()?,
            y: y.parse().ok()?,
            width: width.parse().ok()?,
            height: height.parse().ok()?,
            primary: index == 0,
            name: format!("test-{index}"),
        });
    }
    (!monitors.is_empty()).then_some(monitors)
}

#[cfg(not(debug_assertions))]
fn monitor_override() -> Option<Vec<MonitorInfo>> {
    None
}

/// Map RandR monitor coordinates onto the captured frame and assign each one a viewport.
fn fit_monitors(monitors: &[MonitorInfo], width: u32, height: u32) -> Vec<OverlayMonitor> {
    let width = width.max(1);
    let height = height.max(1);
    if monitors.is_empty() {
        return vec![OverlayMonitor {
            index: 0,
            x: 0,
            y: 0,
            width,
            height,
        }];
    }
    let min_x = monitors.iter().map(|m| m.x).min().unwrap_or(0);
    let min_y = monitors.iter().map(|m| m.y).min().unwrap_or(0);
    let max_x = monitors
        .iter()
        .map(|m| m.x.saturating_add(m.width as i32))
        .max()
        .unwrap_or(min_x.saturating_add(1));
    let max_y = monitors
        .iter()
        .map(|m| m.y.saturating_add(m.height as i32))
        .max()
        .unwrap_or(min_y.saturating_add(1));
    let span_x = f64::from(max_x.saturating_sub(min_x).max(1));
    let span_y = f64::from(max_y.saturating_sub(min_y).max(1));
    let scale_x = f64::from(width) / span_x;
    let scale_y = f64::from(height) / span_y;
    monitors
        .iter()
        .enumerate()
        .map(|(index, monitor)| {
            let left = ((f64::from(monitor.x.saturating_sub(min_x))) * scale_x).round() as i32;
            let top = ((f64::from(monitor.y.saturating_sub(min_y))) * scale_y).round() as i32;
            let right = ((f64::from(monitor.x.saturating_sub(min_x)) + f64::from(monitor.width))
                * scale_x)
                .round() as i32;
            let bottom = ((f64::from(monitor.y.saturating_sub(min_y)) + f64::from(monitor.height))
                * scale_y)
                .round() as i32;
            OverlayMonitor {
                index,
                x: left,
                y: top,
                width: u32::try_from((right - left).max(1)).unwrap_or(width),
                height: u32::try_from((bottom - top).max(1)).unwrap_or(height),
            }
        })
        .collect()
}

fn selection_center_monitor(monitors: &[OverlayMonitor], selection: Rect) -> Option<usize> {
    let center = Point::new(
        selection.x.saturating_add((selection.width / 2) as i32),
        selection.y.saturating_add((selection.height / 2) as i32),
    );
    monitors
        .iter()
        .find(|monitor| monitor.contains(center))
        .or_else(|| {
            monitors
                .iter()
                .find(|monitor| monitor.rect().intersect(selection).is_some())
        })
        .map(|monitor| monitor.index)
}

fn draft_is_valid(annotation: &Annotation) -> bool {
    match &annotation.kind {
        AnnotationKind::Rectangle { rect }
        | AnnotationKind::Ellipse { rect }
        | AnnotationKind::Mosaic { rect }
        | AnnotationKind::Blur { rect } => !rect.is_empty(),
        AnnotationKind::Arrow { start, end } | AnnotationKind::Line { start, end } => start != end,
        AnnotationKind::Pencil { points } | AnnotationKind::Marker { points } => points.len() >= 2,
        _ => false,
    }
}

fn tool_hint(tool: Tool) -> &'static str {
    match tool {
        Tool::Select => "Select, move, or resize the region",
        Tool::Rectangle => "Drag a rectangle",
        Tool::Ellipse => "Drag an ellipse",
        Tool::Arrow => "Drag an arrow",
        Tool::Line => "Drag a line",
        Tool::Pencil => "Draw freehand",
        Tool::Marker => "Draw a translucent highlighter stroke",
        Tool::Text => "Click to place text",
        Tool::Number => "Click to place the next number",
        Tool::Mosaic => "Drag to pixelate",
        Tool::Blur => "Drag to blur",
        Tool::Magnifier => "Click to add a magnifier",
        Tool::Eraser => "Click an annotation to erase it",
    }
}

fn draw_polyline_preview(
    painter: &egui::Painter,
    points: &[Point],
    monitor: &OverlayMonitor,
    pixels_per_point: f32,
    stroke: Stroke,
) {
    let scale = pixels_per_point.max(f32::EPSILON);
    let points = points
        .iter()
        .map(|point| {
            egui::pos2(
                (point.x - monitor.x) as f32 / scale,
                (point.y - monitor.y) as f32 / scale,
            )
        })
        .collect::<Vec<_>>();
    if points.len() == 1 {
        painter.circle_filled(points[0], stroke.width / 2.0, stroke.color);
    } else if points.len() > 1 {
        painter.add(egui::Shape::line(points, stroke));
    }
}

fn draw_dim(ui: &mut egui::Ui, rect: egui::Rect, selection: Option<egui::Rect>) {
    let dim = Color32::from_black_alpha(120);
    let painter = ui.painter();
    let Some(sel) = selection else {
        painter.rect_filled(rect, 0.0, dim);
        return;
    };
    painter.rect_filled(
        egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), sel.top())),
        0.0,
        dim,
    );
    painter.rect_filled(
        egui::Rect::from_min_max(egui::pos2(rect.left(), sel.bottom()), rect.max),
        0.0,
        dim,
    );
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(rect.left(), sel.top()),
            egui::pos2(sel.left(), sel.bottom()),
        ),
        0.0,
        dim,
    );
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(sel.right(), sel.top()),
            egui::pos2(rect.right(), sel.bottom()),
        ),
        0.0,
        dim,
    );
}

fn draw_selection(ui: &mut egui::Ui, screen: egui::Rect, rect: egui::Rect, show_handles: bool) {
    let painter = ui.painter().with_clip_rect(screen);
    let stroke = Stroke::new(2.0, Color32::from_rgb(80, 160, 255));
    painter.rect_stroke(rect, 0.0, stroke, StrokeKind::Inside);
    if !show_handles {
        return;
    }
    for point in [
        rect.left_top(),
        rect.center_top(),
        rect.right_top(),
        rect.left_center(),
        rect.right_center(),
        rect.left_bottom(),
        rect.center_bottom(),
        rect.right_bottom(),
    ] {
        painter.circle_filled(point, 4.0, Color32::from_rgb(80, 160, 255));
        painter.circle_stroke(point, 4.0, Stroke::new(1.0, Color32::WHITE));
    }
}

fn draw_crosshair(ui: &mut egui::Ui, rect: egui::Rect, pointer: egui::Pos2) {
    let stroke = Stroke::new(1.0, Color32::from_white_alpha(180));
    ui.painter().line_segment(
        [
            egui::pos2(pointer.x, rect.top()),
            egui::pos2(pointer.x, rect.bottom()),
        ],
        stroke,
    );
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), pointer.y),
            egui::pos2(rect.right(), pointer.y),
        ],
        stroke,
    );
}

fn draw_tips(ui: &mut egui::Ui, screen: egui::Rect) {
    let lines = [
        "Drag to select · handles resize",
        "Tools: click, then draw in the region",
        "Ctrl+Z / Ctrl+Shift+Z  Undo / Redo",
        "Right-click  Finish tool",
        "Enter / Ctrl+C  Copy    Ctrl+S  Save",
        "P  Pin    Esc  Cancel    Ctrl+A  All",
        "Shift  Square    Arrows  Nudge",
    ];
    let line_height = 18.0;
    let width = 380.0;
    let height = line_height * lines.len() as f32 + 16.0;
    let rect = egui::Rect::from_min_size(
        egui::pos2(screen.left() + 16.0, screen.bottom() - height - 16.0),
        egui::vec2(width, height),
    );
    ui.painter()
        .rect_filled(rect, 6.0, Color32::from_black_alpha(190));
    for (index, line) in lines.iter().enumerate() {
        ui.painter().text(
            rect.left_top() + egui::vec2(10.0, 8.0 + index as f32 * line_height),
            egui::Align2::LEFT_TOP,
            *line,
            egui::FontId::monospace(12.0),
            Color32::from_gray(230),
        );
    }
}

fn draw_size_badge(ui: &mut egui::Ui, selection: egui::Rect) {
    let text = format!(
        "{} x {} px",
        selection.width().round() as u32,
        selection.height().round() as u32
    );
    let pos = egui::pos2(selection.left(), (selection.top() - 22.0).max(4.0));
    let galley = ui
        .painter()
        .layout_no_wrap(text, egui::FontId::monospace(13.0), Color32::WHITE);
    let bg = egui::Rect::from_min_size(
        pos - egui::vec2(4.0, 2.0),
        galley.size() + egui::vec2(8.0, 4.0),
    );
    ui.painter()
        .rect_filled(bg, 3.0, Color32::from_black_alpha(200));
    ui.painter().galley(pos, galley, Color32::WHITE);
}

#[cfg(test)]
mod tests {
    use super::{fit_monitors, selection_center_monitor};
    use cosnip_capture::MonitorInfo;
    use cosnip_core::Rect;

    fn monitor(x: i32, y: i32, width: u32, height: u32) -> MonitorInfo {
        MonitorInfo {
            x,
            y,
            width,
            height,
            primary: false,
            name: String::new(),
        }
    }

    #[test]
    fn empty_layout_falls_back_to_the_whole_frame() {
        let layout = fit_monitors(&[], 1920, 1080);
        assert_eq!(layout.len(), 1);
        assert_eq!(layout[0].x, 0);
        assert_eq!(layout[0].y, 0);
        assert_eq!(layout[0].width, 1920);
        assert_eq!(layout[0].height, 1080);
    }

    #[test]
    fn two_side_by_side_monitors_split_the_frame() {
        let layout = fit_monitors(
            &[monitor(0, 0, 1920, 1080), monitor(1920, 0, 2560, 1440)],
            4480,
            1440,
        );
        assert_eq!(layout.len(), 2);
        assert_eq!((layout[0].x, layout[0].width), (0, 1920));
        assert_eq!((layout[1].x, layout[1].width), (1920, 2560));
        assert_eq!(layout[0].height, 1080);
        assert_eq!(layout[1].height, 1440);
    }

    #[test]
    fn layout_is_normalized_when_monitors_start_below_zero() {
        let layout = fit_monitors(&[monitor(-1920, -200, 1920, 1080)], 1920, 1080);
        assert_eq!(layout[0].x, 0);
        assert_eq!(layout[0].y, 0);
        assert_eq!(layout[0].width, 1920);
        assert_eq!(layout[0].height, 1080);
    }

    #[test]
    fn captured_frame_can_be_larger_than_the_logical_layout() {
        let layout = fit_monitors(
            &[monitor(0, 0, 960, 540), monitor(960, 0, 960, 540)],
            3840,
            2160,
        );
        assert_eq!((layout[0].x, layout[0].width), (0, 1920));
        assert_eq!((layout[1].x, layout[1].width), (1920, 1920));
    }

    #[test]
    fn selection_toolbar_uses_the_monitor_holding_the_selection_center() {
        let layout = fit_monitors(
            &[monitor(0, 0, 1920, 1080), monitor(1920, 0, 1920, 1080)],
            3840,
            1080,
        );
        let selection = Rect::new(1750, 100, 400, 300);
        assert_eq!(selection_center_monitor(&layout, selection), Some(1));
    }
}
