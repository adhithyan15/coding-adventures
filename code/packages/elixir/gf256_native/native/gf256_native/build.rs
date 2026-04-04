// build.rs — linker flags for the Elixir/Erlang gf256_native NIF
//
// Erlang NIFs (.so/.bundle/.dll) are loaded by the Erlang runtime via
// erlang:load_nif/2. The NIF C API symbols (enif_make_int, enif_get_uint,
// etc.) are provided by the ERTS (Erlang RunTime System), not by a library
// we link against at build time.
//
// - macOS: `-undefined dynamic_lookup` — resolve ERTS symbols at load time
// - Linux: ELF shared objects allow undefined symbols by default
// - Windows: Link against the ERTS DLL (complex — not yet implemented)

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            // Apple ld requires an explicit flag to allow undefined symbols.
            // The ERTS symbols (enif_make_int, etc.) are resolved when
            // Erlang calls dlopen() on our .so.
            println!("cargo:rustc-cdylib-link-arg=-undefined");
            println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
        }
        _ => {
            // Linux/Unix: ELF allows undefined symbols by default.
            // Windows: NIF support on Windows is not yet implemented.
        }
    }
}
