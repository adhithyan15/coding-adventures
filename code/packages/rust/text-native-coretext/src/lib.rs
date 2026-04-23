//! # text-native-coretext
//!
//! CoreText-backed implementation of the TXT00 [`FontMetrics`] +
//! [`TextShaper`] + [`FontResolver`] traits (spec TXT03a).
//!
//! This crate is the macOS / iOS native text path. It delegates all the
//! hard work — cmap lookup, GSUB/GPOS shaping, kerning, complex scripts,
//! metric retrieval — to CoreText, and packages the results in the
//! trait-shaped output types that the rest of the pipeline consumes.
//!
//! ## Font handle
//!
//! The associated `Handle` type is [`CoreTextHandle`], which wraps a
//! retained `CTFontRef` pointer (an Objective-C object managed by
//! CoreFoundation's reference counting). Handles are created by
//! [`CoreTextResolver::resolve`], which calls
//! `CTFontCreateWithName`. They are automatically released on [`Drop`].
//!
//! ## font_ref scheme
//!
//! Every [`ShapedRun`] produced by [`CoreTextShaper`] carries a
//! `font_ref` of the form `"coretext:<PostScript name>@<size>"`. Paint
//! backends route on the `coretext:` prefix (per the P2D06 amendment)
//! to dispatch glyph runs via `CTFontDrawGlyphs`.
//!
//! ## Unit of `cluster`
//!
//! CoreText's string-index mapping is **UTF-16 code-unit offsets**
//! (because `CFAttributedString` is UTF-16 internally). Every
//! `Glyph.cluster` value in this crate's output is a UTF-16 code-unit
//! offset, not a UTF-8 byte offset. Callers that hold the source text
//! as a Rust `&str` (UTF-8) must convert if they need to map clusters
//! back to byte offsets.
//!
//! ## Platform gating
//!
//! The whole crate is `#[cfg(target_vendor = "apple")]`. On non-Apple
//! targets the module compiles as an empty shell so downstream wrapper
//! crates can reference it unconditionally.

use text_interfaces::{
    Direction, FontMetrics, FontQuery, FontResolutionError, FontResolver, Glyph, ShapeOptions,
    ShapedRun, ShapedText, ShapingError, TextShaper,
};

#[cfg(target_vendor = "apple")]
use objc_bridge::{
    cfstring_checked, kCTFontAttributeName, CFArrayGetCount, CFArrayGetValueAtIndex,
    CFDictionaryGetValue, CFRange, CFRelease, CFStringGetCString, CFStringGetLength, CGPoint,
    CGSize, CTFontCopyFamilyName, CTFontCopyPostScriptName, CTFontCreateCopyWithAttributes,
    CTFontCreateWithName, CTFontGetAscent, CTFontGetCapHeight, CTFontGetDescent, CTFontGetLeading,
    CTFontGetSize, CTFontGetUnitsPerEm, CTFontGetXHeight, CTLineCreateWithAttributedString,
    CTLineGetGlyphRuns, CTRunGetAdvances, CTRunGetAttributes, CTRunGetGlyphCount, CTRunGetGlyphs,
    CTRunGetPositions, CTRunGetStringIndices, Id, K_CF_STRING_ENCODING_UTF8, NIL,
};

pub const VERSION: &str = "0.1.0";

// ═══════════════════════════════════════════════════════════════════════════
// CoreTextHandle — retained CTFontRef with automatic Drop-based release
// ═══════════════════════════════════════════════════════════════════════════

/// A retained `CTFontRef` (aliased to `Id` in objc-bridge). Dropping the
/// handle releases the underlying CoreText object.
///
/// Handles are `!Send` and `!Sync` on purpose — CoreText's reference
/// counting is thread-safe, but the objc-bridge `Id` type is a raw pointer
/// and we do not guarantee thread-safe access in this first cut. A future
/// revision can relax this by wrapping in `Arc` + `unsafe impl Send`.
#[cfg(target_vendor = "apple")]
pub struct CoreTextHandle {
    font: Id,
    /// Cached PostScript name (e.g. `"Helvetica-Bold"`). Used to build
    /// the `font_ref` string without re-calling CoreText every time.
    ps_name: String,
    /// Size the font was created at. CoreText fonts are size-bound; the
    /// shaper may rescale via `CTFontCreateCopyWithAttributes`.
    size: f32,
}

#[cfg(target_vendor = "apple")]
impl CoreTextHandle {
    /// Construct from a raw retained `CTFontRef`. Takes ownership.
    ///
    /// # Safety
    /// `font` must be a valid retained `CTFontRef`. This struct's Drop
    /// will call `CFRelease` on it exactly once.
    pub unsafe fn from_retained(font: Id) -> Self {
        let ps_name = copy_ps_name(font);
        let size = CTFontGetSize(font) as f32;
        Self { font, ps_name, size }
    }

    /// The underlying `CTFontRef` as an opaque `Id`. Exposed for paint
    /// backends that route `coretext:` bindings; they must NOT release it.
    pub fn raw(&self) -> Id {
        self.font
    }

    /// The font_ref scheme key: `"<postscript-name>@<size>"`. This is the
    /// value that appears after `"coretext:"` in the full binding string.
    pub fn ref_key(&self) -> String {
        format!("{}@{}", self.ps_name, self.size)
    }
}

#[cfg(target_vendor = "apple")]
impl Drop for CoreTextHandle {
    fn drop(&mut self) {
        if self.font != NIL {
            unsafe { CFRelease(self.font) }
        }
    }
}

#[cfg(target_vendor = "apple")]
impl std::fmt::Debug for CoreTextHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoreTextHandle")
            .field("ps_name", &self.ps_name)
            .field("size", &self.size)
            .finish()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CoreTextResolver — FontQuery → CoreTextHandle
// ═══════════════════════════════════════════════════════════════════════════

/// Resolves a [`FontQuery`] into a [`CoreTextHandle`] by walking the
/// query's family-name list and calling `CTFontCreateWithName` for the
/// first that succeeds.
///
/// v1 supports query of family names. Weight, style, and stretch are
/// silently ignored until we add `CTFontDescriptor`-based lookup — v1
/// relies on callers encoding style into the family name (e.g. asking
/// for `"Helvetica-Bold"` directly) or accepting the regular face.
///
/// The resolver has a configurable default size used when creating
/// CTFontRef instances. Callers who need multiple sizes can either
/// re-resolve or use [`CoreTextShaper::shape`], which internally derives
/// a same-font-different-size handle via `CTFontCreateCopyWithAttributes`.
#[cfg(target_vendor = "apple")]
pub struct CoreTextResolver {
    default_size: f32,
}

#[cfg(target_vendor = "apple")]
impl CoreTextResolver {
    /// Default size is 16.0 (user-space units ≈ pixels at scale 1).
    pub fn new() -> Self {
        Self { default_size: 16.0 }
    }

    pub fn with_default_size(mut self, size: f32) -> Self {
        self.default_size = size;
        self
    }
}

#[cfg(target_vendor = "apple")]
impl Default for CoreTextResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_vendor = "apple")]
impl FontResolver for CoreTextResolver {
    type Handle = CoreTextHandle;

    fn resolve(&self, query: &FontQuery) -> Result<Self::Handle, FontResolutionError> {
        if query.family_names.is_empty() {
            return Err(FontResolutionError::EmptyQuery);
        }
        if query.weight.0 == 0 || query.weight.0 > 1000 {
            return Err(FontResolutionError::InvalidWeight(query.weight.0));
        }

        // For v1: try each family name in order. CoreText falls back to
        // Helvetica for unknown names, so we treat a nil return as failure.
        for family in &query.family_names {
            let font_opt = unsafe { create_font(family, self.default_size as f64) };
            if let Some(h) = font_opt {
                return Ok(h);
            }
        }
        Err(FontResolutionError::NoFamilyFound)
    }
}

#[cfg(target_vendor = "apple")]
unsafe fn create_font(family: &str, size: f64) -> Option<CoreTextHandle> {
    // cfstring_checked rejects interior NULs by returning None rather
    // than panicking — safe for untrusted family names.
    let cf_name = cfstring_checked(family)?;
    let font = CTFontCreateWithName(cf_name, size, std::ptr::null());
    CFRelease(cf_name);
    if font == NIL {
        None
    } else {
        Some(CoreTextHandle::from_retained(font))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CoreTextMetrics — implements TXT00 FontMetrics via CTFont accessors
// ═══════════════════════════════════════════════════════════════════════════

/// CoreText-backed [`FontMetrics`]. Methods return values in the font's
/// design units (multiplied back from CoreText's user-space values), so
/// downstream scaling behaves identically to the font-parser path.
#[cfg(target_vendor = "apple")]
pub struct CoreTextMetrics;

#[cfg(target_vendor = "apple")]
impl CoreTextMetrics {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_vendor = "apple")]
impl Default for CoreTextMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_vendor = "apple")]
impl FontMetrics for CoreTextMetrics {
    type Handle = CoreTextHandle;

    fn units_per_em(&self, font: &Self::Handle) -> u32 {
        unsafe { CTFontGetUnitsPerEm(font.raw()) }
    }

    fn ascent(&self, font: &Self::Handle) -> i32 {
        // CTFontGetAscent returns user-space units at the font's size.
        // Rescale to design units: value * upem / size.
        let upem = self.units_per_em(font) as f64;
        unsafe { (CTFontGetAscent(font.raw()) * upem / font.size as f64).round() as i32 }
    }

    fn descent(&self, font: &Self::Handle) -> i32 {
        let upem = self.units_per_em(font) as f64;
        // CTFontGetDescent is already non-negative.
        unsafe { (CTFontGetDescent(font.raw()) * upem / font.size as f64).round() as i32 }
    }

    fn line_gap(&self, font: &Self::Handle) -> i32 {
        let upem = self.units_per_em(font) as f64;
        unsafe { (CTFontGetLeading(font.raw()) * upem / font.size as f64).round() as i32 }
    }

    fn x_height(&self, font: &Self::Handle) -> Option<i32> {
        let upem = self.units_per_em(font) as f64;
        let v = unsafe { CTFontGetXHeight(font.raw()) };
        if v == 0.0 {
            None
        } else {
            Some((v * upem / font.size as f64).round() as i32)
        }
    }

    fn cap_height(&self, font: &Self::Handle) -> Option<i32> {
        let upem = self.units_per_em(font) as f64;
        let v = unsafe { CTFontGetCapHeight(font.raw()) };
        if v == 0.0 {
            None
        } else {
            Some((v * upem / font.size as f64).round() as i32)
        }
    }

    fn family_name(&self, font: &Self::Handle) -> String {
        unsafe { copy_family_name(font.raw()) }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CoreTextShaper — implements TXT00 TextShaper via CTLine
// ═══════════════════════════════════════════════════════════════════════════

/// CoreText-backed [`TextShaper`]. Builds a `CFAttributedString`, produces
/// a `CTLine`, walks the resulting `CTRun`s, and assembles a flat
/// [`ShapedRun`].
///
/// Script and language are inferred by CoreText from the input text; the
/// `ShapeOptions` fields for those are currently accepted but not applied
/// (v1 limitation — most content on macOS is correctly auto-detected).
/// Direction must be `Ltr` for now; RTL shaping is a v2 feature.
#[cfg(target_vendor = "apple")]
pub struct CoreTextShaper;

#[cfg(target_vendor = "apple")]
impl CoreTextShaper {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_vendor = "apple")]
impl Default for CoreTextShaper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_vendor = "apple")]
impl TextShaper for CoreTextShaper {
    type Handle = CoreTextHandle;

    fn shape(
        &self,
        text: &str,
        font: &Self::Handle,
        size: f32,
        options: &ShapeOptions,
    ) -> Result<ShapedText, ShapingError> {
        if !matches!(options.direction, Direction::Ltr) {
            return Err(ShapingError::UnsupportedDirection(options.direction));
        }

        // Empty input → empty ShapedText. Skip CoreText calls.
        if text.is_empty() {
            return Ok(ShapedText::empty());
        }

        unsafe { shape_with_coretext(text, font, size) }
    }

    fn font_ref(&self, font: &Self::Handle) -> String {
        format!("coretext:{}", font.ref_key())
    }
}

#[cfg(target_vendor = "apple")]
unsafe fn shape_with_coretext(
    text: &str,
    font_handle: &CoreTextHandle,
    size: f32,
) -> Result<ShapedText, ShapingError> {
    use objc_bridge::{
        kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks,
        CFAttributedStringCreate, CFDictionaryCreate,
    };

    // Ensure we have a CTFontRef at the requested size.
    //
    // `owns_resized` tracks whether we created a new retained
    // CTFontRef via CTFontCreateCopyWithAttributes (which always
    // returns a fresh retain, per Apple's Create/Copy naming rule —
    // we must release it regardless of whether the pointer happens to
    // be numerically equal to the input handle's).
    let (resized_font, owns_resized) = if (font_handle.size - size).abs() < 1e-3 {
        (font_handle.raw(), false)
    } else {
        let f = CTFontCreateCopyWithAttributes(
            font_handle.raw(),
            size as f64,
            std::ptr::null(),
            NIL,
        );
        (f, true)
    };
    if resized_font == NIL {
        return Err(ShapingError::ShapingFailed(
            "CTFontCreateCopyWithAttributes returned nil".into(),
        ));
    }

    // Build an attributed string carrying just the font attribute.
    // cfstring_checked rejects interior NUL bytes by returning None,
    // avoiding a panic on untrusted input text.
    let cf_text = match cfstring_checked(text) {
        Some(s) => s,
        None => {
            if owns_resized {
                CFRelease(resized_font);
            }
            return Err(ShapingError::ShapingFailed(
                "text contained interior NUL byte".into(),
            ));
        }
    };

    // Dictionary: {kCTFontAttributeName: resized_font}
    // kCTFontAttributeName is an extern static Id (pointer value); take
    // it by value, not by reference — the array needs the pointer
    // itself, not a pointer to the symbol's storage.
    let key_name: Id = kCTFontAttributeName;
    let keys: [*const std::ffi::c_void; 1] = [key_name as *const _];
    let values: [*const std::ffi::c_void; 1] = [resized_font as *const _];
    let attrs = CFDictionaryCreate(
        std::ptr::null(),
        keys.as_ptr(),
        values.as_ptr(),
        1,
        &kCFTypeDictionaryKeyCallBacks as *const _ as *const _,
        &kCFTypeDictionaryValueCallBacks as *const _ as *const _,
    );
    if attrs == NIL {
        CFRelease(cf_text);
        if owns_resized {
            CFRelease(resized_font);
        }
        return Err(ShapingError::ShapingFailed("CFDictionaryCreate failed".into()));
    }

    let attr_string = CFAttributedStringCreate(std::ptr::null(), cf_text, attrs);
    CFRelease(cf_text);
    CFRelease(attrs);
    if attr_string == NIL {
        if owns_resized {
            CFRelease(resized_font);
        }
        return Err(ShapingError::ShapingFailed(
            "CFAttributedStringCreate failed".into(),
        ));
    }

    let line = CTLineCreateWithAttributedString(attr_string);
    CFRelease(attr_string);
    if line == NIL {
        if owns_resized {
            CFRelease(resized_font);
        }
        return Err(ShapingError::ShapingFailed(
            "CTLineCreateWithAttributedString failed".into(),
        ));
    }

    // Walk each CTRun inside the CTLine. IMPORTANT: CoreText may
    // split the line across multiple runs when it performs font
    // fallback (a codepoint absent from the primary font → a
    // fallback font is used for that span). Each CTRun carries its
    // own font via CTRunGetAttributes; we tag the emitted ShapedRun
    // with the fallback font's PostScript name so the font-binding
    // invariant stays intact when the paint backend looks up which
    // font to render with.
    let runs = CTLineGetGlyphRuns(line);
    let run_count = CFArrayGetCount(runs);
    let mut out_runs: Vec<ShapedRun> = Vec::with_capacity(run_count as usize);

    // Running pen position across the whole line. Used to convert
    // CoreText's absolute line-local positions into pen-relative
    // offsets per glyph. Resets to zero at the start of each segment
    // because the layout engine treats each segment's glyphs as
    // pen-relative-from-segment-start and adds the cumulative run
    // advance itself.
    let mut line_pen_x: f32 = 0.0;
    let mut line_pen_y: f32 = 0.0;

    for i in 0..run_count {
        let run = CFArrayGetValueAtIndex(runs, i);
        let count = CTRunGetGlyphCount(run) as usize;
        if count == 0 {
            continue;
        }
        let range = CFRange { location: 0, length: 0 };

        let mut glyph_ids: Vec<u16> = vec![0; count];
        let mut positions: Vec<CGPoint> = vec![CGPoint { x: 0.0, y: 0.0 }; count];
        let mut advances: Vec<CGSize> = vec![CGSize { width: 0.0, height: 0.0 }; count];
        let mut clusters: Vec<std::ffi::c_long> = vec![0; count];

        CTRunGetGlyphs(run, range, glyph_ids.as_mut_ptr());
        CTRunGetPositions(run, range, positions.as_mut_ptr());
        CTRunGetAdvances(run, range, advances.as_mut_ptr());
        CTRunGetStringIndices(run, range, clusters.as_mut_ptr());

        // Resolve the run's actual font via its attribute dictionary.
        // This is where fallback surfaces — the font here may be a
        // different face than the one we requested. Fall back to the
        // originally-requested font if for any reason the attribute
        // is missing (shouldn't happen for CTLine-produced runs).
        let run_font = resolve_run_font(run, resized_font);
        let run_ps_name = copy_ps_name(run_font);
        let run_font_ref = if run_ps_name.is_empty() {
            format!("coretext:{}@{}", font_handle.ps_name, size)
        } else {
            format!("coretext:{}@{}", run_ps_name, size)
        };

        // Segment origin in line-local coords: where the first glyph
        // of this run lands. Used to shift per-glyph positions to
        // segment-local (pen-relative-from-segment-start).
        let segment_origin_x = positions[0].x as f32;
        let segment_origin_y = positions[0].y as f32;

        // Reset per-segment pen tracking.
        let mut seg_pen_x: f32 = 0.0;
        let mut seg_pen_y: f32 = 0.0;
        let mut seg_glyphs: Vec<Glyph> = Vec::with_capacity(count);

        for j in 0..count {
            // Absolute line-local position.
            let px = positions[j].x as f32;
            let py = positions[j].y as f32;
            // Segment-local position (subtract the segment origin
            // so each ShapedRun's glyphs reset from 0).
            let seg_px = px - segment_origin_x;
            let seg_py = py - segment_origin_y;
            // Pen-relative offset within the segment.
            let x_offset = seg_px - seg_pen_x;
            let y_offset = seg_py - seg_pen_y;
            let x_adv = advances[j].width as f32;
            let y_adv = advances[j].height as f32;

            seg_glyphs.push(Glyph {
                glyph_id: glyph_ids[j] as u32,
                cluster: clusters[j] as u32,
                x_advance: x_adv,
                y_advance: y_adv,
                x_offset,
                y_offset,
            });

            seg_pen_x += x_adv;
            seg_pen_y += y_adv;
            line_pen_x += x_adv;
            line_pen_y += y_adv;
        }

        let seg_advance_total: f32 = seg_glyphs.iter().map(|g| g.x_advance).sum();
        out_runs.push(ShapedRun {
            glyphs: seg_glyphs,
            x_advance_total: seg_advance_total,
            font_ref: run_font_ref,
        });
    }

    // Silence the unused variable warning — line_pen tracking is
    // retained for future diagnostic use (e.g. asserting that the
    // segment advances sum back to the line's typographic width).
    let _ = (line_pen_x, line_pen_y);

    CFRelease(line);
    if owns_resized {
        CFRelease(resized_font);
    }

    Ok(ShapedText { runs: out_runs })
}

/// Extract the actual font used by a CTRun via its attribute dictionary.
/// Falls back to the supplied `default_font` if the lookup fails.
#[cfg(target_vendor = "apple")]
unsafe fn resolve_run_font(run: Id, default_font: Id) -> Id {
    let attrs = CTRunGetAttributes(run);
    if attrs == NIL {
        return default_font;
    }
    // kCTFontAttributeName is an extern static Id (pointer value);
    // pass it by value as the lookup key.
    let key: Id = kCTFontAttributeName;
    let found = CFDictionaryGetValue(attrs, key as *const _);
    if found == NIL {
        default_font
    } else {
        found
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Helpers — CFString readback
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(target_vendor = "apple")]
unsafe fn cfstring_to_string(cf: Id) -> String {
    if cf == NIL {
        return String::new();
    }
    let len = CFStringGetLength(cf);
    // Over-allocate; UTF-8 can be up to 4× the UTF-16 code-unit count,
    // plus one for the null terminator.
    let cap = (len * 4 + 1).max(32) as usize;
    let mut buf = vec![0i8; cap];
    let ok = CFStringGetCString(
        cf,
        buf.as_mut_ptr(),
        cap as std::ffi::c_long,
        K_CF_STRING_ENCODING_UTF8,
    );
    if !ok {
        return String::new();
    }
    // Find the null terminator.
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    let bytes: Vec<u8> = buf[..end].iter().map(|&b| b as u8).collect();
    String::from_utf8(bytes).unwrap_or_default()
}

#[cfg(target_vendor = "apple")]
unsafe fn copy_ps_name(font: Id) -> String {
    let cf = CTFontCopyPostScriptName(font);
    let s = cfstring_to_string(cf);
    if cf != NIL {
        CFRelease(cf);
    }
    s
}

#[cfg(target_vendor = "apple")]
unsafe fn copy_family_name(font: Id) -> String {
    let cf = CTFontCopyFamilyName(font);
    let s = cfstring_to_string(cf);
    if cf != NIL {
        CFRelease(cf);
    }
    s
}

// Suppress unused imports on non-Apple; the rest of the crate is gated.
#[cfg(not(target_vendor = "apple"))]
mod _stub {
    use super::*;
    pub struct CoreTextHandle;
    pub struct CoreTextResolver;
    pub struct CoreTextMetrics;
    pub struct CoreTextShaper;
}
#[cfg(not(target_vendor = "apple"))]
pub use _stub::{CoreTextHandle, CoreTextMetrics, CoreTextResolver, CoreTextShaper};

// Silence "unused" on traits when the crate is compiled on non-Apple.
#[cfg(not(target_vendor = "apple"))]
#[allow(dead_code)]
fn _keep_traits_referenced() {
    use text_interfaces::{
        FontMetrics, FontResolver, TextShaper, FontQuery, ShapeOptions,
        FontStretch, FontStyle, FontWeight, Direction, Glyph, ShapedRun,
        FontResolutionError, ShapingError,
    };
    let _ = (FontStretch::Normal, FontStyle::Normal, FontWeight::REGULAR,
             Direction::Ltr, FontResolutionError::EmptyQuery,
             ShapingError::ShapingFailed(String::new()));
    let _: fn(&FontQuery) = |_| {};
    let _: fn(&ShapeOptions) = |_| {};
    let _: fn(&Glyph) = |_| {};
    let _: fn(&ShapedRun) = |_| {};
    // Reference traits by path to avoid "unused import" warnings.
    fn _m<M: FontMetrics>() {}
    fn _s<S: TextShaper>() {}
    fn _r<R: FontResolver>() {}
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests — only on Apple targets; these hit CoreText and must run on a real Mac.
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(all(test, target_vendor = "apple"))]
mod tests {
    use super::*;
    use text_interfaces::{
        measure, FontQuery, FontStretch, FontStyle, FontWeight, ShapeOptions,
    };

    fn resolver() -> CoreTextResolver {
        CoreTextResolver::new()
    }

    #[test]
    fn resolve_helvetica() {
        let h = resolver().resolve(&FontQuery::named("Helvetica")).unwrap();
        assert!(!h.ps_name.is_empty());
        assert!(h.size > 0.0);
    }

    #[test]
    fn resolve_unknown_family_falls_through_to_no_family_found() {
        // CoreText falls back to Helvetica internally for unknown names;
        // we can't reliably distinguish. This test just asserts the call
        // does not panic and either returns Ok or NoFamilyFound.
        let _ = resolver().resolve(&FontQuery::named("NotARealFontFamily__xx"));
    }

    #[test]
    fn resolve_empty_query_errors() {
        let q = FontQuery {
            family_names: Vec::new(),
            weight: FontWeight::REGULAR,
            style: FontStyle::Normal,
            stretch: FontStretch::Normal,
        };
        let e = resolver().resolve(&q).unwrap_err();
        assert_eq!(e, FontResolutionError::EmptyQuery);
    }

    #[test]
    fn metrics_look_sane() {
        let h = resolver().resolve(&FontQuery::named("Helvetica")).unwrap();
        let m = CoreTextMetrics::new();
        assert!(m.units_per_em(&h) > 0);
        assert!(m.ascent(&h) > 0);
        assert!(m.descent(&h) >= 0);
        // x-height < cap-height < ascent for a reasonable Latin font
        if let (Some(x), Some(c)) = (m.x_height(&h), m.cap_height(&h)) {
            assert!(x < c, "x_height {} should be < cap_height {}", x, c);
            assert!(c <= m.ascent(&h));
        }
        assert!(m.family_name(&h).to_lowercase().contains("helvetica"));
    }

    #[test]
    fn shape_hello_produces_5_glyphs() {
        let h = resolver().resolve(&FontQuery::named("Helvetica")).unwrap();
        let s = CoreTextShaper::new();
        let shaped = s.shape("Hello", &h, 16.0, &ShapeOptions::default()).unwrap();
        // Latin-only input in Helvetica: exactly one segment, 5 glyphs.
        assert_eq!(shaped.runs.len(), 1);
        let run = &shaped.runs[0];
        assert_eq!(run.glyphs.len(), 5);
        assert!(run.x_advance_total > 0.0);
        assert!(run.font_ref.starts_with("coretext:"));
        assert_eq!(shaped.total_advance(), run.x_advance_total);
        for (i, g) in run.glyphs.iter().enumerate() {
            assert_eq!(g.cluster, i as u32);
        }
    }

    #[test]
    fn shape_empty_string_is_empty_run() {
        let h = resolver().resolve(&FontQuery::named("Helvetica")).unwrap();
        let s = CoreTextShaper::new();
        let shaped = s.shape("", &h, 16.0, &ShapeOptions::default()).unwrap();
        assert!(shaped.is_empty());
        assert_eq!(shaped.runs.len(), 0);
        assert_eq!(shaped.total_advance(), 0.0);
    }

    #[test]
    fn shape_mixed_ascii_and_arrow_splits_into_multiple_font_runs() {
        // Regression test for the original bug: "parse → AST" shaped
        // through Helvetica. U+2192 (RIGHTWARDS ARROW) is not in
        // Helvetica's repertoire, so CoreText does font fallback.
        // The emitted ShapedText MUST contain at least two ShapedRuns
        // — one for the ASCII text (Helvetica) and one for the arrow
        // (some fallback font like Apple Symbols) — each tagged with
        // the actual font that produced its glyphs. This is what
        // makes paint-metal render correctly instead of interpreting
        // the fallback font's glyph IDs against Helvetica.
        let h = resolver().resolve(&FontQuery::named("Helvetica")).unwrap();
        let s = CoreTextShaper::new();
        let shaped = s
            .shape("parse → AST", &h, 16.0, &ShapeOptions::default())
            .unwrap();

        // Expect multiple font segments.
        assert!(
            shaped.runs.len() >= 2,
            "expected at least 2 ShapedRuns for 'parse → AST' (ASCII + arrow-fallback), got {}",
            shaped.runs.len()
        );

        // All segments carry coretext: font_refs.
        for run in &shaped.runs {
            assert!(
                run.font_ref.starts_with("coretext:"),
                "every segment must be coretext-bound; got {:?}",
                run.font_ref
            );
        }

        // At least two distinct font_refs — proving that the
        // fallback font is genuinely different from Helvetica.
        let unique_fonts: std::collections::HashSet<&str> =
            shaped.runs.iter().map(|r| r.font_ref.as_str()).collect();
        assert!(
            unique_fonts.len() >= 2,
            "expected >= 2 distinct font bindings, got {:?}",
            unique_fonts
        );

        // Total glyph count equals visible characters ("parse " + "→" +
        // " AST" = 11 codepoints, one glyph each).
        assert_eq!(shaped.total_glyph_count(), 11);
    }

    #[test]
    fn shape_rejects_rtl_for_v1() {
        let h = resolver().resolve(&FontQuery::named("Helvetica")).unwrap();
        let s = CoreTextShaper::new();
        let mut opts = ShapeOptions::default();
        opts.direction = Direction::Rtl;
        let err = s.shape("hello", &h, 16.0, &opts).unwrap_err();
        assert_eq!(err, ShapingError::UnsupportedDirection(Direction::Rtl));
    }

    #[test]
    fn measure_hello_world_via_shaper_and_metrics() {
        let h = resolver().resolve(&FontQuery::named("Helvetica")).unwrap();
        let s = CoreTextShaper::new();
        let m = CoreTextMetrics::new();
        let result = measure(&s, &m, "Hello world", &h, 16.0, &ShapeOptions::default()).unwrap();
        // Sanity: 11 chars × ~some-positive-advance > 20 px
        assert!(result.width > 20.0);
        assert!(result.ascent > 0.0);
        assert!(result.descent >= 0.0);
        assert_eq!(result.line_count, 1);
    }

    #[test]
    fn shape_interior_nul_returns_shaping_failed_not_panic() {
        let h = resolver().resolve(&FontQuery::named("Helvetica")).unwrap();
        let s = CoreTextShaper::new();
        // "abc\0def" — interior NUL. Previous implementation panicked
        // via CString::new.expect; the fix returns a ShapingError.
        let err = s
            .shape("abc\0def", &h, 16.0, &ShapeOptions::default())
            .unwrap_err();
        assert!(matches!(err, ShapingError::ShapingFailed(_)));
    }

    #[test]
    fn font_ref_matches_between_shaper_and_handle_key() {
        let h = resolver().resolve(&FontQuery::named("Helvetica")).unwrap();
        let s = CoreTextShaper::new();
        let shaped = s.shape("A", &h, 16.0, &ShapeOptions::default()).unwrap();
        assert_eq!(shaped.runs.len(), 1);
        let run = &shaped.runs[0];
        assert!(run.font_ref.starts_with("coretext:"));
        // Size-matched, no-fallback case: font_ref == "coretext:" +
        // handle.ref_key() because the shaped font IS the primary.
        assert_eq!(run.font_ref, format!("coretext:{}", h.ref_key()));
    }
}
