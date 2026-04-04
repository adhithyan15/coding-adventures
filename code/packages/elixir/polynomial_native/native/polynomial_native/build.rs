// build.rs — linker flags for the Elixir/Erlang polynomial_native NIF
//
// Erlang NIFs are loaded by the ERTS (Erlang RunTime System) via
// erlang:load_nif/2. The NIF C API symbols are provided by ERTS at runtime.
//
// - macOS: `-undefined dynamic_lookup` — resolve ERTS symbols at load time
// - Linux: ELF allows undefined symbols by default
// - Windows: not yet implemented

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            println!("cargo:rustc-cdylib-link-arg=-undefined");
            println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
        }
        _ => {}
    }
}
