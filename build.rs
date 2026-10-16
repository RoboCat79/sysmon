use std::process::Command;

fn main() {
    let output = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .unwrap_or_default();

    let version = output
        .split_whitespace()
        .nth(1)
        .unwrap_or("Unknown");

    println!("cargo:rustc-env=RUSTC_VERSION={}", version);
}