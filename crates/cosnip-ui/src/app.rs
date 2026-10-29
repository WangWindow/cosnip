//! The overlay and pin viewports.
//!
#![allow(deprecated)]

//! eframe is forced to the X11 backend. A 1x1 override-redirect control window keeps the event
//! loop alive; the snip overlay is a WM-managed fullscreen viewport, and pins are image-sized
//! override-redirect viewports.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cosnip_capture::CaptureBackendKind;
use cosnip_core::{Frame, OutputRequest, Point, Rect, Selection};
use cosnip_imaging::{crop, encode_png};
use cosnip_platform::{
    ClipboardOwner, atomic_write, default_save_path, global_pointer, primary_button_down,
};
use eframe::egui;
use egui::viewport::{ViewportCommand, ViewportId, X11WindowType};
use egui::{
    Color32, ColorImage, Id, Key, Order, Sense, Stroke, StrokeKind, TextureHandle, TextureOptions,
};
use winit::platform::x11::EventLoopBuilderExtX11;

const OVERLAY_ID: &str = "cosnip-overlay";
const PIN_ID: &str = "cosnip-pin";

#[derive(Clone, Debug)]
pub struct UiOptions {
    pub backend: CaptureBackendKind,
    pub initial_selection: Option<Rect>,
    pub output: OutputRequest,
    pub save_path: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    All,
    Copy,
    Save,
    Pin,
}

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
    texture: Option<TextureHandle>,
    selection: Selection,
    output: OutputRequest,
    save_path: Option<PathBuf>,
    overlay_open: bool,
    overlay_focused: bool,
    drag_anchor: Option<Point>,
    pointer: Option<Point>,
    pin: Option<Pin>,
    clipboard: Option<ClipboardOwner>,
    outcome: Arc<Mutex<Outcome>>,
    done: bool,
    auto_confirm_at: Option<std::time::Instant>,
    exit_at: Option<std::time::Instant>,
}

struct Pin {
    frame: Frame,
    texture: TextureHandle,
    position: (f32, f32),
    scale: f32,
    viewport: ViewportId,
    drag_start_global: Option<(i32, i32)>,
    drag_start_position: (f32, f32),
    dragging: bool,
}

impl CosnipApp {
    fn new(frame: Frame, options: UiOptions, outcome: Arc<Mutex<Outcome>>) -> Self {
        Self {
            frame: Arc::new(frame),
            texture: None,
            selection: Selection::new(options.initial_selection),
            output: options.output,
            save_path: options.save_path,
            overlay_open: true,
            overlay_focused: false,
            drag_anchor: None,
            pointer: None,
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
        let size = [self.frame.width() as usize, self.frame.height() as usize];
        let image = ColorImage::from_rgba_unmultiplied(size, self.frame.rgba());
        self.texture = Some(
            cc.egui_ctx
                .load_texture("capture", image, TextureOptions::LINEAR),
        );
        self
    }

    fn finish(&mut self, ctx: &egui::Context, code: i32) {
        self.finish_delayed(ctx, code, std::time::Duration::ZERO);
    }

    fn finish_delayed(&mut self, ctx: &egui::Context, code: i32, delay: std::time::Duration) {
        self.done = true;
        self.overlay_open = false;
        self.outcome.lock().unwrap().code = code;
        ctx.send_viewport_cmd_to(ViewportId::from_hash_of(OVERLAY_ID), ViewportCommand::Close);
        self.exit_at = Some(std::time::Instant::now() + delay);
        if !delay.is_zero() {
            ctx.request_repaint_after(delay);
        }
    }

    fn close_overlay(&mut self, ctx: &egui::Context) {
        self.overlay_open = false;
        ctx.send_viewport_cmd_to(ViewportId::from_hash_of(OVERLAY_ID), ViewportCommand::Close);
    }

    fn selected_rect(&self) -> Option<Rect> {
        self.selection.rect().filter(|r| !r.is_empty())
    }

    fn crop_selection(&mut self) -> Result<Frame, String> {
        let rect = self.selected_rect().ok_or("no selection")?;
        crop(&self.frame, rect).map_err(|e| e.to_string())
    }

    fn copy(&mut self) -> Result<(), String> {
        let image = self.crop_selection()?;
        let mut owner = ClipboardOwner::new().map_err(|e| e.to_string())?;
        owner.set_image(&image).map_err(|e| e.to_string())?;
        self.clipboard = Some(owner);
        self.outcome.lock().unwrap().copied = true;
        eprintln!("cosnip: copied selection");
        Ok(())
    }

    fn save(&mut self, explicit: Option<PathBuf>) -> Result<PathBuf, String> {
        let image = self.crop_selection()?;
        let bytes = encode_png(&image).map_err(|e| e.to_string())?;
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

    fn pin(&mut self, ctx: &egui::Context) -> Result<(), String> {
        let rect = self.selected_rect().ok_or("no selection")?;
        let image = crop(&self.frame, rect).map_err(|e| e.to_string())?;
        let size = [image.width() as usize, image.height() as usize];
        let color = ColorImage::from_rgba_unmultiplied(size, image.rgba());
        let texture = ctx.load_texture("pin", color, TextureOptions::LINEAR);
        self.pin = Some(Pin {
            frame: image,
            texture,
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

        if let Err(error) = self.crop_selection() {
            eprintln!("cosnip: {error}");
            self.finish(ctx, 5);
            return;
        }
        if want_copy && let Err(error) = self.copy() {
            eprintln!("cosnip: clipboard failed: {error}");
            self.finish(ctx, 5);
            return;
        }
        if want_save {
            let path = self.save_path.clone();
            match self.save(path) {
                Ok(path) => eprintln!("cosnip: saved {}", path.display()),
                Err(error) => {
                    eprintln!("cosnip: save failed: {error}");
                    self.finish(ctx, 5);
                    return;
                }
            }
        }
        if want_pin {
            if let Err(error) = self.pin(ctx) {
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
        let mut builder = egui::ViewportBuilder::default()
            .with_title("Cosnip Snipper")
            .with_decorations(false)
            .with_always_on_top()
            .with_has_shadow(false);
        if std::env::var_os("COSNIP_TEST_WINDOWED_OVERLAY").is_some() {
            builder = builder
                .with_position([0.0, 0.0])
                .with_inner_size([self.frame.width() as f32, self.frame.height() as f32]);
        } else {
            builder = builder.with_fullscreen(true).with_monitor(0);
        }
        ctx.show_viewport_immediate(
            ViewportId::from_hash_of(OVERLAY_ID),
            builder,
            |ui, _class| {
                let ctx = ui.ctx().clone();
                if !self.overlay_focused {
                    self.overlay_focused = ctx.input(|i| i.viewport().focused.unwrap_or(false));
                    ctx.send_viewport_cmd(ViewportCommand::Focus);
                    ctx.send_viewport_cmd(ViewportCommand::CursorVisible(false));
                }
                let rect = ui.max_rect();
                let bounds = Rect::new(0, 0, self.frame.width(), self.frame.height());
                let pointer = ctx.input(|i| i.pointer.hover_pos());
                let pointer_px =
                    pointer.map(|p| Point::new(p.x.round() as i32, p.y.round() as i32));
                self.pointer = pointer_px;

                let response =
                    ui.interact(rect, Id::new("overlay-canvas"), Sense::click_and_drag());
                if ctx.input(|i| i.pointer.primary_pressed())
                    && response.hovered()
                    && let Some(p) = pointer_px
                {
                    self.drag_anchor = Some(p);
                    self.selection.begin(p);
                }
                if response.drag_started()
                    && self.drag_anchor.is_none()
                    && let Some(p) = pointer_px
                {
                    self.drag_anchor = Some(p);
                    self.selection.begin(p);
                }
                if let (Some(anchor), Some(current)) = (self.drag_anchor, pointer_px)
                    && (response.dragged() || ctx.input(|i| i.pointer.primary_down()))
                {
                    let square = ctx.input(|i| i.modifiers.shift);
                    self.selection.drag_to(anchor, current, bounds, square);
                }
                if response.drag_stopped() || ctx.input(|i| i.pointer.any_released()) {
                    self.drag_anchor = None;
                }

                // Draw the frozen frame.
                if let Some(texture) = &self.texture {
                    ui.painter().image(
                        texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                let selection = self.selected_rect();
                draw_dim(ui, rect, selection);
                if let Some(selection) = selection {
                    draw_selection(ui, selection);
                }
                if let Some(p) = pointer {
                    draw_crosshair(ui, rect, p);
                    self.draw_magnifier(ui, rect, p);
                }
                if let Some(selection) = selection {
                    draw_size_badge(ui, selection);
                    self.show_toolbar(&ctx, selection);
                }
                draw_tips(ui, rect);
                self.handle_keys(&ctx, bounds);
            },
        );
    }

    fn handle_keys(&mut self, ctx: &egui::Context, bounds: Rect) {
        let (escape, enter, ctrl_c, ctrl_s, ctrl_a, pin_key, left, right, up, down) =
            ctx.input(|i| {
                (
                    i.key_pressed(Key::Escape),
                    i.key_pressed(Key::Enter),
                    i.modifiers.command && i.key_pressed(Key::C),
                    i.modifiers.command && i.key_pressed(Key::S),
                    i.modifiers.command && i.key_pressed(Key::A),
                    i.modifiers.command && i.key_pressed(Key::P),
                    i.key_pressed(Key::ArrowLeft),
                    i.key_pressed(Key::ArrowRight),
                    i.key_pressed(Key::ArrowUp),
                    i.key_pressed(Key::ArrowDown),
                )
            });
        if escape {
            self.finish(ctx, 6);
            return;
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

    fn show_toolbar(&mut self, ctx: &egui::Context, selection: Rect) {
        let position = egui::pos2(selection.x as f32, selection.bottom() as f32 + 8.0);
        egui::Area::new(Id::new("cosnip-toolbar"))
            .fixed_pos(position)
            .order(Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(Color32::from_rgb(28, 28, 32))
                    .inner_margin(egui::Margin::same(6))
                    .corner_radius(egui::CornerRadius::same(6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
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

    fn draw_magnifier(&self, ui: &mut egui::Ui, rect: egui::Rect, pointer: egui::Pos2) {
        let center = Point::new(pointer.x.round() as i32, pointer.y.round() as i32);
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
        let texture = pin.texture.id();
        let base_size = base;
        let mut new_position = pin.position;
        let mut new_scale = pin.scale;
        let mut close = false;
        ctx.show_viewport_immediate(viewport, builder, |ui, _class| {
            let ctx = ui.ctx().clone();
            let rect = ui.max_rect();
            ui.painter().image(
                texture,
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
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

fn draw_dim(ui: &mut egui::Ui, rect: egui::Rect, selection: Option<Rect>) {
    let dim = Color32::from_black_alpha(120);
    let painter = ui.painter();
    let Some(selection) = selection else {
        painter.rect_filled(rect, 0.0, dim);
        return;
    };
    let sel = egui::Rect::from_min_size(
        egui::pos2(selection.x as f32, selection.y as f32),
        egui::vec2(selection.width as f32, selection.height as f32),
    );
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

fn draw_selection(ui: &mut egui::Ui, selection: Rect) {
    let rect = egui::Rect::from_min_size(
        egui::pos2(selection.x as f32, selection.y as f32),
        egui::vec2(selection.width as f32, selection.height as f32),
    );
    let stroke = Stroke::new(2.0, Color32::from_rgb(80, 160, 255));
    ui.painter()
        .rect_stroke(rect, 0.0, stroke, StrokeKind::Inside);
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
        ui.painter()
            .circle_filled(point, 4.0, Color32::from_rgb(80, 160, 255));
        ui.painter()
            .circle_stroke(point, 4.0, Stroke::new(1.0, Color32::WHITE));
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
        "Drag to select",
        "Enter / Ctrl+C  Copy",
        "Ctrl+S  Save",
        "P  Pin",
        "Esc  Cancel",
        "Arrows  Nudge    Ctrl+A  All",
        "Shift  Square",
    ];
    let line_height = 18.0;
    let width = 260.0;
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

fn draw_size_badge(ui: &mut egui::Ui, selection: Rect) {
    let text = format!("{} x {} px", selection.width, selection.height);
    let pos = egui::pos2(selection.x as f32, (selection.y as f32 - 22.0).max(4.0));
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
