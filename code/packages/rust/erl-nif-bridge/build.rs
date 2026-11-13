// build.rs -- link flags for the nif_smoke example (a NIF library).
//
// A NIF library is loaded by `erlang:load_nif/2`, and the `enif_*` symbols it
// calls come from the BEAM that loads it, so the library leaves them undefined.
// The library crate itself links nothing; this only matters to the example (and
// to a consumer whose own build.rs asks for the same, like elixir/gf256_native).
//
// - macOS: `-undefined dynamic_lookup`, because Apple ld rejects undefined
//   symbols in a dylib by default.
// - Linux: ELF shared objects may leave symbols undefined.
// - Windows: nothing to link.  A PE/COFF DLL cannot leave imports undefined and
//   beam.smp.dll exports no `enif_*` names, so `win_dyn` calls through the table
//   of function pointers the BEAM passes to `nif_init`.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "macos" {
        // `rustc-link-arg` (not `rustc-cdylib-link-arg`) so the unit-test
        // binary links too, should it ever keep a reference to enif_*.
        println!("cargo:rustc-link-arg=-undefined");
        println!("cargo:rustc-link-arg=dynamic_lookup");
    }
}
