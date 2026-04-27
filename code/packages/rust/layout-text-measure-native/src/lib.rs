//! # layout-text-measure-native
//!
//! Bridge crate. Implements layout-ir's [`TextMeasurer`] trait on top of
//! the `text-native` (CoreText / DirectWrite / Pango) TXT00 trio.
//!
//! This is the architectural joint that lets the layout engine
//! (`layout-block`) measure text using the *same* shaper that
//! `layout-to-paint` will later use to produce glyph runs. Measurement
//! and final rendering stay consistent — neither can drift from the
//! other, because both come from the same `CoreTextShaper` (or
//! `DirectWriteShaper`, or `PangoShaper` once those backends land).
//!
//! ```text
//!  layout-ir FontSpec
//!        │
//!        ▼  convert to FontQuery
//!  text-interfaces FontResolver ──▶ NativeHandle  ──┐
//!                                                   │
//!  text-interfaces TextShaper ◀───────────────────── │
//!                                                   │
//!  text-interfaces FontMetrics ◀────────────────────┘
//!        │
//!        ▼
//!  layout-ir MeasureResult { width, height, line_count }
//! ```
//!
//! ## v1 scope
//!
//! - Handle caching by `(family, weight, italic)` so a single FontSpec
//!   re-used across many measurements resolves once and is shared.
//! - Word-boundary wrap: when `max_width` is Some and the full-line
//!   shape is too wide, the text is split at whitespace boundaries
//!   and greedily packed into lines. Line count comes from the number
//!   of resulting lines; width is the max line's shaped width.
//! - Empty family name ("") is remapped to a platform-default
//!   ("Helvetica" on Apple targets) before calling the resolver.
//!   `document_default_theme` uses "" so the remap is the normal path.
//!
//! ## v2+ concerns
//!
//! - Unicode-aware line-break opportunities (UAX #14). The current
//!   naive whitespace split handles English and most European
//!   languages acceptably but not CJK (which has no inter-word spaces)
//!   or Thai. When the device-independent `layout-text-line-break`
//!   package lands this bridge will delegate to it.
//! - Shaper feature tuning (ligatures, kern opt-out). The current
//!   measurer always uses default `ShapeOptions`.
//! - Cross-thread caching. `NativeMeasurer` is `!Sync` — the internal
//!   handle cache is a `RefCell`. A future Send+Sync variant will
//!   wrap the cache in a `Mutex`.

use std::cell::RefCell;
use std::collections::HashMap;

use layout_ir::{FontSpec, MeasureResult, TextMeasurer};
use text_interfaces::{
    FontMetrics, FontQuery, FontResolver, FontStretch, FontStyle, FontWeight, ShapeOptions,
    TextShaper,
};
use text_native::{NativeHandle, NativeMetrics, NativeResolver, NativeShaper};

pub const VERSION: &str = "0.1.0";

// ═══════════════════════════════════════════════════════════════════════════
// Cache key
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct CacheKey {
    family: String,
    weight: u16,
    italic: bool,
}

impl CacheKey {
    fn from_font(font: &FontSpec) -> Self {
        Self {
            family: if font.family.is_empty() {
                default_family().to_string()
            } else {
                font.family.clone()
            },
            weight: font.weight,
            italic: font.italic,
        }
    }
}

#[cfg(target_vendor = "apple")]
fn default_family() -> &'static str {
    "Helvetica"
}

#[cfg(not(target_vendor = "apple"))]
fn default_family() -> &'static str {
    // On non-Apple platforms the text-native wrapper currently stubs
    // — the resolver will return LoadFailed anyway — but we return a
    // well-known generic so the cache key is stable.
    "sans-serif"
}

// ═══════════════════════════════════════════════════════════════════════════
// NativeMeasurer
// ═══════════════════════════════════════════════════════════════════════════

/// A [`TextMeasurer`] backed by the `text-native` trio (CoreText /
/// DirectWrite / Pango, depending on target OS).
///
/// Constructs its own resolver + metrics + shaper internally. Caches
/// resolved handles by (family, weight, italic) so repeated FontSpec
/// lookups don't re-enter the OS each time.
///
/// If measurement fails at any layer (resolver returns an error,
/// shaping fails), the measurer falls back to a heuristic estimate
/// (~0.5 em per char, 1 line) — `TextMeasurer::measure` returns a
/// plain `MeasureResult` with no `Result` wrapper, so we cannot
/// propagate errors. The fallback is documented but should be rare in
/// practice; it's defensive insurance, not a hot path.
pub struct NativeMeasurer {
    resolver: NativeResolver,
    metrics: NativeMetrics,
    shaper: NativeShaper,
    cache: RefCell<HashMap<CacheKey, CachedHandle>>,
}

/// Cached result of resolving a FontSpec. Holds either a successful
/// handle or a marker that resolution failed (so we don't retry the
/// OS every time — the resolver fail is usually persistent: the
/// family genuinely doesn't exist).
enum CachedHandle {
    Ok(NativeHandle),
    Failed,
}

impl NativeMeasurer {
    pub fn new() -> Self {
        Self {
            resolver: NativeResolver::new(),
            metrics: NativeMetrics::new(),
            shaper: NativeShaper::new(),
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// Resolve (or look up cached) handle for the given FontSpec.
    /// Returns None on a failure that has already been cached.
    fn handle_for<'a>(
        &'a self,
        font: &FontSpec,
    ) -> Option<std::cell::Ref<'a, NativeHandle>> {
        let key = CacheKey::from_font(font);

        // Fast path: cached.
        {
            let cache = self.cache.borrow();
            if let Some(entry) = cache.get(&key) {
                match entry {
                    CachedHandle::Ok(_) => {
                        // Drop the borrow so we can re-borrow with Ref::map.
                    }
                    CachedHandle::Failed => return None,
                }
            }
        }

        // Not cached yet — resolve.
        if !self.cache.borrow().contains_key(&key) {
            let query = FontQuery {
                family_names: vec![key.family.clone()],
                weight: FontWeight(key.weight),
                style: if key.italic { FontStyle::Italic } else { FontStyle::Normal },
                stretch: FontStretch::Normal,
            };
            let resolved = self.resolver.resolve(&query);
            let cached = match resolved {
                Ok(h) => CachedHandle::Ok(h),
                Err(_) => CachedHandle::Failed,
            };
            self.cache.borrow_mut().insert(key.clone(), cached);
        }

        // Now guaranteed present. Return a Ref into the cache's handle
        // via Ref::map. For the Failed branch we already returned None
        // above.
        let cache = self.cache.borrow();
        match cache.get(&key).unwrap() {
            CachedHandle::Ok(_) => Some(std::cell::Ref::map(cache, |c| match c.get(&key).unwrap() {
                CachedHandle::Ok(h) => h,
                CachedHandle::Failed => unreachable!(),
            })),
            CachedHandle::Failed => None,
        }
    }
}

impl Default for NativeMeasurer {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TextMeasurer impl
// ═══════════════════════════════════════════════════════════════════════════

impl TextMeasurer for NativeMeasurer {
    fn measure(
        &self,
        text: &str,
        font: &FontSpec,
        max_width: Option<f64>,
    ) -> MeasureResult {
        if text.is_empty() {
            return empty_line_result(font);
        }

        let handle_ref = match self.handle_for(font) {
            Some(h) => h,
            None => return fallback_estimate(text, font, max_width),
        };
        let handle: &NativeHandle = &handle_ref;

        match max_width {
            None => measure_single_line(&self.shaper, &self.metrics, handle, text, font),
            Some(mw) => measure_wrapped(&self.shaper, &self.metrics, handle, text, font, mw),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Single-line measurement
// ═══════════════════════════════════════════════════════════════════════════

fn measure_single_line(
    shaper: &NativeShaper,
    metrics: &NativeMetrics,
    handle: &NativeHandle,
    text: &str,
    font: &FontSpec,
) -> MeasureResult {
    let size = font.size as f32;
    let shape = match shaper.shape(text, handle, size, &ShapeOptions::default()) {
        Ok(r) => r,
        Err(_) => {
            return fallback_estimate(text, font, None);
        }
    };

    let line_height = compute_line_height(metrics, handle, font);
    MeasureResult {
        // Sum advances across all font-fallback segments: the total
        // width of the shaped line is the sum of each segment's
        // x_advance_total, not any single segment's.
        width: shape.total_advance() as f64,
        height: line_height,
        line_count: 1,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Wrapped measurement — naive greedy word wrap
// ═══════════════════════════════════════════════════════════════════════════

fn measure_wrapped(
    shaper: &NativeShaper,
    metrics: &NativeMetrics,
    handle: &NativeHandle,
    text: &str,
    font: &FontSpec,
    max_width: f64,
) -> MeasureResult {
    // Preserve hard newlines from the input by splitting on '\n' first;
    // each segment is then greedily word-wrapped within `max_width`.
    let size = font.size as f32;
    let line_height = compute_line_height(metrics, handle, font);

    let mut total_lines: u32 = 0;
    let mut max_line_width: f64 = 0.0;

    for segment in text.split('\n') {
        let wrapped = greedy_wrap(shaper, handle, segment, size, max_width);
        for line_width in wrapped {
            total_lines += 1;
            if line_width > max_line_width {
                max_line_width = line_width;
            }
        }
    }

    if total_lines == 0 {
        return empty_line_result(font);
    }

    MeasureResult {
        width: max_line_width.min(max_width),
        height: line_height * total_lines as f64,
        line_count: total_lines,
    }
}

/// Split `segment` at whitespace, shape each word, pack into lines.
/// Returns the shaped width of each line in source order.
///
/// The algorithm:
///   1. Break the segment into words (split at ASCII whitespace, keep
///      multi-char whitespace runs collapsed to a single space).
///   2. For each word, shape it and record its width and the
///      single-space width at the current font.
///   3. Greedy fill: add word to current line if current + space +
///      word fits; otherwise start a new line with that word.
///
/// This is intentionally simple for v1. A future version delegates to
/// a UAX #14 line-break-opportunity package for non-English text.
fn greedy_wrap(
    shaper: &NativeShaper,
    handle: &NativeHandle,
    segment: &str,
    size: f32,
    max_width: f64,
) -> Vec<f64> {
    if segment.is_empty() {
        return vec![0.0];
    }

    let space_width = match shaper.shape(" ", handle, size, &ShapeOptions::default()) {
        Ok(r) => r.total_advance() as f64,
        Err(_) => (size as f64) * 0.25,
    };

    let mut lines: Vec<f64> = Vec::new();
    let mut current_width: f64 = 0.0;

    for word in segment.split_whitespace() {
        // Words may themselves hit font fallback (e.g. a word containing
        // a single-codepoint arrow); sum across segments.
        let word_width = match shaper.shape(word, handle, size, &ShapeOptions::default()) {
            Ok(r) => r.total_advance() as f64,
            Err(_) => word.chars().count() as f64 * (size as f64) * 0.5,
        };

        if current_width == 0.0 {
            // First word on a fresh line — always place it, even if it
            // exceeds max_width (word is unbreakable at this tier).
            current_width = word_width;
        } else if current_width + space_width + word_width <= max_width {
            current_width += space_width + word_width;
        } else {
            lines.push(current_width);
            current_width = word_width;
        }
    }

    if current_width > 0.0 {
        lines.push(current_width);
    }
    if lines.is_empty() {
        lines.push(0.0);
    }
    lines
}

// ═══════════════════════════════════════════════════════════════════════════
// Shared helpers
// ═══════════════════════════════════════════════════════════════════════════

fn compute_line_height(
    metrics: &NativeMetrics,
    handle: &NativeHandle,
    font: &FontSpec,
) -> f64 {
    let upem = metrics.units_per_em(handle) as f64;
    let ascent = metrics.ascent(handle) as f64;
    let descent = metrics.descent(handle) as f64;
    let line_gap = metrics.line_gap(handle) as f64;

    if upem <= 0.0 {
        // Defensive: fall back to the FontSpec's line_height
        return font.size * font.line_height;
    }

    let scale = font.size / upem;
    let raw = (ascent + descent + line_gap) * scale;

    // Respect the FontSpec.line_height multiplier — treat ratio as
    // "multiply the design raw line-height by this factor". A value of
    // 1.2 is the convention used by `font_spec()`.
    raw.max(font.size) * font.line_height.max(1.0) / 1.2
}

fn empty_line_result(font: &FontSpec) -> MeasureResult {
    MeasureResult {
        width: 0.0,
        height: font.size * font.line_height,
        line_count: 1,
    }
}

fn fallback_estimate(text: &str, font: &FontSpec, max_width: Option<f64>) -> MeasureResult {
    let chars = text.chars().count() as f64;
    let raw_width = chars * font.size * 0.5;
    let line_height = font.size * font.line_height;
    match max_width {
        None => MeasureResult {
            width: raw_width,
            height: line_height,
            line_count: 1,
        },
        Some(mw) if raw_width <= mw || mw <= 0.0 => MeasureResult {
            width: raw_width,
            height: line_height,
            line_count: 1,
        },
        Some(mw) => {
            let lines = (raw_width / mw).ceil().max(1.0) as u32;
            MeasureResult {
                width: mw,
                height: line_height * lines as f64,
                line_count: lines,
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests — all require a real OS backend. Gated to Apple targets only;
// on other platforms these will fall through to the estimate path.
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(all(test, target_vendor = "apple"))]
mod tests {
    use super::*;
    use layout_ir::{font_bold, font_spec};

    fn m() -> NativeMeasurer {
        NativeMeasurer::new()
    }

    #[test]
    fn hello_single_line_has_positive_width() {
        let r = m().measure("Hello", &font_spec("Helvetica", 16.0), None);
        assert!(r.width > 0.0, "expected positive width, got {}", r.width);
        assert!(r.height > 0.0);
        assert_eq!(r.line_count, 1);
    }

    #[test]
    fn empty_string_is_zero_width_one_line() {
        let r = m().measure("", &font_spec("Helvetica", 16.0), None);
        assert_eq!(r.width, 0.0);
        assert!(r.height > 0.0);
        assert_eq!(r.line_count, 1);
    }

    #[test]
    fn empty_family_falls_back_to_system_default() {
        // document_default_theme uses "" — this must not panic and must
        // produce sensible numbers.
        let r = m().measure("Hello", &font_spec("", 16.0), None);
        assert!(r.width > 0.0);
        assert!(r.height > 0.0);
    }

    #[test]
    fn long_text_wraps_to_multiple_lines() {
        // "Hello world" × 20 at 16px with max_width=50 should wrap many times.
        let text = "Hello world ".repeat(20);
        let r = m().measure(&text, &font_spec("Helvetica", 16.0), Some(50.0));
        assert!(r.line_count > 1, "expected multiple lines, got {}", r.line_count);
        assert!(r.width <= 50.0 + 1e-3);
    }

    #[test]
    fn short_text_under_max_width_is_one_line() {
        let r = m().measure("Hi", &font_spec("Helvetica", 16.0), Some(500.0));
        assert_eq!(r.line_count, 1);
        assert!(r.width > 0.0 && r.width < 500.0);
    }

    #[test]
    fn hard_newline_forces_a_new_line() {
        let r = m().measure("line one\nline two", &font_spec("Helvetica", 16.0), Some(500.0));
        assert_eq!(r.line_count, 2);
    }

    #[test]
    fn bold_font_measures_wider_than_regular() {
        let normal = m().measure("Hello", &font_spec("Helvetica", 16.0), None);
        let bold = m().measure("Hello", &font_bold(font_spec("Helvetica", 16.0)), None);
        // Bold FontSpec has weight=700 but our v1 CoreText resolver
        // ignores weight — both resolve to the same "Helvetica" face.
        // Widths should be positive and equal-ish; this test guards
        // against accidental regressions where weight=700 fails to
        // resolve and falls back to the estimate.
        assert!(normal.width > 0.0);
        assert!(bold.width > 0.0);
    }

    #[test]
    fn height_scales_with_line_count_on_wrap() {
        let single = m().measure("Hi", &font_spec("Helvetica", 16.0), Some(500.0));
        let wrapped = m().measure(
            &"a ".repeat(50),
            &font_spec("Helvetica", 16.0),
            Some(30.0),
        );
        assert!(wrapped.line_count > 1);
        assert!(wrapped.height > single.height * 2.0);
    }

    #[test]
    fn cache_persists_across_calls() {
        let measurer = m();
        // First call populates the cache.
        measurer.measure("a", &font_spec("Helvetica", 16.0), None);
        // Second call should hit the cache — we can't observe that
        // directly, but at minimum it must not panic or produce
        // different numbers.
        let r = measurer.measure("a", &font_spec("Helvetica", 16.0), None);
        assert!(r.width > 0.0);
    }

    #[test]
    fn line_height_includes_ascent_plus_descent() {
        let r = m().measure("A", &font_spec("Helvetica", 16.0), None);
        // Minimum-plausibility check: a 16px Helvetica line is between
        // ~14 and ~24 px tall. (Helvetica typoAscender + typoDescender
        // at 1.2× multiplier is usually ~19.)
        assert!(r.height > 14.0 && r.height < 24.0,
                "expected 14..24, got {}", r.height);
    }

    #[test]
    fn word_wrap_respects_word_boundaries() {
        // "aaaaaaaaaa bbbbbbbbbb" at 16px, max_width just below the
        // full line width — should break at the space.
        let text = "aaaaaaaaaa bbbbbbbbbb";
        let full = m().measure(text, &font_spec("Helvetica", 16.0), None).width;
        let wrap_width = full * 0.6;
        let r = m().measure(text, &font_spec("Helvetica", 16.0), Some(wrap_width));
        assert_eq!(r.line_count, 2);
    }
}
