//! JavaScript code emitter for the Closure Compiler clone.
//!
//! Per [CLOC07](../../../specs/CLOC07-emit-and-source-map.md). The
//! back end of the compiler — takes a finalized [`Program`] + the
//! type sidecar and produces output JavaScript text plus a
//! companion source-map v3 blob.
//!
//! # Where this sits
//!
//! ```text
//! lexer → parser → AST ──┐
//!                        ├─► passes ──► Program' ──► emitter ──► .js
//!                sidecar ─┘                                       .js.map
//! ```
//!
//! The emitter runs **after** every optimization pass. It depends
//! on `javascript-ast` (the data it reads) and the `type-sidecar`
//! (for emit hints like "stringify this number as `0xFF` not
//! `255`" once that lands), but **not** on any pass crate or on
//! `closure-pass-pipeline` — the emitter doesn't know or care
//! what passes ran before it, only that the program it receives
//! is the final shape.
//!
//! # The two outputs
//!
//! 1. **`code: String`** — printable JavaScript bytes. ASCII-safe
//!    when `EmitOptions::ascii_only`, otherwise UTF-8.
//! 2. **`source_map: Option<String>`** — a serialized source-map
//!    v3 blob mapping output positions back to input
//!    correlation-vector ids. Suppressed (`None`) when
//!    `EmitOptions::source_map = false`.
//!
//! # Why a correlation-vector instead of input file offsets?
//!
//! The AST nodes don't carry source positions — they carry
//! [`CvId`] values per CLOC02. The CV graph maps those ids back
//! to original source bytes. That indirection is what lets
//! every optimization pass between parse and emit rewrite the
//! tree without invalidating source maps — each emitted token
//! cites a CV id, the CV graph still knows what bytes that id
//! traces to.
//!
//! Real source-map generation lands in
//! `coding-adventures-closure-source-map` (CLOC07 Phase 2). This
//! crate just emits a placeholder for now.
//!
//! # Scope (v1)
//!
//! `javascript-ast` ships only `Program` / `SourceType` today
//! (CLOC02 Phase 1) — no `Statement` / `Expression` / `Declaration`
//! variants. With nothing to emit, `code` comes back as an empty
//! string. The function signature, error type, options struct
//! and output struct are the deliverable: they're what the rest
//! of the toolchain links against.
//!
//! What this PR locks down:
//!
//! 1. The `emit()` function signature — the canonical entry
//!    point. Once the AST grows variants, the body fills in but
//!    the call sites don't change.
//! 2. [`EmitOptions`] — `ascii_only`, `pretty`, `source_map`.
//!    These are the three knobs CLOC07 names; pinning them now
//!    means the CLI (CLOC08) can wire them up before the body
//!    actually does anything.
//! 3. [`EmitOutput`] — `code`, `source_map`, `contributions`.
//!    The contributions vector is how emit reports per-token
//!    "emitted" entries to the CV log per CLOC03; vacuous in v1
//!    but the wire is in.
//! 4. [`EmitError`] — error variants so callers can `?` against
//!    a real type, not `Box<dyn Error>`.

use coding_adventures_correlation_vector::{CVLog, Contribution};
use coding_adventures_javascript_ast::Program;
use coding_adventures_type_sidecar::Sidecar;
use std::fmt;

/// Knobs for [`emit`].
///
/// All three are independent. `pretty = true` and
/// `ascii_only = true` are compatible; the emitter just inserts
/// whitespace AND escapes non-ASCII at the same time.
///
/// # Defaults
///
/// - `ascii_only = false` — output is UTF-8 unless explicitly
///   restricted. ASCII-only is needed for environments that
///   can't reliably round-trip UTF-8 (older toolchains,
///   non-UTF-8 transports).
/// - `pretty = false` — production default is compact. Switch
///   on for debugging or human-reviewed output.
/// - `source_map = true` — production default is to emit a
///   companion `.js.map`. Most build systems consume it; turn
///   off only when you specifically don't want one (e.g.,
///   library distribution where shipping the map exposes
///   internals).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitOptions {
    /// When `true`, escape every non-ASCII codepoint as `\uXXXX`
    /// (or `\u{XXXXXX}` for codepoints above U+FFFF). Output is
    /// then byte-safe in any encoding that's a superset of
    /// ASCII (latin-1, windows-1252, etc.).
    pub ascii_only: bool,

    /// When `true`, insert whitespace + newlines for human
    /// readability. When `false`, emit the tightest legal
    /// JavaScript (no comments, single-space separators only
    /// where required by ASI).
    pub pretty: bool,

    /// When `true`, also produce a source-map v3 blob in
    /// [`EmitOutput::source_map`]. When `false`,
    /// [`EmitOutput::source_map`] is `None` — saves work and
    /// output size in pipelines that don't need maps.
    pub source_map: bool,
}

impl Default for EmitOptions {
    fn default() -> Self {
        Self {
            ascii_only: false,
            pretty: false,
            source_map: true,
        }
    }
}

/// Result of [`emit`].
///
/// `contributions` is a parallel out-band channel for callers
/// that want to log per-token "emitted" CV `Contribution`s
/// somewhere besides `cv` directly (e.g., to a structured trace
/// store). In v1 it's always empty; once the emitter walks real
/// AST nodes, each emitted token will contribute one entry.
///
/// `Eq` is not implemented because `Contribution.meta` holds
/// `serde_json::Value`, which is `PartialEq` but not `Eq`
/// (floats etc.). Tests compare individual fields instead.
#[derive(Debug, Clone)]
pub struct EmitOutput {
    /// The output JavaScript bytes as a `String` (UTF-8 or
    /// ASCII-restricted depending on `ascii_only`).
    pub code: String,

    /// A serialized source-map v3 blob, or `None` if
    /// `EmitOptions::source_map = false`. v1 places an empty
    /// placeholder when enabled; real source-map generation
    /// lands in `closure-source-map` (CLOC07 Phase 2).
    pub source_map: Option<String>,

    /// Per-token "emitted" contributions per CLOC03. Empty in
    /// v1 because no tokens are emitted.
    pub contributions: Vec<Contribution>,
}

/// Errors `emit` can return.
///
/// Variants are concrete, not `Box<dyn Error>` — callers can
/// pattern-match. New variants land additively (the enum is
/// `#[non_exhaustive]` to keep that promise without forcing
/// users to update match arms).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum EmitError {
    /// An AST node referenced a CV id that the CV log doesn't
    /// know about. Indicates a bug somewhere upstream — either
    /// the parser fabricated an id, or a pass dropped a
    /// `cv.create()` call. v1 doesn't trigger this (no
    /// `emit()` calls walk nodes), but the variant is here so
    /// upstream code can `?` against it without changing the
    /// signature later.
    UnknownCvId {
        /// The id that wasn't found in the log.
        id: String,
        /// Where in the emitter the lookup happened.
        site: &'static str,
    },

    /// The sidecar had a type annotation the emitter doesn't
    /// know how to render. Future-proofing — v1 doesn't render
    /// anything so this is unreachable today.
    UnsupportedSidecarType {
        /// The CV id whose sidecar record we couldn't render.
        id: String,
        /// What kind of type it was (a tag name from the
        /// sidecar's `Type` enum).
        kind: String,
    },
}

impl fmt::Display for EmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmitError::UnknownCvId { id, site } => {
                write!(
                    f,
                    "emit: AST referenced unknown CV id {:?} at {}",
                    id, site
                )
            }
            EmitError::UnsupportedSidecarType { id, kind } => {
                write!(
                    f,
                    "emit: don't know how to render sidecar type {:?} on id {:?}",
                    kind, id
                )
            }
        }
    }
}

impl std::error::Error for EmitError {}

/// Emit JavaScript text + optional source map for `program`.
///
/// `_sidecar` and `_cv` are reserved for v2+: once the AST has
/// nodes, the emitter consults the sidecar for per-node hints
/// (numeric base, string quote style, etc.) and writes
/// per-token contributions into `_cv` per CLOC03. v1 just
/// honors the options struct and returns an empty result.
///
/// # When this is safe to call without running passes first
///
/// Always. The emitter doesn't require any passes to have run;
/// it just renders whatever `Program` it receives. The pipeline
/// running first is a *quality* concern (you'd get unoptimized
/// output), not a *correctness* concern.
pub fn emit(
    program: &Program,
    _sidecar: &Sidecar,
    _cv: &mut CVLog,
    opts: &EmitOptions,
) -> Result<EmitOutput, EmitError> {
    // v1: no Statement / Expression / Declaration variants in
    // the AST yet, so there are no tokens to render. The output
    // code is empty — but `EmitOutput` is fully populated so
    // call sites can pattern-match against the shape today.
    //
    // The argument we *do* honor in v1 is `opts.source_map`,
    // because turning the source-map blob on or off is observable
    // even when the code is empty. ASCII-only and pretty are
    // no-ops over empty output, but their values still
    // influence `EmitOutput`'s representation once the body
    // grows.
    let _ = program; // suppress unused-variable warning without changing the signature.

    let source_map = if opts.source_map {
        // Placeholder. Real source-map generation lives in
        // `coding-adventures-closure-source-map` (CLOC07 Phase 2).
        // The empty string keeps the field type stable
        // (`Option<String>`) so callers don't need a v2 update.
        Some(String::new())
    } else {
        None
    };

    Ok(EmitOutput {
        code: String::new(),
        source_map,
        contributions: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    //! Tests pin the public surface (`emit` signature,
    //! `EmitOptions` defaults, `EmitOutput` shape, `EmitError`
    //! variants). The body is identity-shaped in v1; what
    //! matters is that downstream crates can link against the
    //! API and that the API stays stable as the body grows.
    use super::*;
    use coding_adventures_correlation_vector::CVLog;
    use coding_adventures_javascript_ast::{Program, SourceType};
    use coding_adventures_javascript_tokens::EsVersion;
    use coding_adventures_type_sidecar::Sidecar;

    fn program() -> Program {
        Program::new("prog.1".to_string(), EsVersion::Es2025, SourceType::Module)
    }

    #[test]
    fn emit_options_default_is_production_safe() {
        // Production defaults: UTF-8 (not ASCII-only), minified
        // (not pretty), source map on. Pinning these so a
        // careless future change shows up here.
        let o = EmitOptions::default();
        assert!(!o.ascii_only);
        assert!(!o.pretty);
        assert!(o.source_map);
    }

    #[test]
    fn emit_on_empty_program_returns_empty_code_with_source_map_by_default() {
        let prog = program();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        let out = emit(&prog, &sidecar, &mut cv, &EmitOptions::default())
            .expect("emit should succeed");

        assert_eq!(out.code, "");
        // Source map field is present (default opts have
        // source_map = true), but empty in v1.
        assert_eq!(out.source_map.as_deref(), Some(""));
        // No tokens emitted → no per-token contributions.
        assert!(out.contributions.is_empty());
    }

    #[test]
    fn emit_omits_source_map_when_option_disabled() {
        let prog = program();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        let opts = EmitOptions {
            source_map: false,
            ..Default::default()
        };
        let out = emit(&prog, &sidecar, &mut cv, &opts).expect("emit should succeed");

        assert_eq!(out.code, "");
        // Disabling source_map drops the field entirely.
        assert!(out.source_map.is_none());
    }

    #[test]
    fn emit_honors_ascii_only_flag() {
        // v1 produces empty output regardless, but the flag
        // must be accepted (it's part of the public API).
        // This test catches accidental signature changes.
        let prog = program();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        let opts = EmitOptions {
            ascii_only: true,
            ..Default::default()
        };
        let out = emit(&prog, &sidecar, &mut cv, &opts).expect("emit should succeed");

        // ASCII-only string trivially passes the is_ascii check
        // when empty.
        assert!(out.code.is_ascii());
    }

    #[test]
    fn emit_accepts_pretty_flag() {
        // Same shape as ascii_only — v1 output is empty so
        // pretty is a no-op, but the flag must round-trip.
        let prog = program();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        let opts = EmitOptions {
            pretty: true,
            ..Default::default()
        };
        let out = emit(&prog, &sidecar, &mut cv, &opts).expect("emit should succeed");
        assert_eq!(out.code, "");
    }

    #[test]
    fn emit_options_is_clone_and_eq() {
        // ZST-like value types: cloning and comparing options
        // is part of the ergonomics — CLI code will diff two
        // configs etc. (EmitOutput doesn't impl Eq because
        // Contribution.meta holds serde_json::Value, but
        // EmitOptions does.)
        let a = EmitOptions::default();
        let b = a.clone();
        assert_eq!(a, b);
        let c = EmitOptions {
            ascii_only: true,
            ..Default::default()
        };
        assert_ne!(a, c);
    }

    #[test]
    fn emit_error_display_unknown_cv_id() {
        // Lock the Display format for stable log lines. The
        // exact text isn't a contract, but the test fails loudly
        // if someone reformats and breaks log-grepping setups.
        let e = EmitError::UnknownCvId {
            id: "node.42".to_string(),
            site: "expression-walker",
        };
        let s = format!("{}", e);
        assert!(s.contains("node.42"));
        assert!(s.contains("expression-walker"));
    }

    #[test]
    fn emit_error_display_unsupported_sidecar_type() {
        let e = EmitError::UnsupportedSidecarType {
            id: "node.7".to_string(),
            kind: "FutureKind".to_string(),
        };
        let s = format!("{}", e);
        assert!(s.contains("node.7"));
        assert!(s.contains("FutureKind"));
    }

    #[test]
    fn emit_error_is_std_error() {
        // Ensure EmitError implements std::error::Error so it
        // can be `?`'d through anyhow / boxed error pipelines.
        fn assert_error<E: std::error::Error>(_: &E) {}
        let e = EmitError::UnknownCvId {
            id: "x".to_string(),
            site: "test",
        };
        assert_error(&e);
    }
}
