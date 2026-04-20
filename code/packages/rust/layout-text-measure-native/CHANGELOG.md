# Changelog

## [0.1.0] — initial release

### Added
- `NativeMeasurer` struct — bridges layout-ir's `TextMeasurer` trait to the text-native trio (`NativeResolver` + `NativeMetrics` + `NativeShaper`). Owns the trio internally; constructed with `::new()`.
- Handle caching keyed on `(family, weight, italic)` via an internal `RefCell<HashMap<CacheKey, CachedHandle>>`. Resolver failures are cached as `CachedHandle::Failed` so repeated misses don't re-enter the OS text stack.
- Empty font family (`""`, the value `document_default_theme` produces for system-default) is remapped to `"Helvetica"` on Apple targets (`"sans-serif"` on others).
- Single-line measurement: shapes the whole string via the native shaper and reads `x_advance_total` for width; computes line height from `(ascent + descent + line_gap) × size / units_per_em` with the FontSpec's line-height multiplier layered on.
- Wrapped measurement: naive greedy word-wrap. Splits the input at whitespace boundaries, shapes each word, and greedily packs words into lines up to `max_width`. Respects hard newlines. Returns the longest line's width and total line count.
- Fallback heuristic (`~0.5 em/char`, single line) for any case where the OS text stack returns an error — `TextMeasurer::measure` returns a plain `MeasureResult` with no `Result`, so errors can't be propagated.

### Design
- Crate depends on `layout-ir`, `text-interfaces`, `text-native` (the trait layer + wrapper). Pulls no external deps.
- Keyed cache uses `family` + `weight` + `italic` but NOT size — the CoreText shaper rescales the CTFontRef internally via `CTFontCreateCopyWithAttributes`, so one cached handle works across all sizes.
- v1 simplifications documented: whitespace-boundary wrap only (no UAX #14), no per-FontSpec feature tuning, `!Sync` cache.

### Tests (11, gated to `target_vendor = "apple"`)
- `hello_single_line_has_positive_width`
- `empty_string_is_zero_width_one_line`
- `empty_family_falls_back_to_system_default`
- `long_text_wraps_to_multiple_lines`
- `short_text_under_max_width_is_one_line`
- `hard_newline_forces_a_new_line`
- `bold_font_measures_wider_than_regular` (guards against weight=700 falling back to the estimate)
- `height_scales_with_line_count_on_wrap`
- `cache_persists_across_calls`
- `line_height_includes_ascent_plus_descent` (plausibility range for 16px Helvetica)
- `word_wrap_respects_word_boundaries`
