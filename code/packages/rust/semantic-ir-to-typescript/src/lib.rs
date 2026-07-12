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
    // SIR16 expression features — emitted natively (sequences → Array,
    // maps → Map, short-circuit → truthy-guarded arrow, interpolation →
    // display-joined), per code/specs/sir-runtime.md.
    Feature::Floats,
    Feature::Sequences,
    Feature::Maps,
    Feature::ShortCircuit,
    Feature::StringInterpolation,
    // SIR16 mutation & loops — emitted natively (assignment → `=`,
    // indexed set → `arr[i] = v` / `map.set(k, v)`, `while` →
    // truthy-guarded `while`, `for`-range → direction-aware C-for,
    // `for`-each → `for…of`), per code/specs/sir-runtime.md.
    Feature::MutableBindings,
    Feature::Loops,
    // SIR17 OOP & scopes — class/module declarations register in the
    // OOP runtime; instance/class vars route through its stores; consts
    // are module-level bindings; `is_a?`-style dispatch goes through
    // `__SirOop.callMethod`.  Per code/specs/sir-runtime.md, with the
    // documented v0 limit (frontend hoists methods without receivers).
    Feature::Classes,
    Feature::Modules,
    Feature::InstanceVars,
    Feature::ClassVars,
    Feature::Constants,
    // SIR17 exceptions — `try/catch/finally` is native; the SIR exception
    // object, `raise`, and ordered rescue-clause class matching come from
    // `@coding-adventures/sir-runtime-exceptions`.  Per code/specs/sir-runtime.md.
    Feature::Exceptions,
    // P2b default parameters — a param with `default = Some(expr)`.  Emitted
    // as a TypeScript-native default (`name: __Sir.Val = <expr>`).  The
    // call-time / param-scope semantics of a SIR default — evaluated per call
    // in the callee's parameter scope, free to reference EARLIER params —
    // line up exactly with TS native defaults, so the lowering is a direct
    // inline (no runtime helper, no call-site padding).  Per
    // code/specs/sir-runtime.md.
    Feature::DefaultParams,
    // KW3 keyword parameters & arguments — a `Keyword` param or an
    // `Expr::KeywordArg`.  TypeScript has no native kwargs, so both lower to
    // the conventional zero-runtime "options object" (see `emit::emit_function`
    // / `emit::emit_call_args` and `code/specs/sir-keyword-params.md` §4): the
    // callee gains a trailing `__kw` object destructured in its prologue, and a
    // call collapses its keyword args into one trailing object literal.  Direct
    // lowering, no runtime helper — mirrors how `DefaultParams` is declared.
    Feature::KeywordParams,
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
        //    module is structurally well-formed.  Collapse the
        //    "not ok" branch to a direct `if let Some(e) = ...` so
        //    that a non-ok result lacking error-severity issues
        //    (warnings-only) cannot silently bypass the guard.
        let r = semantic_ir::validate(module);
        if let Some(e) = r.errors().next().cloned() {
            return Err(BackendError {
                kind: BackendErrorKind::InvalidModule,
                message: format!("module failed validation: {}", e.message),
                span: e.span,
            });
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
    fn end_to_end_ruby_block_capture_emits_native_capture_ts() {
        // RB1/RB2 mirror of the Python execution-proof: `outer`'s block
        // `{ |x| yield x }` is hoisted to `__block_0` whose `yield x` re-targets
        // the *enclosing* method's block, so the block closes over `outer`'s
        // `__sir_block__` — the first non-empty `MakeClosure` capture.  Node
        // cannot run the emitted *TypeScript* directly (it carries type
        // annotations and the runtime package ships as `.ts`), so unlike the
        // Python sibling this proves the binding by shape: TS captures via a
        // native closure that forwards the captured block as the first argument.
        let src = "def twice\n  yield 1\n  yield 2\nend\n\
                   def outer\n  twice { |x| yield x }\nend\n\
                   outer { |n| print n }\n";
        let module = ruby_to_semantic_ir::compile_source(src, "demo").expect("lower ruby");
        let a = compile(&module).expect("compile to ts");

        // The hoisted block declares the captured block as its first parameter.
        assert!(
            a.source.contains("function __block_0(__sir_block__: __Sir.Val, x: __Sir.Val)"),
            "hoisted block must take the captured block first; got:\n{}",
            a.source
        );
        // The enclosing block is closed over and forwarded as the first arg via
        // a native closure (TS has no positional-capture array; it uses scope).
        assert!(
            a.source.contains("__block_0(__sir_block__, ..._a)"),
            "captured block must be forwarded into the hoisted block; got:\n{}",
            a.source
        );
        // And it is threaded into `twice` as that block's value.
        assert!(
            a.source.contains("twice(new __Sir.Closure((..._a: __Sir.Val[]) => __block_0(__sir_block__, ..._a)))"),
            "enclosing block must be threaded through the non-empty capture; got:\n{}",
            a.source
        );
    }

    #[test]
    fn end_to_end_block_captures_outer_local_emits_native_capture_ts() {
        // M4 (TS shape mirror of the Python execution-proof): a block reads an
        // enclosing local (`base`); the hoisted block must declare it as a
        // leading parameter and the closure must close over it.
        let src = "def apply\n  yield 5\nend\n\
                   def run\n  base = 100\n  apply { |n| print n + base }\nend\n\
                   run()\n";
        let module = ruby_to_semantic_ir::compile_source(src, "demo").expect("lower ruby");
        let a = compile(&module).expect("compile to ts");
        assert!(
            a.source.contains("function __block_0(base: __Sir.Val, n: __Sir.Val)"),
            "captured `base` must be the hoisted block's leading param; got:\n{}",
            a.source
        );
        assert!(
            a.source.contains("__block_0(base, ..._a)"),
            "captured `base` must be forwarded into the hoisted block; got:\n{}",
            a.source
        );
    }

    #[test]
    fn end_to_end_case_when_emits_case_eq_and_is_a_ts() {
        // M5: a `when` range/regex/literal → `__SirOop.caseEq`; a `when Const`
        // → an `is_a?` dispatch.
        let src = "x = 5\ncase x\nwhen 1..3\n  y = 1\nwhen Integer\n  y = 2\nend\n";
        let module = ruby_to_semantic_ir::compile_source(src, "demo").expect("lower ruby");
        let a = compile(&module).expect("compile to ts");
        assert!(
            a.source.contains("__SirOop.caseEq("),
            "range `when` must emit caseEq; got:\n{}",
            a.source
        );
        assert!(
            a.source.contains("\"is_a?\""),
            "class `when` must emit an is_a? dispatch; got:\n{}",
            a.source
        );
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
        assert!(a.source.contains("function add(a: __Sir.Val, b: __Sir.Val)"));
        assert!(a.source.contains("__Sir.add(a, b)"));
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
        assert!(a.source.contains("let add5: __Sir.Val = null;"));
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

    // ── SIR16 expression features: Ruby → native TypeScript ─────────

    #[test]
    fn end_to_end_ruby_array_literal_ts() {
        let module =
            ruby_to_semantic_ir::compile_source("x = [10, 20, 30]\nputs(x)\n", "demo")
                .expect("lower ruby");
        let a = compile(&module).expect("compile to ts");
        assert!(a.source.contains("[10, 20, 30]"), "got:\n{}", a.source);
    }

    #[test]
    fn end_to_end_ruby_hash_literal_ts() {
        let module =
            ruby_to_semantic_ir::compile_source("puts({a: 1})\n", "demo").expect("lower ruby");
        let a = compile(&module).expect("compile to ts");
        assert!(
            a.source.contains("new Map<__Sir.Val, __Sir.Val>("),
            "expected a native Map; got:\n{}",
            a.source
        );
    }

    #[test]
    fn end_to_end_ruby_short_circuit_ts() {
        // case/in array pattern desugars to LogicalAnd → truthy-guarded arrow.
        let module = ruby_to_semantic_ir::compile_source(
            "x = [7, 8]\ncase x\nin [7, b]\n  puts(b)\nend\n",
            "demo",
        )
        .expect("lower ruby");
        let a = compile(&module).expect("compile to ts");
        assert!(
            a.source.contains("__Sir.truthy(__l)") && a.source.contains("(__l: __Sir.Val) =>"),
            "expected truthy-guarded arrow for &&; got:\n{}",
            a.source
        );
    }

    #[test]
    fn end_to_end_ruby_interpolation_ts() {
        let module = ruby_to_semantic_ir::compile_source("x = 5\nputs(\"v=#{x}\")\n", "demo")
            .expect("lower ruby");
        let a = compile(&module).expect("compile to ts");
        assert!(
            a.source.contains("__Sir.toDisplay("),
            "expected interpolation via __Sir.toDisplay; got:\n{}",
            a.source
        );
    }

    // ── SIR16 mutation & loops: Ruby / direct SIR → native TS ───────

    #[test]
    fn end_to_end_ruby_while_loop_ts() {
        // A mutated counter must bind with `let`, the condition routes
        // through SIR truthiness, and the reassignment is a bare `=`.
        let module = ruby_to_semantic_ir::compile_source(
            "i = 0\nwhile i < 3\n  i = i + 1\nend\nputs(i)\n",
            "demo",
        )
        .expect("lower ruby");
        let a = compile(&module).expect("compile to ts");
        assert!(a.source.contains("let i: __Sir.Val = 0;"), "got:\n{}", a.source);
        assert!(
            a.source.contains("while (__Sir.truthy(__Sir.lt(i, 3)))"),
            "got:\n{}",
            a.source
        );
        assert!(a.source.contains("i = __Sir.add(i, 1);"), "got:\n{}", a.source);
    }

    #[test]
    fn immutable_binding_stays_const() {
        // No reassignment → the binding keeps `const`.
        let module =
            ruby_to_semantic_ir::compile_source("x = 7\nputs(x)\n", "demo").expect("lower ruby");
        let a = compile(&module).expect("compile to ts");
        assert!(a.source.contains("const x: __Sir.Val = 7;"), "got:\n{}", a.source);
    }

    // Direct-SIR helpers for the loop/index-set kinds the Ruby frontend
    // does not yet construct (they arrive via other frontends / tests).
    fn module_with_main_body(stmts: Vec<semantic_ir::Stmt>, value: Expr, feats: &[Feature]) -> Module {
        Module {
            name: "demo".into(),
            manifest: FeatureManifest::from_features(feats),
            imports: vec![],
            exports: vec![],
            functions: vec![Function {
                name: "main".into(),
                params: vec![],
                return_type: None,
                captures: vec![],
                body: Block { stmts, value, span: s() },
                effects: EffectSet::PURE,
                metadata: Metadata::new(),
                span: s(),
            }],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("test")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        }
    }

    #[test]
    fn emit_for_range_is_direction_aware() {
        use semantic_ir::Stmt;
        let body = Block {
            stmts: vec![Stmt::ExprStmt {
                expr: Expr::BuiltinCall {
                    name: "print".into(),
                    args: vec![Expr::VarRef { name: "i".into(), scope: semantic_ir::Scope::Local, span: s() }],
                    effects: EffectSet::PURE,
                    span: s(),
                },
                span: s(),
            }],
            value: Expr::NilLit { span: s() },
            span: s(),
        };
        let m = module_with_main_body(
            vec![Stmt::ForRange {
                var: "i".into(),
                start: Expr::IntLit { value: 0, span: s() },
                stop: Expr::IntLit { value: 3, span: s() },
                step: Expr::IntLit { value: 1, span: s() },
                body,
                span: s(),
            }],
            Expr::NilLit { span: s() },
            &[Feature::Loops, Feature::MutableBindings],
        );
        let a = compile(&m).expect("compile");
        // `stop`/`step` evaluated once into temporaries; condition is
        // direction-aware so a negative step would still terminate.
        assert!(a.source.contains("const __sir_stop_0: number = (3) as number;"), "got:\n{}", a.source);
        assert!(a.source.contains("const __sir_step_0: number = (1) as number;"), "got:\n{}", a.source);
        assert!(
            a.source.contains("__sir_step_0 >= 0 ? (i as number) < __sir_stop_0 : (i as number) > __sir_stop_0"),
            "got:\n{}",
            a.source
        );
        assert!(a.source.contains("i = (i as number) + __sir_step_0;"), "got:\n{}", a.source);
    }

    #[test]
    fn emit_for_each_and_index_set() {
        use semantic_ir::{Scope, Stmt};
        // for x in arr: arr[0] = x   (SeqSet inside ForEach body)
        let body = Block {
            stmts: vec![Stmt::SeqSet {
                seq: Expr::VarRef { name: "arr".into(), scope: Scope::Local, span: s() },
                index: Expr::IntLit { value: 0, span: s() },
                value: Expr::VarRef { name: "x".into(), scope: Scope::Local, span: s() },
                span: s(),
            }],
            value: Expr::NilLit { span: s() },
            span: s(),
        };
        let m = module_with_main_body(
            vec![
                Stmt::LetBinding {
                    name: "arr".into(),
                    sir_type: None,
                    value: Expr::SeqLit {
                        items: vec![Expr::IntLit { value: 1, span: s() }],
                        span: s(),
                    },
                    span: s(),
                },
                Stmt::ForEach {
                    var: "x".into(),
                    iter: Expr::VarRef { name: "arr".into(), scope: Scope::Local, span: s() },
                    body,
                    span: s(),
                },
            ],
            Expr::VarRef { name: "arr".into(), scope: Scope::Local, span: s() },
            &[Feature::Loops, Feature::Sequences, Feature::MutableBindings],
        );
        let a = compile(&m).expect("compile");
        assert!(
            a.source.contains("for (const x of ((arr) as __Sir.Val[]))"),
            "got:\n{}",
            a.source
        );
        assert!(
            a.source.contains("((arr) as __Sir.Val[])[(0) as number] = x;"),
            "got:\n{}",
            a.source
        );
    }

    #[test]
    fn emit_map_set() {
        use semantic_ir::{Scope, Stmt};
        let m = module_with_main_body(
            vec![
                Stmt::LetBinding {
                    name: "m".into(),
                    sir_type: None,
                    value: Expr::MapLit { entries: vec![], span: s() },
                    span: s(),
                },
                Stmt::MapSet {
                    map: Expr::VarRef { name: "m".into(), scope: Scope::Local, span: s() },
                    key: Expr::IntLit { value: 1, span: s() },
                    value: Expr::IntLit { value: 2, span: s() },
                    span: s(),
                },
            ],
            Expr::VarRef { name: "m".into(), scope: Scope::Local, span: s() },
            &[Feature::Maps, Feature::MutableBindings],
        );
        let a = compile(&m).expect("compile");
        assert!(
            a.source.contains("((m) as Map<__Sir.Val, __Sir.Val>).set(1, 2);"),
            "got:\n{}",
            a.source
        );
    }

    // ── SIR17 OOP & scopes: Ruby → native TS + sir-runtime-oop ──────

    #[test]
    fn end_to_end_ruby_class_inheritance_and_is_a_ts() {
        // `class Dog < Animal` registers ancestry; `is_a?(Integer)`
        // dispatches through the OOP runtime with the class operand
        // passed as a name string.
        let module = ruby_to_semantic_ir::compile_source(
            "class Dog < Animal\n  def speak\n    42\n  end\nend\nd = 5\nputs(d.is_a?(Integer))\n",
            "demo",
        )
        .expect("lower ruby");
        let a = compile(&module).expect("compile to ts");
        assert!(
            a.source.contains(r#"import * as __SirOop from "@coding-adventures/sir-runtime-oop";"#),
            "expected the OOP import; got:\n{}",
            a.source
        );
        assert!(
            a.source.contains(r#"__SirOop.defineClass("Dog", "Animal");"#),
            "got:\n{}",
            a.source
        );
        assert!(
            a.source.contains(r#"__SirOop.callMethod(d, "is_a?", "Integer")"#),
            "got:\n{}",
            a.source
        );
    }

    #[test]
    fn end_to_end_ruby_const_in_class_body_ts() {
        let module =
            ruby_to_semantic_ir::compile_source("class Foo\n  LEGS = 4\nend\n", "demo")
                .expect("lower ruby");
        let a = compile(&module).expect("compile to ts");
        assert!(a.source.contains(r#"__SirOop.defineClass("Foo", null);"#), "got:\n{}", a.source);
        assert!(a.source.contains("const LEGS: __Sir.Val = 4;"), "got:\n{}", a.source);
    }

    #[test]
    fn end_to_end_ruby_class_var_ts() {
        // `@@count` reads/writes route through the class-variable store
        // (no class context survives method hoisting).
        let module = ruby_to_semantic_ir::compile_source(
            "class Foo\n  @@count = 0\n  def inc\n    @@count = @@count + 1\n  end\nend\n",
            "demo",
        )
        .expect("lower ruby");
        let a = compile(&module).expect("compile to ts");
        assert!(a.source.contains(r#"__SirOop.cvarSet("@@count", 0);"#), "got:\n{}", a.source);
        assert!(
            a.source.contains(r#"__SirOop.cvarSet("@@count", __Sir.add(__SirOop.cvarGet("@@count"), 1));"#),
            "got:\n{}",
            a.source
        );
    }

    #[test]
    fn end_to_end_ruby_instance_var_ts() {
        use semantic_ir::{Scope, Stmt};
        // The frontend mis-parses multi-statement method bodies, so
        // exercise the Instance scope directly: a method that writes and
        // reads `@x` → ivarSet/ivarGet against the current self.
        let body = Block {
            stmts: vec![Stmt::Assign {
                name: "@x".into(),
                scope: Scope::Instance,
                value: Expr::IntLit { value: 1, span: s() },
                span: s(),
            }],
            value: Expr::VarRef { name: "@x".into(), scope: Scope::Instance, span: s() },
            span: s(),
        };
        let m = Module {
            name: "demo".into(),
            manifest: FeatureManifest::from_features(&[
                Feature::InstanceVars,
                Feature::MutableBindings,
            ]),
            imports: vec![],
            exports: vec![],
            functions: vec![Function {
                name: "bar".into(),
                params: vec![],
                return_type: None,
                captures: vec![],
                body,
                effects: EffectSet::PURE,
                metadata: Metadata::new(),
                span: s(),
            }],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("test")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        };
        let a = compile(&m).expect("compile");
        assert!(a.source.contains(r#"__SirOop.ivarSet("@x", 1);"#), "got:\n{}", a.source);
        assert!(a.source.contains(r#"__SirOop.ivarGet("@x")"#), "got:\n{}", a.source);
    }

    #[test]
    fn end_to_end_ruby_module_ts() {
        let module =
            ruby_to_semantic_ir::compile_source("module Greet\n  def hi\n    1\n  end\nend\n", "demo")
                .expect("lower ruby");
        let a = compile(&module).expect("compile to ts");
        assert!(a.source.contains(r#"__SirOop.defineClass("Greet", null);"#), "got:\n{}", a.source);
    }

    #[test]
    fn non_oop_module_omits_oop_import() {
        // A pure arithmetic module must not gain a dependency on the OOP
        // runtime package.
        let module = twig_to_semantic_ir::compile_source("(print (+ 1 2))", "demo").expect("lower");
        let a = compile(&module).expect("compile");
        assert!(!a.source.contains("sir-runtime-oop"), "got:\n{}", a.source);
    }

    #[test]
    fn loop_in_expression_position_nests_in_iife() {
        use semantic_ir::{Scope, Stmt};
        // A block holding a While in expression position becomes the
        // then-branch of an `if`; the IIFE wraps the loop natively.
        let loop_block = Block {
            stmts: vec![Stmt::While {
                cond: Expr::BoolLit { value: false, span: s() },
                body: Block { stmts: vec![], value: Expr::NilLit { span: s() }, span: s() },
                span: s(),
            }],
            value: Expr::IntLit { value: 1, span: s() },
            span: s(),
        };
        let m = module_with_main_body(
            vec![],
            Expr::If {
                cond: Box::new(Expr::BoolLit { value: true, span: s() }),
                then_branch: Box::new(loop_block),
                else_branch: Box::new(Block {
                    stmts: vec![],
                    value: Expr::IntLit { value: 2, span: s() },
                    span: s(),
                }),
                span: s(),
            },
            &[Feature::Loops, Feature::MutableBindings],
        );
        let _ = Scope::Local;
        let a = compile(&m).expect("compile");
        assert!(a.source.contains("(() => {"), "expected IIFE; got:\n{}", a.source);
        assert!(a.source.contains("while (__Sir.truthy(false))"), "got:\n{}", a.source);
    }

    // ─── SIR17 exceptions (Q7) ──────────────────────────────────────────────

    #[test]
    fn end_to_end_ruby_begin_rescue_ensure_ts() {
        // begin … raise … rescue Type => e … ensure … end → native
        // try/catch/finally, dispatching on the rescue class through the
        // exception runtime and binding the caught value.
        let module = ruby_to_semantic_ir::compile_source(
            "begin\n  raise ArgumentError, \"bad\"\nrescue ArgumentError => e\n  puts(e)\nensure\n  puts(1)\nend\n",
            "demo",
        )
        .expect("lower ruby");
        let a = compile(&module).expect("compile to ts");
        let src = &a.source;
        assert!(src.contains("import * as __SirExc"), "got:\n{}", src);
        assert!(src.contains("try {"), "got:\n{}", src);
        assert!(
            src.contains(r#"__SirExc.raiseError("ArgumentError", "bad")"#),
            "got:\n{}",
            src
        );
        assert!(src.contains("catch (__exc) {"), "got:\n{}", src);
        assert!(
            src.contains(r#"if (__SirExc.rescueMatches(__exc, ["ArgumentError"]))"#),
            "got:\n{}",
            src
        );
        assert!(src.contains("const e: __Sir.Val = __exc;"), "got:\n{}", src);
        assert!(src.contains("throw __exc;"), "got:\n{}", src);
        assert!(src.contains("finally {"), "got:\n{}", src);
    }

    #[test]
    fn end_to_end_ruby_raise_message_only_ts() {
        // `raise "boom"` (no class) → implicit RuntimeError carrying the
        // message, matching Ruby.
        let module =
            ruby_to_semantic_ir::compile_source("raise \"boom\"\n", "demo").expect("lower ruby");
        let a = compile(&module).expect("compile to ts");
        assert!(
            a.source.contains(r#"__SirExc.raiseError("RuntimeError", "boom")"#),
            "got:\n{}",
            a.source
        );
    }

    #[test]
    fn non_throwing_module_omits_exc_import() {
        // A pure arithmetic module must not depend on the exception runtime.
        let module = twig_to_semantic_ir::compile_source("(print (+ 1 2))", "demo").expect("lower");
        let a = compile(&module).expect("compile");
        assert!(!a.source.contains("sir-runtime-exceptions"), "got:\n{}", a.source);
    }

    #[test]
    fn try_catch_bare_rescue_and_rethrow_ts() {
        use semantic_ir::{RescueClause, Scope, Stmt};
        // A bare `rescue` (no exception types, no binding) is a catch-all:
        // `rescueMatches(__exc, [])` is always true.  No `ensure` → no
        // `finally`.  Built directly because the frontend mis-parses some
        // bare-rescue surface forms.
        let try_stmt = Stmt::TryCatch {
            body: vec![Stmt::ExprStmt {
                expr: Expr::BuiltinCall {
                    name: "raise".into(),
                    args: vec![Expr::VarRef {
                        name: "RuntimeError".into(),
                        scope: Scope::Const,
                        span: s(),
                    }],
                    effects: EffectSet::PURE,
                    span: s(),
                },
                span: s(),
            }],
            rescues: vec![RescueClause {
                exception_types: vec![],
                binding: None,
                body: vec![Stmt::ExprStmt {
                    expr: Expr::IntLit { value: 7, span: s() },
                    span: s(),
                }],
                span: s(),
            }],
            ensure_body: None,
            span: s(),
        };
        let m = module_with_main_body(
            vec![try_stmt],
            Expr::NilLit { span: s() },
            &[Feature::Exceptions, Feature::Constants],
        );
        let a = compile(&m).expect("compile");
        let src = &a.source;
        assert!(src.contains(r#"__SirExc.raiseError("RuntimeError")"#), "got:\n{}", src);
        assert!(src.contains("__SirExc.rescueMatches(__exc, [])"), "got:\n{}", src);
        assert!(src.contains("} else {"), "got:\n{}", src);
        assert!(src.contains("throw __exc;"), "got:\n{}", src);
        // No ensure → no finally clause.
        assert!(!src.contains("finally {"), "got:\n{}", src);
    }

    // ─── SIR cons pairs now ship in the dedicated package (Q8) ──────────────

    #[test]
    fn pairs_emit_from_dedicated_package_ts() {
        // `car(cons(1, 2))` as a direct-SIR value, manifest declaring Pairs.
        let car_of_cons = Expr::BuiltinCall {
            name: "car".into(),
            args: vec![Expr::BuiltinCall {
                name: "cons".into(),
                args: vec![
                    Expr::IntLit { value: 1, span: s() },
                    Expr::IntLit { value: 2, span: s() },
                ],
                effects: EffectSet::PURE,
                span: s(),
            }],
            effects: EffectSet::PURE,
            span: s(),
        };
        let m = module_with_main_body(vec![], car_of_cons, &[Feature::Pairs]);
        let a = compile(&m).expect("compile");
        let src = &a.source;
        assert!(
            src.contains(r#"import * as __SirPairs from "@coding-adventures/sir-runtime-pairs";"#),
            "got:\n{}",
            src
        );
        assert!(src.contains("__SirPairs.car(__SirPairs.cons(1, 2))"), "got:\n{}", src);
        // No longer routed through the core namespace.
        assert!(!src.contains("__Sir.cons("), "got:\n{}", src);
    }

    #[test]
    fn non_pairs_module_omits_pairs_import_ts() {
        let module = twig_to_semantic_ir::compile_source("(print (+ 1 2))", "demo").expect("lower");
        let a = compile(&module).expect("compile");
        assert!(!a.source.contains("sir-runtime-pairs"), "got:\n{}", a.source);
    }

    // ─── SIR regex builtin → sir-runtime-regex (Q8b) ────────────────────────

    #[test]
    fn regex_builtin_emits_regex_runtime_ts() {
        let rx = Expr::BuiltinCall {
            name: "regex".into(),
            args: vec![
                Expr::StrLit { value: "ab+c".into(), span: s() },
                Expr::StrLit { value: "i".into(), span: s() },
            ],
            effects: EffectSet::PURE,
            span: s(),
        };
        let m = module_with_main_body(vec![], rx, &[Feature::Strings]);
        let a = compile(&m).expect("compile");
        let src = &a.source;
        assert!(
            src.contains(r#"import * as __SirRegex from "@coding-adventures/sir-runtime-regex";"#),
            "got:\n{}",
            src
        );
        assert!(src.contains(r#"__SirRegex.compile("ab+c", "i")"#), "got:\n{}", src);
    }

    #[test]
    fn non_regex_module_omits_regex_import_ts() {
        let module = twig_to_semantic_ir::compile_source("(print (+ 1 2))", "demo").expect("lower");
        let a = compile(&module).expect("compile");
        assert!(!a.source.contains("sir-runtime-regex"), "got:\n{}", a.source);
    }

    // ─── SIR backtick builtin → sir-runtime-shell (Q8c) ─────────────────────

    #[test]
    fn backtick_builtin_emits_shell_runtime_ts() {
        let bt = Expr::BuiltinCall {
            name: "backtick".into(),
            args: vec![Expr::StrLit { value: "echo hi".into(), span: s() }],
            effects: EffectSet::PURE,
            span: s(),
        };
        let m = module_with_main_body(vec![], bt, &[Feature::Strings]);
        let a = compile(&m).expect("compile");
        let src = &a.source;
        assert!(
            src.contains(r#"import * as __SirShell from "@coding-adventures/sir-runtime-shell";"#),
            "got:\n{}",
            src
        );
        assert!(src.contains(r#"__SirShell.backtick("echo hi")"#), "got:\n{}", src);
    }

    #[test]
    fn non_shell_module_omits_shell_import_ts() {
        let module = twig_to_semantic_ir::compile_source("(print (+ 1 2))", "demo").expect("lower");
        let a = compile(&module).expect("compile");
        assert!(!a.source.contains("sir-runtime-shell"), "got:\n{}", a.source);
    }

    // ─── boolean / unary operator builtins (Q8d audit) ──────────────────────

    fn bc(name: &str, args: Vec<Expr>) -> Expr {
        Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
    }

    #[test]
    fn and_or_not_neg_builtins_lower_natively_ts() {
        let and = bc("and", vec![Expr::BoolLit { value: true, span: s() }, Expr::IntLit { value: 1, span: s() }]);
        let a = compile(&module_with_main_body(vec![], and, &[])).expect("compile");
        assert!(a.source.contains("__Sir.truthy(__l) ? (1) : __l"), "and: got:\n{}", a.source);

        let or = bc("or", vec![Expr::BoolLit { value: false, span: s() }, Expr::IntLit { value: 2, span: s() }]);
        let a = compile(&module_with_main_body(vec![], or, &[])).expect("compile");
        assert!(a.source.contains("__Sir.truthy(__l) ? __l : (2)"), "or: got:\n{}", a.source);

        let not = bc("not", vec![Expr::BoolLit { value: true, span: s() }]);
        let a = compile(&module_with_main_body(vec![], not, &[])).expect("compile");
        assert!(a.source.contains("(!__Sir.truthy(true))"), "not: got:\n{}", a.source);

        let neg = bc("neg", vec![Expr::IntLit { value: 5, span: s() }]);
        let a = compile(&module_with_main_body(vec![], neg, &[])).expect("compile");
        assert!(a.source.contains("(-(5))"), "neg: got:\n{}", a.source);
        assert!(!a.source.contains("__Sir.callBuiltin(\"neg\""), "got:\n{}", a.source);
    }

    #[test]
    fn lambda_builtin_lowers_to_inner_closure_ts() {
        // Ruby `lambda { … }` / `->{…}` reach the backend as
        // `BuiltinCall("lambda", [MakeClosure])`.  The lambda *is* its closure,
        // so it must emit the inner `MakeClosure` (→ `new __Sir.Closure`)
        // directly, never route through the eager dispatch table.
        let mc = Expr::MakeClosure { fn_name: "main".into(), captures: vec![], span: s() };
        let lam = bc("lambda", vec![mc]);
        let a = compile(&module_with_main_body(vec![], lam, &[Feature::Closures]))
            .expect("compile");
        assert!(a.source.contains("new __Sir.Closure("), "got:\n{}", a.source);
        assert!(!a.source.contains("__Sir.callBuiltin(\"lambda\""), "got:\n{}", a.source);
    }

    #[test]
    fn range_builtin_lowers_to_runtime_and_imports_ts() {
        // Ruby `a..b` / `a...b` reach the backend as
        // `BuiltinCall("range", [start, stop, exclusive])`.  It must lower to the
        // `__SirRange.range(...)` constructor, gate in the range-runtime import,
        // and never route through the eager dispatch table.
        let rng = bc(
            "range",
            vec![
                Expr::IntLit { value: 1, span: s() },
                Expr::IntLit { value: 5, span: s() },
                Expr::BoolLit { value: false, span: s() },
            ],
        );
        let a = compile(&module_with_main_body(vec![], rng, &[])).expect("compile");
        assert!(a.source.contains("__SirRange.range(1, 5, false)"), "got:\n{}", a.source);
        assert!(
            a.source.contains("from \"@coding-adventures/sir-runtime-range\""),
            "missing range import; got:\n{}",
            a.source
        );
        assert!(!a.source.contains("__Sir.callBuiltin(\"range\""), "got:\n{}", a.source);
    }

    #[test]
    fn splat_in_seq_literal_emits_native_spread_ts() {
        use semantic_ir::{Scope, Stmt};
        // Ruby `mid = [9]; [1, *mid, 3]` → a `SeqLit` whose middle element is
        // `BuiltinCall("splat", [mid])`.  TypeScript splices it natively as
        // `...mid` inside the array literal.
        let bind = Stmt::LetBinding {
            name: "mid".into(),
            sir_type: None,
            value: Expr::SeqLit { items: vec![Expr::IntLit { value: 9, span: s() }], span: s() },
            span: s(),
        };
        let mid = Expr::VarRef { name: "mid".into(), scope: Scope::Local, span: s() };
        let seq = Expr::SeqLit {
            items: vec![
                Expr::IntLit { value: 1, span: s() },
                bc("splat", vec![mid]),
                Expr::IntLit { value: 3, span: s() },
            ],
            span: s(),
        };
        let a = compile(&module_with_main_body(vec![bind], seq, &[Feature::Sequences]))
            .expect("compile");
        assert!(a.source.contains("[1, ...mid, 3]"), "got:\n{}", a.source);
        assert!(!a.source.contains("__Sir.callBuiltin(\"splat\""), "got:\n{}", a.source);
    }

    #[test]
    fn splat_call_arg_emits_native_spread_ts() {
        use semantic_ir::{Scope, Stmt};
        // Ruby `a = [1]; main(*a)` → `DirectCall` with arg
        // `BuiltinCall("splat", [a])` → native `...a` argument spread.
        let bind = Stmt::LetBinding {
            name: "a".into(),
            sir_type: None,
            value: Expr::SeqLit { items: vec![Expr::IntLit { value: 1, span: s() }], span: s() },
            span: s(),
        };
        let a_arg = bc("splat", vec![Expr::VarRef { name: "a".into(), scope: Scope::Local, span: s() }]);
        let call = Expr::DirectCall {
            fn_name: "main".into(),
            args: vec![a_arg],
            effects: EffectSet::PURE,
            span: s(),
        };
        let a = compile(&module_with_main_body(vec![bind], call, &[Feature::Sequences]))
            .expect("compile");
        assert!(a.source.contains("(...a)"), "got:\n{}", a.source);
        assert!(!a.source.contains("__Sir.callBuiltin(\"splat\""), "got:\n{}", a.source);
    }

    #[test]
    fn double_splat_call_arg_merges_via_runtime_helper_ts() {
        use semantic_ir::{Scope, Stmt};
        // TS has no keyword-argument call form, so call-position `**h` (Q10f) is
        // collapsed into ONE trailing argument built by the runtime merge
        // helper — `__Sir.doubleSplatMerge(h)` — the conventional JS
        // options-object convention. It is NOT deferred to the eager dispatch
        // and NOT mistaken for a positional spread.
        let bind = Stmt::LetBinding {
            name: "h".into(),
            sir_type: None,
            value: Expr::MapLit { entries: vec![], span: s() },
            span: s(),
        };
        let h_arg = bc("double_splat", vec![Expr::VarRef { name: "h".into(), scope: Scope::Local, span: s() }]);
        let call = Expr::DirectCall {
            fn_name: "main".into(),
            args: vec![h_arg],
            effects: EffectSet::PURE,
            span: s(),
        };
        let a = compile(&module_with_main_body(vec![bind], call, &[Feature::Maps]))
            .expect("compile");
        assert!(a.source.contains("__Sir.doubleSplatMerge(h)"), "got:\n{}", a.source);
        // No fall-through to the eager unknown-builtin dispatch.
        assert!(!a.source.contains("__Sir.callBuiltin(\"double_splat\""), "got:\n{}", a.source);
        // Not mistakenly emitted as a positional JS spread.
        assert!(!a.source.contains("(...h)"), "got:\n{}", a.source);
    }

    #[test]
    fn double_splat_contiguous_run_collapses_to_single_merge_ts() {
        use semantic_ir::{Scope, Stmt};
        // `f(a, **h1, **h2)` → the contiguous `**` run collapses into ONE
        // merged trailing arg, with the leading positional preserved:
        // `f(a, __Sir.doubleSplatMerge(h1, h2))`.
        let bind_a = Stmt::LetBinding {
            name: "a".into(), sir_type: None,
            value: Expr::IntLit { value: 1, span: s() }, span: s(),
        };
        let bind_h1 = Stmt::LetBinding {
            name: "h1".into(), sir_type: None,
            value: Expr::MapLit { entries: vec![], span: s() }, span: s(),
        };
        let bind_h2 = Stmt::LetBinding {
            name: "h2".into(), sir_type: None,
            value: Expr::MapLit { entries: vec![], span: s() }, span: s(),
        };
        let pos = Expr::VarRef { name: "a".into(), scope: Scope::Local, span: s() };
        let h1 = bc("double_splat", vec![Expr::VarRef { name: "h1".into(), scope: Scope::Local, span: s() }]);
        let h2 = bc("double_splat", vec![Expr::VarRef { name: "h2".into(), scope: Scope::Local, span: s() }]);
        let call = Expr::DirectCall {
            fn_name: "main".into(),
            args: vec![pos, h1, h2],
            effects: EffectSet::PURE,
            span: s(),
        };
        let a = compile(&module_with_main_body(vec![bind_a, bind_h1, bind_h2], call, &[Feature::Maps]))
            .expect("compile");
        assert!(a.source.contains("main(a, __Sir.doubleSplatMerge(h1, h2))"), "got:\n{}", a.source);
    }

    #[test]
    fn defined_local_var_emits_static_description_ts() {
        use semantic_ir::{Scope, Stmt};
        // `x = 1; defined?(x)` → the constant string "local-variable", never the
        // dispatch fallthrough.
        let bind = Stmt::LetBinding {
            name: "x".into(),
            sir_type: None,
            value: Expr::IntLit { value: 1, span: s() },
            span: s(),
        };
        let d = bc("defined?", vec![Expr::VarRef { name: "x".into(), scope: Scope::Local, span: s() }]);
        let a = compile(&module_with_main_body(vec![bind], d, &[])).expect("compile");
        assert!(a.source.contains("\"local-variable\""), "got:\n{}", a.source);
        assert!(!a.source.contains("__Sir.callBuiltin(\"defined?\""), "got:\n{}", a.source);
    }

    #[test]
    fn defined_does_not_evaluate_operand_ts() {
        // Ruby contract: `defined?` must NOT evaluate its operand.  `defined?(99)`
        // emits the constant "expression"; the operand `99` must NOT appear.
        let d = bc("defined?", vec![Expr::IntLit { value: 99, span: s() }]);
        let a = compile(&module_with_main_body(vec![], d, &[])).expect("compile");
        assert!(a.source.contains("\"expression\""), "got:\n{}", a.source);
        assert!(!a.source.contains("99"), "operand was evaluated; got:\n{}", a.source);
        assert!(!a.source.contains("__Sir.callBuiltin(\"defined?\""), "got:\n{}", a.source);
    }

    #[test]
    fn defined_method_call_operand_emits_method_ts() {
        
        // Q10h: `defined?(recv.meth)` — the `__method__` dispatch envelope —
        // reports the constant "method", not the generic "expression". The
        // receiver and method name are never rendered (non-evaluation contract).
        let recv = Expr::IntLit { value: 5, span: s() };
        let meth = bc("__method__", vec![recv, Expr::StrLit { value: "foo".into(), span: s() }]);
        let d = bc("defined?", vec![meth]);
        let a = compile(&module_with_main_body(vec![], d, &[Feature::Strings])).expect("compile");
        assert!(a.source.contains("\"method\""), "got:\n{}", a.source);
        assert!(!a.source.contains("__method__"), "operand was rendered; got:\n{}", a.source);
        assert!(!a.source.contains("\"foo\""), "operand was rendered; got:\n{}", a.source);
    }

    // ─── P2b default parameters → TS-native defaults ────────────────────────

    /// Build a module exercising default parameters end-to-end:
    ///
    /// ```text
    /// def f(a, b = a + 1)   # b's default REFERENCES the earlier param a
    ///   b
    /// end
    /// def main
    ///   f(5)        # omits b → native default fills it (b = a + 1 = 6)
    ///   f(5, 10)    # passes b explicitly
    /// end
    /// ```
    ///
    /// The default expression is the canonical "references an earlier param"
    /// form: `BuiltinCall("+", [VarRef{a, Param}, IntLit 1])`, which the TS
    /// backend renders through its ordinary builtin lowering as
    /// `__Sir.add(a, 1)`.
    fn default_params_module() -> Module {
        use semantic_ir::{Param, ParamKind, Scope};
        let default_b = Expr::BuiltinCall {
            name: "+".into(),
            args: vec![
                Expr::VarRef { name: "a".into(), scope: Scope::Param, span: s() },
                Expr::IntLit { value: 1, span: s() },
            ],
            effects: EffectSet::PURE,
            span: s(),
        };
        let f = Function {
            name: "f".into(),
            params: vec![
                Param { name: "a".into(), sir_type: None, kind: ParamKind::Required, default: None, span: s() },
                Param {
                    name: "b".into(),
                    sir_type: None,
                    kind: ParamKind::Required,
                    default: Some(Box::new(default_b)),
                    span: s(),
                },
            ],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::VarRef { name: "b".into(), scope: Scope::Param, span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        // main calls f(5) (b omitted) and f(5, 10) (b supplied).
        let call_one = Expr::DirectCall {
            fn_name: "f".into(),
            args: vec![Expr::IntLit { value: 5, span: s() }],
            effects: EffectSet::PURE,
            span: s(),
        };
        let call_two = Expr::DirectCall {
            fn_name: "f".into(),
            args: vec![
                Expr::IntLit { value: 5, span: s() },
                Expr::IntLit { value: 10, span: s() },
            ],
            effects: EffectSet::PURE,
            span: s(),
        };
        let main = Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![semantic_ir::Stmt::ExprStmt { expr: call_one, span: s() }],
                value: call_two,
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        Module {
            name: "demo".into(),
            manifest: FeatureManifest::from_features(&[
                Feature::DefaultParams,
                Feature::DynamicTyping,
            ]),
            imports: vec![],
            exports: vec![],
            functions: vec![f, main],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("test")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        }
    }

    #[test]
    fn default_param_referencing_earlier_param_emits_native_ts_default() {
        // Capability: the module declares DefaultParams, so it must pass the
        // backend's feature check rather than be rejected.
        let m = default_params_module();
        let a = compile(&m).expect("default-using module must compile");
        let src = &a.source;
        // The param `b` carries a TS-native default in the param position,
        // and that default references the EARLIER param `a` by name —
        // exactly the call-time / param-scope semantics SIR specifies.
        assert!(
            src.contains("function f(a: __Sir.Val, b: __Sir.Val = __Sir.add(a, 1)): __Sir.Val"),
            "expected a native default referencing the earlier param; got:\n{}",
            src
        );
        // The param `a` (no default) is unchanged — no `= ` after it.
        assert!(
            src.contains("function f(a: __Sir.Val, b"),
            "param without a default must be unchanged; got:\n{}",
            src
        );
    }

    #[test]
    fn default_param_partial_and_full_calls_emit_present_args_only() {
        // The 1-arg call `f(5)` omits the trailing defaulted `b`; TS native
        // defaults fill it.  No padding, no placeholder for the omitted arg.
        // The 2-arg call `f(5, 10)` passes both.
        let m = default_params_module();
        let a = compile(&m).expect("compile");
        let src = &a.source;
        assert!(
            src.contains("f(5)"),
            "partial call must emit one arg (omitting the defaulted trailing param); got:\n{}",
            src
        );
        assert!(
            src.contains("f(5, 10)"),
            "full call must emit both args; got:\n{}",
            src
        );
    }

    #[test]
    fn no_range_import_when_unused_ts() {
        // A module that never builds a range must not gain the range dependency.
        let a = compile(&module_with_main_body(
            vec![],
            Expr::IntLit { value: 7, span: s() },
            &[],
        ))
        .expect("compile");
        assert!(
            !a.source.contains("@coding-adventures/sir-runtime-range"),
            "unexpected range import; got:\n{}",
            a.source
        );
    }

    // ── KW3: keyword parameters & arguments (options-object lowering) ──────

    /// A callee with one required and one optional keyword param, plus a
    /// leading positional, and two calls: one that omits the optional and one
    /// that supplies it.  Mirrors the spec's canonical KW3 program shape.
    fn keyword_module() -> Module {
        use semantic_ir::{Param, ParamKind, Scope};

        fn kw(name: &str, default: Option<Expr>) -> Param {
            Param {
                name: name.into(),
                sir_type: None,
                kind: ParamKind::Keyword,
                default: default.map(Box::new),
                span: s(),
            }
        }
        let prefix = Param {
            name: "prefix".into(),
            sir_type: None,
            kind: ParamKind::Required,
            default: None,
            span: s(),
        };
        // greet body just returns the required keyword `x` (value irrelevant to
        // the shape assertions; the destructure prologue is what we inspect).
        let f = Function {
            name: "f".into(),
            params: vec![
                prefix,
                kw("x", None),                                            // required keyword
                kw("y", Some(Expr::IntLit { value: 1, span: s() })),      // optional keyword
            ],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::VarRef { name: "x".into(), scope: Scope::Param, span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };

        // f("p", x: 2)          — optional `y` omitted → one-entry object.
        let call_omit = Expr::DirectCall {
            fn_name: "f".into(),
            args: vec![
                Expr::StrLit { value: "p".into(), span: s() },
                Expr::KeywordArg {
                    name: "x".into(),
                    value: Box::new(Expr::IntLit { value: 2, span: s() }),
                    span: s(),
                },
            ],
            effects: EffectSet::PURE,
            span: s(),
        };
        // f("p", x: 2, y: 3)    — both keywords supplied → two-entry object.
        let call_full = Expr::DirectCall {
            fn_name: "f".into(),
            args: vec![
                Expr::StrLit { value: "p".into(), span: s() },
                Expr::KeywordArg {
                    name: "x".into(),
                    value: Box::new(Expr::IntLit { value: 2, span: s() }),
                    span: s(),
                },
                Expr::KeywordArg {
                    name: "y".into(),
                    value: Box::new(Expr::IntLit { value: 3, span: s() }),
                    span: s(),
                },
            ],
            effects: EffectSet::PURE,
            span: s(),
        };
        let main = Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![semantic_ir::Stmt::ExprStmt { expr: call_omit, span: s() }],
                value: call_full,
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        Module {
            name: "kw".into(),
            manifest: FeatureManifest::from_features(&[
                Feature::KeywordParams,
                Feature::Strings,
                Feature::DynamicTyping,
            ]),
            imports: vec![],
            exports: vec![],
            functions: vec![f, main],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("test")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        }
    }

    #[test]
    fn keyword_params_fold_into_trailing_options_object_with_destructure() {
        let m = keyword_module();
        let a = compile(&m).expect("keyword module must compile (feature accepted)");
        let src = &a.source;
        // The positional `prefix` stays inline; both keyword params collapse
        // into ONE trailing `__kw` object parameter.
        assert!(
            src.contains("function f(prefix: __Sir.Val, __kw: __Sir.Val): __Sir.Val {"),
            "keyword params must fold into a single trailing __kw object; got:\n{src}"
        );
        // Prologue destructure: required keyword bare, optional carries default.
        assert!(
            src.contains(
                "const { x, y = 1 } = (__kw ?? {}) as { [k: string]: __Sir.Val };"
            ),
            "prologue must destructure __kw, bare required + defaulted optional; got:\n{src}"
        );
    }

    #[test]
    fn keyword_call_collapses_args_into_single_object_literal() {
        let m = keyword_module();
        let a = compile(&m).expect("compile");
        let src = &a.source;
        // Omitting the optional → one-entry object; positional stays outside it.
        assert!(
            src.contains("f(\"p\", { x: 2 })"),
            "omitted-optional call collapses keyword args to one object; got:\n{src}"
        );
        // Supplying both → two-entry object.
        assert!(
            src.contains("f(\"p\", { x: 2, y: 3 })"),
            "full call collapses both keyword args into one object; got:\n{src}"
        );
    }

    #[test]
    fn no_keyword_args_emits_no_trailing_object() {
        use semantic_ir::{Param, ParamKind, Scope};
        // A callee with a keyword param, but a caller in an IndirectCall-free
        // world that supplies only positionals for a purely-positional callee:
        // here we build a callee with NO keyword params and confirm the call
        // path is byte-for-byte the ordinary positional form (no `{ … }`).
        let g = Function {
            name: "g".into(),
            params: vec![Param {
                name: "a".into(),
                sir_type: None,
                kind: ParamKind::Required,
                default: None,
                span: s(),
            }],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::VarRef { name: "a".into(), scope: Scope::Param, span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        let call = Expr::DirectCall {
            fn_name: "g".into(),
            args: vec![Expr::IntLit { value: 7, span: s() }],
            effects: EffectSet::PURE,
            span: s(),
        };
        let mut m = module_with_main_body(vec![], call, &[Feature::DynamicTyping]);
        m.functions.insert(0, g);
        let a = compile(&m).expect("compile");
        let src = &a.source;
        assert!(
            src.contains("function g(a: __Sir.Val): __Sir.Val {"),
            "no keyword params → no __kw parameter; got:\n{src}"
        );
        assert!(
            src.contains("g(7)") && !src.contains("g(7, {"),
            "a call with no keyword args emits no trailing object; got:\n{src}"
        );
    }
}
