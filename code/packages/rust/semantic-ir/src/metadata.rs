//! Module / function metadata.
//!
//! Metadata is **advisory** information attached to nodes — strip it
//! all and the program's meaning must remain identical.  Backends
//! may consult metadata for better diagnostics or output formatting,
//! but the IR's correctness must not depend on it.
//!
//! v0 fields:
//!
//! - `source_language` — frontend identifier (`"twig"`, `"python"`, …)
//! - `source_version`  — version string of the source language
//! - `sir_version`     — IR spec version (`"2"` in this build)

use std::collections::BTreeMap;
use std::fmt;

/// Pinned to the SIR major version this crate implements.
///
/// - `"0"` — the original flat type set (through KW1).
/// - `"1"` — SIR21: the `SirType` type lattice grew (`Int{IntSpec}`,
///   `Ptr`/`Struct`/`Optional`) and the feature manifest gained the
///   sized-integer / pointer / struct / bignum flags (T1a + T1b).  The
///   bump lands in T1b, the first milestone whose surface introduces
///   *new text tokens* a reader would need to understand.
/// - `"2"` — SIR22: the array/matrix IR extension.  New `SirType`
///   variants (`NDArray`/`Rational`/`Complex`), new `Expr` variants
///   (`ArrayLit`/`Range`/`MatMul`/`ElementwiseOp`/`Transpose`/
///   `IndexGet`), a new `Stmt` variant (`IndexSet`), and five new
///   `Feature` flags — all additive, all introducing new SIR text
///   tokens, so the same "adding a feature is a v.bump" policy that
///   moved `"0"` → `"1"` applies again here.  A frontend lowering
///   MATLAB/Octave sets `metadata.sir_version` to this value when its
///   module uses any SIR22 node.
/// - `"3"` — SIR23: the symbolic expression + pattern/rewrite IR
///   extension.  One new `SirType` variant (`SymExpr`), seven new
///   `Expr` variants (`SymSymbol`/`SymRational`/`SymApply`/
///   `SymPatternBlank`/`SymPatternNamed`/`SymRule`/`SymReplaceAll`), no
///   new `Stmt` variant (every SIR23 node is `Pure`), and two new
///   `Feature` flags (`SymbolicExpr`/`PatternMatching`; `Rationals`/
///   `Complex` are reused from SIR22) — again additive, again new SIR
///   text tokens, so the same policy bumps `"2"` → `"3"`.  A frontend
///   lowering a Wolfram/Macsyma/Maxima-family CAS language sets
///   `metadata.sir_version` to this value when its module uses any
///   SIR23 node.
pub const CURRENT_SIR_VERSION: &str = "3";

/// Advisory metadata.  All fields are optional.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Metadata {
    pub source_language: Option<String>,
    pub source_version: Option<String>,
    pub sir_version: Option<String>,
    /// Catch-all bag of named string properties.  Kept ordered so
    /// the text representation is deterministic.
    pub extra: BTreeMap<String, String>,
}

impl Metadata {
    /// Empty metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Convenience builder.
    pub fn with_source_language(mut self, name: impl Into<String>) -> Self {
        self.source_language = Some(name.into());
        self
    }

    pub fn with_source_version(mut self, version: impl Into<String>) -> Self {
        self.source_version = Some(version.into());
        self
    }

    pub fn with_sir_version(mut self, version: impl Into<String>) -> Self {
        self.sir_version = Some(version.into());
        self
    }

    pub fn with_extra(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }

    /// `true` iff every field is empty.  Useful to elide the
    /// metadata block in the text format.
    pub fn is_empty(&self) -> bool {
        self.source_language.is_none()
            && self.source_version.is_none()
            && self.sir_version.is_none()
            && self.extra.is_empty()
    }
}

impl fmt::Display for Metadata {
    /// Text-format rendering: a sequence of `(key value)` pairs
    /// inside a `(metadata ...)` wrapper.  Only present fields are
    /// emitted.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(metadata")?;
        if let Some(v) = &self.source_language {
            write!(f, " (source-language {})", v)?;
        }
        if let Some(v) = &self.source_version {
            write!(f, " (source-version {})", v)?;
        }
        if let Some(v) = &self.sir_version {
            write!(f, " (sir-version {})", v)?;
        }
        for (k, v) in &self.extra {
            write!(f, " ({} {})", k, v)?;
        }
        write!(f, ")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_metadata_renders_empty_form() {
        let m = Metadata::new();
        assert!(m.is_empty());
        assert_eq!(format!("{}", m), "(metadata)");
    }

    #[test]
    fn builder_sets_fields() {
        let m = Metadata::new()
            .with_source_language("twig")
            .with_source_version("0.7")
            .with_sir_version(CURRENT_SIR_VERSION);
        assert_eq!(m.source_language.as_deref(), Some("twig"));
        assert_eq!(m.source_version.as_deref(), Some("0.7"));
        assert_eq!(m.sir_version.as_deref(), Some("3"));
        assert!(!m.is_empty());
    }

    #[test]
    fn render_with_fields() {
        let m = Metadata::new()
            .with_source_language("twig")
            .with_source_version("0.7");
        assert_eq!(
            format!("{}", m),
            "(metadata (source-language twig) (source-version 0.7))"
        );
    }

    #[test]
    fn extra_fields_round_trip() {
        let m = Metadata::new()
            .with_extra("origin", "test")
            .with_extra("foo", "bar");
        // BTreeMap orders by key alphabetically.
        assert_eq!(format!("{}", m), "(metadata (foo bar) (origin test))");
    }

    #[test]
    fn current_sir_version_is_three() {
        // Bumped 2 → 3 in SIR23 (symbolic expression + pattern/rewrite IR
        // extension: new SirType/Expr/Feature text tokens), following the
        // same "adding a feature is a v.bump" policy that moved 1 → 2 in
        // SIR22.
        assert_eq!(CURRENT_SIR_VERSION, "3");
    }
}
