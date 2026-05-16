//! # semantic-ir-to-typescript
//!
//! First backend for the narrow-waist Semantic IR — emits **self-
//! contained** TypeScript source code from a [`semantic_ir::Module`].
//!
//! Self-contained means every produced `.ts` file includes the
//! runtime helpers inline (no external `@coding-adventures/*` import
//! required).  Copy-paste deployable.
//!
//! ## Public API
//!
//! ```ignore
//! use semantic_ir_to_typescript::{compile, TypeScriptBackend};
//! use semantic_ir::Backend;
//!
//! let module = /* a semantic_ir::Module from any frontend */;
//!
//! // Direct entry point:
//! let artifact = compile(&module)?;
//!
//! // Or via the Backend trait:
//! let backend = TypeScriptBackend::new();
//! let artifact = backend.compile(&module)?;
//! ```
//!
//! Both paths return [`semantic_ir::Artifact`] with `filename`,
//! `source` (the generated `.ts`), and diagnostic metadata.
//!
//! ## Capability declaration
//!
//! `accepts_features` covers everything the Twig frontend emits in
//! v0.  `accepts_intrinsics` is empty — the v0 TS backend rejects
//! any intrinsic.  Future revisions may add `typescript`-tagged
//! intrinsics for embedding raw TS strings.
//!
//! See [SIR12](../../../specs/SIR12-semantic-ir-to-typescript.md) for
//! the per-node lowering rules.

mod emit;
mod runtime;

use semantic_ir::{
    Artifact, ArtifactMetadata, Backend, BackendError, BackendErrorKind, Feature, Module,
};

pub use emit::sanitize_ident;

/// Convenience entry point: validates the module, runs the
/// capability checks, and lowers to TypeScript source.
pub fn compile(module: &Module) -> Result<Artifact, BackendError> {
    TypeScriptBackend::new().compile(module)
}

/// The v0 TypeScript backend.
pub struct TypeScriptBackend;

impl TypeScriptBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TypeScriptBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Features the TypeScript backend accepts.  Twig's full v0 surface
/// minus `TailCalls` (JS does not guarantee TCO) and `Intrinsics`
/// (no intrinsics whitelisted yet).
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

impl Backend for TypeScriptBackend {
    fn target_tag(&self) -> &'static str {
        "typescript"
    }

    fn accepts_features(&self) -> &'static [Feature] {
        ACCEPTED_FEATURES
    }

    fn accepts_intrinsics(&self) -> &'static [&'static str] {
        &[]
    }

    fn compile(&self, module: &Module) -> Result<Artifact, BackendError> {
        // 1. Validate at the SIR boundary.  Lowering assumes the
        //    module is structurally well-formed; failing fast here
        //    saves cycles and gives the caller clean errors.
        let r = semantic_ir::validate(module);
        if !r.is_ok() {
            // Promote the first error to a BackendError; downstream
            // tooling can re-run the validator for the full list.
            let first = r.errors().next().cloned();
            if let Some(e) = first {
                return Err(BackendError {
                    kind: BackendErrorKind::InvalidModule,
                    message: format!("module failed validation: {}", e.message),
                    span: e.span,
                });
            }
        }

        // 2. Capability check: features + intrinsics.
        let cap_errors = self.check_module(module);
        if let Some(e) = cap_errors.into_iter().next() {
            return Err(e);
        }

        // 3. Tail-calls feature is fundamentally unsupported.
        if module.manifest.contains(Feature::TailCalls) {
            return Err(BackendError {
                kind: BackendErrorKind::UnsupportedFeature,
                message: "typescript backend cannot satisfy `tail_calls` feature".into(),
                span: module.span.clone(),
            });
        }

        // 4. Lower.
        let source = emit::emit_module(module);
        let metadata = ArtifactMetadata {
            bytes: source.len(),
            line_count: source.lines().count(),
            ..Default::default()
        };

        Ok(Artifact {
            filename: format!("{}.ts", module.name.replace('/', "_")),
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

    /// Build a minimal module that just returns 42 from main.
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
        assert!(a.source.contains("function main()"));
        assert!(a.source.contains("return 42;"));
        assert!(a.filename.ends_with(".ts"));
        assert!(a.metadata.bytes > 0);
        assert!(a.metadata.line_count > 0);
    }

    #[test]
    fn target_tag_is_typescript() {
        let b = TypeScriptBackend::new();
        assert_eq!(b.target_tag(), "typescript");
    }

    #[test]
    fn rejects_tail_calls_feature() {
        let mut m = minimal_module();
        m.manifest = FeatureManifest::from_features(&[Feature::TailCalls]);
        let err = compile(&m).expect_err("tail calls rejected");
        // The Backend::check_module default check fires before the
        // explicit TailCalls guard — that's fine, the error kind is
        // still UnsupportedFeature.
        assert_eq!(err.kind, BackendErrorKind::UnsupportedFeature);
    }

    #[test]
    fn rejects_intrinsic_node() {
        use semantic_ir::{SirType, Stmt};
        let mut m = minimal_module();
        m.manifest = FeatureManifest::from_features(&[Feature::Intrinsics]);
        // Replace main's body with an intrinsic.
        let main = m.functions.iter_mut().find(|f| f.name == "main").unwrap();
        main.body.stmts.push(Stmt::ExprStmt {
            expr: Expr::Intrinsic {
                targets: vec!["typescript".into()],
                name: "raw_ts".into(),
                args: vec![],
                return_type: SirType::Any,
                effects: EffectSet::PURE,
                span: s(),
            },
            span: s(),
        });
        let err = compile(&m).expect_err("intrinsic rejected");
        assert!(
            err.kind == BackendErrorKind::UnsupportedFeature
                || err.kind == BackendErrorKind::UnsupportedIntrinsic
        );
    }

    #[test]
    fn end_to_end_from_twig_source() {
        let module = twig_to_semantic_ir::compile_source(
            "(define (add a b) (+ a b))\n(print (add 1 2))",
            "demo",
        )
        .expect("lower");
        let a = compile(&module).expect("compile");
        assert!(a.source.contains("function add(a: __Sir.Val, b: __Sir.Val)"));
        assert!(a.source.contains("__Sir.plus(a, b)"));
        assert!(a.source.contains("__Sir.print(add(1, 2))"));
    }

    #[test]
    fn end_to_end_closure_program() {
        let module = twig_to_semantic_ir::compile_source(
            "(define (adder n) (lambda (x) (+ x n)))\n(define add5 (adder 5))\n(print (add5 3))",
            "demo",
        )
        .expect("lower");
        let a = compile(&module).expect("compile");
        // The synthesised __lambda_0 function should be emitted.
        assert!(a.source.contains("function __lambda_0("));
        // MakeClosure should appear inside `adder`.
        assert!(a.source.contains("new __Sir.Closure"));
        // add5 is a global, set via globalSet in _init.
        assert!(a.source.contains("let add5: __Sir.Val = __Sir.NIL;"));
        // _init should be invoked.
        assert!(a.source.contains("_init();"));
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
    fn factorial_program_emits() {
        let src = "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1)))))\n(print (fact 5))";
        let module = twig_to_semantic_ir::compile_source(src, "fact").expect("lower");
        let a = compile(&module).expect("compile");
        // The if branches should appear via the truthy ternary.
        assert!(a.source.contains("__Sir.truthy"));
        // Recursive direct call.
        assert!(a.source.contains("fact("));
    }

    #[test]
    fn module_filename_sanitised() {
        let mut m = minimal_module();
        m.name = "compiler/lexer".into();
        let a = compile(&m).expect("compile");
        assert_eq!(a.filename, "compiler_lexer.ts");
    }
}
