# layout-text-measure-native

Bridge crate. Implements layout-ir's `TextMeasurer` trait on top of the
`text-native` (CoreText / DirectWrite / Pango) TXT00 trio.

## Why it exists

The layout engine (`layout-block`) and the paint layer (`layout-to-paint`,
coming in PR 8) both need to shape text. They must use the **same**
shaper, or measurement and final rendering can drift apart and text
layouts break. This crate is that architectural joint: a single
`NativeMeasurer` that owns the resolver + metrics + shaper, so every
measurement during layout comes from the same source that paint-time
shaping will use.

## Exports

- `NativeMeasurer` — implements `layout_ir::TextMeasurer`
- Re-exports from `text-native` and `text-interfaces` for ergonomics

## v1 features

- Handle caching by `(family, weight, italic)` — one FontSpec resolves
  once and is reused.
- Word-boundary wrap when `max_width` is Some. Hard newlines in the
  input produce separate line segments.
- Empty font family (`""`) remaps to the platform-default face
  (`Helvetica` on Apple). Since `document_default_theme()` uses `""`,
  this is the normal path.
- Fallback estimate (`~0.5 em/char`, single line) if any backend call
  fails — `TextMeasurer::measure` can't return an error so we degrade
  gracefully.

## v2+ concerns

- UAX #14 Unicode line-break opportunities (for CJK, Thai, etc.)
- Shaper feature tuning (liga / kern / features per FontSpec)
- Send+Sync variant with `Mutex`-wrapped cache

## Tests

11 unit tests (gated to `target_vendor = "apple"`), covering single-line
measurement, empty input, empty-family default, word wrap, short text
under max width, hard newlines, bold font path, multi-line height
scaling, cache persistence, line-height plausibility, and word-boundary
breaking.
