// build.rs — Linker flags for Node.js N-API native addons
// =========================================================
//
// Node.js N-API addons (.node files) are shared libraries loaded by Node.js
// at runtime via `require()`. The N-API symbols (napi_create_string_utf8,
// napi_get_undefined, etc.) are provided by the Node.js process, not by a
// library we link against at compile time.
//
// Platform-specific linker handling:
//
// - macOS: `-undefined dynamic_lookup` — Apple's ld defers symbol resolution
//   to dlopen() time. Without this flag, ld refuses to link because the N-API
//   symbols are not found at link time (they live in node, not in any .dylib).
//
// - Linux: Nothing needed — ELF shared objects allow undefined symbols by
//   default. The dynamic linker resolves them when Node.js calls dlopen().
//
// - Windows: N-API symbols come from node.exe at runtime, and a PE/COFF DLL
//   must name the module behind every import.  src/lib.rs marks its
//   `extern "C"` block `#[link(name = "node.exe", kind = "raw-dylib")]`, so
//   rustc writes that import table itself — no node.lib, nothing for this
//   script to find or download.

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            // macOS linker requires explicit flag to allow undefined symbols.
            // N-API symbols are resolved when Node.js loads our .node addon.
            println!("cargo:rustc-cdylib-link-arg=-undefined");
            println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
        }
        _ => {
            // Linux: ELF allows undefined symbols in shared objects by default.
            // Windows: handled by the raw-dylib `#[link]` in src/lib.rs.
        }
    }
}
