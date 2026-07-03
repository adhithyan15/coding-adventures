// build.rs — compile dart_api_dl.c so Rust can call Dart_PostCObject_DL.
//
// dart_api_dl.c is the "Dynamic Linking" shim from the Dart SDK. It provides
// the _DL-suffixed variants of the Dart C API (e.g. Dart_PostCObject_DL) by
// storing function pointers that are filled in at runtime when the host calls
// Dart_InitializeApiDL(NativeApi.initializeApiDLData).
//
// We compile it as a tiny C file and link it into our dylib.
fn main() {
    // Compile dart_api_dl.c (the Dart SDK DL shim) together with our C
    // bridge shim into one static archive. Both need dart_api_dl.h, so we
    // compile them in the same cc::Build invocation with the same include dir.
    cc::Build::new()
        .file("dart/dart_api_dl.c")
        .file("dart/conduit_dart_bridge.c")
        .include("dart")
        .compile("dart_api_dl");

    println!("cargo:rerun-if-changed=dart/dart_api_dl.c");
    println!("cargo:rerun-if-changed=dart/conduit_dart_bridge.c");
    println!("cargo:rerun-if-changed=dart/dart_api_dl.h");
}
