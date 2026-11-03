// build.rs — linker flags for the Perl Conduit XS extension
//
// Perl extension modules (.so/.bundle/.dll) are loaded by the Perl DynaLoader
// at runtime via `use Module`. The Perl C API symbols (Perl_sv_setiv,
// PL_stack_sp, etc.) are provided by the interpreter process.
//
// - macOS: `-undefined dynamic_lookup` — resolve symbols at load time
// - Linux: ELF shared objects allow undefined symbols by default
// - Windows: perl-bridge supplies the selected interpreter's CORE import library
//
// Threaded-Perl-sensitive stack access now lives in `perl-bridge`, which
// compiles a small shim against the host Perl headers.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "macos" {
        // Apple ld resolves Perl's host-provided symbols when DynaLoader opens
        // the extension. Windows import-library linkage belongs to perl-bridge.
        println!("cargo:rustc-cdylib-link-arg=-undefined");
        println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
    }
}
