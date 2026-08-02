# Changelog

## Unreleased

- Add the native Venture session bridge used by the Mosaic-generated WinUI
  shell, including shared chrome props/events, Direct2D pixel rendering,
  scrolling, and link activation.
- Accept the same host-neutral semantic keyboard-scroll commands as the macOS
  bridge for package-owned WinUI content-surface input.
- Reflow the retained page through the shared Rust pipeline when the generated
  WinUI content surface changes logical size.
- Build and directly launch the Mosaic-emitted WinUI app in a platform-native
  integration gate, requiring a successful Direct2D host-surface render.
- Copy rendered BGRA pixels through WinRT's supported `IBuffer.AsStream()`
  projection and retain opt-in launch-phase diagnostics for hosted acceptance.
- Extend the generated-app launch gate through WinUI value and invoke
  automation providers, requiring the shared Rust session to load and title a
  second deterministic page.
