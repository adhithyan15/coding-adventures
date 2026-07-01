//! End-to-end integration test for KW3: SIR keyword params/args → TypeScript
//! → runnable JavaScript → `node`.
//!
//! The unit tests in `src/lib.rs` prove the emitted *shape* (the trailing
//! `__kw` options-object parameter, the destructure prologue, the collapsed
//! call-site object literal).  This test proves the emitted *behaviour*: a
//! keyword-using program actually produces the right stdout when executed.
//!
//! ## Why a TS→JS shim is needed here (and not in the JS backend)
//!
//! The JavaScript backend emits self-contained `.js` that `node` runs as-is.
//! The TypeScript backend, by contrast, emits genuine TypeScript — `: __Sir.Val`
//! annotations, an `as { … }` cast, and an `import * as __Sir from
//! "@coding-adventures/sir-runtime-core"` — none of which bare `node` accepts.
//! Rather than pull in a TypeScript toolchain (tsc/ts-node) as a test
//! dependency, we perform a *minimal, mechanical* transform that is faithful
//! for the small runtime surface these keyword programs touch:
//!
//!   1. strip the type annotations the emitter adds (`: __Sir.Val[]`,
//!      `: __Sir.Val`, and the `as { [k: string]: __Sir.Val }` cast), and
//!   2. replace the runtime import with a tiny inline `__Sir` stub providing
//!      exactly the helpers the emitted code calls (`print`, `toDisplay`).
//!
//! The transform touches ONLY the type/preamble syntax — the actual emitted
//! logic (the `__kw` destructure, the options-object call sites) runs
//! unchanged, so what executes under `node` is precisely what the backend
//! produced.  Node is optional: when it is absent the test degrades to the
//! shape assertions and skips execution (mirroring the JS harness).

use std::path::PathBuf;
use std::process::Command;

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Param, ParamKind,
    Scope, Span, Stmt,
};
use semantic_ir_to_typescript::compile;

fn sp() -> Span {
    Span::synthetic()
}

/// Is a working `node` on PATH?
fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// A minimal `__Sir` runtime stub covering exactly the helpers the emitted
/// keyword programs call.  `toDisplay` renders a value the way the real
/// runtime does for the cases these tests exercise (strings verbatim); `print`
/// writes one line to stdout.
const SIR_STUB: &str = r#"const __Sir = {
  toDisplay: (v) => (v === null ? "nil" : String(v)),
  print: (v) => { console.log(__Sir.toDisplay(v)); return null; },
};
"#;

/// Turn emitted TypeScript into runnable JavaScript for the node execution
/// proof.  See the module doc-comment for why this is faithful.  Only
/// type-syntax and the runtime import are rewritten.
fn ts_to_runnable_js(ts: &str) -> String {
    let mut js = ts.to_string();
    // 1. Replace the runtime import line with the inline stub.
    js = js.replace(
        "import * as __Sir from \"@coding-adventures/sir-runtime-core\";\n",
        SIR_STUB,
    );
    // 2. Drop the `as { … }` cast the `__kw` destructure carries.  (Fixed
    //    text; the emitter always produces exactly this annotation.)
    js = js.replace(" as { [k: string]: __Sir.Val }", "");
    // 3. Strip type annotations.  `[]` variant first so `: __Sir.Val[]` is not
    //    left with a dangling `[]`.
    js = js.replace(": __Sir.Val[]", "");
    js = js.replace(": __Sir.Val", "");
    js
}

/// Compile a hand-built keyword module, transform to JS, run under node, and
/// return trimmed stdout.  `None` when node is unavailable.
fn run_module(module: &Module, tag: &str) -> Option<String> {
    let artifact = compile(module).expect("compile to typescript");
    if !node_available() {
        eprintln!("note: `node` unavailable — skipping execution for `{tag}`");
        return None;
    }
    let js = ts_to_runnable_js(&artifact.source);
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_ts_kw_{}_{}.js", tag, std::process::id()));
    std::fs::write(&path, &js).expect("write temp js");
    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);
    assert!(
        output.status.success(),
        "node exited non-zero for `{tag}`:\nstdout: {}\nstderr: {}\nsource:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        js,
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    Some(stdout.trim_end_matches(['\n', '\r']).to_string())
}

// ── SIR builder helpers ────────────────────────────────────────────────

fn kw_param(name: &str, default: Option<Expr>) -> Param {
    Param {
        name: name.into(),
        sir_type: None,
        kind: ParamKind::Keyword,
        default: default.map(Box::new),
        span: sp(),
    }
}

fn str_lit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: sp() }
}

fn param_ref(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Param, span: sp() }
}

fn kw_arg(name: &str, value: Expr) -> Expr {
    Expr::KeywordArg { name: name.into(), value: Box::new(value), span: sp() }
}

fn print(expr: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "print".into(),
            args: vec![expr],
            effects: EffectSet::PURE,
            span: sp(),
        },
        span: sp(),
    }
}

/// Build the canonical KW3 program (from the spec's verification section):
///
///   def greet(greeting:, name: "world")
///     "#{greeting}, #{name}"
///   end
///   print(greet(greeting: "hi"))            # omits the optional → "hi, world"
///   print(greet(greeting: "hi", name: "sir"))   # supplies it     → "hi, sir"
fn greet_module() -> Module {
    // Body: greeting + ", " + name, via StrConcat (interpolation lowering).
    let body_val = Expr::StrConcat {
        parts: vec![param_ref("greeting"), str_lit(", "), param_ref("name")],
        span: sp(),
    };
    let greet = Function {
        name: "greet".into(),
        params: vec![
            kw_param("greeting", None),                 // required keyword
            kw_param("name", Some(str_lit("world"))),   // optional keyword
        ],
        return_type: None,
        captures: vec![],
        body: Block { stmts: vec![], value: body_val, span: sp() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sp(),
    };

    let call_omit = Expr::DirectCall {
        fn_name: "greet".into(),
        args: vec![kw_arg("greeting", str_lit("hi"))],
        effects: EffectSet::PURE,
        span: sp(),
    };
    let call_full = Expr::DirectCall {
        fn_name: "greet".into(),
        args: vec![
            kw_arg("greeting", str_lit("hi")),
            kw_arg("name", str_lit("sir")),
        ],
        effects: EffectSet::PURE,
        span: sp(),
    };

    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![print(call_omit), print(call_full)],
            value: Expr::NilLit { span: sp() },
            span: sp(),
        },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sp(),
    };

    Module {
        name: "kwgreet".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::KeywordParams,
            Feature::StringInterpolation,
            Feature::Strings,
            Feature::DynamicTyping,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![greet, main],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("handbuilt")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: sp(),
    }
}

#[test]
fn keyword_call_uses_default_when_optional_omitted() {
    let module = greet_module();

    // Shape check (runs even without node): the destructure prologue and the
    // two collapsed call-site object literals.
    let artifact = compile(&module).expect("compile");
    let src = &artifact.source;
    assert!(
        src.contains("function greet(__kw: __Sir.Val): __Sir.Val {"),
        "keyword params must fold into a single trailing __kw object; got:\n{src}"
    );
    assert!(
        src.contains(
            "const { greeting, name = \"world\" } = (__kw ?? {}) as { [k: string]: __Sir.Val };"
        ),
        "required keyword is bare, optional carries its default; got:\n{src}"
    );
    assert!(
        src.contains("greet({ greeting: \"hi\" })"),
        "omitting the optional collapses to a one-entry object; got:\n{src}"
    );
    assert!(
        src.contains("greet({ greeting: \"hi\", name: \"sir\" })"),
        "supplying both keywords collapses to a two-entry object; got:\n{src}"
    );

    // Behaviour check (runs when node is present).
    if let Some(stdout) = run_module(&module, "greet") {
        assert_eq!(
            stdout, "hi, world\nhi, sir",
            "greet(greeting: \"hi\") must default name to \"world\"; \
             greet(greeting: \"hi\", name: \"sir\") must use \"sir\""
        );
    }
}

#[test]
fn positional_and_keyword_mix_binds_both() {
    // def label(prefix, tag: "-")
    //   "#{prefix}#{tag}"
    // end
    // print(label("A", tag: "!"))   → "A!"
    // print(label("B"))             → "B-"   (optional keyword defaulted)
    let prefix = Param {
        name: "prefix".into(),
        sir_type: None,
        kind: ParamKind::Required,
        default: None,
        span: sp(),
    };
    let tag = kw_param("tag", Some(str_lit("-")));
    let body_val = Expr::StrConcat {
        parts: vec![param_ref("prefix"), param_ref("tag")],
        span: sp(),
    };
    let label = Function {
        name: "label".into(),
        params: vec![prefix, tag],
        return_type: None,
        captures: vec![],
        body: Block { stmts: vec![], value: body_val, span: sp() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sp(),
    };

    let call_with_kw = Expr::DirectCall {
        fn_name: "label".into(),
        args: vec![str_lit("A"), kw_arg("tag", str_lit("!"))],
        effects: EffectSet::PURE,
        span: sp(),
    };
    let call_default = Expr::DirectCall {
        fn_name: "label".into(),
        args: vec![str_lit("B")],
        effects: EffectSet::PURE,
        span: sp(),
    };

    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![print(call_with_kw), print(call_default)],
            value: Expr::NilLit { span: sp() },
            span: sp(),
        },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sp(),
    };

    let module = Module {
        name: "kwmix".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::KeywordParams,
            Feature::StringInterpolation,
            Feature::Strings,
            Feature::DynamicTyping,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![label, main],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("handbuilt")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: sp(),
    };

    // Shape: the positional `prefix` stays inline; only `tag` folds into __kw.
    let artifact = compile(&module).expect("compile");
    let src = &artifact.source;
    assert!(
        src.contains("function label(prefix: __Sir.Val, __kw: __Sir.Val): __Sir.Val {"),
        "positional param inline, keyword param via trailing __kw; got:\n{src}"
    );
    assert!(
        src.contains("label(\"A\", { tag: \"!\" })"),
        "positional then keyword object; got:\n{src}"
    );
    assert!(
        src.contains("label(\"B\")"),
        "no keyword args → no trailing object; got:\n{src}"
    );

    if let Some(stdout) = run_module(&module, "mix") {
        assert_eq!(stdout, "A!\nB-", "keyword mix must bind positional and defaulted keyword");
    }
}
