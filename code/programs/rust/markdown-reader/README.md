# markdown-reader

Native macOS Markdown viewer. The final binary of the Markdown-to-Metal
pipeline that was built across PRs #922, #924, #926, #928, #931, #957,
#960, plus the four local-only PRs (layout-to-paint, paint-metal
glyph_run handler, window-appkit CAMetalLayer attachment, paint-metal
live-drawable variant).

## What it does

```
Markdown string
   ↓  commonmark-parser
DocumentNode
   ↓  document-ast-to-layout + DocumentTheme
LayoutNode tree
   ↓  layout-block + layout-text-measure-native (CoreText-backed)
PositionedNode tree
   ↓  layout-to-paint + same CoreText trio
PaintScene (with pre-shaped PaintGlyphRun instructions)
   ↓  paint-metal (Metal rects/lines + CoreText glyph overlay)
PixelContainer
   ↓  NSImageView in an NSWindow
visible text on screen
```

Every layer specified in the TXT00-TXT05 + FNT02/FNT03 + UI02/UI04/UI06/UI07
series is exercised here, end to end, for the first time.

## Usage

```
cargo run --bin markdown-reader [path/to/file.md]
```

With no argument, a built-in sample Markdown is rendered. This keeps
the first demo run self-contained — run once with no args, get an
immediate visual sanity check of the entire stack.

## v1 limitations (explicit)

- **Inline formatting flattened** — `**bold**`, `*italic*`, `[links](url)`
  all render as plain body text in v1. Styled inline spans are a v2
  extension to `document-ast-to-layout`.
- **Static render** — the window doesn't relayout on resize. Close and
  rerun to see a different width.
- **No scrolling** — content below `WINDOW_HEIGHT` is clipped. Scroll
  view is a v2 concern.
- **Latin-only text** — CoreText handles Unicode internally, but
  complex scripts may not lay out correctly through the
  word-boundary-wrap path.

## Platform

Requires macOS. On non-Apple targets the binary prints the rendered
dimensions to stderr and exits (no window code runs).
