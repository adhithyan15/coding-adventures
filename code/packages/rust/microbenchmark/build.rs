use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=RUSTC");
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let output = Command::new(&rustc)
        .arg("--version")
        .output()
        .unwrap_or_else(|error| panic!("could not query selected Rust compiler: {error}"));
    if !output.status.success() {
        panic!(
            "selected Rust compiler version query failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let raw = output.stdout.strip_suffix(b"\n").unwrap_or(&output.stdout);
    let raw = raw.strip_suffix(b"\r").unwrap_or(raw);
    if !raw.starts_with(b"rustc ") || raw.iter().any(|&byte| matches!(byte, b'\n' | b'\r' | 0)) {
        panic!("selected Rust compiler returned an invalid version string");
    }
    let version = String::from_utf8(raw.to_vec())
        .expect("selected Rust compiler returned non-UTF-8 version output");
    println!("cargo:rustc-env=CA_RUSTC_VERSION={version}");
}
