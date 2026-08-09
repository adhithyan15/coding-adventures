# mosaic-app-bindings

`mosaic-app-bindings` owns package-independent host-language bindings for the
stable `mosaic-app-capi` ABI. Artifact emitters install these sources into native
project shells so applications do not carry handwritten reducers or FFI adapters.

The Compose/JVM binding uses JNA to load the final Rust application library. The
SwiftUI binding uses a generated C dynamic-loader target and a Foundation host.
Both own the opaque runtime handle and returned buffers, supply the native startup
context, sequence semantic events, and return decoded updates to the generated
view.

Set the `mosaic.app.library` JVM property or `MOSAIC_APP_LIBRARY` environment
variable to a library name or absolute path. The conventional fallback name is
`mosaic_app`.

For SwiftUI, set `MOSAIC_APP_LIBRARY` to the application dylib path. Without an
explicit path the loader first checks symbols linked into the process, then tries
`libmosaic_app.dylib` and `mosaic_app.dylib`.
