//! # text-interfaces
//!
//! Rust implementation of the TXT00 spec — the three orthogonal, pluggable
//! traits that sit between the layout layer and the paint layer for all text
//! rendering in the coding-adventures stack:
//!
//! - [`FontMetrics`] — font-global metrics (ascent, descent, line gap, …)
//! - [`TextShaper`]  — codepoints → positioned glyph run (cmap, GSUB, GPOS)
//! - [`measure`]     — thin convenience wrapping a shaper (not a parallel trait)
//!
//! Plus [`FontResolver`], the entry point that maps abstract [`FontQuery`]s
//! to concrete font handles (TXT05).
//!
//! ## Design commitments
//!
//! - **Orthogonal.** Shapers are pluggable independently of metrics; metrics
//!   are pluggable independently of shaping. Swapping one does not force
//!   re-implementation of the others.
//! - **Generic over the backend's handle type.** Each implementation commits
//!   to one handle type (a `font_parser::FontFile`, a `CTFontRef`, an
//!   `IDWriteFontFace`, …) via the `Handle` associated type. This makes the
//!   font-binding invariant a compile-time guarantee: a CoreText handle
//!   cannot be passed to a font-parser metrics implementation, because the
//!   Rust type system rejects the mismatch.
//! - **Infallible on the getter path.** [`FontMetrics`] methods return
//!   concrete values, not [`Result`]s — a handle that made it past the
//!   resolver is assumed to be valid. Only shaping ([`TextShaper::shape`])
//!   and resolution ([`FontResolver::resolve`]) can fail.
//!
//! ## The font-binding invariant
//!
//! Glyph IDs are **opaque tokens** bound to the shaper that produced them.
//! A `ShapedRun` from a CoreText-backed shaper can only be correctly
//! rendered by a rasterizer that understands the same CoreText font. See
//! `font_ref` below — every shaped run carries a scheme-prefixed string
//! (e.g. `"coretext:Helvetica-Bold@16.0"`, `"font-parser:<hash>"`) that
//! the paint backend routes on.

pub const VERSION: &str = "0.1.0";

// ═══════════════════════════════════════════════════════════════════════════
// FontQuery — the abstract font request
// ═══════════════════════════════════════════════════════════════════════════

/// An abstract font request: family name list, weight, style, stretch.
///
/// A `FontQuery` is what an author-facing style description ("font-family:
/// Helvetica; font-weight: 700; font-style: italic") becomes. The
/// [`FontResolver`] walks the family names in order and returns the first
/// available face, picking the best weight/style/stretch distance within
/// that family.
///
/// Pure data — construct and inspect freely.
#[derive(Clone, Debug, PartialEq)]
pub struct FontQuery {
    /// Ordered family-name fallback list, highest preference first.
    /// May mix concrete families ("Inter", "Helvetica") with generics
    /// ("sans-serif", "serif", "monospace", "cursive", "fantasy",
    /// "system-ui"). An empty list is a programming error —
    /// [`FontResolver::resolve`] returns [`FontResolutionError::EmptyQuery`].
    pub family_names: Vec<String>,

    /// CSS-style numeric weight, 1..=1000. Canonical values:
    /// 100 Thin, 200 ExtraLight, 300 Light, 400 Regular (default),
    /// 500 Medium, 600 SemiBold, 700 Bold, 800 ExtraBold, 900 Black.
    /// Variable-font weights accept any value in the axis range.
    pub weight: FontWeight,

    /// Normal / Italic / Oblique. See [`FontStyle`].
    pub style: FontStyle,

    /// Width category from UltraCondensed to UltraExpanded. See
    /// [`FontStretch`].
    pub stretch: FontStretch,
}

impl FontQuery {
    /// Convenience: build a query with a single family name and default
    /// weight (400 Regular), style (Normal), stretch (Normal).
    pub fn named(family: impl Into<String>) -> Self {
        Self {
            family_names: vec![family.into()],
            weight: FontWeight::REGULAR,
            style: FontStyle::Normal,
            stretch: FontStretch::Normal,
        }
    }

    /// Convenience: add a fallback family name to the end of the list.
    pub fn with_fallback(mut self, family: impl Into<String>) -> Self {
        self.family_names.push(family.into());
        self
    }

    /// Convenience setters.
    pub fn with_weight(mut self, w: FontWeight) -> Self {
        self.weight = w;
        self
    }
    pub fn with_style(mut self, s: FontStyle) -> Self {
        self.style = s;
        self
    }
    pub fn with_stretch(mut self, s: FontStretch) -> Self {
        self.stretch = s;
        self
    }
}

/// CSS-style numeric font weight in the range 1..=1000.
///
/// Stored as a simple `u16` newtype. Helper constants name the canonical
/// values; callers can pass any numeric weight for variable fonts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontWeight(pub u16);

impl FontWeight {
    pub const THIN: Self = Self(100);
    pub const EXTRA_LIGHT: Self = Self(200);
    pub const LIGHT: Self = Self(300);
    pub const REGULAR: Self = Self(400);
    pub const MEDIUM: Self = Self(500);
    pub const SEMI_BOLD: Self = Self(600);
    pub const BOLD: Self = Self(700);
    pub const EXTRA_BOLD: Self = Self(800);
    pub const BLACK: Self = Self(900);
}

/// Font style axis: upright, separately-designed italic, or synthesized
/// oblique.
///
/// `Oblique` is an upright face rendered with a slant applied by the shaper
/// or rasterizer. `Italic` is a distinct face with its own design. The
/// resolver's matching algorithm MAY substitute one for the other (at
/// higher distance cost) when the exact style is unavailable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

/// Font width axis: UltraCondensed..UltraExpanded, nine steps.
///
/// Each variant has a rank 1..=9 used for distance computation in the
/// resolver's matching algorithm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontStretch {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    Normal,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
}

impl FontStretch {
    /// Numeric rank 1..=9 for distance computation.
    pub fn rank(self) -> u8 {
        match self {
            Self::UltraCondensed => 1,
            Self::ExtraCondensed => 2,
            Self::Condensed => 3,
            Self::SemiCondensed => 4,
            Self::Normal => 5,
            Self::SemiExpanded => 6,
            Self::Expanded => 7,
            Self::ExtraExpanded => 8,
            Self::UltraExpanded => 9,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// FontResolver — FontQuery → FontHandle
// ═══════════════════════════════════════════════════════════════════════════

/// Resolves an abstract [`FontQuery`] into a concrete backend-specific
/// font handle.
///
/// The `Handle` associated type is implementation-defined. For a
/// font-parser-backed resolver it is a `FontFile` reference; for CoreText
/// it is a `CTFontRef`; for DirectWrite an `IDWriteFontFace`.
///
/// A handle obtained from one backend's resolver is only valid with that
/// backend's [`FontMetrics`] and [`TextShaper`]. Mixing bindings is
/// undefined behaviour. In statically-typed Rust this is enforced by the
/// associated-type machinery: a
/// `FontResolver<Handle = CTFontRef>`'s output can only satisfy the
/// `FontMetrics<Handle = CTFontRef>` and `TextShaper<Handle = CTFontRef>`
/// trait implementations paired with it.
pub trait FontResolver {
    /// The font handle type this resolver produces.
    type Handle;

    /// Resolve the query to a handle. Returns the best available match
    /// per the matching algorithm documented in TXT05. Errors with
    /// [`FontResolutionError`] if nothing matches.
    fn resolve(&self, query: &FontQuery) -> Result<Self::Handle, FontResolutionError>;

    /// Default-implemented: check whether a family name is resolvable at
    /// all, without committing to a full resolution. Backends MAY
    /// override for a faster path.
    fn has_family(&self, family: &str) -> bool {
        self.resolve(&FontQuery::named(family)).is_ok()
    }
}

/// Errors returned by [`FontResolver::resolve`].
#[derive(Clone, Debug, PartialEq)]
pub enum FontResolutionError {
    /// The query's `family_names` list was empty.
    EmptyQuery,
    /// None of the family names resolved to any registered face.
    NoFamilyFound,
    /// The `weight` field was outside 1..=1000.
    InvalidWeight(u16),
    /// A matching face was located but its bytes could not be loaded
    /// (I/O error, malformed file, permission denied, …).
    LoadFailed(String),
}

impl core::fmt::Display for FontResolutionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyQuery => write!(f, "FontResolutionError::EmptyQuery: family_names is empty"),
            Self::NoFamilyFound => {
                write!(f, "FontResolutionError::NoFamilyFound: no registered face matches the query")
            }
            Self::InvalidWeight(w) => {
                write!(f, "FontResolutionError::InvalidWeight({}): weight must be in 1..=1000", w)
            }
            Self::LoadFailed(msg) => write!(f, "FontResolutionError::LoadFailed: {}", msg),
        }
    }
}

impl std::error::Error for FontResolutionError {}

// ═══════════════════════════════════════════════════════════════════════════
// FontMetrics — global font metrics
// ═══════════════════════════════════════════════════════════════════════════

/// Font-global metrics: ascent, descent, line gap, x-height, cap-height,
/// units-per-em, family name.
///
/// Every method takes a handle of the implementation-defined `Handle`
/// type. Values returned by the integer methods are in the font's design
/// units; callers scale by `font_size / units_per_em` to convert to
/// user-space.
///
/// The `descent` method returns a non-negative distance below the
/// baseline (not the signed value from the `hhea` / OS/2 tables).
///
/// Methods are infallible: a handle that made it past the resolver is
/// assumed to correspond to a successfully-loaded font.
pub trait FontMetrics {
    type Handle;

    fn units_per_em(&self, font: &Self::Handle) -> u32;
    fn ascent(&self, font: &Self::Handle) -> i32;
    /// Distance below baseline as a non-negative integer (not the signed
    /// value stored in the font).
    fn descent(&self, font: &Self::Handle) -> i32;
    fn line_gap(&self, font: &Self::Handle) -> i32;
    fn x_height(&self, font: &Self::Handle) -> Option<i32>;
    fn cap_height(&self, font: &Self::Handle) -> Option<i32>;
    fn family_name(&self, font: &Self::Handle) -> String;
}

// ═══════════════════════════════════════════════════════════════════════════
// TextShaper — string → positioned glyph run
// ═══════════════════════════════════════════════════════════════════════════

/// Shape a string of text into a sequence of positioned glyphs ready
/// for rendering.
///
/// Returns [`ShapedText`] — a sequence of one or more [`ShapedRun`]s.
/// Multiple runs are produced when the shaper performs font fallback:
/// codepoints missing from the caller's primary font get shaped with
/// a secondary font, and the resulting glyph IDs belong to that
/// secondary font. Each `ShapedRun` is tagged with the `font_ref` of
/// the actual font that produced its glyphs, so the font-binding
/// invariant is preserved across the segment boundary.
pub trait TextShaper {
    type Handle;

    /// Shape the given text with the given font at the given size.
    ///
    /// Downstream consumers MUST emit ONE `PaintGlyphRun` (P2D00) per
    /// element of `result.runs`, preserving each segment's `font_ref`
    /// verbatim. The paint backend routes on that string's scheme
    /// prefix to pick the matching rasterizer.
    fn shape(
        &self,
        text: &str,
        font: &Self::Handle,
        size: f32,
        options: &ShapeOptions,
    ) -> Result<ShapedText, ShapingError>;

    /// Return the `font_ref` string this shaper would embed in a
    /// [`ShapedRun`] produced from this handle when no font-fallback
    /// is triggered. Used by callers that want to pre-register a
    /// font in a paint-backend registry before shaping.
    fn font_ref(&self, font: &Self::Handle) -> String;
}

/// Per-call shaping options. All fields have defaults (see [`Default`]).
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeOptions {
    /// ISO-15924 script code, e.g. `"Latn"`, `"Arab"`, `"Deva"`. `None`
    /// = auto-detect. Shapers that do not support a requested script
    /// return [`ShapingError::UnsupportedScript`].
    pub script: Option<String>,
    /// BCP-47 language tag, e.g. `"en"`, `"tr"`. `None` = no hint.
    pub language: Option<String>,
    /// Uniform-direction run direction. Bidi resolution is the caller's
    /// job (shapers receive uniform runs).
    pub direction: Direction,
    /// OpenType feature toggles by 4-character tag.
    pub features: Vec<(String, FeatureValue)>,
}

impl Default for ShapeOptions {
    fn default() -> Self {
        Self {
            script: None,
            language: None,
            direction: Direction::Ltr,
            features: Vec::new(),
        }
    }
}

/// Uniform direction of a shaping run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    Ltr,
    Rtl,
    Ttb,
    Btt,
}

/// OpenType feature toggle value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureValue {
    /// On/off toggle (most features).
    Bool(bool),
    /// Alternate-index selector (for `cv01`..`cv99`, `salt`).
    Alt(u32),
}

/// A contiguous span of glyphs from a **single font binding**.
///
/// When a shaper performs font fallback (e.g. the primary font is
/// missing U+2192 → and the system falls back to Apple Symbols), the
/// resulting glyph IDs belong to the fallback font, NOT the primary.
/// Those glyphs are placed in their own `ShapedRun` whose `font_ref`
/// names the fallback. A single call to [`TextShaper::shape`] may
/// therefore produce multiple `ShapedRun`s — see [`ShapedText`].
#[derive(Clone, Debug, PartialEq)]
pub struct ShapedRun {
    /// Per-glyph positioned output. See [`Glyph`].
    pub glyphs: Vec<Glyph>,

    /// Total advance width of this segment, equal to the sum of
    /// `glyph.x_advance` across `glyphs`. Pre-computed so callers do
    /// not re-sum.
    pub x_advance_total: f32,

    /// Scheme-prefixed font-binding identifier for the font that
    /// produced THIS segment's glyph IDs. In a fallback scenario,
    /// different `ShapedRun`s within the same shape() result will
    /// carry different `font_ref`s. Downstream consumers embed this
    /// verbatim in `PaintGlyphRun.font_ref`, emitting one
    /// `PaintGlyphRun` per segment.
    pub font_ref: String,
}

/// The output of [`TextShaper::shape`]: a sequence of one or more
/// [`ShapedRun`]s in source order, each tagged with its actual font
/// binding.
///
/// For single-font content (Latin-only text in a Latin font; the
/// naive TXT02 shaper at all times), `runs` has exactly one element.
/// For content requiring font fallback — an arrow inside Helvetica,
/// emoji in a sentence, CJK in English — `runs` has one entry per
/// contiguous same-font segment.
///
/// Consumers walk `runs` in order, accumulate pen x across segments,
/// and emit **one `PaintGlyphRun` per segment** so each paint
/// instruction carries the correct `font_ref`.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapedText {
    pub runs: Vec<ShapedRun>,
}

impl ShapedText {
    /// An empty result — no glyphs, no runs. Useful as a sentinel for
    /// empty input strings.
    pub fn empty() -> Self {
        Self { runs: Vec::new() }
    }

    /// Wrap a single `ShapedRun` as the degenerate one-segment output
    /// produced by shapers without font-fallback.
    pub fn single(run: ShapedRun) -> Self {
        Self { runs: vec![run] }
    }

    /// Sum of `x_advance_total` across all segments. Useful for
    /// measurement that doesn't care about segmentation.
    pub fn total_advance(&self) -> f32 {
        self.runs.iter().map(|r| r.x_advance_total).sum()
    }

    /// Total glyph count across all segments.
    pub fn total_glyph_count(&self) -> usize {
        self.runs.iter().map(|r| r.glyphs.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.runs.iter().all(|r| r.glyphs.is_empty())
    }
}

/// One glyph in a [`ShapedRun`]. All positional values are in user-space
/// units at the requested shape size (already scaled — no design-unit
/// conversion needed by the caller).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    /// Backend-specific glyph ID. Only meaningful via the shaper that
    /// produced it AND the matching rasterizer.
    pub glyph_id: u32,
    /// Byte / code-unit offset into the original text string for the
    /// codepoint(s) this glyph represents. UTF-8 byte offset in Rust by
    /// default; backends MAY document a different unit per their
    /// implementing language.
    pub cluster: u32,
    /// Pen-advance after drawing this glyph.
    pub x_advance: f32,
    /// Vertical pen-advance (usually 0 for horizontal text).
    pub y_advance: f32,
    /// Horizontal adjustment relative to the pen position.
    pub x_offset: f32,
    /// Vertical adjustment (superscript, diacritic, etc.).
    pub y_offset: f32,
}

/// Errors returned by [`TextShaper::shape`].
#[derive(Clone, Debug, PartialEq)]
pub enum ShapingError {
    /// Shaper does not support the requested script, or auto-detect
    /// produced a script the shaper cannot handle.
    UnsupportedScript(String),
    /// Shaper does not support the requested direction (usually RTL or
    /// vertical on shapers that are ltr-only).
    UnsupportedDirection(Direction),
    /// Internal shaping failure: malformed font, out-of-memory, etc.
    ShapingFailed(String),
}

impl core::fmt::Display for ShapingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedScript(s) => write!(f, "ShapingError::UnsupportedScript({})", s),
            Self::UnsupportedDirection(d) => {
                write!(f, "ShapingError::UnsupportedDirection({:?})", d)
            }
            Self::ShapingFailed(msg) => write!(f, "ShapingError::ShapingFailed: {}", msg),
        }
    }
}

impl std::error::Error for ShapingError {}

// ═══════════════════════════════════════════════════════════════════════════
// TextMeasurer — thin convenience wrapper
// ═══════════════════════════════════════════════════════════════════════════

/// The result of a measurement: width and vertical extents in user-space
/// units at the requested size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeasureResult {
    pub width: f32,
    pub ascent: f32,
    pub descent: f32,
    pub line_count: u32,
}

/// Measure a single-line string by shaping it and reading the shaper's
/// pre-computed total advance plus font-global ascent/descent.
///
/// This is the TXT00 measurement contract — not a trait, just a function.
/// Consumers that need multi-line measurement with wrapping should call
/// the shaper directly and walk the glyph run against their own
/// break-opportunity rules (or use the future line-breaker package).
///
/// Generic over the shaper/metrics pair; both must share a handle type
/// (the font-binding invariant). The caller is responsible for supplying
/// matching implementations.
pub fn measure<S, M>(
    shaper: &S,
    metrics: &M,
    text: &str,
    font: &S::Handle,
    size: f32,
    options: &ShapeOptions,
) -> Result<MeasureResult, ShapingError>
where
    S: TextShaper,
    M: FontMetrics<Handle = S::Handle>,
{
    let shaped = shaper.shape(text, font, size, options)?;
    let upem = metrics.units_per_em(font) as f32;
    let scale = size / upem;
    Ok(MeasureResult {
        // Sum across segments so font-fallback runs contribute to the
        // total line width just like the primary run does.
        width: shaped.total_advance(),
        ascent: metrics.ascent(font) as f32 * scale,
        descent: metrics.descent(font) as f32 * scale,
        line_count: 1,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_weight_constants() {
        assert_eq!(FontWeight::REGULAR, FontWeight(400));
        assert_eq!(FontWeight::BOLD, FontWeight(700));
        assert!(FontWeight::BOLD > FontWeight::REGULAR);
    }

    #[test]
    fn font_stretch_rank_is_monotonic() {
        assert_eq!(FontStretch::UltraCondensed.rank(), 1);
        assert_eq!(FontStretch::Normal.rank(), 5);
        assert_eq!(FontStretch::UltraExpanded.rank(), 9);
    }

    #[test]
    fn font_query_named_sets_defaults() {
        let q = FontQuery::named("Inter");
        assert_eq!(q.family_names, vec!["Inter".to_string()]);
        assert_eq!(q.weight, FontWeight::REGULAR);
        assert_eq!(q.style, FontStyle::Normal);
        assert_eq!(q.stretch, FontStretch::Normal);
    }

    #[test]
    fn font_query_builder_methods() {
        let q = FontQuery::named("Inter")
            .with_fallback("Helvetica")
            .with_weight(FontWeight::BOLD)
            .with_style(FontStyle::Italic)
            .with_stretch(FontStretch::Condensed);
        assert_eq!(q.family_names, vec!["Inter", "Helvetica"]);
        assert_eq!(q.weight, FontWeight::BOLD);
        assert_eq!(q.style, FontStyle::Italic);
        assert_eq!(q.stretch, FontStretch::Condensed);
    }

    #[test]
    fn shape_options_defaults_to_ltr_latin() {
        let o = ShapeOptions::default();
        assert_eq!(o.script, None);
        assert_eq!(o.language, None);
        assert_eq!(o.direction, Direction::Ltr);
        assert!(o.features.is_empty());
    }

    #[test]
    fn font_resolution_error_display() {
        let e = FontResolutionError::NoFamilyFound;
        assert!(e.to_string().contains("NoFamilyFound"));

        let e = FontResolutionError::InvalidWeight(2000);
        assert!(e.to_string().contains("2000"));

        let e = FontResolutionError::LoadFailed("permission denied".into());
        assert!(e.to_string().contains("permission denied"));
    }

    #[test]
    fn shaping_error_display() {
        let e = ShapingError::UnsupportedScript("Arab".into());
        assert!(e.to_string().contains("UnsupportedScript"));
        assert!(e.to_string().contains("Arab"));

        let e = ShapingError::UnsupportedDirection(Direction::Rtl);
        assert!(e.to_string().contains("Rtl"));
    }

    /// A trivial in-memory shaper used to exercise the generic
    /// [`measure`] function. It pretends every glyph is 10 units wide.
    struct FixedWidthShaper;

    impl TextShaper for FixedWidthShaper {
        type Handle = ();

        fn shape(
            &self,
            text: &str,
            _font: &Self::Handle,
            _size: f32,
            _options: &ShapeOptions,
        ) -> Result<ShapedText, ShapingError> {
            let glyphs: Vec<Glyph> = text
                .chars()
                .enumerate()
                .map(|(i, _c)| Glyph {
                    glyph_id: i as u32,
                    cluster: i as u32,
                    x_advance: 10.0,
                    y_advance: 0.0,
                    x_offset: 0.0,
                    y_offset: 0.0,
                })
                .collect();
            let total = glyphs.len() as f32 * 10.0;
            Ok(ShapedText::single(ShapedRun {
                glyphs,
                x_advance_total: total,
                font_ref: "test:fixed-10".into(),
            }))
        }

        fn font_ref(&self, _font: &Self::Handle) -> String {
            "test:fixed-10".into()
        }
    }

    struct FixedMetrics;
    impl FontMetrics for FixedMetrics {
        type Handle = ();
        fn units_per_em(&self, _: &Self::Handle) -> u32 {
            1000
        }
        fn ascent(&self, _: &Self::Handle) -> i32 {
            800
        }
        fn descent(&self, _: &Self::Handle) -> i32 {
            200
        }
        fn line_gap(&self, _: &Self::Handle) -> i32 {
            0
        }
        fn x_height(&self, _: &Self::Handle) -> Option<i32> {
            Some(400)
        }
        fn cap_height(&self, _: &Self::Handle) -> Option<i32> {
            Some(700)
        }
        fn family_name(&self, _: &Self::Handle) -> String {
            "Fixed Test Font".into()
        }
    }

    #[test]
    fn measure_hello_world() {
        let result = measure(
            &FixedWidthShaper,
            &FixedMetrics,
            "Hello",
            &(),
            16.0,
            &ShapeOptions::default(),
        )
        .unwrap();
        assert_eq!(result.width, 50.0);
        // 800 design units at size 16 with upem 1000 = 12.8 user-space
        assert!((result.ascent - 12.8).abs() < 1e-4);
        assert!((result.descent - 3.2).abs() < 1e-4);
        assert_eq!(result.line_count, 1);
    }

    #[test]
    fn measure_empty_string_is_zero_width() {
        let result = measure(
            &FixedWidthShaper,
            &FixedMetrics,
            "",
            &(),
            16.0,
            &ShapeOptions::default(),
        )
        .unwrap();
        assert_eq!(result.width, 0.0);
        assert_eq!(result.line_count, 1);
    }

    #[test]
    fn shaped_run_carries_font_ref() {
        let shaped = FixedWidthShaper
            .shape("ab", &(), 16.0, &ShapeOptions::default())
            .unwrap();
        assert_eq!(shaped.runs.len(), 1);
        assert_eq!(shaped.runs[0].font_ref, "test:fixed-10");
        assert_eq!(shaped.runs[0].glyphs.len(), 2);
        assert_eq!(shaped.runs[0].x_advance_total, 20.0);
        assert_eq!(shaped.total_advance(), 20.0);
        assert_eq!(shaped.total_glyph_count(), 2);
    }

    #[test]
    fn shaped_text_helpers() {
        let empty = ShapedText::empty();
        assert!(empty.is_empty());
        assert_eq!(empty.total_advance(), 0.0);
        assert_eq!(empty.total_glyph_count(), 0);

        let run_a = ShapedRun {
            glyphs: vec![
                Glyph {
                    glyph_id: 1,
                    cluster: 0,
                    x_advance: 8.0,
                    y_advance: 0.0,
                    x_offset: 0.0,
                    y_offset: 0.0,
                },
                Glyph {
                    glyph_id: 2,
                    cluster: 1,
                    x_advance: 8.0,
                    y_advance: 0.0,
                    x_offset: 0.0,
                    y_offset: 0.0,
                },
            ],
            x_advance_total: 16.0,
            font_ref: "coretext:Helvetica@16".into(),
        };
        let run_b = ShapedRun {
            glyphs: vec![Glyph {
                glyph_id: 99,
                cluster: 2,
                x_advance: 14.0,
                y_advance: 0.0,
                x_offset: 0.0,
                y_offset: 0.0,
            }],
            x_advance_total: 14.0,
            font_ref: "coretext:AppleSymbols@16".into(),
        };
        let two_runs = ShapedText {
            runs: vec![run_a, run_b],
        };
        assert!(!two_runs.is_empty());
        assert_eq!(two_runs.total_advance(), 30.0);
        assert_eq!(two_runs.total_glyph_count(), 3);
        assert_ne!(two_runs.runs[0].font_ref, two_runs.runs[1].font_ref);
    }
}
