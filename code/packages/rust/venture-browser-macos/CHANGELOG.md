# Changelog

## Unreleased

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

## 0.1.0

- Add a runnable macOS Venture host using AppKit, CoreText, and Metal.
- Add a native acceptance test carrying canned HTML through CoreText glyph
  bindings into non-empty Metal-rendered pixels.
