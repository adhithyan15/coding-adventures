# Changelog

## [0.1.0] — initial release

### Added
- `markdown-reader` binary — native macOS Markdown viewer exercising the whole coding-adventures text stack end to end.
- Input: Markdown from a file passed as the first CLI arg, or a built-in sample when no arg given.
- Pipeline: commonmark-parser → document-ast-to-layout → layout-block (with `NativeMeasurer` for CoreText-backed text measurement) → layout-to-paint (with the same CoreText trio for shaping) → paint-metal (Metal rects/lines + CoreText glyph_run overlay) → NSImageView in an NSWindow.
- Window setup replicates the known-working pattern from `draw-instructions-metal-mac-window`: NSApplication with regular activation policy, NSWindow with standard style mask, NSBitmapImageRep wrapping the PixelContainer without copy, NSImageView as the content view, dynamically-registered window delegate that terminates the app on close.

### v1 limitations (intentional)
- Inline formatting flattened to plain text (upstream in document-ast-to-layout).
- Static render — no relayout on window resize.
- No scrolling — content below the viewport height is clipped.
- Latin-only — complex scripts may not flow correctly through the word-boundary-wrap path.

### Platform
- macOS (target_vendor = "apple"). Non-Apple targets print the rendered pixel dimensions to stderr and exit without opening a window.
