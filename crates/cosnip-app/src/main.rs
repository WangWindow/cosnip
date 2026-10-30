#![forbid(unsafe_code)]
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use cosnip_capture::{CaptureError, backend, detect_backend, ffmpeg_available, screen_info};
use cosnip_core::{OutputRequest, Rect};
use cosnip_platform::{Config, clipboard_available, config_path};
use cosnip_ui::{UiOptions, UiStyle, run};

#[derive(Debug, Parser)]
#[command(name = "cosnip", version, about = "A minimal Linux snip and pin tool")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    /// Override the XDG config file.
    #[arg(long, global = true, value_name = "PATH")]
    config: Option<PathBuf>,
    /// Print environment/capability diagnostics and exit.
    #[arg(long)]
    doctor: bool,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Capture the desktop and select a region.
    Snip(SnipArgs),
    /// Paste a clipboard image as a pin.
    Paste(PasteArgs),
    /// Open the settings window.
    Settings,
    /// Run the lightweight system tray.
    Tray,
}

#[derive(Debug, clap::Args)]
struct PasteArgs {
    /// Pin this PNG file instead of the clipboard image.
    #[arg(long, value_name = "PATH")]
    file: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
struct SnipArgs {
    /// Preselect an area: X Y WIDTH HEIGHT.
    #[arg(long, num_args = 4, value_names = ["X", "Y", "WIDTH", "HEIGHT"])]
    area: Option<Vec<i32>>,
    /// Select the whole captured desktop.
    #[arg(long)]
    full: bool,
    /// Delay before capture, in milliseconds.
    #[arg(long)]
    delay: Option<u64>,
    /// Copy the selection (default when no output is specified).
    #[arg(long)]
    copy: bool,
    /// Do not copy the selection.
    #[arg(long)]
    no_copy: bool,
    /// Save the selection as PNG.
    #[arg(long)]
    save: Option<PathBuf>,
    /// Pin the selection on screen.
    #[arg(long)]
    pin: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if cli.doctor {
        return doctor(cli.config.as_deref());
    }
    let config_file = cli
        .config
        .clone()
        .unwrap_or_else(cosnip_platform::config_path);
    let config = load_config(cli.config.as_deref());
    match cli.command {
        Some(Command::Snip(args)) => snip(args, &config),
        Some(Command::Paste(args)) => paste(args, &config),
        Some(Command::Settings) => settings(&config_file),
        Some(Command::Tray) => tray(&config_file),
        None => {
            eprintln!(
                "cosnip: use `cosnip snip`, `cosnip paste`, `cosnip settings`, `cosnip tray`, or `cosnip --doctor`"
            );
            ExitCode::from(2)
        }
    }
}

fn snip(args: SnipArgs, config: &Config) -> ExitCode {
    if args.area.is_some() && args.full {
        eprintln!("cosnip: --area and --full are mutually exclusive");
        return ExitCode::from(2);
    }
    if let Some(values) = &args.area {
        if values.len() != 4 {
            eprintln!("cosnip: --area requires four integers");
            return ExitCode::from(2);
        }
        if values[2] <= 0 || values[3] <= 0 {
            eprintln!("cosnip: --area width and height must be positive");
            return ExitCode::from(2);
        }
    }
    let delay = args.delay.unwrap_or(config.general.delay_ms);
    if delay > 0 {
        std::thread::sleep(Duration::from_millis(delay));
    }

    let kind = match detect_backend() {
        Ok(kind) => kind,
        Err(error) => {
            eprintln!("cosnip: {error}");
            return ExitCode::from(3);
        }
    };
    if let Err(error) = backend(kind).is_available() {
        eprintln!("cosnip: {error}");
        return ExitCode::from(3);
    }

    let started = Instant::now();
    let captured = match cosnip_capture::capture_desktop(kind) {
        Ok(captured) => captured,
        Err(error) => {
            eprintln!("cosnip: {error}");
            return ExitCode::from(4);
        }
    };
    let frame = captured.frame;
    eprintln!(
        "cosnip: captured {}x{} via {} in {} ms",
        frame.width(),
        frame.height(),
        kind.as_str(),
        started.elapsed().as_millis()
    );

    let initial_selection = match initial_selection(&args, frame.width(), frame.height()) {
        Ok(selection) => selection,
        Err(message) => {
            eprintln!("cosnip: {message}");
            return ExitCode::from(2);
        }
    };

    if args.copy && args.no_copy {
        eprintln!("cosnip: --copy and --no-copy are mutually exclusive");
        return ExitCode::from(2);
    }
    let explicit_output = args.copy || args.no_copy || args.save.is_some() || args.pin;
    let (copy, save, pin) = if explicit_output {
        (
            if args.no_copy {
                false
            } else if args.copy {
                true
            } else {
                args.save.is_none() && !args.pin
            },
            args.save.clone(),
            args.pin,
        )
    } else {
        match config.general.default_output {
            cosnip_platform::DefaultOutput::Copy => (true, None, false),
            cosnip_platform::DefaultOutput::Save => {
                let path = match config.save_path() {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("cosnip: {error}");
                        return ExitCode::from(2);
                    }
                };
                (false, Some(path), false)
            }
            cosnip_platform::DefaultOutput::Pin => (false, None, true),
        }
    };
    let output = OutputRequest {
        copy,
        save: save.clone(),
        pin,
    };
    if !output.has_output() {
        eprintln!("cosnip: no output selected");
        return ExitCode::from(2);
    }

    let outcome = run(
        frame,
        UiOptions {
            backend: kind,
            initial_selection,
            output,
            save_path: save,
            save_dir: config.save_dir(),
            filename: config.general.filename.clone(),
            monitors: cosnip_capture::monitors().unwrap_or_default(),
            native: if config.general.native_output {
                captured.native
            } else {
                Vec::new()
            },
            style: style(config),
        },
    );
    if outcome.code != 0 {
        return ExitCode::from(outcome.code as u8);
    }
    ExitCode::SUCCESS
}

fn paste(args: PasteArgs, config: &Config) -> ExitCode {
    if std::env::var_os("DISPLAY").is_none() {
        eprintln!("cosnip: paste requires X11 or XWayland; DISPLAY is not set");
        return ExitCode::from(3);
    }
    let frame = match load_paste_frame(&args) {
        Ok(frame) => frame,
        Err((code, message)) => {
            eprintln!("cosnip: {message}");
            return ExitCode::from(code);
        }
    };
    let position = cosnip_platform::global_pointer();
    let outcome = cosnip_ui::run_pin(frame, position, style(config));
    if outcome.code != 0 {
        return ExitCode::from(outcome.code as u8);
    }
    ExitCode::SUCCESS
}

fn load_paste_frame(args: &PasteArgs) -> Result<cosnip_core::Frame, (u8, String)> {
    if let Some(path) = &args.file {
        let bytes = std::fs::read(path)
            .map_err(|error| (2u8, format!("cannot read {}: {error}", path.display())))?;
        return cosnip_imaging::decode_png(&bytes).map_err(|error| {
            (
                2u8,
                format!("cannot decode {} as PNG: {error}", path.display()),
            )
        });
    }
    cosnip_platform::clipboard_image().map_err(|error| (3u8, error.to_string()))
}

fn load_config(path: Option<&Path>) -> Config {
    let path = path.map(Path::to_path_buf).unwrap_or_else(config_path);
    match Config::load(&path) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("cosnip: {error}; using defaults");
            Config::default()
        }
    }
}

fn style(config: &Config) -> UiStyle {
    UiStyle {
        annotation_color: config.annotations.color_rgba(),
        line_width: config.annotations.line_width,
        font_size: config.annotations.font_size,
        text_shadow: config.annotations.text_shadow,
        cursor_mode: config.overlay.cursor_mode,
        magnifier: config.overlay.magnifier,
        show_tips: config.overlay.show_tips,
        pin_backend: config.pin.backend,
        pin_zoom_step: config.pin.zoom_step,
    }
}

fn settings(path: &Path) -> ExitCode {
    let outcome = cosnip_ui::run_settings(path.to_path_buf());
    if outcome.code != 0 {
        return ExitCode::from(outcome.code as u8);
    }
    ExitCode::SUCCESS
}

fn tray(config: &Path) -> ExitCode {
    match cosnip_platform::run_tray(Some(config.to_path_buf())) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("cosnip: tray failed: {error}");
            ExitCode::from(3)
        }
    }
}

fn initial_selection(args: &SnipArgs, width: u32, height: u32) -> Result<Option<Rect>, String> {
    if args.full {
        return Ok(Some(Rect::new(0, 0, width, height)));
    }
    let Some(values) = &args.area else {
        return Ok(None);
    };
    if values.len() != 4 {
        return Err("--area requires four integers".into());
    }
    if values[2] <= 0 || values[3] <= 0 {
        return Err("--area width and height must be positive".into());
    }
    let rect = Rect::new(values[0], values[1], values[2] as u32, values[3] as u32);
    let bounds = Rect::new(0, 0, width, height);
    rect.clamp_to(bounds)
        .filter(|clamped| *clamped == rect)
        .map(Some)
        .ok_or_else(|| "area is outside the captured desktop".into())
}

fn doctor(config: Option<&Path>) -> ExitCode {
    let wayland = cosnip_capture::session_is_wayland();
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    println!("session: {}", if wayland { "wayland" } else { "x11" });
    println!("desktop: {desktop}");
    println!("ffmpeg: {}", if ffmpeg_available() { "yes" } else { "no" });
    println!(
        "clipboard: {}",
        if clipboard_available() { "yes" } else { "no" }
    );
    let config_path = config
        .map(Path::to_path_buf)
        .unwrap_or_else(cosnip_platform::config_path);
    println!("config: {}", config_path.display());
    println!(
        "tray: {}",
        if cosnip_platform::tray_host_available() {
            "available"
        } else {
            "unavailable (icon hidden)"
        }
    );
    match screen_info() {
        Ok(info) => println!(
            "monitors: {} ({}x{}, scale {}, name {})",
            info.count, info.width, info.height, info.scale, info.name
        ),
        Err(error) => println!("monitors: unavailable ({error})"),
    }
    if let Ok(monitors) = cosnip_capture::monitors() {
        for (index, monitor) in monitors.iter().enumerate() {
            println!(
                "monitor {index}: {} {}x{} at {},{}",
                monitor.name, monitor.width, monitor.height, monitor.x, monitor.y
            );
        }
    }
    println!(
        "shell pins: {}",
        if cosnip_platform::shell_pin::available() {
            "available"
        } else {
            "unavailable (window fallback)"
        }
    );
    match detect_backend() {
        Ok(kind) => {
            println!("backend: {}", kind.as_str());
            let backend = backend(kind);
            match backend.is_available() {
                Ok(()) => {
                    println!("verdict: ready");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    println!("verdict: unavailable ({error})");
                    ExitCode::from(3)
                }
            }
        }
        Err(CaptureError::Unsupported(message)) => {
            println!("verdict: unsupported ({message})");
            ExitCode::from(3)
        }
        Err(error) => {
            println!("verdict: unavailable ({error})");
            ExitCode::from(3)
        }
    }
}
