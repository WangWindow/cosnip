//! The TOML-backed settings window.
//!
//! The layout follows the reference preferences dialog: a top tab row, a
//! bordered content page, optional sub-tabs, and a footer with help and
//! restore-defaults actions. Changes save automatically after a short debounce.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use cosnip_platform::{
    Config, CursorMode, DefaultOutput, PinBackend, autostart_enabled, clipboard_available,
    config_dir, install_autostart, pictures_dir, remove_autostart, shell_pin, tray_host_available,
};
use eframe::egui;
use egui::viewport::{ViewportCommand, ViewportId};
use egui::{Color32, Key, RichText, Stroke};
use winit::platform::x11::EventLoopBuilderExtX11;

const BORDER: Color32 = Color32::from_gray(155);
const GROUP_BORDER: Color32 = Color32::from_gray(198);
const WINDOW_BG: Color32 = Color32::from_gray(246);
const TAB_BG: Color32 = Color32::from_gray(238);
const SUBTAB_BG: Color32 = Color32::from_gray(244);

#[derive(Clone, Copy, Debug, Default)]
pub struct SettingsOutcome {
    pub code: i32,
}

pub fn run_settings(path: PathBuf) -> SettingsOutcome {
    let app = SettingsApp::new(path);
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Cosnip")
            .with_inner_size([1.0, 1.0])
            .with_position([0.0, 0.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_override_redirect(true)
            .with_window_type(egui::viewport::X11WindowType::Utility)
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
        Box::new(move |_cc| Ok(Box::new(app))),
    );
    match result {
        Ok(()) => SettingsOutcome::default(),
        Err(error) => {
            eprintln!("cosnip: settings UI failed: {error}");
            SettingsOutcome { code: 4 }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Section {
    #[default]
    General,
    Interface,
    Snip,
    Pin,
    Output,
    Controls,
    About,
}

impl Section {
    const ALL: [Self; 7] = [
        Self::General,
        Self::Interface,
        Self::Snip,
        Self::Pin,
        Self::Output,
        Self::Controls,
        Self::About,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Interface => "Interface",
            Self::Snip => "Snip",
            Self::Pin => "Pin",
            Self::Output => "Output",
            Self::Controls => "Controls",
            Self::About => "About",
        }
    }

    fn subtabs(self) -> &'static [&'static str] {
        match self {
            Self::Snip => &["Display", "Behavior", "Keys"],
            Self::Pin => &["Display", "Behavior"],
            Self::Output => &["File", "Clipboard"],
            Self::Controls => &["Global Hotkeys", "Mouse"],
            Self::General | Self::Interface | Self::About => &[],
        }
    }
}

struct SettingsApp {
    path: PathBuf,
    config: Config,
    saved: Config,
    section: Section,
    subtab: usize,
    status: String,
    dirty_since: Option<Instant>,
    folder_pick: Option<Receiver<Option<PathBuf>>>,
    tray_available: bool,
    clipboard_available: bool,
    shell_pins_available: bool,
    done: bool,
    viewport: ViewportId,
}

impl SettingsApp {
    fn new(path: PathBuf) -> Self {
        let (config, status) = match Config::load(&path) {
            Ok(config) => (config, String::new()),
            Err(error) => (Config::default(), format!("{error}; showing defaults")),
        };
        Self {
            path,
            saved: config.clone(),
            config,
            section: Section::default(),
            subtab: 0,
            status,
            dirty_since: None,
            folder_pick: None,
            tray_available: tray_host_available(),
            clipboard_available: clipboard_available(),
            shell_pins_available: shell_pin::available(),
            done: false,
            viewport: ViewportId::from_hash_of("cosnip-settings"),
        }
    }

    fn save(&mut self) {
        self.config = self.config.clone().normalized();
        match self.config.save(&self.path) {
            Ok(()) => {
                self.saved = self.config.clone();
                self.status.clear();
            }
            Err(error) => {
                // Do not retry every frame after a hard failure; the next edit will retry.
                self.saved = self.config.clone();
                self.status = format!("Save failed: {error}");
            }
        }
    }

    fn autosave(&mut self, ctx: &egui::Context) {
        if self.config == self.saved {
            self.dirty_since = None;
            return;
        }
        let now = Instant::now();
        let since = *self.dirty_since.get_or_insert(now);
        if now.duration_since(since) >= Duration::from_millis(350) {
            self.save();
            self.dirty_since = None;
        } else {
            ctx.request_repaint_after(Duration::from_millis(80));
        }
    }

    fn pick_folder(&mut self) {
        let start = self
            .config
            .save_dir()
            .or_else(|| pictures_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let picked = rfd::FileDialog::new().set_directory(start).pick_folder();
            let _ = tx.send(picked);
        });
        self.folder_pick = Some(rx);
    }

    fn poll_folder(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.folder_pick else {
            return;
        };
        match rx.try_recv() {
            Ok(Some(path)) => {
                self.config.general.save_dir = path.display().to_string();
                self.folder_pick = None;
            }
            Ok(None) => self.folder_pick = None,
            Err(mpsc::TryRecvError::Disconnected) => self.folder_pick = None,
            Err(mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }
    }

    fn open_config_dir(&mut self) {
        let dir = config_dir();
        if let Err(error) = std::fs::create_dir_all(&dir) {
            self.status = format!("Cannot create {}: {error}", dir.display());
            return;
        }
        self.open_path(&dir);
    }

    fn open_config_file(&mut self) {
        let path = self.path.clone();
        self.open_path(&path);
    }

    fn open_path(&mut self, path: &std::path::Path) {
        match std::process::Command::new("xdg-open").arg(path).spawn() {
            Ok(_) => self.status = format!("Opened {}", path.display()),
            Err(error) => self.status = format!("Cannot open {}: {error}", path.display()),
        }
    }

    fn open_keyboard_settings(&mut self) {
        match std::process::Command::new("gnome-control-center")
            .arg("keyboard")
            .spawn()
        {
            Ok(_) => self.status = "Opened GNOME Keyboard settings".into(),
            Err(error) => {
                self.status = format!(
                    "Cannot open GNOME settings ({error}); open Keyboard → Custom Shortcuts manually"
                );
            }
        }
    }

    fn exe_command(&self, subcommand: &str) -> String {
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("cosnip"));
        format!("\"{}\" {subcommand}", exe.display())
    }

    fn select_section(&mut self, section: Section) {
        if self.section != section {
            self.section = section;
            self.subtab = 0;
        }
    }

    fn restore_defaults(&mut self) {
        let defaults = Config::default();
        match self.section {
            Section::General => {
                let _ = remove_autostart();
            }
            Section::Interface => {
                self.config.overlay.show_tips = defaults.overlay.show_tips;
                self.config.annotations = defaults.annotations;
            }
            Section::Snip => {
                self.config.overlay.cursor_mode = defaults.overlay.cursor_mode;
                self.config.overlay.magnifier = defaults.overlay.magnifier;
                self.config.overlay.show_tips = defaults.overlay.show_tips;
                self.config.general.delay_ms = defaults.general.delay_ms;
                self.config.general.native_output = defaults.general.native_output;
            }
            Section::Pin => {
                self.config.pin = defaults.pin;
            }
            Section::Output => {
                self.config.general.default_output = defaults.general.default_output;
                self.config.general.save_dir = defaults.general.save_dir;
                self.config.general.filename = defaults.general.filename;
                self.config.general.native_output = defaults.general.native_output;
            }
            Section::Controls | Section::About => {}
        }
        self.status = "Restored defaults for this page".into();
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        let mut visuals = egui::Visuals::light();
        visuals.panel_fill = WINDOW_BG;
        visuals.window_fill = Color32::WHITE;
        visuals.extreme_bg_color = Color32::WHITE;
        visuals.faint_bg_color = Color32::from_gray(245);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::from_gray(38));
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_gray(38));
        visuals.widgets.inactive.bg_fill = Color32::from_gray(242);
        visuals.widgets.inactive.weak_bg_fill = Color32::from_gray(242);
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_gray(178));
        visuals.widgets.hovered.bg_fill = Color32::from_gray(230);
        visuals.widgets.hovered.weak_bg_fill = Color32::from_gray(230);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_gray(120));
        visuals.widgets.active.bg_fill = Color32::from_gray(215);
        visuals.widgets.active.weak_bg_fill = Color32::from_gray(215);
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, Color32::from_gray(110));
        ui.ctx().set_visuals(visuals);
        egui::Frame::NONE
            .fill(WINDOW_BG)
            .inner_margin(egui::Margin::same(0))
            .show(ui, |ui| {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    self.tab_bar(ui);
                });
                ui.add_space(2.0);

                let content_height = (ui.available_height() - 80.0).max(220.0);
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 12.0).max(320.0));
                        ui.set_height(content_height);
                        egui::Frame::NONE
                            .fill(Color32::WHITE)
                            .stroke(Stroke::new(1.0, BORDER))
                            .inner_margin(egui::Margin::same(12))
                            .show(ui, |ui| {
                                self.page(ui, content_height);
                            });
                        self.footer(ui);
                    });
                });
            });
    }

    fn tab_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            for section in Section::ALL {
                if tab_button(ui, self.section == section, section.label()).clicked() {
                    self.select_section(section);
                }
            }
        });
    }

    fn subtab_bar(&mut self, ui: &mut egui::Ui) -> bool {
        let labels = self.section.subtabs();
        if labels.is_empty() {
            self.subtab = 0;
            return false;
        }
        self.subtab = self.subtab.min(labels.len() - 1);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            for (index, label) in labels.iter().enumerate() {
                if sub_tab_button(ui, self.subtab == index, label).clicked() {
                    self.subtab = index;
                }
            }
        });
        ui.add_space(10.0);
        true
    }

    fn page(&mut self, ui: &mut egui::Ui, content_height: f32) {
        let has_subtabs = self.subtab_bar(ui);
        let scroll_height = content_height - if has_subtabs { 42.0 } else { 24.0 } - 24.0;
        egui::ScrollArea::vertical()
            .id_salt("cosnip-settings-scroll")
            .auto_shrink([false, false])
            .max_height(scroll_height.max(160.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                match self.section {
                    Section::General => self.general_page(ui),
                    Section::Interface => self.interface_page(ui),
                    Section::Snip => self.snip_page(ui),
                    Section::Pin => self.pin_page(ui),
                    Section::Output => self.output_page(ui),
                    Section::Controls => self.controls_page(ui),
                    Section::About => self.about_page(ui),
                }
                ui.add_space(4.0);
            });
    }

    fn footer(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let help = egui::Button::new(RichText::new("?").strong())
                .min_size(egui::vec2(24.0, 24.0))
                .corner_radius(egui::CornerRadius::same(12))
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0, BORDER));
            if ui
                .add(help)
                .on_hover_text("Changes are saved automatically. Hover over options for details.")
                .clicked()
            {
                self.status = "Changes save automatically 350 ms after the last edit.".to_string();
            }
            if !self.status.is_empty() {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(&self.status)
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(egui::Button::new("Restore Defaults").min_size(egui::vec2(132.0, 26.0)))
                    .clicked()
                {
                    self.restore_defaults();
                }
            });
        });
    }

    fn general_page(&mut self, ui: &mut egui::Ui) {
        group(ui, "Startup", |ui| {
            let mut enabled = autostart_enabled();
            if ui
                .checkbox(&mut enabled, "Run the tray at login")
                .on_hover_text("Writes an XDG autostart desktop entry")
                .changed()
            {
                let result = if enabled {
                    install_autostart(Some(&self.path))
                        .map(|path| format!("Autostart enabled: {}", path.display()))
                } else {
                    remove_autostart().map(|()| "Autostart disabled".to_string())
                };
                self.status = match result {
                    Ok(message) => message,
                    Err(error) => error,
                };
            }
        });

        group(ui, "Tray", |ui| {
            let (color, text) = if self.tray_available {
                (
                    Color32::from_rgb(60, 165, 95),
                    "StatusNotifierItem host detected",
                )
            } else {
                (
                    Color32::from_rgb(180, 145, 55),
                    "No StatusNotifierItem host; the icon stays hidden",
                )
            };
            ui.horizontal(|ui| {
                ui.colored_label(color, "●");
                ui.label(text);
            });
            ui.label(
                RichText::new(
                    "On GNOME, enable the AppIndicator/KStatusNotifier extension for the icon.",
                )
                .small()
                .color(ui.visuals().weak_text_color()),
            );
        });

        group(ui, "Configuration Storage", |ui| {
            egui::Grid::new("config-storage")
                .num_columns(2)
                .min_col_width(130.0)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Path:");
                    let mut path = self.path.display().to_string();
                    ui.add(
                        egui::TextEdit::singleline(&mut path)
                            .desired_width(ui.available_width().min(420.0))
                            .interactive(false),
                    );
                    ui.end_row();
                });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Open containing folder").clicked() {
                    self.open_config_dir();
                }
                if ui.button("Open config file").clicked() {
                    self.open_config_file();
                }
            });
        });
    }

    fn interface_page(&mut self, ui: &mut egui::Ui) {
        group(ui, "Annotation Defaults", |ui| {
            egui::Grid::new("annotation-grid")
                .num_columns(2)
                .min_col_width(140.0)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Color:");
                    ui.horizontal(|ui| {
                        let mut color = self.config.annotations.color_rgba();
                        if ui
                            .color_edit_button_srgba_unmultiplied(&mut color)
                            .changed()
                        {
                            self.config.annotations.color = format!(
                                "#{:02X}{:02X}{:02X}{:02X}",
                                color[0], color[1], color[2], color[3]
                            );
                        }
                        ui.add(
                            egui::TextEdit::singleline(&mut self.config.annotations.color)
                                .desired_width(96.0),
                        );
                        let preview = self.config.annotations.color_rgba();
                        ui.colored_label(
                            Color32::from_rgba_unmultiplied(
                                preview[0], preview[1], preview[2], preview[3],
                            ),
                            "Aa",
                        );
                    });
                    ui.end_row();

                    ui.label("Line width:");
                    ui.add(
                        egui::Slider::new(&mut self.config.annotations.line_width, 1..=32)
                            .suffix(" px"),
                    );
                    ui.end_row();

                    ui.label("Font size:");
                    ui.add(
                        egui::Slider::new(&mut self.config.annotations.font_size, 6..=96)
                            .suffix(" px"),
                    );
                    ui.end_row();

                    ui.label("Text shadow:");
                    ui.checkbox(
                        &mut self.config.annotations.text_shadow,
                        "Shadow while editing",
                    );
                    ui.end_row();
                });
        });

        group(ui, "Overlay Chrome", |ui| {
            ui.checkbox(
                &mut self.config.overlay.show_tips,
                "Show the keyboard tips panel",
            );
        });
    }

    fn snip_page(&mut self, ui: &mut egui::Ui) {
        match self.subtab {
            0 => {
                group(ui, "Cursor", |ui| {
                    egui::Grid::new("snip-cursor")
                        .num_columns(2)
                        .min_col_width(140.0)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Cursor:");
                            egui::ComboBox::from_id_salt("cursor-mode")
                                .width(180.0)
                                .selected_text(cursor_mode_label(self.config.overlay.cursor_mode))
                                .show_ui(ui, |ui| {
                                    for mode in
                                        [CursorMode::Auto, CursorMode::Always, CursorMode::Hidden]
                                    {
                                        ui.selectable_value(
                                            &mut self.config.overlay.cursor_mode,
                                            mode,
                                            cursor_mode_label(mode),
                                        );
                                    }
                                });
                            ui.end_row();
                        });
                });
                group(ui, "Magnifier", |ui| {
                    ui.checkbox(
                        &mut self.config.overlay.magnifier,
                        "Show the pixel magnifier",
                    );
                });
            }
            1 => {
                group(ui, "Capture", |ui| {
                    egui::Grid::new("snip-capture")
                        .num_columns(2)
                        .min_col_width(140.0)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Capture delay:");
                            ui.add(
                                egui::DragValue::new(&mut self.config.general.delay_ms)
                                    .range(0..=60_000)
                                    .suffix(" ms"),
                            );
                            ui.end_row();

                            ui.label("Native output:");
                            ui.checkbox(
                                &mut self.config.general.native_output,
                                "Use native monitor pixels for save and pin",
                            );
                            ui.end_row();
                        });
                });
            }
            _ => {
                group(ui, "Overlay Shortcuts", |ui| {
                    egui::Grid::new("overlay-keys")
                        .num_columns(2)
                        .min_col_width(140.0)
                        .spacing([12.0, 5.0])
                        .show(ui, |ui| {
                            for (key, action) in [
                                ("Drag", "Create a selection"),
                                ("Drag inside", "Move the selection"),
                                ("Shift", "Keep a square"),
                                ("Ctrl+A", "Select the whole desktop"),
                                ("Enter / Ctrl+C", "Copy"),
                                ("Ctrl+S", "Save"),
                                ("P", "Pin"),
                                ("R / C / A / B / H / M / T / E", "Switch tools"),
                                ("N / G", "Number / Magnifier"),
                                ("Ctrl+Z", "Undo"),
                                ("Ctrl+Shift+Z", "Redo"),
                                ("Esc", "Cancel"),
                            ] {
                                ui.label(RichText::new(key).monospace());
                                ui.label(action);
                                ui.end_row();
                            }
                        });
                });
            }
        }
    }

    fn pin_page(&mut self, ui: &mut egui::Ui) {
        match self.subtab {
            0 => {
                group(ui, "Pin Display", |ui| {
                    egui::Grid::new("pin-display")
                        .num_columns(2)
                        .min_col_width(140.0)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Backend:");
                            egui::ComboBox::from_id_salt("pin-backend")
                                .width(220.0)
                                .selected_text(pin_backend_label(self.config.pin.backend))
                                .show_ui(ui, |ui| {
                                    for backend in
                                        [PinBackend::Auto, PinBackend::Shell, PinBackend::Window]
                                    {
                                        ui.selectable_value(
                                            &mut self.config.pin.backend,
                                            backend,
                                            pin_backend_label(backend),
                                        );
                                    }
                                });
                            ui.end_row();

                            ui.label("Zoom step:");
                            ui.add(
                                egui::DragValue::new(&mut self.config.pin.zoom_step)
                                    .range(0.0001..=0.05)
                                    .speed(0.0001)
                                    .max_decimals(4),
                            );
                            ui.end_row();
                        });
                });
                group(ui, "GNOME Shell Extension", |ui| {
                    ui.horizontal(|ui| {
                        let (color, text) = if self.shell_pins_available {
                            (Color32::from_rgb(60, 165, 95), "Available")
                        } else {
                            (Color32::from_rgb(180, 145, 55), "Not running")
                        };
                        ui.colored_label(color, "●");
                        ui.label(text);
                        if ui.button("Check again").clicked() {
                            self.shell_pins_available = shell_pin::available();
                        }
                    });
                    ui.label(
                        RichText::new(
                            "The optional cosnip-pins@cosnip extension keeps pins sharp and above \
                             normal windows without XWayland. Run extensions/install.sh, then log \
                             in again.",
                        )
                        .small()
                        .color(ui.visuals().weak_text_color()),
                    );
                });
            }
            _ => {
                group(ui, "Pin Appearance", |ui| {
                    ui.label("Pins are image-sized and have no shadow or decoration.");
                    ui.label(
                        RichText::new(
                            "The window backend uses an override-redirect XWayland window. The \
                             Shell backend draws the same RGBA image through a compositor actor.",
                        )
                        .small()
                        .color(ui.visuals().weak_text_color()),
                    );
                });
                group(ui, "Mouse", |ui| {
                    egui::Grid::new("pin-mouse")
                        .num_columns(2)
                        .min_col_width(140.0)
                        .spacing([12.0, 5.0])
                        .show(ui, |ui| {
                            for (key, action) in [
                                ("Drag", "Move the pin"),
                                ("Wheel", "Zoom the pin"),
                                ("Right-click", "Close the pin"),
                            ] {
                                ui.label(RichText::new(key).monospace());
                                ui.label(action);
                                ui.end_row();
                            }
                        });
                });
            }
        }
    }

    fn output_page(&mut self, ui: &mut egui::Ui) {
        match self.subtab {
            0 => {
                group(ui, "Default Output", |ui| {
                    egui::Grid::new("output-default")
                        .num_columns(2)
                        .min_col_width(140.0)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Action:");
                            egui::ComboBox::from_id_salt("default-output")
                                .width(180.0)
                                .selected_text(default_output_label(
                                    self.config.general.default_output,
                                ))
                                .show_ui(ui, |ui| {
                                    for output in [
                                        DefaultOutput::Copy,
                                        DefaultOutput::Save,
                                        DefaultOutput::Pin,
                                    ] {
                                        ui.selectable_value(
                                            &mut self.config.general.default_output,
                                            output,
                                            default_output_label(output),
                                        );
                                    }
                                });
                            ui.end_row();
                        });
                });

                group(ui, "Save File", |ui| {
                    egui::Grid::new("output-file")
                        .num_columns(2)
                        .min_col_width(140.0)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Directory:");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.config.general.save_dir)
                                        .desired_width(300.0)
                                        .hint_text("Pictures"),
                                );
                                let picking = self.folder_pick.is_some();
                                if ui
                                    .add_enabled(!picking, egui::Button::new("Browse…"))
                                    .clicked()
                                {
                                    self.pick_folder();
                                }
                            });
                            ui.end_row();

                            ui.label("File name:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.config.general.filename)
                                    .desired_width(360.0),
                            );
                            ui.end_row();

                            ui.label("Preview:");
                            let preview = jiff::Zoned::now();
                            ui.label(
                                RichText::new(self.config.file_name(&preview))
                                    .monospace()
                                    .small(),
                            );
                            ui.end_row();

                            ui.label("Native size:");
                            ui.checkbox(
                                &mut self.config.general.native_output,
                                "Save/pin at the monitor's native pixel size",
                            );
                            ui.end_row();
                        });
                });
            }
            _ => {
                group(ui, "Clipboard", |ui| {
                    let (color, text) = if self.clipboard_available {
                        (Color32::from_rgb(60, 165, 95), "Available")
                    } else {
                        (Color32::from_rgb(180, 145, 55), "Unavailable")
                    };
                    ui.horizontal(|ui| {
                        ui.colored_label(color, "●");
                        ui.label(text);
                    });
                    ui.label(
                        "Copied images are encoded as PNG and written directly to the clipboard.",
                    );
                });
            }
        }
    }

    fn controls_page(&mut self, ui: &mut egui::Ui) {
        match self.subtab {
            0 => {
                group(ui, "GNOME Custom Shortcuts", |ui| {
                    ui.label(
                        RichText::new(
                            "Cosnip does not grab global keys. Bind these commands in GNOME \
                             Settings → Keyboard → Custom Shortcuts.",
                        )
                        .small()
                        .color(ui.visuals().weak_text_color()),
                    );
                    ui.add_space(6.0);
                    for (label, subcommand, suggested) in
                        [("Snip", "snip", "F1"), ("Paste", "paste", "F3")]
                    {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("{label} ({suggested})")).strong());
                            let command = self.exe_command(subcommand);
                            let mut text = command.clone();
                            ui.add(
                                egui::TextEdit::singleline(&mut text)
                                    .desired_width(360.0)
                                    .font(egui::TextStyle::Monospace)
                                    .interactive(false),
                            );
                            if ui.button("Copy").clicked() {
                                ui.ctx().copy_text(command);
                            }
                        });
                        ui.add_space(4.0);
                    }
                    if ui.button("Open GNOME Keyboard settings").clicked() {
                        self.open_keyboard_settings();
                    }
                });
            }
            _ => {
                group(ui, "Snipping", |ui| {
                    egui::Grid::new("mouse-snip")
                        .num_columns(2)
                        .min_col_width(140.0)
                        .spacing([12.0, 5.0])
                        .show(ui, |ui| {
                            for (key, action) in [
                                ("Drag", "Create or move the selection"),
                                ("Right-click a tool", "Choose color and line width"),
                                ("Wheel", "Zoom the pin window"),
                                ("Esc", "Cancel the current tool or overlay"),
                            ] {
                                ui.label(RichText::new(key).monospace());
                                ui.label(action);
                                ui.end_row();
                            }
                        });
                });
            }
        }
    }

    fn about_page(&mut self, ui: &mut egui::Ui) {
        group(ui, "Cosnip", |ui| {
            ui.label(RichText::new("Cosnip").size(22.0).strong());
            ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
            ui.label("A lightweight Linux snip and pin tool.");
            ui.label("License: MIT OR Apache-2.0");
        });
        group(ui, "Configuration", |ui| {
            ui.label(RichText::new(self.path.display().to_string()).monospace());
            ui.horizontal(|ui| {
                if ui.button("Open containing folder").clicked() {
                    self.open_config_dir();
                }
                if ui.button("Open config file").clicked() {
                    self.open_config_file();
                }
            });
        });
        group(ui, "GNOME Shell Pins", |ui| {
            ui.label("Extension: cosnip-pins@cosnip");
            let (color, text) = if self.shell_pins_available {
                (Color32::from_rgb(60, 165, 95), "Running")
            } else {
                (
                    Color32::from_rgb(180, 145, 55),
                    "Not running (window fallback)",
                )
            };
            ui.horizontal(|ui| {
                ui.colored_label(color, "●");
                ui.label(text);
            });
            ui.label(
                RichText::new(
                    "Run extensions/install.sh and log in again to load the Shell extension.",
                )
                .small()
                .color(ui.visuals().weak_text_color()),
            );
        });
    }
}

impl eframe::App for SettingsApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.done {
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }
        self.poll_folder(ctx);
        self.autosave(ctx);
        let viewport = self.viewport;
        let builder = egui::ViewportBuilder::default()
            .with_title("Cosnip Preferences")
            .with_inner_size([560.0, 440.0])
            .with_min_inner_size([500.0, 400.0])
            .with_app_id("cosnip-settings")
            .with_active(true);
        let mut close = false;
        ctx.show_viewport_immediate(viewport, builder, |ui, _class| {
            if ui.ctx().input(|input| {
                input.viewport().close_requested()
                    || (input.key_pressed(Key::Escape) && !input.modifiers.command)
            }) {
                close = true;
            }
            self.settings_ui(ui);
        });
        if close {
            self.save();
            self.done = true;
            ctx.send_viewport_cmd_to(viewport, ViewportCommand::Close);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let _ = ui;
    }
}

fn tab_button(ui: &mut egui::Ui, selected: bool, label: &str) -> egui::Response {
    let fill = if selected { Color32::WHITE } else { TAB_BG };
    let text = RichText::new(label).color(if selected {
        Color32::from_gray(25)
    } else {
        Color32::from_gray(55)
    });
    ui.add(
        egui::Button::new(text)
            .fill(fill)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(egui::CornerRadius::same(0))
            .min_size(egui::vec2(68.0, 30.0)),
    )
}

fn sub_tab_button(ui: &mut egui::Ui, selected: bool, label: &str) -> egui::Response {
    let fill = if selected { Color32::WHITE } else { SUBTAB_BG };
    ui.add(
        egui::Button::new(RichText::new(label).size(12.0))
            .fill(fill)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(egui::CornerRadius::same(0))
            .min_size(egui::vec2(74.0, 26.0)),
    )
}

fn group<R>(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.label(RichText::new(title).strong().size(13.0));
    let response = egui::Frame::NONE
        .fill(Color32::from_gray(250))
        .stroke(Stroke::new(1.0, GROUP_BORDER))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        });
    ui.add_space(10.0);
    response.inner
}

fn default_output_label(output: DefaultOutput) -> &'static str {
    match output {
        DefaultOutput::Copy => "Copy",
        DefaultOutput::Save => "Save",
        DefaultOutput::Pin => "Pin",
    }
}

fn pin_backend_label(backend: PinBackend) -> &'static str {
    match backend {
        PinBackend::Auto => "Auto (Shell when available)",
        PinBackend::Shell => "GNOME Shell extension",
        PinBackend::Window => "Window (XWayland)",
    }
}

fn cursor_mode_label(mode: CursorMode) -> &'static str {
    match mode {
        CursorMode::Auto => "Auto",
        CursorMode::Always => "Always visible",
        CursorMode::Hidden => "Hidden",
    }
}
