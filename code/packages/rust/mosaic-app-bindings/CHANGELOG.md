# Changelog

## Unreleased

- Execute the generated Flutter/Dart FFI host against the shared Rust
  conformance library in Linux CI, covering startup, dispatch,
  snapshot/restore, notification, buffer ownership, and teardown.
- Execute the generated Qt/QML host against the shared Rust conformance library
  in headless Linux CI, covering startup, dispatch, snapshot/restore, buffer
  ownership, and teardown through the real ABI.
- Execute the generated SwiftUI binding and C loader against the shared Rust
  conformance dylib in macOS CI, covering startup, dispatch, snapshot/restore,
  prop-change notification, buffer ownership, and teardown through the real ABI.
- Make the XAML Rust-runtime conformance executable independent of WinUI desktop
  initialization after the complete generated TaskApp has compiled against the
  real Windows App SDK, and bound the CI execution with an explicit timeout.
- Execute the generated XAML binding against the shared Rust conformance DLL in
  Windows CI, covering startup, initial props, dispatch, revised props, Rust
  buffer release, and teardown through the real ABI.
- Make the hosted-runner WinUI launch smoke test tolerate transient runtime
  startup failures while printing Windows application events when every real
  launch attempt fails.
- Add the package-independent Qt/QML binding using Qt Core dynamic loading,
  JSON, variants, and QObject invocation.
- Add the package-independent Flutter/Dart FFI binding and preserve the public
  injectable `MosaicHost` contract for tests and specialized packages.
- Add the package-independent XAML/.NET binding using built-in native loading
  and JSON support.
- Add the package-independent SwiftUI/Foundation binding and C dynamic loader.
- Own Swift application startup, successful event sequencing, snapshots, Rust
  buffers, and teardown through the same fixed C ABI as Compose.

## 0.1.0

- Add the package-independent Compose/JNA binding for the Mosaic C ABI.
- Manage native app handles and Rust-owned output buffers without app glue.
- Generate startup and event envelopes with the shared protocol version.
- Decode complete runtime updates into the generated Compose host contract.
