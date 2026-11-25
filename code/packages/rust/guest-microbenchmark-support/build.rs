use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=RUSTC");
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let output = Command::new(rustc)
        .arg("--version")
        .output()
        .expect("could not query the selected Rust compiler");
    assert!(output.status.success(), "selected rustc --version failed");
    let version = String::from_utf8(output.stdout).expect("rustc version output must be UTF-8");
    let version = version.trim_end_matches(['\r', '\n']);
    assert!(version.starts_with("rustc ") && !version.contains(['\r', '\n', '\0']));
    println!("cargo:rustc-env=CA_GUEST_RUSTC_VERSION={version}");
}
