//! The overlay and pin viewports.
//!
#![allow(deprecated)]

//! eframe is forced to the X11 backend. A 1x1 override-redirect control window keeps the event
//! loop alive; the snip overlay is a WM-managed fullscreen viewport, and pins are image-sized
//! override-redirect viewports.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cosnip_capture::{CaptureBackendKind, CapturedMonitor, MonitorInfo};
use cosnip_core::{
    Annotation, AnnotationDocument, AnnotationKind, Frame, Handle, OutputRequest, Point, Rect,
    Selection, Tool,
};
use cosnip_imaging::{crop, encode_png, render};
use cosnip_platform::{
    ClipboardOwner, atomic_write, format_file_name, global_pointer, primary_button_down, save_path,
};
use eframe::egui;
use egui::viewport::{ViewportCommand, ViewportId, X11WindowType};
use egui::{
    Color32, ColorImage, Id, Key, Order, Sense, Stroke, StrokeKind, TextureHandle, TextureOptions,
};
use winit::platform::x11::EventLoopBuilderExtX11;

use crate::icons::{self, Icon};

const PIN_ID: &str = "cosnip-pin";

#[derive(Clone, Debug)]
pub struct UiOptions {
    pub backend: CaptureBackendKind,
    pub initial_selection: Option<Rect>,
    pub output: OutputRequest,
    pub save_path: Option<PathBuf>,
    pub save_dir: Option<PathBuf>,
    pub filename: String,
    pub monitors: Vec<MonitorInfo>,
    pub native: Vec<CapturedMonitor>,
    pub style: UiStyle,
}

#[derive(Clone, Debug)]
pub struct UiStyle {
    pub annotation_color: [u8; 4],
    pub line_width: u32,
    pub font_size: u32,
    pub text_shadow: bool,
    pub cursor_mode: cosnip_platform::CursorMode,
    pub magnifier: bool,
    pub show_tips: bool,
    pub pin_backend: cosnip_platform::PinBackend,
    pub pin_zoom_step: f32,
}

impl Default for UiStyle {
    fn default() -> Self {
        Self {
            annotation_color: PALETTE[0],
            line_width: 3,
            font_size: 20,
            text_shadow: true,
            cursor_mode: cosnip_platform::CursorMode::Auto,
            magnifier: true,
            show_tips: true,
            pin_backend: cosnip_platform::PinBackend::Auto,
            pin_zoom_step: 0.002,
        }
    }
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
const COLOR_NAMES: [&str; 7] = ["Red", "Amber", "Green", "Blue", "Purple", "White", "Black"];
const TOOLBAR_BG: Color32 = Color32::from_rgb(236, 236, 236);
const TOOLBAR_INK: Color32 = Color32::from_rgb(50, 50, 50);

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
    run_native(app, outcome)
}

pub fn run_pin(frame: Frame, position: Option<(i32, i32)>, style: UiStyle) -> Outcome {
    let outcome = Arc::new(Mutex::new(Outcome {
        code: 4,
        ..Outcome::default()
    }));
    let app = CosnipApp::new_pin(frame, position, style, Arc::clone(&outcome));
    run_native(app, outcome)
}

fn run_native(app: CosnipApp, outcome: Arc<Mutex<Outcome>>) -> Outcome {
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
    save_dir: Option<PathBuf>,
    filename: String,
    native: Vec<CapturedMonitor>,
    overlay_open: bool,
    monitors: Vec<OverlayMonitor>,
    focused_monitor: Option<usize>,
    cursor_hidden: Vec<bool>,
    drag_anchor: Option<Point>,
    drag_monitor: Option<usize>,
    drag_mode: Option<DragMode>,
    annotations: AnnotationDocument,
    draft: Option<Annotation>,
    tool: Tool,
    color: [u8; 4],
    line_width: u32,
    font_size: u32,
    text_shadow: bool,
    cursor_mode: cosnip_platform::CursorMode,
    magnifier: bool,
    show_tips: bool,
    pin_backend: cosnip_platform::PinBackend,
    pin_zoom_step: f32,
    next_number: u32,
    text_editor: Option<TextEditor>,
    pin: Option<Pin>,
    pin_start: Option<(Frame, (i32, i32))>,
    clipboard: Option<ClipboardOwner>,
    outcome: Arc<Mutex<Outcome>>,
    done: bool,
    auto_confirm_at: Option<std::time::Instant>,
    pin_close_at: Option<std::time::Instant>,
    exit_at: Option<std::time::Instant>,
}

struct Pin {
    width: u32,
    height: u32,
    display_width: u32,
    display_height: u32,
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
        let style = options.style;
        Self {
            frame: Arc::new(frame),
            tiles: Vec::new(),
            selection: Selection::new(options.initial_selection),
            output: options.output,
            save_path: options.save_path,
            save_dir: options.save_dir,
            filename: options.filename,
            native: options.native,
            overlay_open: true,
            monitors,
            focused_monitor: None,
            cursor_hidden,
            drag_anchor: None,
            drag_monitor: None,
            drag_mode: None,
            annotations: AnnotationDocument::default(),
            draft: None,
            tool: Tool::Select,
            color: style.annotation_color,
            line_width: style.line_width,
            font_size: style.font_size,
            text_shadow: style.text_shadow,
            cursor_mode: style.cursor_mode,
            magnifier: style.magnifier,
            show_tips: style.show_tips,
            pin_backend: style.pin_backend,
            pin_zoom_step: style.pin_zoom_step,
            next_number: 1,
            text_editor: None,
            pin: None,
            pin_start: None,
            clipboard: None,
            outcome,
            done: false,
            auto_confirm_at: std::env::var("COSNIP_TEST_AUTOCONFIRM_MS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .map(|ms| std::time::Instant::now() + std::time::Duration::from_millis(ms)),
            pin_close_at: None,
            exit_at: None,
        }
    }

    fn new_pin(
        frame: Frame,
        position: Option<(i32, i32)>,
        style: UiStyle,
        outcome: Arc<Mutex<Outcome>>,
    ) -> Self {
        let placeholder = placeholder_frame();
        Self {
            frame: Arc::new(placeholder),
            tiles: Vec::new(),
            selection: Selection::new(None),
            output: OutputRequest {
                copy: false,
                save: None,
                pin: true,
            },
            save_path: None,
            save_dir: None,
            filename: "Cosnip_%Y-%m-%d_%H-%M-%S.png".into(),
            native: Vec::new(),
            overlay_open: false,
            monitors: Vec::new(),
            focused_monitor: None,
            cursor_hidden: Vec::new(),
            drag_anchor: None,
            drag_monitor: None,
            drag_mode: None,
            annotations: AnnotationDocument::default(),
            draft: None,
            tool: Tool::Select,
            color: style.annotation_color,
            line_width: style.line_width,
            font_size: style.font_size,
            text_shadow: style.text_shadow,
            cursor_mode: style.cursor_mode,
            magnifier: style.magnifier,
            show_tips: style.show_tips,
            pin_backend: style.pin_backend,
            pin_zoom_step: style.pin_zoom_step,
            next_number: 1,
            text_editor: None,
            pin: None,
            pin_start: Some((frame, position.unwrap_or((0, 0)))),
            clipboard: None,
            outcome,
            done: false,
            auto_confirm_at: None,
            pin_close_at: std::env::var("COSNIP_TEST_PIN_CLOSE_MS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .map(|ms| std::time::Instant::now() + std::time::Duration::from_millis(ms)),
            exit_at: None,
        }
    }

    fn with_context(mut self, cc: &eframe::CreationContext<'_>) -> Self {
        if let Some((image, position)) = self.pin_start.take() {
            let image_size = (image.width(), image.height());
            let desktop = cosnip_capture::screen_info()
                .map(|info| (info.width, info.height))
                .unwrap_or(image_size);
            let (x, y) = clamp_pin_position(position, image_size, desktop);
            let rect = Rect::new(x, y, image.width(), image.height());
            let display = (image.width(), image.height());
            if self.pin_backend != cosnip_platform::PinBackend::Window {
                let scale = cc.egui_ctx.pixels_per_point().max(f32::EPSILON);
                let (sx, sy) = pin_position(rect, scale);
                match cosnip_platform::shell_pin::pin(&image, sx, sy, 0.0, 0.0) {
                    Ok(_) => {
                        self.outcome.lock().unwrap().pinned = true;
                        self.finish(&cc.egui_ctx, 0);
                        return self;
                    }
                    Err(error) if self.pin_backend == cosnip_platform::PinBackend::Auto => {
                        eprintln!("cosnip: {error}; using a window pin");
                    }
                    Err(error) => {
                        eprintln!("cosnip: {error}");
                        self.finish(&cc.egui_ctx, 5);
                        return self;
                    }
                }
            }
            match create_pin(&cc.egui_ctx, image, rect, display) {
                Ok(pin) => {
                    self.pin = Some(pin);
                    self.outcome.lock().unwrap().pinned = true;
                }
                Err(error) => {
                    eprintln!("cosnip: pin failed: {error}");
                    self.outcome.lock().unwrap().code = 5;
                    self.done = true;
                    self.exit_at = Some(std::time::Instant::now());
                }
            }
            return self;
        }

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
        self.restore_cursor(ctx);
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
        self.tiles.clear();
        self.restore_cursor(ctx);
        for monitor in &self.monitors {
            ctx.send_viewport_cmd_to(monitor.viewport(), ViewportCommand::Close);
        }
    }

    fn restore_cursor(&mut self, ctx: &egui::Context) {
        for (index, monitor) in self.monitors.iter().enumerate() {
            ctx.send_viewport_cmd_to(monitor.viewport(), ViewportCommand::CursorVisible(true));
            if let Some(hidden) = self.cursor_hidden.get_mut(index) {
                *hidden = false;
            }
        }
    }

    fn selected_rect(&self) -> Option<Rect> {
        self.selection.rect().filter(|r| !r.is_empty())
    }

    fn rendered_selection(&self) -> Result<RenderedSelection, String> {
        let rect = self.selected_rect().ok_or("no selection")?;
        if let Some((native, scale_x, scale_y)) = self.native.iter().find_map(|native| {
            native_selection(native, rect).map(|scale| (native, scale.0, scale.1))
        }) {
            let monitor = &native.monitor;
            let origin = Point::new(monitor.x, monitor.y);
            let local_rect = native_rect(
                rect,
                monitor,
                scale_x,
                scale_y,
                (native.width, native.height),
            );
            let annotations = self
                .annotations
                .items()
                .iter()
                .map(|annotation| scale_annotation(annotation, origin, scale_x, scale_y))
                .collect::<Vec<_>>();
            let native_frame = native.frame().map_err(|error| error.to_string())?;
            let image = render(&native_frame, local_rect, &annotations)
                .map_err(|error| error.to_string())?;
            return Ok(RenderedSelection {
                image,
                rect,
                display: (rect.width, rect.height),
            });
        }
        let image =
            render(&self.frame, rect, self.annotations.items()).map_err(|e| e.to_string())?;
        Ok(RenderedSelection {
            image,
            rect,
            display: (rect.width, rect.height),
        })
    }

    /// Drop the frozen desktop CPU pixels once the output image is rendered.
    ///
    /// The GL tiles stay alive while the overlay is still visible (for example during the
    /// clipboard-owner delay) and are cleared when the overlay is closed.
    fn release_capture_frame(&mut self) {
        self.frame = Arc::new(placeholder_frame());
        self.native.clear();
        self.draft = None;
        self.annotations = AnnotationDocument::default();
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
                let now = jiff::Zoned::now();
                let name = format_file_name(&self.filename, &now);
                save_path(self.save_dir.as_deref(), &name).map_err(|e| e.to_string())?
            }
        };
        atomic_write(&path, &bytes).map_err(|e| e.to_string())?;
        self.outcome.lock().unwrap().saved = true;
        Ok(path)
    }

    fn pin_image(
        &mut self,
        ctx: &egui::Context,
        image: Frame,
        rect: Rect,
        display: (u32, u32),
    ) -> Result<(), String> {
        // The capture is no longer needed once the pin image is rendered. Free its textures
        // before uploading the pin textures so both sets are not live at the same time.
        self.tiles.clear();
        if self.pin_backend != cosnip_platform::PinBackend::Window {
            let scale = ctx.pixels_per_point().max(f32::EPSILON);
            let (x, y) = pin_position(rect, scale);
            let width = display.0 as f32 / scale;
            let height = display.1 as f32 / scale;
            match cosnip_platform::shell_pin::pin(&image, x, y, width, height) {
                Ok(_) => {
                    self.outcome.lock().unwrap().pinned = true;
                    self.finish(ctx, 0);
                    return Ok(());
                }
                Err(error) if self.pin_backend == cosnip_platform::PinBackend::Auto => {
                    eprintln!("cosnip: {error}; using a window pin");
                }
                Err(error) => return Err(error.to_string()),
            }
        }
        let pin = create_pin(ctx, image, rect, display)?;
        self.pin = Some(pin);
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

        let rendered = match self.rendered_selection() {
            Ok(result) => result,
            Err(error) => {
                eprintln!("cosnip: {error}");
                self.finish(ctx, 5);
                return;
            }
        };
        self.release_capture_frame();
        let image = rendered.image;
        let rect = rendered.rect;
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
            if let Err(error) = self.pin_image(ctx, image, rect, rendered.display) {
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

        let needs_focus = (pointer_inside && self.focused_monitor != Some(monitor.index))
            || (self.focused_monitor.is_none() && monitor.index == 0);
        if needs_focus && !focused {
            ctx.send_viewport_cmd(ViewportCommand::Focus);
            self.focused_monitor = Some(monitor.index);
        }

        let response = ui.interact(
            screen,
            Id::new(("overlay-canvas", monitor.index)),
            Sense::click_and_drag(),
        );
        let text_editing = self.text_editor.is_some();
        let over_chrome = ctx.is_pointer_over_egui();
        let cursor_on_canvas = pointer_inside && !over_chrome;
        let hide_cursor = match self.cursor_mode {
            cosnip_platform::CursorMode::Auto => {
                cursor_on_canvas && !text_editing && self.tool != Tool::Text
            }
            cosnip_platform::CursorMode::Always => false,
            cosnip_platform::CursorMode::Hidden => cursor_on_canvas && !text_editing,
        };
        if hide_cursor && !self.cursor_hidden[monitor.index] {
            ctx.send_viewport_cmd(ViewportCommand::CursorVisible(false));
            self.cursor_hidden[monitor.index] = true;
        } else if !hide_cursor && self.cursor_hidden[monitor.index] {
            ctx.send_viewport_cmd(ViewportCommand::CursorVisible(true));
            self.cursor_hidden[monitor.index] = false;
        }
        if (text_editing || (self.tool == Tool::Text && !over_chrome)) && !hide_cursor {
            ctx.set_cursor_icon(egui::CursorIcon::Text);
        }
        let show_crosshair = hide_cursor && cursor_on_canvas;
        if ctx.input(|i| i.pointer.primary_pressed())
            && response.hovered()
            && let Some(p) = pointer_global
        {
            if self.text_editor.is_some() {
                if !over_chrome {
                    self.commit_text_editor();
                }
            } else if !over_chrome && self.drag_mode.is_none() {
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
        if self.drag_mode.is_some() {
            let current = global_pointer()
                .map(|(x, y)| Point::new(x, y))
                .or(pointer_global);
            if let Some(current) = current
                && (is_drag_monitor || pointer_inside)
            {
                let square = ctx.input(|i| i.modifiers.shift);
                self.update_drag(current, square, bounds);
            }
        }
        if self.drag_mode.is_some()
            && (response.drag_stopped() || ctx.input(|i| i.pointer.any_released()))
        {
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
            && show_crosshair
            && !text_editing
        {
            draw_crosshair(ui, screen, local_pointer);
        }
        if let Some(local_pointer) = hover
            && cursor_on_canvas
            && !text_editing
            && self.magnifier
        {
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
        if self.show_tips {
            let tips_monitor = selection
                .and_then(|selection| selection_center_monitor(&self.monitors, selection))
                .or(self.focused_monitor)
                .unwrap_or(0);
            if tips_monitor == monitor.index {
                draw_tips(ui, screen);
            }
        }
        self.show_text_editor(ui, monitor, pixels_per_point);
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
            Tool::Text => {
                self.text_editor = Some(TextEditor {
                    monitor: monitor_index,
                    position: point,
                    text: String::new(),
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
                points: vec![point],
            },
            Tool::Blur => AnnotationKind::Blur {
                points: vec![point],
            },
            Tool::Eraser => AnnotationKind::Eraser {
                points: vec![point],
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
                    AnnotationKind::Rectangle { rect } | AnnotationKind::Ellipse { rect } => {
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
                    AnnotationKind::Pencil { points }
                    | AnnotationKind::Marker { points }
                    | AnnotationKind::Mosaic { points }
                    | AnnotationKind::Blur { points }
                    | AnnotationKind::Eraser { points }
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
                size: self.font_size.max(6),
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
        let color = Color32::from_rgba_unmultiplied(
            self.color[0],
            self.color[1],
            self.color[2],
            self.color[3],
        );
        let scale = pixels_per_point.max(f32::EPSILON);
        let font_size = self.font_size.max(6) as f32 / scale;
        let text_shadow = self.text_shadow;
        let Some(editor) = self.text_editor.as_mut() else {
            return;
        };
        let id = Id::new("cosnip-text-editor");
        let mut commit = false;
        let mut cancel = false;
        egui::Area::new(id)
            .fixed_pos(position)
            .order(Order::Foreground)
            .show(ui.ctx(), |ui| {
                if text_shadow && !editor.text.is_empty() {
                    ui.painter().text(
                        ui.next_widget_position() + egui::vec2(1.0, 1.0),
                        egui::Align2::LEFT_TOP,
                        &editor.text,
                        egui::FontId::proportional(font_size),
                        Color32::from_black_alpha(170),
                    );
                }
                let response = ui.add(
                    egui::TextEdit::singleline(&mut editor.text)
                        .frame(egui::Frame::NONE)
                        .margin(egui::Margin::ZERO)
                        .background_color(Color32::TRANSPARENT)
                        .text_color(color)
                        .font(egui::FontId::proportional(font_size))
                        .desired_width(220.0)
                        .hint_text("Text"),
                );
                if !response.has_focus() {
                    response.request_focus();
                }
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
        let scale = pixels_per_point.max(f32::EPSILON);
        let width_px = annotation.width.max(1) as f32;
        let width = width_px / scale;
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
                let length = (12.0 + width_px * 2.0).min(40.0) / scale;
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
                    (width_px * 4.0).max(4.0) / scale,
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
                    egui::FontId::proportional(*size as f32 / scale),
                    color,
                );
            }
            AnnotationKind::Number { position, value } => {
                let position = self.point_to_local(*position, monitor, pixels_per_point);
                let radius_px = (14.0 + width_px).clamp(12.0, 24.0);
                let radius = radius_px / scale;
                painter.circle_filled(position, radius, color);
                painter.text(
                    position,
                    egui::Align2::CENTER_CENTER,
                    value.to_string(),
                    egui::FontId::proportional((radius_px * 2.0).max(14.0) / scale),
                    Color32::WHITE,
                );
            }
            AnnotationKind::Mosaic { points } => {
                self.draw_stroke_effect_preview(
                    &painter,
                    monitor,
                    pixels_per_point,
                    points,
                    annotation.kind.brush_radius(annotation.width),
                    (annotation.width * 4).max(8),
                );
            }
            AnnotationKind::Blur { points } => {
                self.draw_stroke_effect_preview(
                    &painter,
                    monitor,
                    pixels_per_point,
                    points,
                    annotation.kind.brush_radius(annotation.width),
                    (annotation.width * 2).max(3),
                );
            }
            AnnotationKind::Eraser { points } => {
                self.draw_eraser_preview(
                    &painter,
                    monitor,
                    pixels_per_point,
                    points,
                    annotation.kind.brush_radius(annotation.width),
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

    fn draw_stroke_effect_preview(
        &self,
        painter: &egui::Painter,
        monitor: &OverlayMonitor,
        pixels_per_point: f32,
        points: &[Point],
        radius: i32,
        requested_block: u32,
    ) {
        let bounds = Rect::new(0, 0, self.frame.width(), self.frame.height());
        let Some(mask) = cosnip_core::stroke_mask(points, radius, bounds) else {
            return;
        };
        let rect = mask.rect();
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
            let height = block.min((rect.bottom() - y).max(1) as u32);
            let mut x = rect.x;
            while x < rect.right() {
                let width = block.min((rect.right() - x).max(1) as u32);
                let cell = Rect::new(x, y, width, height);
                if mask.any_in(cell) {
                    let sample = self
                        .frame
                        .pixel(x + width as i32 / 2, y + height as i32 / 2)
                        .map_or(Color32::BLACK, |p| Color32::from_rgb(p[0], p[1], p[2]));
                    let local = self.global_to_local(cell, monitor, pixels_per_point);
                    painter.rect_filled(local, 0.0, sample);
                }
                x += block as i32;
            }
            y += block as i32;
        }
    }

    fn draw_eraser_preview(
        &self,
        painter: &egui::Painter,
        monitor: &OverlayMonitor,
        pixels_per_point: f32,
        points: &[Point],
        radius: i32,
    ) {
        let bounds = Rect::new(0, 0, self.frame.width(), self.frame.height());
        let Some(mask) = cosnip_core::stroke_mask(points, radius, bounds) else {
            return;
        };
        let rect = mask.rect();
        let block = 3;
        let mut y = rect.y;
        while y < rect.bottom() {
            let height = block.min((rect.bottom() - y).max(1) as u32);
            let mut x = rect.x;
            while x < rect.right() {
                let width = block.min((rect.right() - x).max(1) as u32);
                let cell = Rect::new(x, y, width, height);
                if mask.any_in(cell) {
                    let sample = self
                        .frame
                        .pixel(x + width as i32 / 2, y + height as i32 / 2)
                        .map_or(Color32::BLACK, |p| {
                            Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3])
                        });
                    let local = self.global_to_local(cell, monitor, pixels_per_point);
                    painter.rect_filled(local, 0.0, sample);
                }
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
        let stroke_width = (width / scale).max(0.5);
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
        painter.circle_stroke(center_local, radius_local, Stroke::new(stroke_width, color));
    }

    fn handle_keys(&mut self, ctx: &egui::Context, bounds: Rect) {
        let (
            escape,
            enter,
            ctrl_c,
            ctrl_s,
            ctrl_a,
            pin_key,
            number_key,
            magnifier_key,
            undo,
            redo,
            left,
            right,
            up,
            down,
        ) = ctx.input(|i| {
            (
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::Enter),
                i.modifiers.command && i.key_pressed(Key::C),
                i.modifiers.command && i.key_pressed(Key::S),
                i.modifiers.command && i.key_pressed(Key::A),
                i.key_pressed(Key::P),
                !i.modifiers.command && i.key_pressed(Key::N),
                !i.modifiers.command && i.key_pressed(Key::G),
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
        if number_key {
            self.set_tool(Tool::Number);
        }
        if magnifier_key {
            self.set_tool(Tool::Magnifier);
        }
        let (
            rect_key,
            ellipse_key,
            arrow_key,
            pencil_key,
            marker_key,
            mosaic_key,
            text_key,
            eraser_key,
            select_key,
        ) = ctx.input(|i| {
            (
                !i.modifiers.command && i.key_pressed(Key::R),
                !i.modifiers.command && i.key_pressed(Key::C),
                !i.modifiers.command && i.key_pressed(Key::A),
                !i.modifiers.command && i.key_pressed(Key::B),
                !i.modifiers.command && i.key_pressed(Key::H),
                !i.modifiers.command && i.key_pressed(Key::M),
                !i.modifiers.command && i.key_pressed(Key::T),
                !i.modifiers.command && i.key_pressed(Key::E),
                !i.modifiers.command && i.key_pressed(Key::V),
            )
        });
        if rect_key {
            self.set_tool(Tool::Rectangle);
        }
        if ellipse_key {
            self.set_tool(Tool::Ellipse);
        }
        if arrow_key {
            self.set_tool(Tool::Arrow);
        }
        if pencil_key {
            self.set_tool(Tool::Pencil);
        }
        if marker_key {
            self.set_tool(Tool::Marker);
        }
        if mosaic_key {
            self.set_tool(Tool::Mosaic);
        }
        if text_key {
            self.set_tool(Tool::Text);
        }
        if eraser_key {
            self.set_tool(Tool::Eraser);
        }
        if select_key {
            self.set_tool(Tool::Select);
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
        let width = 414.0;
        let height = 30.0;
        let x = (local.center().x - width / 2.0).clamp(
            screen.left() + 4.0,
            (screen.right() - width - 4.0).max(screen.left() + 4.0),
        );
        let mut y = local.bottom() + 8.0;
        if y + height + 8.0 > screen.bottom() {
            y = (local.top() - height - 8.0).max(screen.top() + 4.0);
        }
        egui::Area::new(Id::new(("cosnip-toolbar", monitor.index)))
            .fixed_pos(egui::pos2(x, y))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(TOOLBAR_BG)
                    .stroke(Stroke::new(1.0, Color32::from_black_alpha(40)))
                    .inner_margin(egui::Margin::symmetric(5, 0))
                    .corner_radius(egui::CornerRadius::same(4))
                    .show(ui, |ui| {
                        ui.set_max_width(width);
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        ui.horizontal(|ui| {
                            let shapes_active =
                                matches!(self.tool, Tool::Rectangle | Tool::Ellipse | Tool::Line);
                            let response = icon_button(
                                ui,
                                Icon::Shapes,
                                shapes_active,
                                true,
                                "Shapes: rectangle → ellipse → line\nClick to cycle · right-click for style",
                            );
                            if response.clicked() {
                                self.set_tool(next_shape_tool(self.tool));
                            }
                            toolbar_style_on_right_click(
                                &response,
                                &mut self.color,
                                &mut self.line_width,
                            );

                            let arrow = icon_button(ui, Icon::Arrow, self.tool == Tool::Arrow, true, "Arrow");
                            if arrow.clicked() {
                                self.set_tool(Tool::Arrow);
                            }
                            toolbar_style_on_right_click(
                                &arrow,
                                &mut self.color,
                                &mut self.line_width,
                            );

                            let pencil = icon_button(ui, Icon::Pencil, self.tool == Tool::Pencil, true, "Pencil");
                            if pencil.clicked() {
                                self.set_tool(Tool::Pencil);
                            }
                            toolbar_style_on_right_click(
                                &pencil,
                                &mut self.color,
                                &mut self.line_width,
                            );

                            let marker = icon_button(ui, Icon::Marker, self.tool == Tool::Marker, true, "Highlighter");
                            if marker.clicked() {
                                self.set_tool(Tool::Marker);
                            }
                            toolbar_style_on_right_click(
                                &marker,
                                &mut self.color,
                                &mut self.line_width,
                            );

                            let mosaic_active = matches!(self.tool, Tool::Mosaic | Tool::Blur);
                            let mosaic = icon_button(
                                ui,
                                Icon::Mosaic,
                                mosaic_active,
                                true,
                                "Mosaic: mosaic → blur\nClick to cycle · width sets the brush size",
                            );
                            if mosaic.clicked() {
                                self.set_tool(if self.tool == Tool::Mosaic {
                                    Tool::Blur
                                } else {
                                    Tool::Mosaic
                                });
                            }
                            toolbar_style_on_right_click(
                                &mosaic,
                                &mut self.color,
                                &mut self.line_width,
                            );

                            let text = icon_button(ui, Icon::Text, self.tool == Tool::Text, true, "Text");
                            if text.clicked() {
                                self.set_tool(Tool::Text);
                            }
                            toolbar_style_on_right_click(
                                &text,
                                &mut self.color,
                                &mut self.line_width,
                            );

                            let eraser = icon_button(
                                ui,
                                Icon::Eraser,
                                self.tool == Tool::Eraser,
                                true,
                                "Eraser: drag to erase parts of annotations\nWidth sets the brush size",
                            );
                            if eraser.clicked() {
                                self.set_tool(Tool::Eraser);
                            }

                            toolbar_separator(ui);
                            let can_undo = self.annotations.can_undo();
                            if icon_button(ui, Icon::Undo, false, can_undo, "Undo (Ctrl+Z)")
                                .clicked()
                                && can_undo
                            {
                                self.annotations.undo();
                            }
                            let can_redo = self.annotations.can_redo();
                            if icon_button(ui, Icon::Redo, false, can_redo, "Redo (Ctrl+Shift+Z)")
                                .clicked()
                                && can_redo
                            {
                                self.annotations.redo();
                            }
                            toolbar_separator(ui);
                            if icon_button(ui, Icon::Cancel, false, true, "Cancel (Esc)").clicked()
                            {
                                self.finish(ui.ctx(), 6);
                            }
                            if icon_button(ui, Icon::Pin, false, true, "Pin (P)").clicked() {
                                self.confirm(ui.ctx(), Action::Pin);
                            }
                            if icon_button(ui, Icon::Save, false, true, "Save (Ctrl+S)").clicked() {
                                self.confirm(ui.ctx(), Action::Save);
                            }
                            if icon_button(ui, Icon::Copy, false, true, "Copy (Enter / Ctrl+C)")
                                .clicked()
                            {
                                self.confirm(ui.ctx(), Action::Copy);
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
        let zoom_step = self.pin_zoom_step;
        let Some(pin) = self.pin.as_mut() else { return };
        let pixels_per_point = ctx.pixels_per_point().max(f32::EPSILON);
        let base = egui::vec2(
            pin.display_width as f32 / pixels_per_point,
            pin.display_height as f32 / pixels_per_point,
        );
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
        let scale_x = pin.display_width as f32 / pin.width.max(1) as f32;
        let scale_y = pin.display_height as f32 / pin.height.max(1) as f32;
        let tiles = pin
            .tiles
            .iter()
            .map(|tile| {
                let min = egui::pos2(
                    tile.x as f32 * pin.scale * scale_x / pixels_per_point,
                    tile.y as f32 * pin.scale * scale_y / pixels_per_point,
                );
                let max = egui::pos2(
                    (tile.x + tile.width as i32) as f32 * pin.scale * scale_x / pixels_per_point,
                    (tile.y + tile.height as i32) as f32 * pin.scale * scale_y / pixels_per_point,
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
                pin.drag_start_global = Some(current);
                pin.drag_start_position = new_position;
                pin.dragging = true;
            }
            if pin.dragging {
                if let (Some(start), Some(current)) = (pin.drag_start_global, global_pointer()) {
                    new_position = (
                        pin.drag_start_position.0 + (current.0 - start.0) as f32 / pixels_per_point,
                        pin.drag_start_position.1 + (current.1 - start.1) as f32 / pixels_per_point,
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
                new_scale = (new_scale * (1.0 + scroll * zoom_step)).clamp(0.1, 5.0);
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
            if self.drag_mode.is_some() {
                if primary_button_down() {
                    ctx.request_repaint_after(std::time::Duration::from_millis(8));
                } else {
                    self.finish_drag();
                }
            }
        }
        if self.pin.is_some() {
            self.show_pin(ctx);
        }
        if let Some(at) = self.pin_close_at {
            let now = std::time::Instant::now();
            if self.pin.is_some() && now >= at {
                self.pin_close_at = None;
                if let Some(pin) = self.pin.take() {
                    ctx.send_viewport_cmd_to(pin.viewport, ViewportCommand::Close);
                }
                self.finish(ctx, 0);
            } else {
                ctx.request_repaint_after(at.saturating_duration_since(now));
            }
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

struct RenderedSelection {
    image: Frame,
    rect: Rect,
    display: (u32, u32),
}

fn native_selection(native: &CapturedMonitor, rect: Rect) -> Option<(f32, f32)> {
    let monitor = &native.monitor;
    if rect.x < monitor.x
        || rect.y < monitor.y
        || rect.right() > monitor.x.saturating_add(monitor.width as i32)
        || rect.bottom() > monitor.y.saturating_add(monitor.height as i32)
    {
        return None;
    }
    let scale_x = native.width as f32 / monitor.width as f32;
    let scale_y = native.height as f32 / monitor.height as f32;
    (scale_x > 0.0 && scale_y > 0.0).then_some((scale_x, scale_y))
}

fn native_rect(
    rect: Rect,
    monitor: &MonitorInfo,
    scale_x: f32,
    scale_y: f32,
    frame_size: (u32, u32),
) -> Rect {
    let x = ((rect.x - monitor.x) as f32 * scale_x).round() as i32;
    let y = ((rect.y - monitor.y) as f32 * scale_y).round() as i32;
    let width = (rect.width as f32 * scale_x).round().max(1.0) as u32;
    let height = (rect.height as f32 * scale_y).round().max(1.0) as u32;
    let width = width
        .min(frame_size.0.saturating_sub(x.max(0) as u32))
        .max(1);
    let height = height
        .min(frame_size.1.saturating_sub(y.max(0) as u32))
        .max(1);
    Rect::new(x, y, width, height)
}

fn scale_annotation(
    annotation: &Annotation,
    origin: Point,
    scale_x: f32,
    scale_y: f32,
) -> Annotation {
    let scale = ((scale_x + scale_y) / 2.0).max(f32::EPSILON);
    let map_point = |point: Point| {
        Point::new(
            (((point.x - origin.x) as f32) * scale_x).round() as i32,
            (((point.y - origin.y) as f32) * scale_y).round() as i32,
        )
    };
    let rect = |rect: Rect| {
        let left = (((rect.x - origin.x) as f32) * scale_x).round() as i32;
        let top = (((rect.y - origin.y) as f32) * scale_y).round() as i32;
        Rect::new(
            left,
            top,
            (rect.width as f32 * scale_x).round().max(1.0) as u32,
            (rect.height as f32 * scale_y).round().max(1.0) as u32,
        )
    };
    let kind = match &annotation.kind {
        AnnotationKind::Rectangle { rect: value } => {
            AnnotationKind::Rectangle { rect: rect(*value) }
        }
        AnnotationKind::Ellipse { rect: value } => AnnotationKind::Ellipse { rect: rect(*value) },
        AnnotationKind::Arrow { start, end } => AnnotationKind::Arrow {
            start: map_point(*start),
            end: map_point(*end),
        },
        AnnotationKind::Line { start, end } => AnnotationKind::Line {
            start: map_point(*start),
            end: map_point(*end),
        },
        AnnotationKind::Pencil { points } => AnnotationKind::Pencil {
            points: points
                .iter()
                .map(|point_value| map_point(*point_value))
                .collect(),
        },
        AnnotationKind::Marker { points } => AnnotationKind::Marker {
            points: points
                .iter()
                .map(|point_value| map_point(*point_value))
                .collect(),
        },
        AnnotationKind::Text {
            position,
            text,
            size,
        } => AnnotationKind::Text {
            position: map_point(*position),
            text: text.clone(),
            size: (*size as f32 * scale).round().max(6.0) as u32,
        },
        AnnotationKind::Number { position, value } => AnnotationKind::Number {
            position: map_point(*position),
            value: *value,
        },
        AnnotationKind::Mosaic { points } => AnnotationKind::Mosaic {
            points: points.iter().map(|value| map_point(*value)).collect(),
        },
        AnnotationKind::Blur { points } => AnnotationKind::Blur {
            points: points.iter().map(|value| map_point(*value)).collect(),
        },
        AnnotationKind::Eraser { points } => AnnotationKind::Eraser {
            points: points.iter().map(|value| map_point(*value)).collect(),
        },
        AnnotationKind::Magnifier {
            center,
            radius,
            zoom,
        } => AnnotationKind::Magnifier {
            center: map_point(*center),
            radius: (*radius as f32 * scale).round().max(8.0) as u32,
            zoom: *zoom,
        },
    };
    Annotation::new(
        kind,
        annotation.color,
        (annotation.width as f32 * scale).round().max(1.0) as u32,
    )
}

fn placeholder_frame() -> Frame {
    Frame::new(1, 1, vec![0, 0, 0, 255]).expect("1x1 frame")
}

fn create_pin(
    ctx: &egui::Context,
    image: Frame,
    rect: Rect,
    display: (u32, u32),
) -> Result<Pin, String> {
    let mut tiles = Vec::new();
    let max_side = texture_tile_size(ctx);
    for y in (0..image.height()).step_by(max_side as usize) {
        for x in (0..image.width()).step_by(max_side as usize) {
            let width = max_side.min(image.width() - x);
            let height = max_side.min(image.height() - y);
            let tile_rect = Rect::new(x as i32, y as i32, width, height);
            let tile = crop(&image, tile_rect).map_err(|error| error.to_string())?;
            let size = [tile.width() as usize, tile.height() as usize];
            let color = ColorImage::from_rgba_unmultiplied(size, tile.rgba());
            let texture = ctx.load_texture(format!("pin-{x}-{y}"), color, TextureOptions::LINEAR);
            tiles.push(PinTile {
                texture,
                x: tile_rect.x,
                y: tile_rect.y,
                width,
                height,
            });
        }
    }
    let position = pin_position(rect, ctx.pixels_per_point());
    Ok(Pin {
        width: image.width(),
        height: image.height(),
        display_width: display.0,
        display_height: display.1,
        tiles,
        position,
        scale: 1.0,
        viewport: ViewportId::from_hash_of(PIN_ID),
        drag_start_global: None,
        drag_start_position: position,
        dragging: false,
    })
}

fn clamp_pin_position(position: (i32, i32), image: (u32, u32), desktop: (u32, u32)) -> (i32, i32) {
    let max_x = desktop.0.saturating_sub(image.0) as i32;
    let max_y = desktop.1.saturating_sub(image.1) as i32;
    (position.0.clamp(0, max_x), position.1.clamp(0, max_y))
}

fn pin_position(rect: Rect, pixels_per_point: f32) -> (f32, f32) {
    let scale = pixels_per_point.max(f32::EPSILON);
    (rect.x as f32 / scale, rect.y as f32 / scale)
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
        AnnotationKind::Rectangle { rect } | AnnotationKind::Ellipse { rect } => !rect.is_empty(),
        AnnotationKind::Mosaic { points }
        | AnnotationKind::Blur { points }
        | AnnotationKind::Eraser { points } => points.len() >= 2,
        AnnotationKind::Arrow { start, end } | AnnotationKind::Line { start, end } => start != end,
        AnnotationKind::Pencil { points } | AnnotationKind::Marker { points } => points.len() >= 2,
        _ => false,
    }
}

fn icon_button(
    ui: &mut egui::Ui,
    icon: Icon,
    active: bool,
    enabled: bool,
    tooltip: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::click());
    let background = if active {
        Some(Color32::from_rgb(52, 104, 178))
    } else if enabled && response.hovered() {
        Some(Color32::from_black_alpha(22))
    } else {
        None
    };
    if let Some(background) = background {
        ui.painter().rect_filled(rect.shrink(2.0), 4.0, background);
    }
    let color = if !enabled {
        Color32::from_gray(125)
    } else if active {
        Color32::WHITE
    } else {
        TOOLBAR_INK
    };
    icons::paint(ui.painter(), rect.shrink(5.0), icon, color);
    response.on_hover_text(tooltip)
}

fn toolbar_style_on_right_click(response: &egui::Response, color: &mut [u8; 4], width: &mut u32) {
    egui::Popup::context_menu(response).show(|ui| {
        ui.label(egui::RichText::new("Color").strong());
        ui.horizontal(|ui| {
            for (swatch, name) in PALETTE.iter().zip(COLOR_NAMES) {
                if color_button(ui, *swatch, *color == *swatch, name).clicked() {
                    *color = *swatch;
                    ui.close();
                }
            }
        });
        ui.separator();
        ui.label(egui::RichText::new("Stroke width").strong());
        ui.horizontal(|ui| {
            for value in WIDTHS {
                if ui
                    .selectable_label(*width == value, format!("{value} px"))
                    .clicked()
                {
                    *width = value;
                    ui.close();
                }
            }
        });
    });
}

fn next_shape_tool(tool: Tool) -> Tool {
    match tool {
        Tool::Rectangle => Tool::Ellipse,
        Tool::Ellipse => Tool::Line,
        _ => Tool::Rectangle,
    }
}

fn color_button(ui: &mut egui::Ui, color: [u8; 4], selected: bool, name: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), Sense::click());
    ui.painter().rect_filled(
        rect,
        4.0,
        Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]),
    );
    let border = if selected {
        Color32::from_rgb(52, 104, 178)
    } else if response.hovered() {
        Color32::from_black_alpha(120)
    } else {
        Color32::from_black_alpha(55)
    };
    ui.painter().rect_stroke(
        rect.expand(0.5),
        4.0,
        Stroke::new(if selected { 1.5 } else { 1.0 }, border),
        StrokeKind::Outside,
    );
    response.on_hover_text(name)
}

fn toolbar_separator(ui: &mut egui::Ui) {
    ui.add_space(3.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 18.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, 0.0, Color32::from_black_alpha(45));
    ui.add_space(3.0);
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
    const ROWS: [(&str, &str); 8] = [
        ("Shift", "Keep a square"),
        ("Enter / Ctrl+C", "Copy"),
        ("Ctrl+S", "Save"),
        ("P", "Pin"),
        ("N / G", "Number / Magnifier"),
        ("Ctrl+Z", "Undo"),
        ("Ctrl+A", "Select all"),
        ("Esc", "Cancel"),
    ];
    let painter = ui.painter();
    let key_font = egui::FontId::monospace(11.0);
    let text_font = egui::FontId::proportional(11.0);
    let key_pad = egui::vec2(5.0, 1.0);
    let row_height = 18.0;
    let gap = 8.0;
    let padding = 8.0;
    let galleys = ROWS
        .iter()
        .map(|(key, text)| {
            (
                painter.layout_no_wrap((*key).to_owned(), key_font.clone(), Color32::WHITE),
                painter.layout_no_wrap(
                    (*text).to_owned(),
                    text_font.clone(),
                    Color32::from_white_alpha(220),
                ),
            )
        })
        .collect::<Vec<_>>();
    let key_width = galleys
        .iter()
        .map(|(key, _)| key.size().x + key_pad.x * 2.0)
        .fold(0.0, f32::max);
    let text_width = galleys
        .iter()
        .map(|(_, text)| text.size().x)
        .fold(0.0, f32::max);
    let width = padding * 2.0 + key_width + gap + text_width;
    let height = padding * 2.0 + row_height * ROWS.len() as f32 - 3.0;
    let origin = egui::pos2(screen.left() + 12.0, screen.bottom() - 12.0 - height);
    painter.rect_filled(
        egui::Rect::from_min_size(origin, egui::vec2(width, height)),
        6.0,
        Color32::from_black_alpha(200),
    );
    painter.rect_stroke(
        egui::Rect::from_min_size(origin, egui::vec2(width, height)),
        6.0,
        Stroke::new(1.0, Color32::from_white_alpha(22)),
        StrokeKind::Inside,
    );
    for (index, (key, text)) in galleys.iter().enumerate() {
        let row_top = origin.y + padding + index as f32 * row_height;
        let key_rect = egui::Rect::from_min_size(
            egui::pos2(origin.x + padding, row_top - 1.0),
            egui::vec2(key_width, row_height - 2.0),
        );
        painter.rect_filled(key_rect, 3.0, Color32::from_white_alpha(28));
        painter.galley(
            egui::pos2(key_rect.left() + key_pad.x, key_rect.top() + 2.0),
            key.clone(),
            Color32::WHITE,
        );
        painter.galley(
            egui::pos2(key_rect.right() + gap, row_top + 2.0),
            text.clone(),
            Color32::from_white_alpha(220),
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
    let galley =
        ui.painter()
            .layout_no_wrap(text, egui::FontId::monospace(12.0), Color32::from_gray(25));
    let bg = egui::Rect::from_min_size(
        pos - egui::vec2(4.0, 2.0),
        galley.size() + egui::vec2(8.0, 4.0),
    );
    ui.painter()
        .rect_filled(bg, 3.0, Color32::from_rgb(228, 228, 228));
    ui.painter().rect_stroke(
        bg,
        3.0,
        Stroke::new(1.0, Color32::from_black_alpha(45)),
        StrokeKind::Inside,
    );
    ui.painter().galley(pos, galley, Color32::from_gray(25));
}

#[cfg(test)]
mod tests {
    use super::{
        fit_monitors, native_selection, pin_position, scale_annotation, selection_center_monitor,
    };
    use cosnip_capture::{CapturedMonitor, MonitorInfo};
    use cosnip_core::{Annotation, AnnotationKind, Frame, Point, Rect};

    #[test]
    fn native_selection_maps_monitor_pixels() {
        let monitor = monitor(500, 500, 800, 600);
        let native = CapturedMonitor::new(
            monitor.clone(),
            &Frame::new(500, 375, vec![0; 500 * 375 * 4]).unwrap(),
        )
        .unwrap();
        assert_eq!(
            native_selection(&native, Rect::new(500, 500, 800, 600)),
            Some((0.625, 0.625))
        );
        assert_eq!(
            native_selection(&native, Rect::new(400, 500, 800, 600)),
            None
        );
    }

    #[test]
    fn native_annotations_scale_with_the_monitor() {
        let annotation = Annotation::new(
            AnnotationKind::Text {
                position: Point::new(600, 600),
                text: "hi".into(),
                size: 20,
            },
            [255, 0, 0, 255],
            4,
        );
        let scaled = scale_annotation(&annotation, Point::new(500, 500), 0.625, 0.625);
        assert_eq!(
            scaled.kind,
            AnnotationKind::Text {
                position: Point::new(63, 63),
                text: "hi".into(),
                size: 13,
            }
        );
        assert_eq!(scaled.width, 3);
    }

    #[test]
    fn pin_position_converts_frame_pixels_to_points() {
        assert_eq!(
            pin_position(Rect::new(4000, 1000, 300, 200), 2.0),
            (2000.0, 500.0)
        );
        assert_eq!(
            pin_position(Rect::new(300, 200, 50, 50), 1.0),
            (300.0, 200.0)
        );
    }

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
