//! Manual integration smoke test. Run with a running Xvfb:
//! `DISPLAY=:99 XDG_SESSION_TYPE=x11 cargo test -p cosnip-app --test xvfb_smoke -- --ignored`

use std::process::Command;

use cosnip_imaging::decode_png;

#[test]
#[ignore = "requires a running Xvfb/Xorg display"]
fn area_snip_saves_exact_file() {
    let Some(display) = std::env::var_os("DISPLAY") else {
        return;
    };
    let path = std::env::temp_dir().join(format!("cosnip-xvfb-{}.png", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let status = Command::new(env!("CARGO_BIN_EXE_cosnip"))
        .env("DISPLAY", display)
        .env("XDG_SESSION_TYPE", "x11")
        .env_remove("WAYLAND_DISPLAY")
        .env("COSNIP_TEST_WINDOWED_OVERLAY", "1")
        .env("COSNIP_TEST_AUTOCONFIRM_MS", "800")
        .args([
            "snip",
            "--area",
            "100",
            "100",
            "500",
            "300",
            "--save",
            path.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let bytes = std::fs::read(&path).unwrap();
    let frame = decode_png(&bytes).unwrap();
    assert_eq!((frame.width(), frame.height()), (500, 300));
    let _ = std::fs::remove_file(path);
}
