//! # semantic-ir-to-rust
//!
//! Second backend for the narrow-waist Semantic IR — emits **self-
//! contained** Rust source code from a [`semantic_ir::Module`].
//!
//! Self-contained means every produced `.rs` file embeds an `__sir`
//! runtime module — no external crate dependencies; the output can
//! be compiled with `rustc <file>.rs` and run.
//!
//! ## Public API
//!
//! ```ignore
//! use semantic_ir_to_rust::{compile, RustBackend};
//! use semantic_ir::Backend;
//!
//! let module = /* a semantic_ir::Module from any frontend */;
//!
//! let artifact = compile(&module)?;
//! // or:
//! let backend = RustBackend::new();
//! let artifact = backend.compile(&module)?;
//! ```
//!
//! See [SIR13](../../../specs/SIR13-semantic-ir-to-rust.md) for the
//! per-node lowering rules.

mod emit;
mod runtime;

use semantic_ir::{
    Artifact, ArtifactMetadata, Backend, BackendError, BackendErrorKind, Feature, Module,
};

pub use emit::sanitize_ident;

/// Convenience entry point.
pub fn compile(module: &Module) -> Result<Artifact, BackendError> {
    RustBackend::new().compile(module)
}

/// The v0 Rust backend.
pub struct RustBackend;

impl RustBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RustBackend {
    fn default() -> Self {
        Self::new()
    }
}

const ACCEPTED_FEATURES: &[Feature] = &[
    Feature::Closures,
    Feature::Pairs,
    Feature::Symbols,
    Feature::Strings,
    Feature::DynamicTyping,
    Feature::OptionalTypeAnnotations,
    Feature::MutualRecursion,
    Feature::Globals,
];

impl Backend for RustBackend {
    fn target_tag(&self) -> &'static str {
        "rust"
    }

    fn accepts_features(&self) -> &'static [Feature] {
        ACCEPTED_FEATURES
    }

    fn accepts_intrinsics(&self) -> &'static [&'static str] {
        &[]
    }

    fn compile(&self, module: &Module) -> Result<Artifact, BackendError> {
        // 1. Validate.
        let r = semantic_ir::validate(module);
        if !r.is_ok() {
            let first = r.errors().next().cloned();
            if let Some(e) = first {
                return Err(BackendError {
                    kind: BackendErrorKind::InvalidModule,
                    message: format!("module failed validation: {}", e.message),
                    span: e.span,
                });
            }
        }

        // 2. Capability checks.
        let cap_errors = self.check_module(module);
        if let Some(e) = cap_errors.into_iter().next() {
            return Err(e);
        }

        // 3. TailCalls cannot be satisfied.
        if module.manifest.contains(Feature::TailCalls) {
            return Err(BackendError {
                kind: BackendErrorKind::UnsupportedFeature,
                message: "rust backend cannot satisfy `tail_calls` feature".into(),
                span: module.span.clone(),
            });
        }

        // 4. Emit.
        let source = emit::emit_module_with_arity_table(module);
        let metadata = ArtifactMetadata {
            bytes: source.len(),
            line_count: source.lines().count(),
            ..Default::default()
        };

        Ok(Artifact {
            filename: format!("{}.rs", module.name.replace('/', "_")),
            source,
            metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use semantic_ir::{
        Block, EffectSet, Expr, FeatureManifest, Function, Metadata, Span,
    };

    fn s() -> Span {
        Span::synthetic()
    }

    fn minimal_module() -> Module {
        Module {
            name: "demo".into(),
            manifest: FeatureManifest::new(),
            imports: vec![],
            exports: vec![],
            functions: vec![Function {
                name: "main".into(),
                params: vec![],
                return_type: None,
                captures: vec![],
                body: Block {
                    stmts: vec![],
                    value: Expr::IntLit { value: 42, span: s() },
                    span: s(),
                },
                effects: EffectSet::PURE,
                metadata: Metadata::new(),
                span: s(),
            }],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("twig")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        }
    }

    #[test]
    fn compiles_minimal_module() {
        let m = minimal_module();
        let a = compile(&m).expect("compile");
        assert!(a.source.contains("fn __sir_user_main()"));
        assert!(a.source.contains("__sir::Value::Int(42i64)"));
        assert!(a.filename.ends_with(".rs"));
        assert!(a.metadata.bytes > 0);
    }

    #[test]
    fn target_tag_is_rust() {
        let b = RustBackend::new();
        assert_eq!(b.target_tag(), "rust");
    }

    #[test]
    fn rejects_tail_calls() {
        let mut m = minimal_module();
        m.manifest = FeatureManifest::from_features(&[Feature::TailCalls]);
        let err = compile(&m).expect_err("tail calls rejected");
        assert_eq!(err.kind, BackendErrorKind::UnsupportedFeature);
    }

    #[test]
    fn end_to_end_twig_to_rust_id_function() {
        let module = twig_to_semantic_ir::compile_source(
            "(define (id x) x)\n(id 42)",
            "demo",
        )
        .expect("lower");
        let a = compile(&module).expect("compile");
        assert!(a.source.contains("fn id(x: __sir::Value) -> __sir::Value"));
        assert!(a.source.contains("x.clone()"));
        assert!(a.source.contains("id(__sir::Value::Int(42i64))"));
    }

    #[test]
    fn end_to_end_twig_to_rust_arith() {
        let module = twig_to_semantic_ir::compile_source(
            "(define (add a b) (+ a b))\n(print (add 1 2))",
            "demo",
        )
        .expect("lower");
        let a = compile(&module).expect("compile");
        assert!(a.source.contains("fn add("));
        assert!(a.source.contains("__sir::plus(vec!["));
        assert!(a.source.contains("__sir::print(add("));
    }

    #[test]
    fn end_to_end_twig_to_rust_closure_program() {
        let module = twig_to_semantic_ir::compile_source(
            "(define (adder n) (lambda (x) (+ x n)))\n(define add5 (adder 5))\n(print (add5 3))",
            "demo",
        )
        .expect("lower");
        let a = compile(&module).expect("compile");
        assert!(a.source.contains("fn __lambda_0("));
        assert!(a.source.contains("__sir::Value::Closure(::std::rc::Rc::new"));
        assert!(a.source.contains("__sir::apply_closure"));
        assert!(a.source.contains("Globals (initialised in _init): add5"));
    }

    #[test]
    fn output_is_deterministic() {
        let module = twig_to_semantic_ir::compile_source(
            "(define (id x) x)\n(id 7)",
            "demo",
        )
        .expect("lower");
        let a = compile(&module).expect("compile");
        let b = compile(&module).expect("compile again");
        assert_eq!(a.source, b.source);
    }

    #[test]
    fn module_filename_sanitised() {
        let mut m = minimal_module();
        m.name = "compiler/lexer".into();
        let a = compile(&m).expect("compile");
        assert_eq!(a.filename, "compiler_lexer.rs");
    }
}
