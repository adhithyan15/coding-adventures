# text-native-coretext

CoreText-backed implementation of the TXT00 `FontMetrics`, `TextShaper`,
and `FontResolver` traits (spec TXT03a). Rust wrapper over the
`objc-bridge` CoreText bindings.

## What's in the crate

| Type                  | Role                                                               |
|-----------------------|--------------------------------------------------------------------|
| `CoreTextHandle`      | Retained `CTFontRef` with RAII release on `Drop`                   |
| `CoreTextResolver`    | Maps a `FontQuery` → `CoreTextHandle` via `CTFontCreateWithName`   |
| `CoreTextMetrics`     | Reads font-global metrics from `CTFontGet*` functions              |
| `CoreTextShaper`      | Shapes strings into `ShapedRun` by walking `CTLine` → `CTRun`s    |

## Font binding

Every `ShapedRun` produced by `CoreTextShaper` carries a `font_ref` of
the form `"coretext:<postscript-name>@<size>"`. Paint backends route on
the `coretext:` prefix (per P2D06) to dispatch via `CTFontDrawGlyphs`.

## Cluster unit

CoreText returns **UTF-16 code-unit offsets** via `CTRunGetStringIndices`.
Every `Glyph.cluster` in this crate's output is a UTF-16 offset. Callers
holding the source text as a Rust `&str` (UTF-8) must convert if they
need to map back to byte offsets.

## v1 limitations

- **ltr only.** `Direction::Rtl` / `Ttb` / `Btt` throw `UnsupportedDirection`.
- **Family-name resolution only.** Weight, style, and stretch are
  accepted on `FontQuery` but not yet translated to a `CTFontDescriptor`
  lookup. Callers encode style into family name for now
  (`"Helvetica-Bold"`).
- **No feature tag translation.** OpenType feature requests in
  `ShapeOptions.features` are accepted but not wired to CoreText's
  feature dictionary.

These limitations are tracked for v2. The trait surface is stable.

## Platform gating

The crate is `#[cfg(target_vendor = "apple")]`. On non-Apple targets it
compiles as empty shells so downstream wrapper crates (TXT03 aggregator,
`text-native`) can reference it unconditionally.

## Spec

See [code/specs/TXT03-native-shapers.md](../../../specs/TXT03-native-shapers.md).
