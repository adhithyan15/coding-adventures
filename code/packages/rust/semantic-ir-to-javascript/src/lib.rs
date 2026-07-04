//! # semantic-ir-to-javascript
//!
//! Fifth backend for the narrow-waist Semantic IR (after TypeScript,
//! Rust, Python, Go) — emits **self-contained** JavaScript from a
//! [`semantic_ir::Module`].
//!
//! "Self-contained" means every produced `.js` file pastes the runtime
//! helpers inline (a `__Sir` IIFE; see [`runtime`]).  There is no
//! `require()`, no `import`, and no `npm install`: the file runs
//! directly via `node <file>.js`.  This is the key contrast with the
//! TypeScript backend, which imports a published
//! `@coding-adventures/sir-runtime-*` package and carries type
//! annotations.  Strip the annotations, inline the runtime, and the two
//! emitters are otherwise the same shape.
//!
//! ## Public API
//!
//! ```ignore
//! use semantic_ir_to_javascript::{compile, JavaScriptBackend};
//! use semantic_ir::Backend;
//!
//! let module = /* a semantic_ir::Module from any frontend */;
//!
//! // Direct entry point:
//! let artifact = compile(&module)?;
//!
//! // Or via the Backend trait:
//! let artifact = JavaScriptBackend::new().compile(&module)?;
//! ```
//!
//! Both paths return [`semantic_ir::Artifact`] with `filename`,
//! `source` (the generated `.js`), and metadata.
//!
//! ## Capability declaration (this milestone, D4)
//!
//! This milestone accepts the **v0 feature set plus all of SIR16 / v1**:
//!
//! - v0: `Closures`, `Pairs`, `Symbols`, `Strings`, `DynamicTyping`,
//!   `OptionalTypeAnnotations`, `MutualRecursion`, `Globals`.
//! - SIR16: `Floats`, `ShortCircuit`, `Sequences`, `Maps`,
//!   `MutableBindings`, `Loops` — JavaScript supports all six natively,
//!   so emit is direct (arrays, `Map`, `while`/`for`, reassignable `let`).
//!
//! It also accepts the SIR17 exception features (`Exceptions`,
//! `Classes`, `Constants`) and, as of O3, full user-defined-class OOP
//! (`InstanceVars`, `ClassVars` alongside `Classes`): instantiation,
//! method dispatch, `super`, `self`, and `@ivar`/`@@cvar` access lower
//! to the inlined `__Sir` OOP runtime.
//!
//! It **rejects** everything else at the capability check — the
//! remaining SIR18 features (e.g. string interpolation), `TailCalls` (V8
//! does not reliably tail-call optimise), and `Intrinsics` (empty
//! whitelist).  The accept-set is deliberately matched to exactly what
//! [`emit`](crate::emit) lowers, so a module that uses a deferred node is
//! turned away *before* lowering rather than producing wrong code.
//!
//! See [SIR18](../../../specs/SIR18-semantic-ir-to-javascript.md) for the
//! full per-node lowering rules.

mod emit;
mod runtime;

use semantic_ir::{
    Artifact, ArtifactMetadata, Backend, BackendError, BackendErrorKind, Feature, Module,
};

pub use emit::sanitize_ident;

/// Convenience entry point: validates the module, runs the capability
/// checks, rejects unsupported features, and lowers to JavaScript.
pub fn compile(module: &Module) -> Result<Artifact, BackendError> {
    JavaScriptBackend::new().compile(module)
}

/// The v0 JavaScript backend.
pub struct JavaScriptBackend;

impl JavaScriptBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JavaScriptBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Features the JavaScript backend accepts in this milestone: the v0
/// surface plus all of SIR16 / v1 — exactly what [`emit`](crate::emit)
/// knows how to lower.
///
/// `TailCalls` and `Intrinsics` are excluded deliberately (the former is
/// fundamentally unsupported on V8; the latter has an empty whitelist).
/// SIR17 exceptions (`Exceptions`), the class *ancestry* slice of
/// `Classes`, and `Constants` are accepted (E1 / E2's JS half); the
/// remaining SIR17/18 OOP-dispatch and string-interpolation features stay
/// excluded because their emit arms are deferred — a module that declares
/// one is rejected here, never silently mis-compiled.
const ACCEPTED_FEATURES: &[Feature] = &[
    // v0 surface.
    Feature::Closures,
    Feature::Pairs,
    Feature::Symbols,
    Feature::Strings,
    Feature::DynamicTyping,
    Feature::OptionalTypeAnnotations,
    Feature::MutualRecursion,
    Feature::Globals,
    // SIR16 / v1 surface (all native in JavaScript).
    Feature::Floats,
    Feature::ShortCircuit,
    Feature::Sequences,
    Feature::Maps,
    Feature::MutableBindings,
    Feature::Loops,
    // P2d: default parameters — JavaScript native default params have
    // exactly SIR's call-time, param-scope semantics, so `emit` lowers a
    // `Param { default: Some(_) }` to a native `name = <default>` and a
    // short DirectCall to the bare args (defaults fill omitted trailing
    // params).  See `emit_function` / `emit_expr`'s `DirectCall` arm.
    Feature::DefaultParams,
    // KW4: keyword parameters & arguments.  JavaScript has no native
    // keyword-call form, so `emit` lowers `Keyword` params to a trailing
    // `__kw` options object (destructured in the body prologue) and each
    // `Expr::KeywordArg` at a call site into a trailing object literal —
    // the same zero-dependency, direct lowering the TypeScript backend
    // uses (spec §4).  Accepted here exactly as `DefaultParams` is.
    Feature::KeywordParams,
    // Constants — a `Const`-scoped `VarRef` (an uppercase name like a
    // class or a named constant).  Accepted because a `raise Foo` names
    // its exception class as a `Const` VarRef; the `raise` arm consumes
    // that Const as a *string* class name, and any other constant read
    // emits its bare identifier.  See `emit_var_ref`'s `Const` arm.
    Feature::Constants,
    // E1 (SIR17): structured exception handling.  `Stmt::TryCatch` lowers
    // to a native `try { … } catch (__exc) { … } finally { … }` whose
    // catch body is a `__Sir.rescueMatches`-guarded if/else-if chain, and
    // the `raise` builtin lowers to `__Sir.raiseError(cls, msg)` — the
    // same direct lowering the TypeScript backend uses, but against the
    // *inlined* exception runtime rather than an imported package.  See
    // `emit`'s `TryCatch` / `raise` arms and `runtime::RUNTIME`.
    Feature::Exceptions,
    // E2 (SIR17) + O3 (SIR18): class definitions.  A `class MyErr <
    // StandardError` supplies a user ancestry edge (the emitter collects
    // every `ClassDef { superclass: Some(_) }` pair into one
    // `__Sir.registerAncestry({ … })` at program init) *and* — as of O3 —
    // full user-defined-class OOP is executed: instantiation, method
    // definition + dispatch, `super`, and `self` all lower to the inlined
    // `__Sir` OOP runtime.  The frontend hoists method `def`s to top-level
    // functions and registers them with `__def_method__` /
    // `__def_class_method__`; `Klass.new` lowers to `__new__`, `super` to
    // `__super__`, and `self` to `__self__`.
    Feature::Classes,
    // O3: instance variables (`@x`) and class variables (`@@x`).  A
    // `VarRef`/`Assign` with `Scope::Instance` lowers to
    // `__Sir.ivarGet("@x")` / `ivarSet("@x", v)` on the current `self`;
    // `Scope::ClassVar` lowers to the analogous `cvarGet`/`cvarSet`.  Both
    // act on the dynamic `self` a running method pushed, so they are only
    // meaningful inside a method body (a bare read outside one reads nil,
    // matching Ruby's "no prior declaration" rule for these scopes).
    Feature::InstanceVars,
    Feature::ClassVars,
    // MX4 (SIR mixins): `module M … end` + `include` / `extend`.  A module
    // body's `def`s register into the SAME runtime method table as a class
    // (keyed by the module name via `__def_method__`), and `include M` /
    // `extend M` lower to `__include__("Owner","M")` / `__extend__(…)` — the
    // frontend triggers `Feature::Modules` for all three.  The inlined OOP
    // runtime's method-resolution walk now follows Ruby's MRO (class →
    // included modules, most-recent-first → superclass → …), and `extend`
    // registers a module's methods as class ("singleton") methods, so a
    // mixed-in method is found on an including class's instances (`include`)
    // or on the class itself (`extend`).  See `emit`'s `__include__` /
    // `__extend__` / `__class_method__` arms and `runtime::resolveMethod`.
    Feature::Modules,
];

impl Backend for JavaScriptBackend {
    fn target_tag(&self) -> &'static str {
        "javascript"
    }

    fn accepts_features(&self) -> &'static [Feature] {
        ACCEPTED_FEATURES
    }

    fn accepts_intrinsics(&self) -> &'static [&'static str] {
        &[]
    }

    fn compile(&self, module: &Module) -> Result<Artifact, BackendError> {
        // 1. Validate at the SIR boundary.  Lowering assumes the module
        //    is structurally well-formed.  A non-ok validation result
        //    that carries an error-severity issue blocks lowering;
        //    warnings-only results pass through.
        let r = semantic_ir::validate(module);
        if let Some(e) = r.errors().next().cloned() {
            return Err(BackendError {
                kind: BackendErrorKind::InvalidModule,
                message: format!("module failed validation: {}", e.message),
                span: e.span,
            });
        }

        // 2. Capability check: every declared feature must be accepted,
        //    and every intrinsic must be whitelisted (none are).
        if let Some(e) = self.check_module(module).into_iter().next() {
            return Err(e);
        }

        // 3. Tail-calls are fundamentally unsupported on V8.  The
        //    capability check in step 2 already rejects them (TailCalls
        //    is not in the accept-set), but keep an explicit, clearer
        //    error in case the accept-set ever changes.
        if module.manifest.contains(Feature::TailCalls) {
            return Err(BackendError {
                kind: BackendErrorKind::UnsupportedFeature,
                message: "javascript backend cannot satisfy `tail-calls` feature".into(),
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
            filename: format!("{}.js", module.name.replace('/', "_")),
            source,
            metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use semantic_ir::{
        Block, EffectSet, Expr, FeatureManifest, Function, Metadata, Span, Stmt,
    };

    fn s() -> Span {
        Span::synthetic()
    }

    /// A minimal module: `main` returns 42.
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
    fn target_tag_is_javascript() {
        assert_eq!(JavaScriptBackend::new().target_tag(), "javascript");
    }

    #[test]
    fn compiles_minimal_module() {
        let a = compile(&minimal_module()).expect("compile");
        assert!(a.source.contains("function main() {"));
        assert!(a.source.contains("return 42;"));
        assert!(a.source.contains("\"use strict\";"));
        assert!(a.filename.ends_with(".js"));
        assert_eq!(a.filename, "demo.js");
        assert!(a.metadata.bytes > 0);
        assert!(a.metadata.line_count > 0);
    }

    #[test]
    fn module_filename_sanitised() {
        let mut m = minimal_module();
        m.name = "compiler/lexer".into();
        let a = compile(&m).expect("compile");
        assert_eq!(a.filename, "compiler_lexer.js");
    }

    #[test]
    fn rejects_tail_calls_feature() {
        let mut m = minimal_module();
        m.manifest = FeatureManifest::from_features(&[Feature::TailCalls]);
        let err = compile(&m).expect_err("tail calls rejected");
        assert_eq!(err.kind, BackendErrorKind::UnsupportedFeature);
    }

    #[test]
    fn accepts_sir16_loops_feature() {
        // Loops are now part of the accepted SIR16 surface, so a module
        // that merely declares the feature compiles.
        let mut m = minimal_module();
        m.manifest = FeatureManifest::from_features(&[Feature::Loops]);
        compile(&m).expect("loops accepted");
    }

    #[test]
    fn accepts_all_sir16_features() {
        // Every SIR16 / v1 feature is accepted as of D4.
        let mut m = minimal_module();
        m.manifest = FeatureManifest::from_features(&[
            Feature::Floats,
            Feature::ShortCircuit,
            Feature::Sequences,
            Feature::Maps,
            Feature::MutableBindings,
            Feature::Loops,
        ]);
        compile(&m).expect("all SIR16 features accepted");
    }

    #[test]
    fn accepts_sir17_exceptions_feature() {
        // E1: the SIR17 exception feature is now lowered (TryCatch →
        // native try/catch, `raise` → `__Sir.raiseError`), so a module
        // declaring it passes the capability check.
        let mut m = minimal_module();
        m.manifest = FeatureManifest::from_features(&[Feature::Exceptions]);
        compile(&m).expect("exceptions accepted");
    }

    #[test]
    fn accepts_sir17_classes_feature() {
        // E2 (JS half): `Feature::Classes` is accepted for its ancestry
        // edge — a `class Child < Super` supplies a user-defined
        // superclass relation the exception runtime merges in.
        let mut m = minimal_module();
        m.manifest = FeatureManifest::from_features(&[Feature::Classes]);
        compile(&m).expect("classes accepted");
    }

    #[test]
    fn rejects_intrinsic_node() {
        use semantic_ir::SirType;
        let mut m = minimal_module();
        m.manifest = FeatureManifest::from_features(&[Feature::Intrinsics]);
        let main = m.functions.iter_mut().find(|f| f.name == "main").unwrap();
        main.body.stmts.push(Stmt::ExprStmt {
            expr: Expr::Intrinsic {
                targets: vec!["javascript".into()],
                name: "raw_js".into(),
                args: vec![],
                return_type: SirType::Dynamic,
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
        assert!(a.source.contains("function add(a, b) {"), "got:\n{}", a.source);
        // `+` is Ruby-polymorphic, so it routes through the runtime
        // dispatch helper (numeric add, String/Array concat) rather than
        // native infix.
        assert!(a.source.contains("return __Sir.plus(a, b);"), "got:\n{}", a.source);
        // Top-level print of a direct call.
        assert!(a.source.contains("__Sir.print(add(1, 2))"), "got:\n{}", a.source);
    }

    #[test]
    fn end_to_end_closure_program() {
        let module = twig_to_semantic_ir::compile_source(
            "(define (adder n) (lambda (x) (+ x n)))\n(define add5 (adder 5))\n(print (add5 3))",
            "demo",
        )
        .expect("lower");
        let a = compile(&module).expect("compile");
        // A synthesised lambda function is emitted.
        assert!(a.source.contains("function __lambda_0("), "got:\n{}", a.source);
        // MakeClosure → a `new __Sir.Closure`.
        assert!(a.source.contains("new __Sir.Closure"), "got:\n{}", a.source);
        // `add5` is a module global, initialised in `_init`.
        assert!(a.source.contains("let add5 = null;"), "got:\n{}", a.source);
        assert!(a.source.contains("_init();"), "got:\n{}", a.source);
        // The indirect call to the closure routes through applyClosure.
        assert!(a.source.contains("__Sir.applyClosure"), "got:\n{}", a.source);
    }

    #[test]
    fn factorial_program_emits() {
        let src = "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1)))))\n(print (fact 5))";
        let module = twig_to_semantic_ir::compile_source(src, "fact").expect("lower");
        let a = compile(&module).expect("compile");
        // The `if` lowers through the truthy ternary.
        assert!(a.source.contains("__Sir.truthy"), "got:\n{}", a.source);
        // Native comparison and recursive direct call.
        assert!(a.source.contains("(n === 0)"), "got:\n{}", a.source);
        assert!(a.source.contains("fact("), "got:\n{}", a.source);
    }

    #[test]
    fn output_is_deterministic() {
        let module =
            twig_to_semantic_ir::compile_source("(define (id x) x)\n(id 7)", "demo").expect("lower");
        let a = compile(&module).expect("compile");
        let b = compile(&module).expect("compile again");
        assert_eq!(a.source, b.source);
    }
}
