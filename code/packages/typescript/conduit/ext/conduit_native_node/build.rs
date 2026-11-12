// build.rs — Linker flags for the conduit_native_node N-API addon
//
// N-API symbols (napi_create_string_utf8, napi_call_function, etc.) are
// provided by the Node.js process at dlopen() time, not by a static lib.
// We need platform-specific linker flags to allow this unresolved-at-link-time
// pattern.
//
// This file mirrors node-bridge/build.rs; both must agree on the approach.

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            // Apple ld refuses to link if symbols are undefined at link time.
            // `-undefined dynamic_lookup` defers resolution to dlopen().
            println!("cargo:rustc-cdylib-link-arg=-undefined");
            println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
        }
        _ => {
            // Linux ELF: undefined symbols in shared objects are allowed by
            // default — the dynamic linker resolves them when Node.js loads
            // the .node addon.
            //
            // Windows: a PE/COFF DLL may not leave imports undefined, so the
            // napi_* symbols are imported from node.exe through node-bridge's
            // `raw-dylib` table (see node-bridge/src/lib.rs); no node.lib needed.
        }
    }
}
