//! Formatting via spec.format_fn.
//!
//! ## Implementation plan (LS02 PR A)
//!
//! If spec.format_fn is None → return None (feature not supported).
//! If spec.format_fn is Some(f):
//!   1. Call f(source).
//!   2. On Ok(formatted): compute text edits (replace whole document).
//!      ls00 likely expects Vec<ls00::TextEdit> or a full-document replacement.
//!      Verify the exact return type from ls00::LanguageBridge::format().
//!   3. On Err(msg): return Some(Err(msg)) so ls00 can surface it as an LSP error.

use crate::LanguageSpec;

/// Format `source` using `spec.format_fn`.
///
/// Returns `None` if formatting is not supported by this language.
/// Returns `Some(Err(msg))` if formatting failed.
/// Returns `Some(Ok(formatted))` on success.
///
/// ## TODO — implement (LS02 PR A)
pub fn format(spec: &'static LanguageSpec, source: &str) -> Option<Result<String, String>> {
    spec.format_fn.map(|f| f(source))
}
