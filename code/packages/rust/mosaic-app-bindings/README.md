# mosaic-app-bindings

`mosaic-app-bindings` owns package-independent host-language bindings for the
stable `mosaic-app-capi` ABI. Artifact emitters install these sources into native
project shells so applications do not carry handwritten reducers or FFI adapters.

The first binding is Compose/JVM. It uses JNA to load the final Rust application
library, owns the opaque runtime handle and returned buffers, supplies the native
startup context, sequences semantic events, and returns decoded updates to the
generated Compose view.

Set the `mosaic.app.library` JVM property or `MOSAIC_APP_LIBRARY` environment
variable to a library name or absolute path. The conventional fallback name is
`mosaic_app`.
