# Changelog

## Unreleased

- Wire normalized AppKit wheel events into the shared `BrowserSession` scroll
  model and repaint the translated viewport through Metal.
- Activate viewport links from primary-button input, update the native title,
  and repaint across repeated transactional navigation.

## 0.1.0

- Add a runnable macOS Venture host using AppKit, CoreText, and Metal.
- Add a native acceptance test carrying canned HTML through CoreText glyph
  bindings into non-empty Metal-rendered pixels.
