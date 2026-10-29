//! Manual X11 capture test. Run under Xvfb:
//! `cargo test -p cosnip-capture --test x11_capture -- --ignored`
//! with `DISPLAY=:99 XDG_SESSION_TYPE=x11`.

use std::time::Duration;

use cosnip_capture::{CaptureBackend, x11::X11Backend};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, CreateWindowAux, WindowClass};

#[test]
#[ignore = "requires a running Xvfb/Xorg display"]
fn captures_a_known_window() {
    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let screen = &conn.setup().roots[screen_num];
    let window = conn.generate_id().unwrap();
    conn.create_window(
        screen.root_depth,
        window,
        screen.root,
        10,
        10,
        100,
        100,
        0,
        WindowClass::INPUT_OUTPUT,
        screen.root_visual,
        &CreateWindowAux::new().background_pixel(0x00ff0000),
    )
    .unwrap();
    conn.map_window(window).unwrap();
    conn.flush().unwrap();
    std::thread::sleep(Duration::from_millis(200));

    let frame = X11Backend.capture().unwrap();
    let pixel = frame.pixel(50, 50).unwrap();
    assert!(
        pixel[0] > 200 && pixel[1] < 50 && pixel[2] < 50,
        "{pixel:?}"
    );
    conn.destroy_window(window).unwrap();
    conn.flush().unwrap();
}
