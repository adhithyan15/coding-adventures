fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=SIR_TEST_RUSTC_LINKER");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let linker = std::env::var("SIR_TEST_RUSTC_LINKER")
            .ok()
            .filter(|value| {
                !value.is_empty() && !value.chars().any(|ch| matches!(ch, '\r' | '\n'))
            })
            .unwrap_or_else(|| "rust-lld".to_string());
        println!("cargo:rustc-env=SIR_TEST_RUSTC_LINKER={linker}");
    }
}
