//! Global pointer position and button state for override-redirect pin dragging.

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt as _, KeyButMask};

#[must_use]
pub fn global_pointer() -> Option<(i32, i32)> {
    let (conn, screen_num) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots[screen_num].root;
    let reply = conn.query_pointer(root).ok()?.reply().ok()?;
    Some((i32::from(reply.root_x), i32::from(reply.root_y)))
}

#[must_use]
pub fn primary_button_down() -> bool {
    let Ok((conn, screen_num)) = x11rb::connect(None) else {
        return false;
    };
    let root = conn.setup().roots[screen_num].root;
    let Ok(cookie) = conn.query_pointer(root) else {
        return false;
    };
    let Ok(reply) = cookie.reply() else {
        return false;
    };
    reply.mask.contains(KeyButMask::BUTTON1)
}
