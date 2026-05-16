//! # semantic-ir — narrow-waist Semantic IR
//!
//! Implementation of [SIR10](../../../specs/SIR10-narrow-waist-semantic-ir.md).
//!
//! The Semantic IR is a neutral intermediate representation that
//! sits between language frontends and code-emitting backends.
//! N frontends × M backends communicate through this single waist —
//! so the total work scales as **N + M** rather than N × M.
//!
//! The IR is **opinionated about its own semantics** but **has no
//! opinion** about how those semantics are realised in any source
//! or target language.  Every semantic concept is a distinct node
//! kind so backends never have to ask "what did the programmer
//! mean?".
//!
//! ## Where to start
//!
//! - [`Module`] — top-level compilation unit.
//! - [`Function`] — a callable with params, optional return type,
//!   optional captures (for closures), and a body block.
//! - [`Expr`] — the expression grammar with one variant per
//!   distinct semantic concept.
//! - [`Backend`] — the trait every backend implements.
//! - [`validate`] — the v0 validator.
//! - [`text::print_module`] — render a `Module` to its canonical
//!   S-expression form.
//!
//! ## Pipeline
//!
//! ```text
//! source code
//!    │
//!    ▼  language-specific frontend (e.g. twig-to-semantic-ir)
//! semantic_ir::Module
//!    │
//!    ▼  validator (semantic_ir::validate)
//! validated Module
//!    │
//!    ▼  language-specific backend (e.g. semantic-ir-to-typescript)
//! Artifact { source, ... }
//! ```

pub mod backend;
pub mod effects;
pub mod limits;
pub mod manifest;
pub mod metadata;
pub mod nodes;
pub mod span;
pub mod text;
pub mod types;
pub mod validator;
pub mod walker;

// ---------------------------------------------------------------------------
// Re-exports — flatten the public API so callers do
// `use semantic_ir::{Module, Function, validate}` rather than digging.
// ---------------------------------------------------------------------------

pub use backend::{
    Artifact, ArtifactMetadata, Backend, BackendError, BackendErrorKind, BackendRegistry,
};
pub use effects::{Effect, EffectSet};
pub use limits::MAX_IR_DEPTH;
pub use manifest::{Feature, FeatureManifest};
pub use metadata::{Metadata, CURRENT_SIR_VERSION};
pub use nodes::{
    Block, Capture, CaptureValue, ExportName, Expr, Function, Global, Import, ImportName, Module,
    Param, Scope, Stmt,
};
pub use span::Span;
pub use text::{print_block, print_expr, print_function, print_module};
pub use types::SirType;
pub use validator::{validate, Severity, ValidationResult, ValidatorIssue};
pub use walker::{walk_expr_default, walk_function_default, walk_module_default, walk_stmt_default, Visitor};

#[cfg(test)]
mod smoke {
    use super::*;

    #[test]
    fn build_minimal_module_and_print() {
        // The simplest possible module: empty.
        let m = Module {
            name: "empty".into(),
            manifest: FeatureManifest::new(),
            imports: vec![],
            exports: vec![],
            functions: vec![],
            globals: vec![],
            metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
            span: Span::synthetic(),
        };
        let r = validate(&m);
        assert!(r.is_ok());
        let t = print_module(&m);
        assert!(t.contains("(sir-module empty v0"));
    }

    #[test]
    fn build_one_function_module() {
        // (define (id x) x) — identity function.
        let body = Block {
            stmts: vec![],
            value: Expr::VarRef {
                name: "x".into(),
                scope: Scope::Param,
                span: Span::synthetic(),
            },
            span: Span::synthetic(),
        };
        let f = Function {
            name: "id".into(),
            params: vec![Param {
                name: "x".into(),
                sir_type: None,
                span: Span::synthetic(),
            }],
            return_type: None,
            captures: vec![],
            body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: Span::synthetic(),
        };
        let m = Module {
            name: "ident".into(),
            manifest: FeatureManifest::from_features(&[Feature::DynamicTyping]),
            imports: vec![],
            exports: vec![],
            functions: vec![f],
            globals: vec![],
            metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
            span: Span::synthetic(),
        };
        let r = validate(&m);
        assert!(r.is_ok(), "{:?}", r.issues);
        let t = print_module(&m);
        assert!(t.contains("(function id ((x any)) any"));
    }
}
