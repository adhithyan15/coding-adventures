# venture-browser-qt

This crate supplies the live page-content bridge used by Venture's
Mosaic-generated Qt Quick shell. Mosaic remains the source of truth for the
browser chrome; this library projects the shared `BrowserHostController`
through a C ABI and renders the retained page to RGBA pixels with Cairo for a
`QQuickPaintedItem` host surface.

The package-owned C++ adapter dynamically loads this library, registers the
surface as a QML type, and forwards generated Mosaic events and native surface
input to the same Rust session. Its direct-launch test emits and builds the
real Qt project when CMake and Qt6 are present, serves a deterministic page,
and requires a mounted Cairo frame before the application can report success.

```sh
cargo test -p venture-browser-qt
```
