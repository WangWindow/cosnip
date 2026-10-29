//! Lossless GNOME Wayland capture through Mutter ScreenCast + PipeWire.
//!
//! The Shell screenshot APIs are restricted on GNOME 49. Mutter's ScreenCast API is available to
//! the user session: create a session, record the area, start it, receive the PipeWire node id,
//! and read raw `video/x-raw` frames. This is the same media path used by GNOME's own recorder,
//! without the H.264 encoding step.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

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
    CaptureBackend, CaptureBackendKind, CaptureError, desktop_is_gnome, session_is_wayland,
};

type FrameSender = Sender<Result<Frame, String>>;
type FrameReceiver = Receiver<Result<Frame, String>>;

const MUTTER_NAME: &str = "org.gnome.Mutter.ScreenCast";
const MUTTER_PATH: &str = "/org/gnome/Mutter/ScreenCast";
const SESSION_IFACE: &str = "org.gnome.Mutter.ScreenCast.Session";
const STREAM_IFACE: &str = "org.gnome.Mutter.ScreenCast.Stream";

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
        self.is_available()?;
        let info = crate::screen_info()?;
        let session = match MutterSession::start(info.width, info.height) {
            Ok(session) => session,
            Err(error) => {
                eprintln!("cosnip: Mutter ScreenCast failed ({error}); trying Screencast fallback");
                return crate::gnome::GnomeScreencast.capture();
            }
        };
        let result = pipewire_capture(session.node, info.width, info.height);
        session.stop();
        match result {
            Ok(frame) => Ok(frame),
            Err(error) => {
                eprintln!("cosnip: PipeWire capture failed ({error}); trying Screencast fallback");
                crate::gnome::GnomeScreencast.capture()
            }
        }
    }
}

struct MutterSession {
    connection: zbus::Connection,
    path: String,
    node: u32,
}

impl MutterSession {
    fn start(width: u32, height: u32) -> Result<Self, CaptureError> {
        let (connection, path, node) = block_on(async {
            let connection = zbus::Connection::session()
                .await
                .map_err(|e| CaptureError::Dbus(e.to_string()))?;
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
                .map_err(|e| CaptureError::Dbus(e.to_string()))?;
            let session_path: zbus::OwnedObjectPath = reply
                .body()
                .deserialize()
                .map_err(|e| CaptureError::Dbus(e.to_string()))?;
            let session = zbus::Proxy::new(
                &connection,
                MUTTER_NAME,
                session_path.as_str(),
                SESSION_IFACE,
            )
            .await
            .map_err(|e| CaptureError::Dbus(e.to_string()))?;

            let mut options: HashMap<String, zbus::Variant<'static>> = HashMap::new();
            options.insert("is-recording".into(), zbus::Variant::from(false));
            options.insert("cursor-mode".into(), zbus::Variant::from(0u32));
            let record = session
                .call_method(
                    "RecordArea",
                    &(0i32, 0i32, width as i32, height as i32, options),
                )
                .await
                .map_err(|e| CaptureError::Dbus(e.to_string()))?;
            let stream_path: zbus::OwnedObjectPath = record
                .body()
                .deserialize()
                .map_err(|e| CaptureError::Dbus(e.to_string()))?;
            let stream =
                zbus::Proxy::new(&connection, MUTTER_NAME, stream_path.as_str(), STREAM_IFACE)
                    .await
                    .map_err(|e| CaptureError::Dbus(e.to_string()))?;
            let mut signals = stream
                .receive_signal("PipeWireStreamAdded")
                .await
                .map_err(|e| CaptureError::Dbus(e.to_string()))?;

            session
                .call_method("Start", &())
                .await
                .map_err(|e| CaptureError::Dbus(e.to_string()))?;

            let message = futures_lite::future::or(signals.next(), async {
                async_io::Timer::after(Duration::from_secs(3)).await;
                None
            })
            .await
            .ok_or(CaptureError::Timeout)?;
            let node: u32 = message
                .body()
                .deserialize()
                .map_err(|e| CaptureError::Dbus(e.to_string()))?;
            Ok::<_, CaptureError>((connection, session_path.to_string(), node))
        })?;
        Ok(Self {
            connection,
            path,
            node,
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

struct PipewireUserData {
    format: VideoInfoRaw,
    tx: Option<FrameSender>,
    mainloop: pw::main_loop::MainLoopRc,
}

fn pipewire_capture(node: u32, width: u32, height: u32) -> Result<Frame, CaptureError> {
    pw::init();
    let mainloop = pw::main_loop::MainLoopRc::new(None)
        .map_err(|e| CaptureError::Unavailable(format!("PipeWire main loop: {e}")))?;
    let context = pw::context::ContextRc::new(&mainloop, None)
        .map_err(|e| CaptureError::Unavailable(format!("PipeWire context: {e}")))?;
    let core = context
        .connect_rc(None)
        .map_err(|e| CaptureError::Unavailable(format!("PipeWire core: {e}")))?;
    let (tx, rx): (FrameSender, FrameReceiver) = mpsc::channel();
    let data = PipewireUserData {
        format: VideoInfoRaw::default(),
        tx: Some(tx),
        mainloop: mainloop.clone(),
    };
    let stream = pw::stream::StreamBox::new(
        &core,
        "cosnip-capture",
        pw::properties::properties! {
            *pw::keys::MEDIA_TYPE => "Video",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Screen",
        },
    )
    .map_err(|e| CaptureError::Unavailable(format!("PipeWire stream: {e}")))?;

    let _listener = stream
        .add_local_listener_with_user_data(data)
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
            if data.tx.is_none() {
                return;
            }
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let Some(d) = buffer.datas_mut().first_mut() else {
                return;
            };
            let chunk = d.chunk();
            let stride = chunk.stride().unsigned_abs() as usize;
            let Some(bytes) = d.data() else {
                let _ = data
                    .tx
                    .take()
                    .unwrap()
                    .send(Err("PipeWire buffer is not CPU-mappable".into()));
                data.mainloop.quit();
                return;
            };
            let result = frame_from_buffer(&data.format, bytes, stride);
            if let Some(tx) = data.tx.take() {
                let _ = tx.send(result);
            }
            data.mainloop.quit();
        })
        .state_changed(|_, data, _old, new| {
            if let pw::stream::StreamState::Error(message) = new {
                eprintln!("cosnip: PipeWire stream error: {message}");
                if let Some(tx) = data.tx.take() {
                    let _ = tx.send(Err(message.to_string()));
                }
                data.mainloop.quit();
            }
        })
        .register()
        .map_err(|e| CaptureError::Unavailable(format!("PipeWire listener: {e}")))?;

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
                width: 8192,
                height: 8192
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
    .map_err(|e| CaptureError::Decode(format!("PipeWire format: {e}")))?;
    let bytes = values.0.into_inner();
    let mut params = [Pod::from_bytes(&bytes)
        .ok_or_else(|| CaptureError::Decode("invalid PipeWire format pod".into()))?];

    stream
        .connect(
            spa::utils::Direction::Input,
            Some(node),
            pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
            &mut params,
        )
        .map_err(|e| CaptureError::Unavailable(format!("PipeWire connect: {e}")))?;

    let weak = mainloop.downgrade();
    let timer = mainloop.loop_().add_timer(move |_| {
        if let Some(mainloop) = weak.upgrade() {
            mainloop.quit();
        }
    });
    let _ = timer.update_timer(Some(Duration::from_secs(3)), None);

    mainloop.run();
    drop(timer);
    match rx.try_recv() {
        Ok(Ok(frame)) => Ok(frame),
        Ok(Err(message)) => Err(CaptureError::Decode(message)),
        Err(_) => Err(CaptureError::Timeout),
    }
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
    Frame::new(width, height, rgba).map_err(|e| e.to_string())
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
