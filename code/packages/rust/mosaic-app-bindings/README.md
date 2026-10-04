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

## Platform libraries (UI87 §7)

Beside each runtime binding, this crate ships the operating-system
capabilities every generated app gets, so no app carries its own copy. The
first is Compose's, in two files. `MosaicFileEffects.kt`
(`compose_file_effects()`) is the half every Compose target shares: the rules
a request must meet, the MIME table, a router that sends each effect to the
app's own `[host_effects]` handler or to this library by kind, and the
asynchronous path from a picker's answer to the effect's outcome (UI89 §3.8).
`MosaicPlatformEffects.kt` (`compose_platform_effects()`) is the desktop's:
`files.open` and `files.save` through the native file dialog. Both are tested
with fake dialogs and fake pickers by
`conformance/compose/MosaicPlatformEffectsTest.kt`.

SwiftUI's `MosaicPlatformEffects.swift` (`swift_platform_effects()`) answers
the same contract through `NSOpenPanel` / `NSSavePanel` on macOS and
`UIDocumentPickerViewController` on iOS and iPadOS (UI89 §3.8), behind the
same asynchronous picker seam as Compose. It is tested with a fake host, fake
panels and fake pickers by
`conformance/swiftui/Sources/Conformance/PlatformEffectsChecks.swift`
(`--platform-effects`).

Qt's `MosaicPlatformEffects.{h,cpp}` (`qt_platform_effects()`) answers the same
contract through `QFileDialog`. The Qt host delivers effects through one
routed handler (`MosaicHost::setEffectHandler`) that the library's router
occupies, falling back to the `effectRequested` signal for the app's own kinds
(UI87 §7.4a). It is tested headless, with fake dialogs, by
`tests/qt_effect_driver`.

XAML's `MosaicPlatformEffects.cs` (`xaml_platform_effects(namespace)`) answers
the same contract through WinUI 3's `FileOpenPicker` / `FileSavePicker`, owned
by the app's window. Each standard effect is deferred and answered from the
window's `DispatcherQueue`, because the static host runs its handler inside
the settle (UI87 §7.6). The WinUI half sits behind `#if !MOSAIC_HEADLESS_TEST`,
so `conformance/xaml-platform-effects/` runs everything else on plain .NET
with a fake host and a fake picker; `tests/xaml_platform_effects.rs` builds
and runs that harness wherever `dotnet` is installed.

Flutter's library (`flutter_platform_effects()`, UI87 §7.7) is two files,
because Dart has no conditional compilation: `mosaic_platform_effects_core.dart`
is the contract, the file I/O and the router in plain Dart, and
`mosaic_platform_effects.dart` adds `package:file_selector`'s native dialogs
(Linux, macOS, Windows) and `installMosaicPlatformEffects(host, appKinds:)`,
which the generated `main.dart` calls. Each standard effect is deferred, its
dialog run after the settle, and the file read or written in a background
isolate; the router holds the `MosaicHost` it was installed on, so a late
answer after a retried start meets the disposed host and is dropped. Every
generated project depends on `file_selector` pinned exactly
(`FLUTTER_FILE_SELECTOR_VERSION`, added by
`flutter_pubspec_with_platform_effects`); a package's `[host_assets]`
coordinate for a package the project already declares is left out rather
than duplicated. `conformance/flutter-platform-effects/` runs the core on the
plain Dart VM with a fake host and fake dialogs, and
`tests/flutter_platform_effects.rs` runs it wherever `dart` is installed. On
Linux the library asks "Replace it?" before a save goes onto an existing name,
because GTK's chooser (as `file_selector` opens it) does not;
`conformance/flutter-replace-dialog/` drives that dialog with the widget
tester. On Android and iOS each request fails with a message until UI89.

## Persistence

Emitted applications also persist the runtime's opaque snapshot after every
successful dispatch whose revision the state file does not already hold (the
first answer after launch always writes), and supply it as
`restoredSnapshot` before the first visible render. An environment report the
app ignored (UI48 §7.1: an answer at the revision already showing, and so
already saved) changed nothing the app saves, so it writes nothing -- a window
drag costs no disk writes -- unless an earlier save failed: while a `persistenceWarning` is
pending, each ignored report retries the save (UI48 §7.12). Writes use a same-directory temporary file plus the platform's atomic
replacement facility. Invalid JSON and runtime-incompatible snapshots are moved
to `mosaic-state.v1.json.corrupt`; the app starts clean and exposes a
`persistenceWarning` (or native status warning) instead of becoming unusable.
Set `MOSAIC_APP_STATE_PATH` to an explicit file for tests or portable launches.
Otherwise the generated application id selects these per-user locations:

| Host | Default state location |
| --- | --- |
| Compose | `%LOCALAPPDATA%/<app-id>/mosaic-state.v1.json` on Windows, `~/Library/Application Support/<app-id>/...` on macOS, `$XDG_DATA_HOME/<app-id>/...` or `~/.local/share/<app-id>/...` on Linux |
| SwiftUI | Foundation's user Application Support directory, then `<app-id>/mosaic-state.v1.json` |
| XAML | `Environment.SpecialFolder.LocalApplicationData/<app-id>/mosaic-state.v1.json` |
| Flutter | the same Windows/macOS/Linux roots as Compose, derived without a plugin |
| Qt | `QStandardPaths::AppDataLocation/<app-id>/mosaic-state.v1.json` |

The unscoped binding helpers used by ABI conformance remain ephemeral. Artifact
generation calls the application-scoped helpers, so independent apps never
share state accidentally.

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
