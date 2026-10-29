#![forbid(unsafe_code)]
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use cosnip_capture::{CaptureError, backend, detect_backend, ffmpeg_available, screen_info};
use cosnip_core::{OutputRequest, Rect};
use cosnip_platform::clipboard_available;
use cosnip_ui::{UiOptions, run};

#[derive(Debug, Parser)]
#[command(name = "cosnip", version, about = "A minimal Linux snip and pin tool")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
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
    #[arg(long, default_value_t = 0)]
    delay: u64,
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
        return doctor();
    }
    match cli.command {
        Some(Command::Snip(args)) => snip(args),
        Some(Command::Paste(args)) => paste(args),
        None => {
            eprintln!("cosnip: use `cosnip snip`, `cosnip paste`, or `cosnip --doctor`");
            ExitCode::from(2)
        }
    }
}

fn snip(args: SnipArgs) -> ExitCode {
    if args.area.is_some() && args.full {
        eprintln!("cosnip: --area and --full are mutually exclusive");
        return ExitCode::from(2);
    }
    if args.delay > 0 {
        std::thread::sleep(Duration::from_millis(args.delay));
    }

    let kind = match detect_backend() {
        Ok(kind) => kind,
        Err(error) => {
            eprintln!("cosnip: {error}");
            return ExitCode::from(3);
        }
    };
    let backend = backend(kind);
    if let Err(error) = backend.is_available() {
        eprintln!("cosnip: {error}");
        return ExitCode::from(3);
    }

    let started = Instant::now();
    let frame = match backend.capture() {
        Ok(frame) => frame,
        Err(error) => {
            eprintln!("cosnip: {error}");
            return ExitCode::from(4);
        }
    };
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
    let copy = if args.no_copy {
        false
    } else if args.copy {
        true
    } else {
        args.save.is_none() && !args.pin
    };
    let output = OutputRequest {
        copy,
        save: args.save.clone(),
        pin: args.pin,
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
            save_path: args.save,
            monitors: cosnip_capture::monitors().unwrap_or_default(),
        },
    );
    if outcome.code != 0 {
        return ExitCode::from(outcome.code as u8);
    }
    ExitCode::SUCCESS
}

fn paste(args: PasteArgs) -> ExitCode {
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
    let outcome = cosnip_ui::run_pin(frame, position);
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

fn doctor() -> ExitCode {
    let wayland = cosnip_capture::session_is_wayland();
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    println!("session: {}", if wayland { "wayland" } else { "x11" });
    println!("desktop: {desktop}");
    println!("ffmpeg: {}", if ffmpeg_available() { "yes" } else { "no" });
    println!(
        "clipboard: {}",
        if clipboard_available() { "yes" } else { "no" }
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
