//! Error types for all packager operations.
//!
//! Every function in this crate that can fail returns `Result<_, PackagerError>`.
//! The variants describe *what* went wrong at a high level; the embedded `String`
//! carries the human-readable detail.
//!
//! ## Design note
//!
//! We derive `Clone` and `PartialEq` so that callers can store or compare errors
//! in tests without needing a reference. `Display` is implemented manually to
//! produce tidy messages for the end user.

/// The unified error type for the `code-packager` crate.
///
/// ```text
/// Error hierarchy
///
///   PackagerError
///   ├── UnsupportedTarget — the requested binary format has no packager
///   └── WasmEncodeError   — the WASM module encoder returned an error
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackagerError {
    /// The target is not supported by the chosen packager.
    ///
    /// Example: asking the ELF64 packager to pack a `windows_x64` artifact.
    /// The embedded string names what was unsupported, e.g.
    /// `"no packager for binary_format=\"pe\""`.
    UnsupportedTarget(String),

    /// WASM module encoding failed inside `wasm-module-encoder`.
    ///
    /// The embedded string is the `Display` output of the underlying
    /// `WasmEncodeError`.
    WasmEncodeError(String),
}

impl std::fmt::Display for PackagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Forward the detail message verbatim so callers can log it cleanly.
            PackagerError::UnsupportedTarget(msg) => {
                write!(f, "unsupported target: {msg}")
            }
            PackagerError::WasmEncodeError(msg) => {
                write!(f, "WASM encode error: {msg}")
            }
        }
    }
}

// Implement `std::error::Error` with no `source()` override; the detail is
// already in the `Display` string.
impl std::error::Error for PackagerError {}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: UnsupportedTarget Display
    #[test]
    fn display_unsupported_target() {
        let err = PackagerError::UnsupportedTarget("no packager for binary_format=\"xyz\"".into());
        let text = err.to_string();
        assert!(
            text.contains("unsupported target"),
            "expected 'unsupported target' in {text:?}"
        );
        assert!(text.contains("xyz"), "expected detail in {text:?}");
    }

    // Test 2: WasmEncodeError Display
    #[test]
    fn display_wasm_encode_error() {
        let err = PackagerError::WasmEncodeError("invalid function body".into());
        let text = err.to_string();
        assert!(
            text.contains("WASM encode error"),
            "expected 'WASM encode error' in {text:?}"
        );
        assert!(text.contains("invalid function body"), "expected detail in {text:?}");
    }

    // Test 3: PartialEq
    #[test]
    fn equality() {
        let a = PackagerError::UnsupportedTarget("foo".into());
        let b = PackagerError::UnsupportedTarget("foo".into());
        let c = PackagerError::UnsupportedTarget("bar".into());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // Test 4: Clone
    #[test]
    fn clone_preserves_message() {
        let orig = PackagerError::WasmEncodeError("some error".into());
        let cloned = orig.clone();
        assert_eq!(orig, cloned);
    }
}
