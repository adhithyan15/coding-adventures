//! # semantic-ir-to-go
//!
//! Fourth backend for the narrow-waist Semantic IR — emits
//! **self-contained** Go source code from a [`semantic_ir::Module`].
//!
//! Output is a single `.go` file with `package main` plus inlined
//! runtime helpers; no `go.mod` dependencies required.  The file
//! compiles with `go build <file>.go` and runs.

mod emit;
mod runtime;

use semantic_ir::{
    Artifact, ArtifactMetadata, Backend, BackendError, BackendErrorKind, Feature, Module,
};

pub use emit::sanitize_ident;

pub fn compile(module: &Module) -> Result<Artifact, BackendError> {
    GoBackend::new().compile(module)
}

pub struct GoBackend;

impl GoBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GoBackend {
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

impl Backend for GoBackend {
    fn target_tag(&self) -> &'static str {
        "go"
    }

    fn accepts_features(&self) -> &'static [Feature] {
        ACCEPTED_FEATURES
    }

    fn accepts_intrinsics(&self) -> &'static [&'static str] {
        &[]
    }

    fn compile(&self, module: &Module) -> Result<Artifact, BackendError> {
        let r = semantic_ir::validate(module);
        if let Some(e) = r.errors().next().cloned() {
            return Err(BackendError {
                kind: BackendErrorKind::InvalidModule,
                message: format!("module failed validation: {}", e.message),
                span: e.span,
            });
        }

        let cap_errors = self.check_module(module);
        if let Some(e) = cap_errors.into_iter().next() {
            return Err(e);
        }

        if module.manifest.contains(Feature::TailCalls) {
            return Err(BackendError {
                kind: BackendErrorKind::UnsupportedFeature,
                message: "go backend cannot satisfy `tail_calls` feature".into(),
                span: module.span.clone(),
            });
        }

        let source = emit::emit_module(module);
        let metadata = ArtifactMetadata {
            bytes: source.len(),
            line_count: source.lines().count(),
            ..Default::default()
        };

        Ok(Artifact {
            filename: format!("{}.go", module.name.replace('/', "_")),
            source,
            metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_tag_is_go() {
        assert_eq!(GoBackend::new().target_tag(), "go");
    }

    #[test]
    fn end_to_end_twig_to_go_identity() {
        let m = twig_to_semantic_ir::compile_source("(define (id x) x)\n(id 42)", "demo")
            .expect("lower");
        let a = compile(&m).expect("compile");
        assert!(a.source.contains("package main"));
        assert!(a.source.contains("func id(x Value) Value"));
        assert!(a.source.contains("id(Value(int64(42)))"));
    }

    #[test]
    fn end_to_end_twig_to_go_arithmetic() {
        let m = twig_to_semantic_ir::compile_source(
            "(define (add a b) (+ a b))\n(print (add 1 2))",
            "demo",
        )
        .expect("lower");
        let a = compile(&m).expect("compile");
        assert!(a.source.contains("func add(a Value, b Value) Value"));
        assert!(a.source.contains("_sir_plus([]Value{a, b})"));
        assert!(a.source.contains("_sir_print([]Value{add("));
    }

    #[test]
    fn end_to_end_twig_to_go_closure() {
        let m = twig_to_semantic_ir::compile_source(
            "(define (adder n) (lambda (x) (+ x n)))\n(define add5 (adder 5))\n(print (add5 3))",
            "demo",
        )
        .expect("lower");
        let a = compile(&m).expect("compile");
        assert!(a.source.contains("func _0x5flambda_5f0_(n Value, x Value) Value")
            || a.source.contains("func __lambda_0(n Value, x Value) Value"));
        assert!(a.source.contains("_sir_make_closure"));
    }

    #[test]
    fn rejects_tail_calls() {
        let mut m = twig_to_semantic_ir::compile_source("(+ 1 2)", "demo").expect("lower");
        m.manifest = semantic_ir::FeatureManifest::from_features(&[Feature::TailCalls]);
        let err = compile(&m).expect_err("tail calls rejected");
        assert_eq!(err.kind, BackendErrorKind::UnsupportedFeature);
    }

    #[test]
    fn output_is_deterministic() {
        let m = twig_to_semantic_ir::compile_source("(define (id x) x)\n(id 7)", "demo")
            .expect("lower");
        let a = compile(&m).expect("compile");
        let b = compile(&m).expect("compile again");
        assert_eq!(a.source, b.source);
    }

    #[test]
    fn module_filename_sanitised() {
        let m = twig_to_semantic_ir::compile_source("(+ 1 2)", "compiler/lexer").expect("lower");
        let a = compile(&m).expect("compile");
        assert_eq!(a.filename, "compiler_lexer.go");
    }
}
