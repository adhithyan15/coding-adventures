# mosaic-app-bindings

`mosaic-app-bindings` owns package-independent host-language bindings for the
stable `mosaic-app-capi` ABI. Artifact emitters install these sources into native
project shells so applications do not carry handwritten reducers or FFI adapters.

The Compose/JVM binding uses JNA to load the final Rust application library. The
SwiftUI binding uses a generated C dynamic-loader target and a Foundation host.
The XAML binding uses .NET's built-in `NativeLibrary` and `System.Text.Json` APIs.
The Flutter binding uses Dart FFI and the standard `ffi` allocation helper.
Qt/QML uses Qt Core's `QLibrary`, JSON, and variant APIs. All five own the opaque
runtime handle and returned buffers, supply the native startup context, sequence
semantic events, and return decoded updates to the generated view.

Set the `mosaic.app.library` JVM property or `MOSAIC_APP_LIBRARY` environment
variable to a library name or absolute path. The conventional fallback name is
`mosaic_app`.

For SwiftUI, set `MOSAIC_APP_LIBRARY` to the application dylib path. Without an
explicit path the loader first checks symbols linked into the process, then tries
`libmosaic_app.dylib` and `mosaic_app.dylib`.
macOS CI compiles the exact binding and C loader from a complete generated
TaskApp project with the shared `mosaic-app-conformance` dylib, then verifies
startup, semantic dispatch, snapshot/restore, notification, buffer ownership,
and teardown.

For XAML, set `MOSAIC_APP_LIBRARY` to the application DLL path or place
`mosaic_app.dll` beside the emitted project. The project copies native DLLs next
to the unpackaged WinUI executable during its build.
Windows CI compiles the exact generated binding with the shared
`mosaic-app-conformance` DLL and verifies real startup, prop projection,
semantic dispatch, revision application, buffer ownership, and teardown.

For Flutter, set `MOSAIC_APP_LIBRARY` to the application library path. Without
an explicit path the host checks symbols linked into the process, then the
platform's conventional `mosaic_app` dynamic-library names.
Linux CI runs the exact binding from a complete generated TaskApp project with
the shared `mosaic-app-conformance` library, verifying startup, semantic
dispatch, snapshot/restore, notification, buffer ownership, and teardown.

For Qt/QML, set `MOSAIC_APP_LIBRARY` to the application library path or package
it under the platform's conventional `mosaic_app` name. The generated QObject
host uses only Qt Core APIs and is installed automatically by the artifact builder.
Linux CI compiles the exact host from a complete generated TaskApp project with
the shared `mosaic-app-conformance` library, then verifies startup, semantic
dispatch, snapshot/restore, buffer ownership, and teardown without a display.
