# venture-browser-qt

This crate supplies the live page-content bridge used by Venture's
Mosaic-generated Qt Quick shell. Mosaic remains the source of truth for the
browser chrome; this library projects the shared `BrowserHostController`
through a C ABI and renders the retained page to RGBA pixels with Cairo for a
`QQuickPaintedItem` host surface.

The crate also exports Flutter-named wrappers over that exact controller and
Cairo session. Venture's Dart FFI host uses those wrappers to consume RGBA
frames and native input without introducing a second browser implementation.

The package-owned C++ adapter dynamically loads this library, registers the
surface as a QML type, and forwards generated Mosaic events and native surface
input to the same Rust session. Its direct-launch test emits and builds the
real Qt project when CMake and Qt6 are present, serves a deterministic page,
and requires a mounted Cairo frame before the application can report success.
The same direct gate drives native Return through the generated address field,
Back/Forward through the emitted buttons, and wheel/hover/click input through
the real `QQuickPaintedItem`, verifying every transition through the shared
Rust session rather than a recording Qt host.

```sh
cargo test -p venture-browser-qt
```
