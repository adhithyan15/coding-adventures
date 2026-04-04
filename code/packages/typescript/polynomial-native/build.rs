// build.rs — Linker flags for the polynomial-native Node.js N-API addon
// ======================================================================
//
// Node.js N-API addons are cdylib shared libraries loaded by Node.js via
// `require()`. The N-API symbols (napi_create_double, etc.) are provided
// by the Node.js process at runtime, not by a library at link time.
//
// - macOS: `-undefined dynamic_lookup` — defers symbol resolution to dlopen()
//   Without this flag, Apple's ld refuses to link because N-API symbols are
//   not found in any library at link time.
//
// - Linux: Nothing needed — ELF shared objects allow undefined symbols.
//
// - Windows: Handled via BUILD_windows skip (see BUILD_windows in this dir).

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "macos" {
        println!("cargo:rustc-cdylib-link-arg=-undefined");
        println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
    }
}
