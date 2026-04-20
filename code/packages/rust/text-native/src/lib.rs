//! # text-native
//!
//! Cross-platform facade for the **device-dependent** (native-OS) text
//! pipeline. Selects the appropriate TXT03 backend at compile time via
//! `cfg` and re-exports it under a common name so downstream callers
//! write one line regardless of target OS:
//!
//! ```ignore
//! use text_native::{NativeResolver, NativeMetrics, NativeShaper};
//! ```
//!
//! ## Backend selection
//!
//! | Target                    | Backend                                          |
//! |---------------------------|--------------------------------------------------|
//! | `target_vendor = "apple"` | [`text_native_coretext`] (TXT03a, CoreText)     |
//! | Windows                   | Not yet implemented (TXT03b, DirectWrite)        |
//! | Linux / BSD               | Not yet implemented (TXT03c, Pango)              |
//!
//! On non-Apple platforms this crate currently compiles as empty stubs
//! so downstream binaries still link; attempting to construct the
//! facade types returns `FontResolutionError::LoadFailed` with a message
//! identifying the missing backend. This lets a cross-platform consumer
//! define a compile-time path to swap in TXT01 + TXT02 / TXT04 (the
//! device-independent path) when no native backend is available.
//!
//! ## Why a wrapper
//!
//! A consumer that writes "give me a native text shaper" does not want
//! to care which OS it is running on. The wrapper:
//! - Keeps the downstream call sites portable.
//! - Preserves the font-binding invariant from TXT00 — each native
//!   backend's handle type is distinct; the wrapper re-exports the
//!   matching resolver + metrics + shaper triple for a single OS, so
//!   the compile-time type unification guarantees they share a binding.
//!
//! ## v1 scope
//!
//! Only macOS / iOS is implemented in v1. Windows and Linux stubs
//! return recoverable errors at construction time so downstream code
//! can detect the platform gap and degrade — for example, by falling
//! back to the device-independent path (text-metrics-font-parser +
//! text-shaper-naive).

pub use text_interfaces;

#[cfg(target_vendor = "apple")]
pub use text_native_coretext as backend;

/// The native `FontResolver` for the current target OS.
///
/// On macOS this is [`text_native_coretext::CoreTextResolver`]. On
/// non-Apple targets the type alias is not available — that's by
/// design: a Rust program that depends on `text_native::NativeResolver`
/// will fail to compile on an unsupported platform, signalling to the
/// build system that a different backend is needed.
#[cfg(target_vendor = "apple")]
pub type NativeResolver = text_native_coretext::CoreTextResolver;

#[cfg(target_vendor = "apple")]
pub type NativeMetrics = text_native_coretext::CoreTextMetrics;

#[cfg(target_vendor = "apple")]
pub type NativeShaper = text_native_coretext::CoreTextShaper;

#[cfg(target_vendor = "apple")]
pub type NativeHandle = text_native_coretext::CoreTextHandle;

// ---------------------------------------------------------------------------
// Non-Apple stubs — empty structs that compile but cannot be constructed
// into functioning implementations. A future TXT03b (DirectWrite) or TXT03c
// (Pango) PR will flesh these out.
// ---------------------------------------------------------------------------

#[cfg(not(target_vendor = "apple"))]
pub type NativeResolver = UnimplementedNativeBackend;

#[cfg(not(target_vendor = "apple"))]
pub type NativeMetrics = UnimplementedNativeBackend;

#[cfg(not(target_vendor = "apple"))]
pub type NativeShaper = UnimplementedNativeBackend;

#[cfg(not(target_vendor = "apple"))]
pub type NativeHandle = ();

/// Placeholder used in all three "native" aliases on non-Apple
/// platforms. Constructing it is legal; calling into it produces a
/// [`text_interfaces::FontResolutionError`] or panics depending on the
/// trait method.
#[cfg(not(target_vendor = "apple"))]
pub struct UnimplementedNativeBackend;

#[cfg(not(target_vendor = "apple"))]
impl UnimplementedNativeBackend {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(target_vendor = "apple"))]
impl Default for UnimplementedNativeBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_vendor = "apple"))]
impl text_interfaces::FontResolver for UnimplementedNativeBackend {
    type Handle = ();

    fn resolve(
        &self,
        _query: &text_interfaces::FontQuery,
    ) -> Result<Self::Handle, text_interfaces::FontResolutionError> {
        Err(text_interfaces::FontResolutionError::LoadFailed(
            "text-native has no backend implemented for this target OS; \
             use the device-independent path (text-metrics-font-parser + \
             text-shaper-naive) or enable a TXT03 backend once available."
                .into(),
        ))
    }
}

// Stub FontMetrics + TextShaper so downstream bridge crates like
// layout-text-measure-native compile on non-Apple platforms. All
// getters return zero-ish values; shape() returns an error. Every
// call reports a consistent failure — callers should gate their use
// on cfg(target_vendor = "apple") or prefer the device-independent
// (font-parser) stack on non-Apple.

#[cfg(not(target_vendor = "apple"))]
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

#[cfg(not(target_vendor = "apple"))]
impl text_interfaces::TextShaper for UnimplementedNativeBackend {
    type Handle = ();

    fn shape(
        &self,
        _text: &str,
        _font: &Self::Handle,
        _size: f32,
        _options: &text_interfaces::ShapeOptions,
    ) -> Result<text_interfaces::ShapedRun, text_interfaces::ShapingError> {
        Err(text_interfaces::ShapingError::ShapingFailed(
            "text-native has no backend implemented for this target OS; \
             use the device-independent path or a TXT03 backend."
                .into(),
        ))
    }

    fn font_ref(&self, _font: &Self::Handle) -> String {
        String::from("unimplemented:")
    }
}

// ---------------------------------------------------------------------------

pub const VERSION: &str = "0.1.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_constant_is_set() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[cfg(target_vendor = "apple")]
    #[test]
    fn native_types_are_the_coretext_types_on_apple() {
        // Compile-time check: the NativeResolver alias resolves to
        // CoreTextResolver.
        fn _takes_coretext(_r: text_native_coretext::CoreTextResolver) {}
        let r: NativeResolver = NativeResolver::new();
        _takes_coretext(r);
    }

    #[cfg(not(target_vendor = "apple"))]
    #[test]
    fn non_apple_resolver_returns_load_failed() {
        use text_interfaces::{FontQuery, FontResolutionError, FontResolver};
        let r = NativeResolver::new();
        let err = r.resolve(&FontQuery::named("Helvetica")).unwrap_err();
        assert!(matches!(err, FontResolutionError::LoadFailed(_)));
    }
}
