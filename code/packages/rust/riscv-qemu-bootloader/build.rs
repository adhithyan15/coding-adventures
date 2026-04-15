use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=linker.ld");
    println!("cargo:rerun-if-changed=src/entry.S");
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let linker_script = manifest_dir.join("linker.ld");
    println!("cargo:rustc-link-arg=-T{}", linker_script.display());
}
