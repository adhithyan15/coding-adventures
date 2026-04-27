//! # text-native
//!
//! Cross-platform facade for the device-dependent native text pipeline.
//! It re-exports the resolver, metrics, shaper, and handle types for the
//! current platform so downstream code can keep one call site while the
//! font-binding invariant stays typed.
//!
//! Backends:
//! - Apple targets: CoreText via `text-native-coretext`.
//! - Windows: DirectWrite via the `windows` crate.
//! - Other platforms: stubs that return recoverable errors until a native
//!   backend lands.

pub use text_interfaces;

#[cfg(target_vendor = "apple")]
pub use text_native_coretext as backend;

#[cfg(target_vendor = "apple")]
pub type NativeResolver = text_native_coretext::CoreTextResolver;
#[cfg(target_vendor = "apple")]
pub type NativeMetrics = text_native_coretext::CoreTextMetrics;
#[cfg(target_vendor = "apple")]
pub type NativeShaper = text_native_coretext::CoreTextShaper;
#[cfg(target_vendor = "apple")]
pub type NativeHandle = text_native_coretext::CoreTextHandle;

#[cfg(target_os = "windows")]
mod directwrite {
    use text_interfaces::{
        Direction, FontMetrics, FontQuery, FontResolutionError, FontResolver, FontStretch,
        FontStyle, Glyph, ShapeOptions, ShapedRun, ShapedText, ShapingError, TextShaper,
    };
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{BOOL, FALSE};
    use windows::Win32::Graphics::DirectWrite::{
        DWriteCreateFactory, IDWriteFactory, IDWriteFontCollection, IDWriteFontFace,
        DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_METRICS, DWRITE_FONT_STRETCH,
        DWRITE_FONT_STRETCH_CONDENSED, DWRITE_FONT_STRETCH_EXPANDED,
        DWRITE_FONT_STRETCH_EXTRA_CONDENSED, DWRITE_FONT_STRETCH_EXTRA_EXPANDED,
        DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STRETCH_SEMI_CONDENSED,
        DWRITE_FONT_STRETCH_SEMI_EXPANDED, DWRITE_FONT_STRETCH_ULTRA_CONDENSED,
        DWRITE_FONT_STRETCH_ULTRA_EXPANDED, DWRITE_FONT_STYLE, DWRITE_FONT_STYLE_ITALIC,
        DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STYLE_OBLIQUE, DWRITE_FONT_WEIGHT,
        DWRITE_GLYPH_METRICS,
    };

    #[derive(Clone)]
    pub struct DirectWriteHandle {
        face: IDWriteFontFace,
        family: String,
        weight: u16,
        style: FontStyle,
        stretch: FontStretch,
    }

    impl std::fmt::Debug for DirectWriteHandle {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("DirectWriteHandle")
                .field("family", &self.family)
                .field("weight", &self.weight)
                .field("style", &self.style)
                .field("stretch", &self.stretch)
                .finish()
        }
    }

    pub struct DirectWriteResolver {
        _factory: IDWriteFactory,
        collection: IDWriteFontCollection,
    }

    impl DirectWriteResolver {
        pub fn new() -> Self {
            unsafe {
                let factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)
                    .expect("failed to create DirectWrite factory");
                let mut collection = None;
                factory
                    .GetSystemFontCollection(&mut collection, FALSE)
                    .expect("failed to load DirectWrite system font collection");
                Self {
                    _factory: factory,
                    collection: collection.expect("DirectWrite system font collection"),
                }
            }
        }

        fn resolve_family(
            &self,
            family: &str,
            weight: u16,
            style: FontStyle,
            stretch: FontStretch,
        ) -> Option<DirectWriteHandle> {
            let family = map_family_name(family);
            unsafe {
                let family_w = wide_null(&family);
                let mut index = 0u32;
                let mut exists = BOOL(0);
                self.collection
                    .FindFamilyName(PCWSTR(family_w.as_ptr()), &mut index, &mut exists)
                    .ok()?;
                if !exists.as_bool() {
                    return None;
                }
                let family_obj = self.collection.GetFontFamily(index).ok()?;
                let font = family_obj
                    .GetFirstMatchingFont(
                        DWRITE_FONT_WEIGHT(weight as i32),
                        dwrite_stretch(stretch),
                        dwrite_style(style),
                    )
                    .ok()?;
                let face = font.CreateFontFace().ok()?;
                Some(DirectWriteHandle {
                    face,
                    family,
                    weight,
                    style,
                    stretch,
                })
            }
        }
    }

    impl Default for DirectWriteResolver {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FontResolver for DirectWriteResolver {
        type Handle = DirectWriteHandle;

        fn resolve(&self, query: &FontQuery) -> Result<Self::Handle, FontResolutionError> {
            if query.family_names.is_empty() {
                return Err(FontResolutionError::EmptyQuery);
            }
            if query.weight.0 == 0 || query.weight.0 > 1000 {
                return Err(FontResolutionError::InvalidWeight(query.weight.0));
            }

            for family in &query.family_names {
                if let Some(handle) =
                    self.resolve_family(family, query.weight.0, query.style, query.stretch)
                {
                    return Ok(handle);
                }
            }

            self.resolve_family("Segoe UI", query.weight.0, query.style, query.stretch)
                .ok_or(FontResolutionError::NoFamilyFound)
        }
    }

    pub struct DirectWriteMetrics;

    impl DirectWriteMetrics {
        pub fn new() -> Self {
            Self
        }
    }

    impl Default for DirectWriteMetrics {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FontMetrics for DirectWriteMetrics {
        type Handle = DirectWriteHandle;

        fn units_per_em(&self, font: &Self::Handle) -> u32 {
            metrics(font).designUnitsPerEm as u32
        }

        fn ascent(&self, font: &Self::Handle) -> i32 {
            metrics(font).ascent as i32
        }

        fn descent(&self, font: &Self::Handle) -> i32 {
            metrics(font).descent as i32
        }

        fn line_gap(&self, font: &Self::Handle) -> i32 {
            metrics(font).lineGap as i32
        }

        fn x_height(&self, font: &Self::Handle) -> Option<i32> {
            let v = metrics(font).xHeight as i32;
            (v > 0).then_some(v)
        }

        fn cap_height(&self, font: &Self::Handle) -> Option<i32> {
            let v = metrics(font).capHeight as i32;
            (v > 0).then_some(v)
        }

        fn family_name(&self, font: &Self::Handle) -> String {
            font.family.clone()
        }
    }

    pub struct DirectWriteShaper;

    impl DirectWriteShaper {
        pub fn new() -> Self {
            Self
        }
    }

    impl Default for DirectWriteShaper {
        fn default() -> Self {
            Self::new()
        }
    }

    impl TextShaper for DirectWriteShaper {
        type Handle = DirectWriteHandle;

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
            if text.is_empty() {
                return Ok(ShapedText::empty());
            }

            let codepoints: Vec<u32> = text.chars().map(|c| c as u32).collect();
            let clusters: Vec<u32> = text.char_indices().map(|(i, _)| i as u32).collect();
            let mut glyph_indices = vec![0u16; codepoints.len()];
            unsafe {
                font.face
                    .GetGlyphIndices(
                        codepoints.as_ptr(),
                        codepoints.len() as u32,
                        glyph_indices.as_mut_ptr(),
                    )
                    .map_err(|e| ShapingError::ShapingFailed(e.to_string()))?;
            }

            let mut glyph_metrics = vec![DWRITE_GLYPH_METRICS::default(); glyph_indices.len()];
            unsafe {
                font.face
                    .GetDesignGlyphMetrics(
                        glyph_indices.as_ptr(),
                        glyph_indices.len() as u32,
                        glyph_metrics.as_mut_ptr(),
                        FALSE,
                    )
                    .map_err(|e| ShapingError::ShapingFailed(e.to_string()))?;
            }

            let upem = metrics(font).designUnitsPerEm.max(1) as f32;
            let scale = size / upem;
            let mut total = 0.0f32;
            let glyphs = glyph_indices
                .iter()
                .zip(glyph_metrics.iter())
                .zip(clusters.iter())
                .map(|((glyph_id, metric), cluster)| {
                    let advance = metric.advanceWidth as f32 * scale;
                    total += advance;
                    Glyph {
                        glyph_id: *glyph_id as u32,
                        cluster: *cluster,
                        x_advance: advance,
                        y_advance: 0.0,
                        x_offset: 0.0,
                        y_offset: 0.0,
                    }
                })
                .collect();

            Ok(ShapedText::single(ShapedRun {
                glyphs,
                x_advance_total: total,
                font_ref: self.font_ref(font),
            }))
        }

        fn font_ref(&self, font: &Self::Handle) -> String {
            format!(
                "directwrite:{}@16;w={};style={};stretch={}",
                escape_ref_component(&font.family),
                font.weight,
                style_name(font.style),
                font.stretch.rank()
            )
        }
    }

    fn metrics(font: &DirectWriteHandle) -> DWRITE_FONT_METRICS {
        let mut metrics = DWRITE_FONT_METRICS::default();
        unsafe {
            font.face.GetMetrics(&mut metrics);
        }
        metrics
    }

    fn map_family_name(family: &str) -> String {
        match family.trim().to_ascii_lowercase().as_str() {
            "" | "system-ui" | "sans-serif" | "helvetica" | "menlo" => "Segoe UI".to_string(),
            "monospace" => "Consolas".to_string(),
            _ => family.to_string(),
        }
    }

    fn style_name(style: FontStyle) -> &'static str {
        match style {
            FontStyle::Italic => "italic",
            FontStyle::Oblique => "oblique",
            FontStyle::Normal => "normal",
        }
    }

    fn dwrite_style(style: FontStyle) -> DWRITE_FONT_STYLE {
        match style {
            FontStyle::Italic => DWRITE_FONT_STYLE_ITALIC,
            FontStyle::Oblique => DWRITE_FONT_STYLE_OBLIQUE,
            FontStyle::Normal => DWRITE_FONT_STYLE_NORMAL,
        }
    }

    fn dwrite_stretch(stretch: FontStretch) -> DWRITE_FONT_STRETCH {
        match stretch.rank() {
            1 => DWRITE_FONT_STRETCH_ULTRA_CONDENSED,
            2 => DWRITE_FONT_STRETCH_EXTRA_CONDENSED,
            3 => DWRITE_FONT_STRETCH_CONDENSED,
            4 => DWRITE_FONT_STRETCH_SEMI_CONDENSED,
            6 => DWRITE_FONT_STRETCH_SEMI_EXPANDED,
            7 => DWRITE_FONT_STRETCH_EXPANDED,
            8 => DWRITE_FONT_STRETCH_EXTRA_EXPANDED,
            9 => DWRITE_FONT_STRETCH_ULTRA_EXPANDED,
            _ => DWRITE_FONT_STRETCH_NORMAL,
        }
    }

    fn escape_ref_component(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => out.push(b as char),
                _ => out.push_str(&format!("%{:02X}", b)),
            }
        }
        out
    }

    fn wide_null(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(target_os = "windows")]
pub use directwrite::{
    DirectWriteHandle as NativeHandle, DirectWriteMetrics as NativeMetrics,
    DirectWriteResolver as NativeResolver, DirectWriteShaper as NativeShaper,
};

#[cfg(all(not(target_vendor = "apple"), not(target_os = "windows")))]
pub type NativeResolver = UnimplementedNativeBackend;
#[cfg(all(not(target_vendor = "apple"), not(target_os = "windows")))]
pub type NativeMetrics = UnimplementedNativeBackend;
#[cfg(all(not(target_vendor = "apple"), not(target_os = "windows")))]
pub type NativeShaper = UnimplementedNativeBackend;
#[cfg(all(not(target_vendor = "apple"), not(target_os = "windows")))]
pub type NativeHandle = ();

#[cfg(all(not(target_vendor = "apple"), not(target_os = "windows")))]
pub struct UnimplementedNativeBackend;

#[cfg(all(not(target_vendor = "apple"), not(target_os = "windows")))]
impl UnimplementedNativeBackend {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(all(not(target_vendor = "apple"), not(target_os = "windows")))]
impl Default for UnimplementedNativeBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(not(target_vendor = "apple"), not(target_os = "windows")))]
impl text_interfaces::FontResolver for UnimplementedNativeBackend {
    type Handle = ();

    fn resolve(
        &self,
        _query: &text_interfaces::FontQuery,
    ) -> Result<Self::Handle, text_interfaces::FontResolutionError> {
        Err(text_interfaces::FontResolutionError::LoadFailed(
            "text-native has no backend implemented for this target OS".into(),
        ))
    }
}

#[cfg(all(not(target_vendor = "apple"), not(target_os = "windows")))]
impl text_interfaces::FontMetrics for UnimplementedNativeBackend {
    type Handle = ();

    fn units_per_em(&self, _font: &Self::Handle) -> u32 {
        0
    }
    fn ascent(&self, _font: &Self::Handle) -> i32 {
        0
    }
    fn descent(&self, _font: &Self::Handle) -> i32 {
        0
    }
    fn line_gap(&self, _font: &Self::Handle) -> i32 {
        0
    }
    fn x_height(&self, _font: &Self::Handle) -> Option<i32> {
        None
    }
    fn cap_height(&self, _font: &Self::Handle) -> Option<i32> {
        None
    }
    fn family_name(&self, _font: &Self::Handle) -> String {
        String::from("unimplemented")
    }
}

#[cfg(all(not(target_vendor = "apple"), not(target_os = "windows")))]
impl text_interfaces::TextShaper for UnimplementedNativeBackend {
    type Handle = ();

    fn shape(
        &self,
        _text: &str,
        _font: &Self::Handle,
        _size: f32,
        _options: &text_interfaces::ShapeOptions,
    ) -> Result<text_interfaces::ShapedText, text_interfaces::ShapingError> {
        Err(text_interfaces::ShapingError::ShapingFailed(
            "text-native has no backend implemented for this target OS".into(),
        ))
    }

    fn font_ref(&self, _font: &Self::Handle) -> String {
        String::from("unimplemented:")
    }
}

pub const VERSION: &str = "0.1.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[cfg(target_vendor = "apple")]
    #[test]
    fn native_types_are_the_coretext_types_on_apple() {
        fn _takes_coretext(_r: text_native_coretext::CoreTextResolver) {}
        let r: NativeResolver = NativeResolver::new();
        _takes_coretext(r);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_resolver_shapes_segoe_ui_text() {
        use text_interfaces::{FontQuery, FontResolver, ShapeOptions, TextShaper};

        let resolver = NativeResolver::new();
        let shaper = NativeShaper::new();
        let handle = resolver.resolve(&FontQuery::named("Segoe UI")).unwrap();
        let shaped = shaper
            .shape("Hello", &handle, 16.0, &ShapeOptions::default())
            .unwrap();
        assert_eq!(shaped.total_glyph_count(), 5);
        assert!(shaped.total_advance() > 0.0);
        assert!(shaped.runs[0].font_ref.starts_with("directwrite:"));
    }

    #[cfg(all(not(target_vendor = "apple"), not(target_os = "windows")))]
    #[test]
    fn unsupported_resolver_returns_load_failed() {
        use text_interfaces::{FontQuery, FontResolutionError, FontResolver};
        let r = NativeResolver::new();
        let err = r.resolve(&FontQuery::named("Helvetica")).unwrap_err();
        assert!(matches!(err, FontResolutionError::LoadFailed(_)));
    }
}
