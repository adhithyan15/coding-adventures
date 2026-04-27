// build.rs -- napi-rs build script
// =================================
//
// This is a required build script for napi-rs native addons. It generates
// the C header file and linker configuration that Node.js needs to load
// the shared library as a native addon.
//
// Without this, the compiled .so/.dylib/.dll won't have the right symbol
// exports and Node.js won't recognize it as a valid addon.

extern crate napi_build;

fn main() {
    napi_build::setup();
}
