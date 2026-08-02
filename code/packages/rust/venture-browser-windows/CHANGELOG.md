# Changelog

## Unreleased

- Extend the generated WinUI interaction gate through a native Enter message
  in the Mosaic-authored address field after the existing Go-button path.
- Require the generated WinUI interaction gate to verify programmatic focus on
  the hosted content surface before surface input.
- Require the generated WinUI resize gate to observe a successfully rendered
  bitmap whose pixel size changed after shared-session reflow.
- Extend the generated WinUI direct-launch gate through Alt-Left/Right history
  traversal after native link activation.
- Extend the generated WinUI direct-launch gate through the same wheel-delta
  reducer used by `PointerWheelChanged`, requiring shared viewport scroll.
- Extend the generated WinUI direct-launch gate through a real content-surface
  size change and require the package-owned adapter to report shared-session
  reflow.
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
