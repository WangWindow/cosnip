use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_cosnip")
}

#[test]
fn version_is_printed() {
    let output = Command::new(bin()).arg("--version").output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("cosnip"));
}

#[test]
fn help_lists_snip() {
    let output = Command::new(bin()).arg("--help").output().unwrap();
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("snip"));
    assert!(text.contains("doctor"));
}

#[test]
fn conflicting_area_and_full_fail() {
    let output = Command::new(bin())
        .args(["snip", "--area", "0", "0", "10", "10", "--full"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("mutually exclusive"));
}

#[test]
fn invalid_area_fails() {
    let output = Command::new(bin())
        .args([
            "snip",
            "--area",
            "0",
            "0",
            "0",
            "10",
            "--save",
            "/tmp/x.png",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}
