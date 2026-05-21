//! Backend-agnostic JavaScript AST.
//!
//! # What this crate is for
//!
//! `javascript-ast` defines the AST that the JavaScript frontend emits and
//! that *every* downstream consumer reads. Per
//! [CLOC02](../../specs/CLOC02-javascript-ast.md), this includes:
//!
//! - The Closure-Compiler-clone's typechecker, optimization passes, emitter.
//! - The future V8-in-Rust clone's bytecode lowering pass.
//! - JSDoc and TypeScript types extractors (they walk the AST looking for
//!   comment anchors).
//! - IDE tooling — LSP, hover, completion, debug adapters.
//!
//! Because so many consumers depend on it, the AST is intentionally **small**.
//! See the invariants below.
//!
//! # Backend-agnostic invariants (CLOC02 §"Backend-agnostic invariants")
//!
//! 1. **No backend types in AST nodes.** The crate imports only from
//!    `correlation-vector` and `javascript-tokens`. Never from `closure-*`,
//!    `type-sidecar`, IR/bytecode crates.
//! 2. **Every node carries a CV ID, not a span.** Spans live in CV `Origin`
//!    records in a parallel log. The AST stores only the lightweight `CvId`.
//! 3. **No mutation in the public surface.** Passes return new trees; they
//!    do not mutate inputs.
//! 4. **Version tag on `Program`, not on every node.** Per-node version tags
//!    would bloat every variant; the parser refuses to emit nodes that aren't
//!    legal at the requested version, so downstream readers can assume
//!    "every variant present is legal."
//! 5. **No type information in the AST.** Types live in
//!    `coding-adventures-type-sidecar`, keyed by `CvId`.
//! 6. **No optimization metadata in the AST.** "This is dead," "this was
//!    inlined" — all of that lives in CV contributions.
//!
//! # What v1 contains
//!
//! This is the scaffolding PR: just the [`Program`] root node and the
//! [`SourceType`] enum it carries. The big variant trees (`Statement`,
//! `Expression`, declarations, patterns, class members, module syntax,
//! literals) ship in their own follow-up PRs to keep diffs small.
//!
//! # Note on `CvId`
//!
//! CLOC02 specifies `cv: CvId` where `CvId` is a "copy-cheap newtype". The
//! current `correlation-vector` crate represents CV IDs as plain `String`
//! (see `correlation-vector` v0.x). To avoid coupling the two crates'
//! release cycles, this crate type-aliases [`CvId`] to `String` for v1. A
//! future PR can promote it to a real newtype without changing the public
//! surface much — the existing fields would just take `CvId` directly.
//!
//! [`CvId`]: type@CvId

use coding_adventures_javascript_tokens::EsVersion;

/// A correlation-vector identifier. Aliased to `String` for v1 to match the
/// current `correlation-vector` crate's representation. See the module-level
/// docs note on `CvId` for the migration plan.
pub type CvId = String;

/// The top of the AST. Every JavaScript compile produces exactly one
/// `Program`. Holds the ES version that produced the tree, the
/// script/module discriminator, and (in follow-up PRs) the body.
///
/// Per CLOC02 the `version` field is recorded *only here* — individual
/// nodes never carry a version. Downstream readers can assume any variant
/// they see is legal at `self.version`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// CV identifier for this program. Populated by the parser via
    /// `cv.merge(token_ids, Origin{source: filename, ...})`.
    pub cv: CvId,

    /// The ECMAScript edition the parser used.
    pub version: EsVersion,

    /// Whether this source was parsed as a script or a module. The parser
    /// picks based on caller hint, file extension, or shebang. Closure
    /// passes and the V8 clone both need this — module and script have
    /// different top-level scoping rules.
    pub source_type: SourceType,
}

/// Whether a [`Program`] was parsed as a script or a module.
///
/// The top-level scoping rules differ between the two: scripts have a
/// shared global scope and allow non-strict-mode features by default;
/// modules are always strict and have per-file scopes plus
/// `import`/`export` syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceType {
    Script,
    Module,
}

impl Program {
    /// Construct a `Program` with the given CV id, version, and source type.
    /// Helper used by the parser; tests and synthetic-AST authors can use
    /// it too.
    pub fn new(cv: CvId, version: EsVersion, source_type: SourceType) -> Self {
        Self {
            cv,
            version,
            source_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn program_constructs_with_synthetic_cv() {
        // The parser will populate `cv` via correlation-vector's
        // `cv.merge(..)` call. For unit testing we just use a synthetic
        // string — the AST crate doesn't care what's inside.
        let program = Program::new(
            "synthetic.1".to_string(),
            EsVersion::Es2025,
            SourceType::Module,
        );

        assert_eq!(program.cv, "synthetic.1");
        assert_eq!(program.version, EsVersion::Es2025);
        assert_eq!(program.source_type, SourceType::Module);
    }

    #[test]
    fn program_supports_both_source_types() {
        let script = Program::new(
            "s.1".to_string(),
            EsVersion::Es5,
            SourceType::Script,
        );
        let module = Program::new(
            "m.1".to_string(),
            EsVersion::Es2025,
            SourceType::Module,
        );

        assert_eq!(script.source_type, SourceType::Script);
        assert_eq!(module.source_type, SourceType::Module);
        assert_ne!(script.source_type, module.source_type);
    }

    #[test]
    fn program_clone_eq() {
        let p1 = Program::new(
            "c.1".to_string(),
            EsVersion::Es2020,
            SourceType::Script,
        );
        let p2 = p1.clone();
        assert_eq!(p1, p2);
    }

    #[test]
    fn source_type_is_copy() {
        // SourceType derives Copy so it's cheap to pass around. Compile-time
        // check via the `Copy` bound.
        fn assert_copy<T: Copy>() {}
        assert_copy::<SourceType>();
    }

    #[test]
    fn version_is_copy() {
        // EsVersion is also Copy, so it can live on every node without
        // worrying about clones.
        fn assert_copy<T: Copy>() {}
        assert_copy::<EsVersion>();
    }
}
