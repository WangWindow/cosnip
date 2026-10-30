//! Lossless GNOME Wayland capture through Mutter ScreenCast + PipeWire.
//!
//! The Shell screenshot APIs are restricted on GNOME 49. Mutter's ScreenCast API is available to
//! the user session: create a session, record each monitor (or one desktop area as a fallback),
//! start it, receive the PipeWire node ids, and read raw `video/x-raw` frames. This is the same
//! media path used by GNOME's own recorder, without the H.264 encoding step.
//!
//! On mixed-DPI layouts Mutter's area stream can paint only the primary stage view. Cosnip
//! therefore records every monitor with `RecordMonitor`, scales each frame into the XWayland
//! desktop coordinate space, and composes one virtual-desktop frame for the overlay.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use cosnip_core::Frame;
use futures_lite::StreamExt;
use pipewire as pw;
use pw::spa;
use pw::spa::param::format::{MediaSubtype, MediaType};
use pw::spa::param::format_utils;
use pw::spa::param::video::VideoInfoRaw;
use pw::spa::pod::Pod;
use zbus::block_on;

use crate::{
    CaptureBackend, CaptureBackendKind, CaptureError, CapturedDesktop, CapturedMonitor,
    MonitorInfo, desktop_is_gnome, session_is_wayland,
};

const MUTTER_NAME: &str = "org.gnome.Mutter.ScreenCast";
const MUTTER_PATH: &str = "/org/gnome/Mutter/ScreenCast";
const SESSION_IFACE: &str = "org.gnome.Mutter.ScreenCast.Session";
const STREAM_IFACE: &str = "org.gnome.Mutter.ScreenCast.Stream";

const DISPLAY_CONFIG_NAME: &str = "org.gnome.Mutter.DisplayConfig";
const DISPLAY_CONFIG_PATH: &str = "/org/gnome/Mutter/DisplayConfig";

const STREAM_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Default)]
pub struct MutterPipewire;

impl CaptureBackend for MutterPipewire {
    fn kind(&self) -> CaptureBackendKind {
        CaptureBackendKind::GnomePipewire
    }

    fn is_available(&self) -> Result<(), CaptureError> {
        if !session_is_wayland() || !desktop_is_gnome() {
            return Err(CaptureError::Unsupported(
                "Mutter PipeWire capture requires a GNOME Wayland session".into(),
            ));
        }
        if std::env::var_os("DISPLAY").is_none() {
            return Err(CaptureError::Unavailable(
                "XWayland is required for the overlay; DISPLAY is not set".into(),
            ));
        }
        let info = crate::screen_info()?;
        if info.width == 0 || info.height == 0 {
            return Err(CaptureError::Unavailable(
                "cannot determine the desktop size".into(),
            ));
        }
        Ok(())
    }

    fn capture(&self) -> Result<Frame, CaptureError> {
        Ok(capture_desktop()?.frame)
    }
}

/// Capture the composed desktop and any native monitor frames needed for sharp output.
pub(crate) fn capture_desktop() -> Result<CapturedDesktop, CaptureError> {
    let backend = MutterPipewire;
    backend.is_available()?;
    let info = crate::screen_info()?;
    let monitors = crate::monitors().unwrap_or_default();

    match capture_per_monitor(&monitors, info.width, info.height) {
        Ok((frame, native)) => Ok(CapturedDesktop { frame, native }),
        Err(error) => {
            eprintln!("cosnip: per-monitor Mutter capture failed ({error}); trying area capture");
            match capture_area(info.width, info.height) {
                Ok(frame) => Ok(CapturedDesktop {
                    frame,
                    native: Vec::new(),
                }),
                Err(area_error) => {
                    eprintln!(
                        "cosnip: Mutter ScreenCast failed ({area_error}); trying Screencast fallback"
                    );
                    crate::gnome::GnomeScreencast
                        .capture()
                        .and_then(|frame| normalize_frame(frame, info.width, info.height))
                        .map(|frame| CapturedDesktop {
                            frame,
                            native: Vec::new(),
                        })
                }
            }
        }
    }
}

/// Capture every RandR monitor with its own Mutter stream and compose them into one frame.
///
/// The composed frame uses the XWayland root coordinate space, which is also the coordinate
/// space of the overlay viewports. Frames from monitors whose stream scale differs from the
/// global XWayland scale are resampled with bilinear interpolation.
fn capture_per_monitor(
    monitors: &[MonitorInfo],
    width: u32,
    height: u32,
) -> Result<(Frame, Vec<CapturedMonitor>), CaptureError> {
    if monitors.is_empty() {
        return Err(CaptureError::Unavailable("no monitors detected".into()));
    }
    if monitors
        .iter()
        .any(|monitor| monitor.name.is_empty() || monitor.name.starts_with("monitor-"))
    {
        return Err(CaptureError::Unavailable(
            "monitor connector names are unavailable".into(),
        ));
    }

    let targets = monitors
        .iter()
        .map(|monitor| StreamTarget {
            name: monitor.name.clone(),
            width: monitor.width,
            height: monitor.height,
        })
        .collect::<Vec<_>>();
    let session = MutterSession::start(RecordRequest::Monitors(targets))?;
    let frames = pipewire_capture(&session.nodes);
    session.stop();
    let frames = frames?;
    if monitors.len() == 1 {
        let monitor = &monitors[0];
        let frame = frames.into_iter().next().ok_or(CaptureError::Timeout)?;
        if frame.width() == width && frame.height() == height {
            return Ok((frame, Vec::new()));
        }
        let native = CapturedMonitor::new(monitor.clone(), &frame)?;
        let scaled = scale_frame(frame, width, height)?;
        return Ok((scaled, vec![native]));
    }

    let mut desktop = Frame::new(
        width,
        height,
        [0u8, 0, 0, 255].repeat(width as usize * height as usize),
    )
    .map_err(|error| CaptureError::InvalidFrame(error.to_string()))?;
    let mut native = Vec::new();
    for (monitor, frame) in monitors.iter().zip(frames) {
        if frame.width() == monitor.width && frame.height() == monitor.height {
            desktop.blit(&frame, monitor.x, monitor.y);
        } else {
            native.push(CapturedMonitor::new(monitor.clone(), &frame)?);
            let scaled = scale_frame(frame, monitor.width, monitor.height)?;
            desktop.blit(&scaled, monitor.x, monitor.y);
        }
    }
    Ok((desktop, native))
}

/// Fall back to one Mutter area stream covering the logical stage.
fn capture_area(width: u32, height: u32) -> Result<Frame, CaptureError> {
    let (x, y, area_width, area_height) = stage_area()?;
    let session = MutterSession::start(RecordRequest::Area {
        x,
        y,
        width: area_width,
        height: area_height,
    })?;
    let frames = pipewire_capture(&session.nodes);
    session.stop();
    let frame = frames?.into_iter().next().ok_or(CaptureError::Timeout)?;
    normalize_frame(frame, width, height)
}

fn normalize_frame(frame: Frame, width: u32, height: u32) -> Result<Frame, CaptureError> {
    scale_frame(frame, width, height)
}

/// Resize a captured frame with Lanczos3. Upscaling with a sharp kernel keeps the image closer
/// to the native monitor pixels after Mutter's compositor downscale than bilinear resampling.
fn scale_frame(frame: Frame, width: u32, height: u32) -> Result<Frame, CaptureError> {
    if frame.width() == width && frame.height() == height {
        return Ok(frame);
    }
    let (source_width, source_height) = (frame.width(), frame.height());
    let image = image::RgbaImage::from_raw(source_width, source_height, frame.into_rgba())
        .ok_or_else(|| CaptureError::InvalidFrame("invalid frame buffer".into()))?;
    let resized =
        image::imageops::resize(&image, width, height, image::imageops::FilterType::Nearest);
    Frame::new(width, height, resized.into_raw())
        .map_err(|error| CaptureError::InvalidFrame(error.to_string()))
}

struct MutterSession {
    connection: zbus::Connection,
    path: String,
    nodes: Vec<StreamNode>,
}

struct StreamNode {
    node: u32,
    width: u32,
    height: u32,
}

enum RecordRequest {
    Monitors(Vec<StreamTarget>),
    Area {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
}

struct StreamTarget {
    name: String,
    width: u32,
    height: u32,
}

impl MutterSession {
    fn start(request: RecordRequest) -> Result<Self, CaptureError> {
        let (connection, path, nodes) = block_on(async {
            let connection = zbus::Connection::session()
                .await
                .map_err(|error| CaptureError::Dbus(error.to_string()))?;
            let session_options: HashMap<String, zbus::Variant<'static>> = HashMap::new();
            let reply = connection
                .call_method(
                    Some(MUTTER_NAME),
                    MUTTER_PATH,
                    Some(MUTTER_NAME),
                    "CreateSession",
                    &(session_options,),
                )
                .await
                .map_err(|error| CaptureError::Dbus(error.to_string()))?;
            let session_path: zbus::OwnedObjectPath = reply
                .body()
                .deserialize()
                .map_err(|error| CaptureError::Dbus(error.to_string()))?;
            let session = zbus::Proxy::new(
                &connection,
                MUTTER_NAME,
                session_path.as_str(),
                SESSION_IFACE,
            )
            .await
            .map_err(|error| CaptureError::Dbus(error.to_string()))?;

            let mut streams = Vec::new();
            match request {
                RecordRequest::Monitors(targets) => {
                    for target in targets {
                        let reply = session
                            .call_method("RecordMonitor", &(target.name.as_str(), record_options()))
                            .await
                            .map_err(|error| CaptureError::Dbus(error.to_string()))?;
                        let stream_path: zbus::OwnedObjectPath = reply
                            .body()
                            .deserialize()
                            .map_err(|error| CaptureError::Dbus(error.to_string()))?;
                        let stream = stream_proxy(&connection, stream_path.as_str()).await?;
                        let signals = stream
                            .receive_signal("PipeWireStreamAdded")
                            .await
                            .map_err(|error| CaptureError::Dbus(error.to_string()))?;
                        streams.push((target, signals));
                    }
                }
                RecordRequest::Area {
                    x,
                    y,
                    width,
                    height,
                } => {
                    let reply = session
                        .call_method(
                            "RecordArea",
                            &(x, y, width as i32, height as i32, record_options()),
                        )
                        .await
                        .map_err(|error| CaptureError::Dbus(error.to_string()))?;
                    let stream_path: zbus::OwnedObjectPath = reply
                        .body()
                        .deserialize()
                        .map_err(|error| CaptureError::Dbus(error.to_string()))?;
                    let stream = stream_proxy(&connection, stream_path.as_str()).await?;
                    let signals = stream
                        .receive_signal("PipeWireStreamAdded")
                        .await
                        .map_err(|error| CaptureError::Dbus(error.to_string()))?;
                    streams.push((
                        StreamTarget {
                            name: "desktop".into(),
                            width,
                            height,
                        },
                        signals,
                    ));
                }
            }

            session
                .call_method("Start", &())
                .await
                .map_err(|error| CaptureError::Dbus(error.to_string()))?;

            let deadline = Instant::now() + STREAM_TIMEOUT;
            let mut nodes = Vec::with_capacity(streams.len());
            for (target, mut signals) in streams {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(CaptureError::Timeout);
                }
                let message = futures_lite::future::or(signals.next(), async move {
                    async_io::Timer::after(remaining).await;
                    None
                })
                .await
                .ok_or(CaptureError::Timeout)?;
                let node: u32 = message
                    .body()
                    .deserialize()
                    .map_err(|error| CaptureError::Dbus(error.to_string()))?;
                nodes.push(StreamNode {
                    node,
                    width: target.width,
                    height: target.height,
                });
            }
            Ok::<_, CaptureError>((connection, session_path.to_string(), nodes))
        })?;

        Ok(Self {
            connection,
            path,
            nodes,
        })
    }

    fn stop(self) {
        let _ = block_on(self.connection.call_method(
            Some(MUTTER_NAME),
            self.path.as_str(),
            Some(SESSION_IFACE),
            "Stop",
            &(),
        ));
    }
}

async fn stream_proxy<'a>(
    connection: &'a zbus::Connection,
    path: &'a str,
) -> Result<zbus::Proxy<'a>, CaptureError> {
    zbus::Proxy::new(connection, MUTTER_NAME, path, STREAM_IFACE)
        .await
        .map_err(|error| CaptureError::Dbus(error.to_string()))
}

fn record_options() -> HashMap<String, zbus::Variant<'static>> {
    HashMap::from([
        ("is-recording".into(), zbus::Variant::from(false)),
        ("cursor-mode".into(), zbus::Variant::from(0u32)),
    ])
}

struct SharedCapture {
    results: RefCell<Vec<Option<Result<Frame, String>>>>,
    remaining: Cell<usize>,
    mainloop: pw::main_loop::MainLoopRc,
}

impl SharedCapture {
    fn complete(&self, index: usize, result: Result<Frame, String>) {
        let already_done = {
            let mut results = self.results.borrow_mut();
            if results[index].is_some() {
                true
            } else {
                results[index] = Some(result);
                false
            }
        };
        if already_done {
            return;
        }
        let remaining = self.remaining.get().saturating_sub(1);
        self.remaining.set(remaining);
        if remaining == 0 {
            self.mainloop.quit();
        }
    }
}

struct StreamData {
    index: usize,
    format: VideoInfoRaw,
    shared: Rc<SharedCapture>,
}

fn pipewire_capture(nodes: &[StreamNode]) -> Result<Vec<Frame>, CaptureError> {
    if nodes.is_empty() {
        return Err(CaptureError::Unavailable("no capture streams".into()));
    }

    pw::init();
    let mainloop = pw::main_loop::MainLoopRc::new(None)
        .map_err(|error| CaptureError::Unavailable(format!("PipeWire main loop: {error}")))?;
    let context = pw::context::ContextRc::new(&mainloop, None)
        .map_err(|error| CaptureError::Unavailable(format!("PipeWire context: {error}")))?;
    let core = context
        .connect_rc(None)
        .map_err(|error| CaptureError::Unavailable(format!("PipeWire core: {error}")))?;

    let shared = Rc::new(SharedCapture {
        results: RefCell::new(vec![None; nodes.len()]),
        remaining: Cell::new(nodes.len()),
        mainloop: mainloop.clone(),
    });

    let mut streams = Vec::with_capacity(nodes.len());
    let mut listeners = Vec::with_capacity(nodes.len());
    for (index, target) in nodes.iter().enumerate() {
        let stream = pw::stream::StreamBox::new(
            &core,
            "cosnip-capture",
            pw::properties::properties! {
                *pw::keys::MEDIA_TYPE => "Video",
                *pw::keys::MEDIA_CATEGORY => "Capture",
                *pw::keys::MEDIA_ROLE => "Screen",
            },
        )
        .map_err(|error| CaptureError::Unavailable(format!("PipeWire stream: {error}")))?;

        let listener = stream
            .add_local_listener_with_user_data(StreamData {
                index,
                format: VideoInfoRaw::default(),
                shared: Rc::clone(&shared),
            })
            .param_changed(|_, data, id, param| {
                let Some(param) = param else { return };
                if id != spa::param::ParamType::Format.as_raw() {
                    return;
                }
                let Ok((media_type, media_subtype)) = format_utils::parse_format(param) else {
                    return;
                };
                if media_type != MediaType::Video || media_subtype != MediaSubtype::Raw {
                    return;
                }
                let _ = data.format.parse(param);
            })
            .process(|stream, data| {
                if data.shared.results.borrow()[data.index].is_some() {
                    return;
                }
                let Some(mut buffer) = stream.dequeue_buffer() else {
                    return;
                };
                let Some((stride, offset)) = buffer.datas_mut().first_mut().map(|chunk| {
                    let chunk = chunk.chunk();
                    (
                        chunk.stride().unsigned_abs() as usize,
                        chunk.offset() as usize,
                    )
                }) else {
                    return;
                };
                let Some(bytes) = buffer
                    .datas_mut()
                    .first_mut()
                    .and_then(|chunk| chunk.data())
                else {
                    data.shared.complete(
                        data.index,
                        Err("PipeWire buffer is not CPU-mappable".into()),
                    );
                    return;
                };
                if offset > bytes.len() {
                    data.shared
                        .complete(data.index, Err("PipeWire buffer offset is invalid".into()));
                    return;
                }
                let result = frame_from_buffer(&data.format, &bytes[offset..], stride)
                    .map_err(|error| format!("{}: {error}", data.index));
                data.shared.complete(data.index, result);
            })
            .state_changed(|_, data, _old, new| {
                if let pw::stream::StreamState::Error(message) = new {
                    eprintln!("cosnip: PipeWire stream error: {message}");
                    data.shared.complete(data.index, Err(message.to_string()));
                }
            })
            .register()
            .map_err(|error| CaptureError::Unavailable(format!("PipeWire listener: {error}")))?;

        let bytes = format_params(target.width, target.height)?;
        let pod = Pod::from_bytes(&bytes)
            .ok_or_else(|| CaptureError::Decode("invalid PipeWire format pod".into()))?;
        let mut params = [pod];
        stream
            .connect(
                spa::utils::Direction::Input,
                Some(target.node),
                pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
                &mut params,
            )
            .map_err(|error| CaptureError::Unavailable(format!("PipeWire connect: {error}")))?;

        streams.push(stream);
        listeners.push(listener);
    }

    let weak = mainloop.downgrade();
    let timer = mainloop.loop_().add_timer(move |_| {
        if let Some(mainloop) = weak.upgrade() {
            mainloop.quit();
        }
    });
    let _ = timer.update_timer(Some(STREAM_TIMEOUT), None);

    mainloop.run();
    drop(timer);
    drop(listeners);
    drop(streams);

    let results = shared.results.borrow();
    let mut frames = Vec::with_capacity(results.len());
    for (index, result) in results.iter().enumerate() {
        match result {
            Some(Ok(frame)) => frames.push(frame.clone()),
            Some(Err(message)) => {
                return Err(CaptureError::Decode(format!("stream {index}: {message}")));
            }
            None => return Err(CaptureError::Timeout),
        }
    }
    Ok(frames)
}

/// Read the negotiated format. The buffer must stay borrowed while `bytes` is used.
fn format_params(width: u32, height: u32) -> Result<Vec<u8>, CaptureError> {
    let obj = pw::spa::pod::object!(
        pw::spa::utils::SpaTypes::ObjectParamFormat,
        spa::param::ParamType::EnumFormat,
        pw::spa::pod::property!(
            spa::param::format::FormatProperties::MediaType,
            Id,
            spa::param::format::MediaType::Video
        ),
        pw::spa::pod::property!(
            spa::param::format::FormatProperties::MediaSubtype,
            Id,
            spa::param::format::MediaSubtype::Raw
        ),
        pw::spa::pod::property!(
            spa::param::format::FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            spa::param::video::VideoFormat::BGRx,
            spa::param::video::VideoFormat::BGRA,
            spa::param::video::VideoFormat::RGBx,
            spa::param::video::VideoFormat::RGBA
        ),
        pw::spa::pod::property!(
            spa::param::format::FormatProperties::VideoSize,
            Choice,
            Range,
            Rectangle,
            spa::utils::Rectangle { width, height },
            spa::utils::Rectangle {
                width: 1,
                height: 1
            },
            spa::utils::Rectangle {
                width: 16_384,
                height: 16_384
            }
        ),
        pw::spa::pod::property!(
            spa::param::format::FormatProperties::VideoFramerate,
            Choice,
            Range,
            Fraction,
            spa::utils::Fraction { num: 30, denom: 1 },
            spa::utils::Fraction { num: 0, denom: 1 },
            spa::utils::Fraction { num: 120, denom: 1 }
        ),
    );
    let values = pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(obj),
    )
    .map_err(|error| CaptureError::Decode(format!("PipeWire format: {error}")))?;
    Ok(values.0.into_inner())
}

fn frame_from_buffer(format: &VideoInfoRaw, bytes: &[u8], stride: usize) -> Result<Frame, String> {
    let width = format.size().width;
    let height = format.size().height;
    if width == 0 || height == 0 {
        return Err("PipeWire returned an empty frame".into());
    }
    let row = width as usize * 4;
    let stride = if stride == 0 { row } else { stride };
    if stride < row {
        return Err(format!("invalid stride {stride} for width {width}"));
    }
    if bytes.len() < stride.saturating_mul(height as usize) {
        return Err(format!(
            "short PipeWire frame: {} bytes for {width}x{height} stride {stride}",
            bytes.len()
        ));
    }
    let mut rgba = vec![0u8; row * height as usize];
    let format = format.format();
    for (y, out_row) in rgba.chunks_exact_mut(row).enumerate() {
        let row_start = y * stride;
        let row_bytes = &bytes[row_start..row_start + row];
        for (dst, px) in out_row
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(row_bytes.as_chunks::<4>().0)
        {
            *dst = match format {
                spa::param::video::VideoFormat::BGRx => [px[2], px[1], px[0], 255],
                spa::param::video::VideoFormat::BGRA => [px[2], px[1], px[0], px[3]],
                spa::param::video::VideoFormat::RGBx => [px[0], px[1], px[2], 255],
                spa::param::video::VideoFormat::RGBA => [px[0], px[1], px[2], px[3]],
                other => {
                    return Err(format!("unsupported PipeWire video format {other:?}"));
                }
            };
        }
    }
    Frame::new(width, height, rgba).map_err(|error| error.to_string())
}

type DisplaySpec = (String, String, String, String);
type DisplayMode = (
    String,
    i32,
    i32,
    f64,
    f64,
    Vec<f64>,
    HashMap<String, zbus::OwnedVariant>,
);
type DisplayMonitor = (
    DisplaySpec,
    Vec<DisplayMode>,
    HashMap<String, zbus::OwnedVariant>,
);
type LogicalMonitor = (
    i32,
    i32,
    f64,
    u32,
    bool,
    Vec<DisplaySpec>,
    HashMap<String, zbus::OwnedVariant>,
);

/// Ask Mutter for the logical stage bounds used by `RecordArea`.
fn stage_area() -> Result<(i32, i32, u32, u32), CaptureError> {
    block_on(async {
        let connection = zbus::Connection::session()
            .await
            .map_err(|error| CaptureError::Dbus(error.to_string()))?;
        let reply = connection
            .call_method(
                Some(DISPLAY_CONFIG_NAME),
                DISPLAY_CONFIG_PATH,
                Some(DISPLAY_CONFIG_NAME),
                "GetCurrentState",
                &(),
            )
            .await
            .map_err(|error| CaptureError::Dbus(error.to_string()))?;
        let (_serial, monitors, logical_monitors, _properties): (
            u32,
            Vec<DisplayMonitor>,
            Vec<LogicalMonitor>,
            HashMap<String, zbus::OwnedVariant>,
        ) = reply
            .body()
            .deserialize()
            .map_err(|error| CaptureError::Dbus(error.to_string()))?;

        let mut bounds: Option<(i32, i32, i32, i32)> = None;
        for (x, y, scale, transform, _primary, specs, _properties) in logical_monitors {
            let Some(spec) = specs.first() else {
                continue;
            };
            let Some((_, modes, _)) = monitors
                .iter()
                .find(|(candidate, _, _)| candidate.0 == spec.0)
            else {
                continue;
            };
            let Some(mode) = modes
                .iter()
                .find(|mode| is_current_mode(mode))
                .or_else(|| modes.first())
            else {
                continue;
            };
            let scale = if scale > 0.0 { scale } else { 1.0 };
            let mut width = (f64::from(mode.1) / scale).round() as i32;
            let mut height = (f64::from(mode.2) / scale).round() as i32;
            if transform == 1 || transform == 3 {
                std::mem::swap(&mut width, &mut height);
            }
            let right = x.saturating_add(width.max(1));
            let bottom = y.saturating_add(height.max(1));
            bounds = Some(match bounds {
                Some((left, top, current_right, current_bottom)) => (
                    left.min(x),
                    top.min(y),
                    current_right.max(right),
                    current_bottom.max(bottom),
                ),
                None => (x, y, right, bottom),
            });
        }

        let (left, top, right, bottom) = bounds.ok_or_else(|| {
            CaptureError::Unavailable("Mutter did not report logical monitors".into())
        })?;
        Ok((
            left,
            top,
            u32::try_from(right.saturating_sub(left).max(1)).unwrap_or(1),
            u32::try_from(bottom.saturating_sub(top).max(1)).unwrap_or(1),
        ))
    })
}

fn is_current_mode(mode: &DisplayMode) -> bool {
    mode.6
        .get("is-current")
        .and_then(|value| value.downcast_ref::<bool>().ok())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::frame_from_buffer;
    use cosnip_core::Frame;
    use pipewire::spa;
    use pipewire::spa::param::video::{VideoFormat, VideoInfoRaw};

    #[test]
    fn converts_bgrx_to_rgba() {
        let mut info = VideoInfoRaw::default();
        info.set_format(VideoFormat::BGRx);
        info.set_size(spa::utils::Rectangle {
            width: 2,
            height: 1,
        });
        let bytes = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let frame = frame_from_buffer(&info, &bytes, 8).unwrap();
        assert_eq!(frame.pixel(0, 0), Some([3, 2, 1, 255]));
        assert_eq!(frame.pixel(1, 0), Some([7, 6, 5, 255]));
    }

    #[test]
    fn rejects_short_buffer() {
        let info = VideoInfoRaw::default();
        assert!(frame_from_buffer(&info, &[0; 4], 8).is_err());
    }

    #[allow(dead_code)]
    fn _frame_is_send(_: Frame) {}
}
