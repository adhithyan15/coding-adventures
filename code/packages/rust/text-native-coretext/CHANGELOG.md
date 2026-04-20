# Changelog

## [0.1.0] — initial release

### Added
- `CoreTextHandle` newtype wrapping a retained `CTFontRef` with RAII release on `Drop`. Stores the cached PostScript name and creation size for `font_ref` construction.
- `CoreTextResolver` implementing TXT00 `FontResolver<Handle = CoreTextHandle>`. Walks `FontQuery.family_names` and calls `CTFontCreateWithName` for each.
- `CoreTextMetrics` implementing TXT00 `FontMetrics<Handle = CoreTextHandle>`. Reads via `CTFontGet{UnitsPerEm, Ascent, Descent, Leading, XHeight, CapHeight}` and `CTFontCopyFamilyName`. Rescales CoreText's user-space metric values back to design units via `value * upem / size` so downstream math is portable with the font-parser path.
- `CoreTextShaper` implementing TXT00 `TextShaper<Handle = CoreTextHandle>`. Builds a `CFAttributedString`, calls `CTLineCreateWithAttributedString`, walks the resulting `CTRun` array via `CTLineGetGlyphRuns` + `CFArrayGet*`, and extracts per-glyph `glyph_id`, `cluster`, advances, and pen-relative offsets into a flat `ShapedRun`.
- `font_ref` scheme: `"coretext:<postscript-name>@<size>"`. Paint backends route on the `coretext:` prefix (per P2D06 amendment).

### Dependencies
- `text-interfaces` (TXT00 trait definitions).
- `objc-bridge` — extended in this release with the CoreText / CoreFoundation bindings required by the shaper: `CTFontCreateCopyWithAttributes`, `CTLineGetGlyphRuns`, `CTRunGetGlyphCount`, `CTRunGetGlyphs`, `CTRunGetPositions`, `CTRunGetAdvances`, `CTRunGetStringIndices`, `CTFontGet{UnitsPerEm, Ascent, Descent, Leading, XHeight, CapHeight, Size}`, `CTFontCopy{FamilyName, PostScriptName}`, `CTFontDrawGlyphs`, `CFArrayGetCount`, `CFArrayGetValueAtIndex`, `CFStringGetLength`, `CFStringGetCString`, plus the `CFRange` struct.

### v1 scope (intentional)
- LTR direction only. RTL / vertical throw `UnsupportedDirection`.
- Family-name resolution only — weight / style / stretch are accepted on `FontQuery` but currently not translated to `CTFontDescriptor` attributes. Callers encode style via family name (`"Helvetica-Bold"`) for now.
- OpenType features are accepted in `ShapeOptions.features` but not yet applied to the CoreText attribute dictionary.
- Non-Apple targets compile as empty shells so the `text-native` wrapper crate can reference this crate unconditionally.

### Tests (7, gated to `target_vendor = "apple"`)
- `resolve_helvetica` — basic family-name lookup succeeds.
- `resolve_empty_query_errors` — empty `family_names` yields `EmptyQuery`.
- `resolve_unknown_family_falls_through_to_no_family_found` — robustness smoke test.
- `metrics_look_sane` — ascent > 0, descent ≥ 0, x_height < cap_height < ascent, family name contains "Helvetica".
- `shape_hello_produces_5_glyphs` — 5-glyph output with positive total advance; cluster values 0..=4.
- `shape_empty_string_is_empty_run` — empty input yields zero-glyph run with zero advance.
- `shape_rejects_rtl_for_v1` — `Direction::Rtl` → `UnsupportedDirection`.
- `measure_hello_world_via_shaper_and_metrics` — generic `measure()` wrap produces positive width/ascent/descent.
- `font_ref_matches_between_shaper_and_handle_key` — `font_ref` invariant holds.
