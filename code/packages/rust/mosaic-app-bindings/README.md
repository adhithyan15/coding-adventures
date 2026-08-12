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
`mosaic_app`. Generated Compose native distributions add an app-relative lookup
before that conventional fallback: `compose.application.resources.dir` plus
`libmosaic_app.dylib`, `libmosaic_app.so`, or `mosaic_app.dll`.
Generated Qt applications likewise check `QCoreApplication::applicationDirPath()`
for the conventional target filename before global lookup, so the CMake build and
install trees can carry the selected Rust engine without an environment override.
Generated XAML applications check `AppContext.BaseDirectory` for
`mosaic_app.dll` before global lookup, matching the WinUI project's native DLL
copy target.
Linux CI compiles the exact Compose/JNA binding from a generated strict package,
verifies the selected `mosaic-app-conformance` library was installed in that
resource directory byte-for-byte, then exercises startup, semantic dispatch,
snapshot/restore, notification, buffer ownership, and teardown without
`MOSAIC_APP_LIBRARY`.
The Qt lane performs the equivalent byte-for-byte install check, launches the
generated QML application, and runs the full standard-binding conformance binary
from the install directory with `MOSAIC_APP_LIBRARY` unset.
The Windows lane verifies the selected engine copied beside the generated WinUI
executable, then runs the exact .NET binding from that app-relative directory
with the override removed.

Generated SwiftUI packages copy the selected application dylib into SwiftPM's
`Runtime` resource bundle and pass its `Bundle.module` path to the standard
loader. `MOSAIC_APP_LIBRARY` remains the explicit development override. Without
either path the loader checks symbols linked into the process, then tries
`libmosaic_app.dylib` and `mosaic_app.dylib`.
Strict generated shells call `MosaicRuntimeHost.loadRequired()` so a missing
Rust library fails at startup rather than silently entering preview mode.
macOS CI verifies that the selected `mosaic-app-conformance` dylib reaches the
SwiftPM resource bundle byte-for-byte, then compiles the exact binding and C
loader and verifies startup, semantic dispatch, snapshot/restore, notification,
buffer ownership, and teardown without `MOSAIC_APP_LIBRARY`.

For XAML, set `MOSAIC_APP_LIBRARY` to the application DLL path or place
`mosaic_app.dll` beside the emitted project. The project copies native DLLs next
to the unpackaged WinUI executable during its build.
Windows CI compiles the exact generated binding with the shared
`mosaic-app-conformance` DLL and verifies real startup, prop projection,
semantic dispatch, revision application, buffer ownership, and teardown.

Generated Flutter distributions register the selected application library as a
bundled Dart code asset. `@Native` resolves the packaged framework or dynamic
library without platform-specific paths; `MOSAIC_APP_LIBRARY` remains an
explicit development override. Unbundled permissive projects still check
symbols linked into the process and the platform's conventional `mosaic_app`
dynamic-library names.
Strict generated shells call `MosaicHost.loadRequired()` so a missing Rust
library fails at startup rather than silently entering preview mode.
Linux CI packages the shared `mosaic-app-conformance` library into a complete
generated app, verifies the installed code asset, and runs the exact binding
without `MOSAIC_APP_LIBRARY`, covering startup, semantic dispatch,
snapshot/restore, notification, buffer ownership, and teardown.

For Qt/QML, set `MOSAIC_APP_LIBRARY` to the application library path or package
it under the platform's conventional `mosaic_app` name. The generated QObject
host uses only Qt Core APIs and is installed automatically by the artifact builder.
Linux CI compiles the exact host from a complete generated TaskApp project with
the shared `mosaic-app-conformance` library, then verifies startup, semantic
dispatch, snapshot/restore, buffer ownership, and teardown without a display.
