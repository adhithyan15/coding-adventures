# Changelog

## Unreleased

- Extend the generated WinUI direct-launch gate through the content surface's
  pointer-coordinate activation path after restoring the scrolled viewport to
  document start.
- Exercise the generated WinUI content surface's End-key mapping against a
  scrollable live page and require the shared Rust viewport to change.

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
- Extend the generated-app launch gate through the native WinUI address
  control and button invoke provider, requiring the shared Rust session to
  load and title a second deterministic page.
- Extend direct generated-app interaction acceptance through the native WinUI
  Back and Forward invoke providers, requiring both history entries to reload
  through the shared Rust session before the gate succeeds.
- Extend that direct acceptance through the native WinUI Reload and Home
  invoke providers, requiring changed reload content and shared-session home
  navigation before the generated shell reports success.
