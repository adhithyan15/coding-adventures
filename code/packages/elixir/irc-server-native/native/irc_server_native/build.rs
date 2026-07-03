// build.rs — linker flags for the Elixir/Erlang conduit_native NIF
//
// Erlang NIFs (.so/.bundle/.dll) are loaded by the BEAM via
// `:erlang.load_nif/2`. The `enif_*` symbols are provided by the running
// ERTS runtime, not by a library we link against at build time. We tell
// the linker to allow undefined symbols and resolve them at dlopen() time.
//
// - macOS: explicit `-undefined dynamic_lookup` flag for ld
// - Linux: ELF allows undefined symbols by default in shared objects
// - Windows: the BEAM exports erts.dll; not yet wired up here

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "macos" {
        println!("cargo:rustc-cdylib-link-arg=-undefined");
        println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
    }
}
