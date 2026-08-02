# Changelog

## Unreleased

- Require the generated SwiftUI resize gate to observe a successfully rendered
  Metal frame whose drawable size changed after shared-session reflow.
- Extend the generated SwiftUI direct-launch gate through Command-Left/Right
  history traversal after native link activation.
- Extend the generated SwiftUI direct-launch gate through a synthesized AppKit
  wheel event and require shared viewport scroll before keyboard acceptance.
- Extend the generated SwiftUI direct-launch gate through a real AppKit surface
  resize and require the package-owned adapter to report shared-session reflow.
- Extend the generated SwiftUI direct-launch gate through an AppKit click on a
  deterministic HTML link after restoring the scrolled viewport to document
  start.
- Exercise the generated SwiftUI content surface's End-key path against a
  scrollable live page and require the shared Rust viewport to change.

- Wire normalized AppKit wheel events into the shared `BrowserSession` scroll
  model and repaint the translated viewport through Metal.
- Activate viewport links from primary-button input, update the native title,
  and repaint across repeated transactional navigation.
- Drive the clamped viewport and Metal repaint path from named navigation keys.
- Reload Back/Forward history entries from Command-Left/Right shortcuts, then
  update the native title and Metal viewport.
- Export the concrete dynamic bridge used by Venture's generated SwiftUI
  `MosaicHost`: shared chrome props and events, Metal content rendering, scroll,
  and link activation all stay backed by one Rust browser session.
- Accept shared semantic keyboard-scroll commands through that dynamic bridge
  so the package-owned SwiftUI content surface can drive the same Rust model.
- Reflow the retained page through the shared Rust pipeline when the generated
  SwiftUI content surface changes logical size.
- Build and directly launch the Mosaic-emitted SwiftUI app in a platform-native
  integration gate, requiring a successful Metal host-surface render.
- Extend the generated-app launch gate through native address editing and Go
  activation, requiring the shared Rust session to load and title a second
  deterministic page.
- Extend direct generated-app interaction acceptance through the native Back
  and Forward buttons, requiring both history entries to reload through the
  shared Rust session before the gate succeeds.
- Extend that direct acceptance through the Mosaic-authored Reload and Home
  controls, requiring Reload to fetch changed content at the current URL and
  Home to return through the shared Rust navigation session.

## 0.1.0

- Add a runnable macOS Venture host using AppKit, CoreText, and Metal.
- Add a native acceptance test carrying canned HTML through CoreText glyph
  bindings into non-empty Metal-rendered pixels.
