# venture-browser-qt

This crate is the thin Qt facade over `venture-browser-cairo`, which owns the
live shared browser session, C ABI compatibility exports, and Cairo RGBA
renderer used by Venture's Mosaic-generated Qt, Flutter, and Compose shells.
Mosaic remains the source of truth for browser chrome, while the Qt adapter
only loads the shared library into a `QQuickPaintedItem` host surface.

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
cargo test -p venture-browser-cairo -p venture-browser-qt
```
