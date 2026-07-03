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

/// A minimal `__SirExc` runtime stub for the E2 execution proof, mirroring the
/// real `sir-runtime-exceptions` package's logic: the built-in ancestry table,
/// a mutable live copy, `registerAncestry` that merges user edges, and a
/// `rescueMatches` that walks the merged chain.  We inline it (rather than
/// resolve the workspace package under bare `node`) for the same reason the
/// `__Sir` stub is inlined — see the module doc-comment.  Keeping the built-in
/// table and merge here means the proof genuinely exercises "user edge is
/// walked", not just "the emitted call is syntactically present".
const SIR_EXC_STUB: &str = r#"const __SirExc = (() => {
  const BUILTIN = {
    RuntimeError: "StandardError", ArgumentError: "StandardError",
    TypeError: "StandardError", NameError: "StandardError",
    NoMethodError: "NameError", IndexError: "StandardError",
    KeyError: "IndexError", RangeError: "StandardError",
    ZeroDivisionError: "StandardError", IOError: "StandardError",
    StopIteration: "StandardError", NotImplementedError: "StandardError",
    StandardError: "Exception",
  };
  const ANCESTRY = { ...BUILTIN };
  class SirError extends Error {
    constructor(sirClass, message) {
      super(message == null ? sirClass : String(message));
      this.sirClass = sirClass;
    }
  }
  const registerAncestry = (m) => { for (const k of Object.keys(m)) ANCESTRY[k] = m[k]; };
  const raiseError = (c, m) => { throw new SirError(c ?? "RuntimeError", m); };
  const classOfThrown = (e) => (e instanceof SirError ? e.sirClass : "StandardError");
  const isAncestorOrSelf = (actual, target) => {
    let cur = actual; const seen = new Set();
    while (cur !== undefined && !seen.has(cur)) {
      if (cur === target) return true;
      seen.add(cur); cur = ANCESTRY[cur];
    }
    return false;
  };
  const rescueMatches = (e, names) => {
    if (names.length === 0) return true;
    const actual = classOfThrown(e);
    return names.some((n) => n === "Exception" || isAncestorOrSelf(actual, n));
  };
  return { registerAncestry, raiseError, rescueMatches };
})();
"#;

/// A minimal `__SirOop` stub: the E2 programs only call `defineClass`, which
/// for exception purposes is a no-op (ancestry is threaded via `__SirExc`).
const SIR_OOP_STUB: &str = r#"const __SirOop = { defineClass: () => null };
"#;

/// Transform emitted TypeScript that imports the core + OOP + exceptions
/// runtimes into runnable JavaScript by swapping each import for its inline
/// stub and stripping type syntax.  Faithful for the E2 surface (see the
/// `__SirExc` stub doc).
fn ts_to_runnable_js_with_exceptions(ts: &str) -> String {
    let mut js = ts.to_string();
    js = js.replace(
        "import * as __Sir from \"@coding-adventures/sir-runtime-core\";\n",
        SIR_STUB,
    );
    js = js.replace(
        "import * as __SirOop from \"@coding-adventures/sir-runtime-oop\";\n",
        SIR_OOP_STUB,
    );
    js = js.replace(
        "import * as __SirExc from \"@coding-adventures/sir-runtime-exceptions\";\n",
        SIR_EXC_STUB,
    );
    js = js.replace(" as { [k: string]: __Sir.Val }", "");
    js = js.replace(": __Sir.Val[]", "");
    js = js.replace(": __Sir.Val", "");
    js
}

/// Run a compiled exception-using module under node, returning `(success,
/// stdout)`.  `None` when node is unavailable.  Unlike [`run_module`] this does
/// NOT assert a zero exit — the no-match proof expects a non-zero exit from an
/// unrescued re-throw.
fn run_exc_module(module: &Module, tag: &str) -> Option<(bool, String)> {
    let artifact = compile(module).expect("compile to typescript");
    if !node_available() {
        eprintln!("note: `node` unavailable — skipping execution for `{tag}`");
        return None;
    }
    let js = ts_to_runnable_js_with_exceptions(&artifact.source);
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_ts_exc_{}_{}.js", tag, std::process::id()));
    std::fs::write(&path, &js).expect("write temp js");
    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);
    let stdout = String::from_utf8_lossy(&output.stdout)
        .trim_end_matches(['\n', '\r'])
        .to_string();
    Some((output.status.success(), stdout))
}

fn sir_span() -> Span {
    Span::synthetic()
}

/// Build `class <name> < <superclass>; end; begin; raise <name>, "x"; rescue
/// <rescued> => e; print("caught"); end` as a hand-built SIR module.  Mirrors
/// what the Ruby frontend lowers, so the TS execution proof does not depend on
/// the Ruby crate.
fn exc_module(name: &str, superclass: &str, rescued: &str) -> Module {
    let classdef = Stmt::ClassDef {
        name: name.into(),
        superclass: Some(superclass.into()),
        body: vec![],
        span: sir_span(),
    };
    let raise = Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "raise".into(),
            args: vec![
                Expr::VarRef { name: name.into(), scope: Scope::Const, span: sir_span() },
                str_lit("x"),
            ],
            effects: EffectSet::PURE,
            span: sir_span(),
        },
        span: sir_span(),
    };
    let try_stmt = Stmt::TryCatch {
        body: vec![raise],
        rescues: vec![semantic_ir::RescueClause {
            exception_types: vec![rescued.into()],
            binding: Some("e".into()),
            body: vec![print(str_lit("caught"))],
            span: sir_span(),
        }],
        ensure_body: None,
        span: sir_span(),
    };
    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![classdef, try_stmt],
            value: Expr::NilLit { span: sir_span() },
            span: sir_span(),
        },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sir_span(),
    };
    Module {
        name: "excmod".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Exceptions,
            Feature::Classes,
            Feature::Constants,
            Feature::Strings,
            Feature::DynamicTyping,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![main],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("handbuilt")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: sir_span(),
    }
}

#[test]
fn e2_emits_register_ancestry_for_user_subclass_ts() {
    // The module has a `class MyErr < StandardError`, so a single program-init
    // `registerAncestry` call threads its edge before any code runs.
    let module = exc_module("MyErr", "StandardError", "StandardError");
    let artifact = compile(&module).expect("compile");
    let src = &artifact.source;
    assert!(
        src.contains("__SirExc.registerAncestry({\"MyErr\": \"StandardError\"});"),
        "expected user ancestry registration; got:\n{src}"
    );
    // Must precede `function main` so ancestry is known before any rescue.
    let reg = src.find("__SirExc.registerAncestry(").expect("reg present");
    let main = src.find("function main").expect("main present");
    assert!(reg < main, "registration must come before main; got:\n{src}");
}

#[test]
fn e2_no_register_ancestry_without_superclass_ts() {
    // A throwing module whose only class has no superclass emits no empty,
    // meaningless registration.
    let classdef = Stmt::ClassDef {
        name: "Foo".into(),
        superclass: None,
        body: vec![],
        span: sir_span(),
    };
    let raise = Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "raise".into(),
            args: vec![
                Expr::VarRef { name: "RuntimeError".into(), scope: Scope::Const, span: sir_span() },
                str_lit("boom"),
            ],
            effects: EffectSet::PURE,
            span: sir_span(),
        },
        span: sir_span(),
    };
    let try_stmt = Stmt::TryCatch {
        body: vec![raise],
        rescues: vec![semantic_ir::RescueClause {
            exception_types: vec!["RuntimeError".into()],
            binding: None,
            body: vec![print(str_lit("x"))],
            span: sir_span(),
        }],
        ensure_body: None,
        span: sir_span(),
    };
    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![classdef, try_stmt],
            value: Expr::NilLit { span: sir_span() },
            span: sir_span(),
        },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sir_span(),
    };
    let module = Module {
        name: "excmod".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Exceptions,
            Feature::Classes,
            Feature::Constants,
            Feature::Strings,
            Feature::DynamicTyping,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![main],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("handbuilt")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: sir_span(),
    };
    let artifact = compile(&module).expect("compile");
    assert!(
        !artifact.source.contains("__SirExc.registerAncestry("),
        "should not register ancestry with no superclass edge; got:\n{}",
        artifact.source
    );
}

#[test]
fn e2_user_subclass_rescued_by_ancestor_executes_ts() {
    // Execution proof: `raise MyErr` (a user `StandardError` subclass) IS caught
    // by `rescue StandardError` under node — the registered edge is walked.
    let module = exc_module("MyErr", "StandardError", "StandardError");
    if let Some((ok, stdout)) = run_exc_module(&module, "match") {
        assert!(ok, "matched program should exit zero; stdout={stdout:?}");
        assert_eq!(stdout, "caught", "user subclass must be rescued by its ancestor");
    }
}

#[test]
fn e2_unrelated_user_class_not_rescued_executes_ts() {
    // Dual: `Other < RuntimeError` raised, `rescue TypeError` does NOT catch it,
    // so the exception propagates and node exits non-zero (nothing printed).
    let module = exc_module("Other", "RuntimeError", "TypeError");
    if let Some((ok, stdout)) = run_exc_module(&module, "nomatch") {
        assert!(!ok, "unmatched program must propagate (exit non-zero)");
        assert_ne!(stdout, "caught", "unrelated user class must not be rescued");
    }
}

// ── O1: OOP method-table execution proof ─────────────────────────────────────

/// A faithful inline `__SirOop` stub for the O1 execution proof, mirroring the
/// real `@coding-adventures/sir-runtime-oop` package's method-table logic:
/// `defMethod` registers a `(class, method)` closure, `callNew` allocates an
/// instance and runs `initialize`, and `callMethod` dispatches a user method by
/// explicit `Map` lookup (never reflection).  We inline it — rather than resolve
/// the workspace package under bare `node` — for the same reason the other stubs
/// are inlined (see the module doc-comment).  Keeping the real dispatch logic
/// here means the proof genuinely exercises "the emitted `defMethod`/`callNew`/
/// `callMethod` calls drive the method table", not just that they are present.
const SIR_OOP_DISPATCH_STUB: &str = r#"const __SirOop = (() => {
  const SEP = "\x00";
  const key = (c, m) => c + SEP + m;
  const supers = new Map();
  const instanceMethods = new Map();
  const selfStack = [];
  class SirInstance { constructor(c) { this.sirClass = c; this.ivars = new Map(); } }
  const superclassOf = (n) => (supers.has(n) ? supers.get(n) : null);
  const defineClass = (n, s) => { supers.set(n, s ?? null); };
  const defMethod = (c, m, fn) => { instanceMethods.set(key(c, m), fn); };
  const resolve = (c, m) => {
    let cur = c; const seen = new Set();
    while (cur !== null && !seen.has(cur)) {
      const fn = instanceMethods.get(key(cur, m));
      if (fn !== undefined) return fn;
      seen.add(cur); cur = superclassOf(cur);
    }
    return null;
  };
  const callNew = (c, ...args) => {
    const obj = new SirInstance(c);
    selfStack.push(obj);
    try { const init = resolve(c, "initialize"); if (init !== null) __Sir.apply(init, args); }
    finally { selfStack.pop(); }
    return obj;
  };
  const callMethod = (recv, name, ...args) => {
    if (recv instanceof SirInstance) {
      const fn = resolve(recv.sirClass, name);
      if (fn !== null) {
        selfStack.push(recv);
        try { return __Sir.apply(fn, args); } finally { selfStack.pop(); }
      }
    }
    return null;
  };
  return { defineClass, defMethod, callNew, callMethod };
})();
"#;

/// A `__Sir` stub carrying a `Closure` + `apply` (the emitted `MakeClosure`
/// renders `new __Sir.Closure(...)`, and the OOP stub applies method closures
/// through `__Sir.apply`), plus `print`/`toDisplay`.
const SIR_CLOSURE_STUB: &str = r#"const __Sir = {
  Closure: class { constructor(fn) { this.fn = fn; } },
  apply: (c, args) => c.fn(...args),
  toDisplay: (v) => (v === null ? "nil" : String(v)),
  print: (v) => { console.log(__Sir.toDisplay(v)); return null; },
};
"#;

fn ts_to_runnable_js_oop(ts: &str) -> String {
    let mut js = ts.to_string();
    js = js.replace(
        "import * as __Sir from \"@coding-adventures/sir-runtime-core\";\n",
        SIR_CLOSURE_STUB,
    );
    js = js.replace(
        "import * as __SirOop from \"@coding-adventures/sir-runtime-oop\";\n",
        SIR_OOP_DISPATCH_STUB,
    );
    js = js.replace(" as { [k: string]: __Sir.Val }", "");
    js = js.replace(": __Sir.Val[]", "");
    js = js.replace(": __Sir.Val", "");
    js
}

#[test]
fn end_to_end_oop_new_and_dispatch_executes_ts() {
    // O1 execution proof (hand-built SIR — the frontend does not emit these
    // builtins until O2).  Model `Dog.new.speak`:
    //   • a hoisted `Dog_speak` returning "Rex says woof",
    //   • `__def_method__("Dog", "speak", MakeClosure(Dog_speak))`,
    //   • `d = __new__("Dog")`,
    //   • `print(__method__(d, "speak"))`.
    // Running under node with the faithful `__SirOop` dispatch stub proves the
    // emitted `defMethod` → `callNew` → `callMethod` chain executes.
    let speak_fn = Function {
        name: "Dog_speak".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![],
            value: str_lit("Rex says woof"),
            span: sir_span(),
        },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sir_span(),
    };
    let def_method = Expr::BuiltinCall {
        name: "__def_method__".into(),
        args: vec![
            str_lit("Dog"),
            str_lit("speak"),
            Expr::MakeClosure { fn_name: "Dog_speak".into(), captures: vec![], span: sir_span() },
        ],
        effects: EffectSet::PURE,
        span: sir_span(),
    };
    let new_dog = Expr::BuiltinCall {
        name: "__new__".into(),
        args: vec![str_lit("Dog")],
        effects: EffectSet::PURE,
        span: sir_span(),
    };
    let dispatch = Expr::BuiltinCall {
        name: "__method__".into(),
        args: vec![
            Expr::VarRef { name: "d".into(), scope: Scope::Local, span: sir_span() },
            str_lit("speak"),
        ],
        effects: EffectSet::PURE,
        span: sir_span(),
    };
    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![
                Stmt::ExprStmt { expr: def_method, span: sir_span() },
                Stmt::LetBinding {
                    name: "d".into(),
                    sir_type: None,
                    value: new_dog,
                    span: sir_span(),
                },
                print(dispatch),
            ],
            value: Expr::NilLit { span: sir_span() },
            span: sir_span(),
        },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sir_span(),
    };
    let module = Module {
        name: "oopmod".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Classes,
            Feature::Closures,
            Feature::Strings,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![speak_fn, main],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("handbuilt")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: sir_span(),
    };

    let artifact = compile(&module).expect("compile to typescript");
    // Shape: the three O1 helper calls appear.
    assert!(
        artifact.source.contains("__SirOop.defMethod(\"Dog\", \"speak\","),
        "got:\n{}",
        artifact.source
    );
    assert!(artifact.source.contains("__SirOop.callNew(\"Dog\")"), "got:\n{}", artifact.source);
    assert!(
        artifact.source.contains("__SirOop.callMethod(d, \"speak\")"),
        "got:\n{}",
        artifact.source
    );
    // Execution: the chain must run under node and print the method's result.
    if !node_available() {
        eprintln!("note: `node` unavailable — skipping O1 TS execution proof");
        return;
    }
    let js = ts_to_runnable_js_oop(&artifact.source);
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_ts_oop_{}.js", std::process::id()));
    std::fs::write(&path, &js).expect("write temp js");
    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);
    assert!(
        output.status.success(),
        "node exited non-zero:\nstderr: {}\nsource:\n{}",
        String::from_utf8_lossy(&output.stderr),
        js
    );
    let stdout = String::from_utf8_lossy(&output.stdout)
        .trim_end_matches(['\n', '\r'])
        .to_string();
    assert_eq!(stdout, "Rex says woof", "O1 dispatch produced wrong output");
}

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

// ── O2: Ruby OOP frontend → TS → node execution proof ─────────────────────────
//
// The O1 proof above hand-built the SIR.  This O2 proof lowers REAL Ruby source
// through `ruby-to-semantic-ir`, compiles to TypeScript, and runs it under
// `node` — the same P1 program the Python backend proves, showing the Ruby
// frontend's OOP production executes on the TS side too.
//
// As with every node proof here, the workspace runtime packages cannot be
// resolved under bare `node`, so we swap the two runtime imports for faithful
// inline stubs (see the module doc-comment).  This stub extends the O1
// dispatch stub with the instance-variable store (`ivarSet`/`ivarGet` on the
// current self) and a Ruby `toDisplay` — exactly the surface P1 touches — so
// what runs is the real dispatch + ivar logic, only the import lines rewritten.

const SIR_OOP_P1_STUB: &str = r#"const __SirOop = (() => {
  const SEP = "\x00";
  const key = (c, m) => c + SEP + m;
  const supers = new Map();
  const instanceMethods = new Map();
  const selfStack = [];
  const defaultSelf = { sirClass: "Object", ivars: new Map() };
  class SirInstance { constructor(c) { this.sirClass = c; this.ivars = new Map(); } }
  const curSelf = () => (selfStack.length ? selfStack[selfStack.length - 1] : defaultSelf);
  const superclassOf = (n) => (supers.has(n) ? supers.get(n) : null);
  const defineClass = (n, s) => { supers.set(n, s ?? null); };
  const defMethod = (c, m, fn) => { instanceMethods.set(key(c, m), fn); };
  const ivarSet = (n, v) => { curSelf().ivars.set(n, v); return v; };
  const ivarGet = (n) => { const iv = curSelf().ivars; return iv.has(n) ? iv.get(n) : null; };
  const resolve = (c, m) => {
    let cur = c; const seen = new Set();
    while (cur !== null && !seen.has(cur)) {
      const fn = instanceMethods.get(key(cur, m));
      if (fn !== undefined) return fn;
      seen.add(cur); cur = superclassOf(cur);
    }
    return null;
  };
  const callNew = (c, ...args) => {
    const obj = new SirInstance(c);
    selfStack.push(obj);
    try { const init = resolve(c, "initialize"); if (init !== null) __Sir.apply(init, args); }
    finally { selfStack.pop(); }
    return obj;
  };
  const callMethod = (recv, name, ...args) => {
    if (recv instanceof SirInstance) {
      const fn = resolve(recv.sirClass, name);
      if (fn !== null) {
        selfStack.push(recv);
        try { return __Sir.apply(fn, args); } finally { selfStack.pop(); }
      }
    }
    return null;
  };
  return { defineClass, defMethod, callNew, callMethod, ivarSet, ivarGet };
})();
"#;

/// A `__Sir` stub carrying `Closure` + `apply` (for method closures) plus a
/// Ruby-flavoured `toDisplay` (strings verbatim, `null`→`nil`) and `print`.
const SIR_CLOSURE_P1_STUB: &str = r#"const __Sir = {
  Closure: class { constructor(fn) { this.fn = fn; } },
  apply: (c, args) => c.fn(...args),
  toDisplay: (v) => (v === null ? "nil" : String(v)),
  print: (v) => { console.log(__Sir.toDisplay(v)); return null; },
};
"#;

fn ruby_p1_ts_to_runnable_js(ts: &str) -> String {
    let mut js = ts.to_string();
    js = js.replace(
        "import * as __Sir from \"@coding-adventures/sir-runtime-core\";\n",
        SIR_CLOSURE_P1_STUB,
    );
    js = js.replace(
        "import * as __SirOop from \"@coding-adventures/sir-runtime-oop\";\n",
        SIR_OOP_P1_STUB,
    );
    js = js.replace(" as { [k: string]: __Sir.Val }", "");
    js = js.replace(": __Sir.Val[]", "");
    js = js.replace(": __Sir.Val", "");
    js
}

#[test]
fn end_to_end_ruby_oop_new_and_dispatch_executes_ts() {
    // P1 lowered from real Ruby → TS → node.  Proves the frontend's
    // `__def_method__`/`__new__`/`__method__`/`@ivar` production drives the OOP
    // runtime through the TypeScript backend and prints "Rex says woof".
    let src = "class Dog\n  def initialize(name)\n    @name = name\n  end\n  \
               def speak\n    \"#{@name} says woof\"\n  end\nend\n\
               print Dog.new(\"Rex\").speak\n";
    let module = ruby_to_semantic_ir::compile_source(src, "demo").expect("lower ruby");
    let artifact = compile(&module).expect("compile to typescript");

    // Shape: registration, construction, dispatch, and ivar access all present.
    assert!(
        artifact.source.contains("__SirOop.defMethod(\"Dog\", \"initialize\","),
        "got:\n{}",
        artifact.source
    );
    assert!(artifact.source.contains("__SirOop.callNew(\"Dog\", \"Rex\")"), "got:\n{}", artifact.source);
    assert!(
        artifact.source.contains("__SirOop.callMethod(__SirOop.callNew(\"Dog\", \"Rex\"), \"speak\")"),
        "chained new().speak must nest callMethod over callNew; got:\n{}",
        artifact.source
    );

    if !node_available() {
        eprintln!("note: `node` unavailable — skipping O2 TS execution proof");
        return;
    }
    let js = ruby_p1_ts_to_runnable_js(&artifact.source);
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_ts_oop_p1_{}.js", std::process::id()));
    std::fs::write(&path, &js).expect("write temp js");
    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);
    assert!(
        output.status.success(),
        "node exited non-zero:\nstderr: {}\nsource:\n{}",
        String::from_utf8_lossy(&output.stderr),
        js
    );
    let stdout = String::from_utf8_lossy(&output.stdout)
        .trim_end_matches(['\n', '\r'])
        .to_string();
    assert_eq!(stdout, "Rex says woof", "P1 OOP dispatch produced wrong output under node");
}


// ── End-to-end: Ruby `puts` → TypeScript → node ────────────────────────
//
// Proves the Ruby frontend's `puts` (a `BuiltinCall("puts", …)`) drives the
// runtime-core `puts` through the TypeScript backend end-to-end.  As with the
// other node proofs, the workspace runtime package can't be resolved under
// bare `node`, so the runtime import is swapped for a faithful inline stub
// implementing Ruby `puts` semantics (string+newline, no-arg → one newline).

/// A `__Sir` stub whose `puts` mirrors the real runtime-core: variadic,
/// writes each string arg + "\n" via process.stdout.write (so the exact byte
/// stream — including a trailing blank line — is observable), and a no-arg
/// `puts` writes one newline.
const SIR_PUTS_STUB: &str = r#"const __Sir = {
  toDisplay: (v) => (v === null ? "nil" : String(v)),
  puts: (...args) => {
    if (args.length === 0) { process.stdout.write("\n"); return null; }
    for (const a of args) {
      const t = __Sir.toDisplay(a);
      process.stdout.write(t.endsWith("\n") ? t : t + "\n");
    }
    return null;
  },
};
"#;

fn ruby_puts_ts_to_runnable_js(ts: &str) -> String {
    let mut js = ts.to_string();
    js = js.replace(
        "import * as __Sir from \"@coding-adventures/sir-runtime-core\";\n",
        SIR_PUTS_STUB,
    );
    js = js.replace(" as { [k: string]: __Sir.Val }", "");
    js = js.replace(": __Sir.Val[]", "");
    js = js.replace(": __Sir.Val", "");
    js
}

#[test]
fn end_to_end_ruby_puts_executes_ts() {
    // `puts "hi"` lowered from real Ruby → TS → node must print exactly
    // `hi\n` (Ruby's string+newline semantics).
    let module = ruby_to_semantic_ir::compile_source("puts \"hi\"\n", "demo")
        .expect("lower ruby");
    let artifact = compile(&module).expect("compile to typescript");

    // Shape: `puts` maps to the variadic `__Sir.puts(...)` helper.
    assert!(
        artifact.source.contains("__Sir.puts(\"hi\")"),
        "expected puts to map to __Sir.puts; got:\n{}",
        artifact.source
    );

    if !node_available() {
        eprintln!("note: `node` unavailable — skipping Ruby puts TS execution proof");
        return;
    }
    let js = ruby_puts_ts_to_runnable_js(&artifact.source);
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_ts_ruby_puts_{}.js", std::process::id()));
    std::fs::write(&path, &js).expect("write temp js");
    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);
    assert!(
        output.status.success(),
        "node exited non-zero:\nstderr: {}\nsource:\n{}",
        String::from_utf8_lossy(&output.stderr),
        js
    );
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(stdout, "hi\n", "Ruby `puts \"hi\"` should print `hi` + newline");
}

// ── T2: typed runtime errors → TypeScript → node execution proof ──────────────
//
// The unit tests in the TS packages (`sir-runtime-core`, `sir-runtime-oop`)
// prove the helpers RAISE the right typed `SirError`.  This proof closes the
// loop end-to-end: a Ruby `begin … rescue <TypedError> => e … end` lowered
// through the frontend → TypeScript → `node` actually CATCHES the faulting
// runtime op with the matching typed class, and the plain index operators
// (`arr[i]`/`hash[k]`) still return nil (no over-raise).
//
// As with every node proof in this file, the workspace runtime packages cannot
// be resolved under bare `node`, so we swap the three runtime imports for
// faithful inline stubs.  The stubs here TRANSCRIBE the exact T2 logic added to
// the real packages:
//   • `__Sir.div` adds the explicit zero-divisor check (native `/` gives
//     Infinity) and raises `ZeroDivisionError` via `__SirExc.raiseError`;
//   • `__SirOop.callMethod` implements `Array#fetch`→IndexError,
//     `Hash#fetch`→KeyError, and the unknown-method→NoMethodError floor
//     (guarded by a `respondsTo` check so a known-but-block-less method is not
//     mis-raised);
//   • `__SirExc` is the real ancestry + `raiseError` + `rescueMatches` (reused
//     from the E2 stub) — CRUCIALLY the `__Sir`/`__SirOop` stubs raise through
//     `__SirExc.raiseError` so every thrown `SirError` shares the one class
//     identity `classOfThrown` checks, exactly as the real packages do by
//     importing a single `SirError`.
// Keeping the real fault logic in the stubs means the proof genuinely exercises
// "the faulting op raises the typed error that rescue matches", not just that
// the emitted call sites are present.

/// `__Sir` stub carrying the T2 `div` (with zero-check), plus `puts`/`toDisplay`
/// for the rescue-body output.  `div` mirrors the real runtime-core: truncating
/// integer division, but an explicit `=== 0` divisor check that raises
/// `ZeroDivisionError` through `__SirExc` before dividing.
const SIR_T2_STUB: &str = r#"const __Sir = {
  toDisplay: (v) => (v === null ? "nil" : String(v)),
  print: (v) => { console.log(__Sir.toDisplay(v)); return null; },
  puts: (...args) => {
    if (args.length === 0) { process.stdout.write("\n"); return null; }
    for (const a of args) {
      const t = __Sir.toDisplay(a);
      process.stdout.write(t.endsWith("\n") ? t : t + "\n");
    }
    return null;
  },
  div: (...args) => {
    if (args.length === 0) return 0;
    let acc = args[0];
    for (let i = 1; i < args.length; i++) {
      const d = args[i];
      if (d === 0) __SirExc.raiseError("ZeroDivisionError", "divided by 0");
      acc = Math.trunc(acc / d);
    }
    return acc;
  },
};
"#;

/// `__SirOop` stub whose `callMethod` transcribes the T2 fault paths: `fetch`
/// (IndexError / KeyError), the unknown-method NoMethodError floor guarded by a
/// minimal `respondsTo`, plus `classOf`/`nil?` for the receiver-class message
/// and the nil-regression proof.  Faithful to the real dispatch for exactly the
/// surface these programs touch.
const SIR_OOP_T2_STUB: &str = r#"const __SirOop = (() => {
  const classOf = (v) => {
    if (v === null || v === undefined) return "NilClass";
    switch (typeof v) {
      case "boolean": return v ? "TrueClass" : "FalseClass";
      case "number": return Number.isInteger(v) ? "Integer" : "Float";
      case "string": return "String";
      default:
        if (Array.isArray(v)) return "Array";
        if (v instanceof Map) return "Hash";
        return "Object";
    }
  };
  const rubyInspect = (v) => (typeof v === "string" ? JSON.stringify(v) : String(v));
  const respondsTo = (recv, name) => {
    if (name === "nil?" || name === "class" || name === "fetch") return true;
    return false;
  };
  const callMethod = (recv, name, ...args) => {
    if (name === "class") return classOf(recv);
    if (name === "nil?") return recv === null || recv === undefined;
    if (name === "fetch") {
      if (Array.isArray(recv)) {
        const raw = args[0];
        const idx = raw < 0 ? recv.length + raw : raw;
        if (idx >= 0 && idx < recv.length) return recv[idx];
        if (args.length > 1) return args[1];
        __SirExc.raiseError("IndexError",
          "index " + raw + " outside of array bounds: " + (-recv.length) + "..." + recv.length);
      }
      if (recv instanceof Map) {
        if (recv.has(args[0])) return recv.get(args[0]);
        if (args.length > 1) return args[1];
        __SirExc.raiseError("KeyError", "key not found: " + rubyInspect(args[0]));
      }
    }
    if (!respondsTo(recv, name)) {
      __SirExc.raiseError("NoMethodError", "undefined method '" + name + "' for " + classOf(recv));
    }
    return null;
  };
  return { callMethod };
})();
"#;

/// Transform emitted TypeScript into runnable JavaScript for the T2 proof.
fn ts_to_runnable_js_t2(ts: &str) -> String {
    let mut js = ts.to_string();
    js = js.replace(
        "import * as __Sir from \"@coding-adventures/sir-runtime-core\";\n",
        SIR_T2_STUB,
    );
    js = js.replace(
        "import * as __SirOop from \"@coding-adventures/sir-runtime-oop\";\n",
        SIR_OOP_T2_STUB,
    );
    js = js.replace(
        "import * as __SirExc from \"@coding-adventures/sir-runtime-exceptions\";\n",
        SIR_EXC_STUB,
    );
    js = js.replace(" as { [k: string]: __Sir.Val }", "");
    // Strip the type casts/generics the SeqIndex/MapGet emitter adds — node
    // parses the bare `<…>`/`as …` as syntax errors.  Longer patterns first so
    // no fragment is left dangling.
    js = js.replace(" as Map<__Sir.Val, __Sir.Val>", "");
    js = js.replace("<__Sir.Val, __Sir.Val>", "");
    js = js.replace(" as __Sir.Val[]", "");
    js = js.replace(" as number", "");
    js = js.replace(": __Sir.Val[]", "");
    js = js.replace(": __Sir.Val", "");
    js
}

/// Compile a module, transform to JS with the T2 stubs, run under node, and
/// return trimmed stdout.  `None` when node is unavailable.  Asserts a zero
/// exit — every T2 prog is expected to CATCH its fault and print, so an
/// uncaught throw (wrong typed class → no rescue match) would exit non-zero.
fn run_t2_module(module: &Module, tag: &str) -> Option<String> {
    let artifact = compile(module).expect("compile to typescript");
    if !node_available() {
        eprintln!("note: `node` unavailable — skipping T2 execution for `{tag}`");
        return None;
    }
    let js = ts_to_runnable_js_t2(&artifact.source);
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_ts_t2_{}_{}.js", tag, std::process::id()));
    std::fs::write(&path, &js).expect("write temp js");
    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);
    assert!(
        output.status.success(),
        "node exited non-zero for `{tag}` (fault not caught by the typed rescue?):\n\
         stdout: {}\nstderr: {}\nsource:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        js,
    );
    let stdout = String::from_utf8_lossy(&output.stdout)
        .trim_end_matches(['\n', '\r'])
        .to_string();
    Some(stdout)
}

#[test]
fn t2_zero_division_caught_by_zerodivisionerror_ts() {
    // `begin; 1 / 0; rescue ZeroDivisionError => e; ...; end` must catch — native
    // JS `/` gives Infinity, so the runtime's explicit check is what raises.
    let module = ruby_to_semantic_ir::compile_source(
        "begin\n  x = 1 / 0\nrescue ZeroDivisionError => e\n  puts \"caught zde\"\nend\n",
        "t2div",
    )
    .expect("lower ruby");
    // Shape: division routes through the `__Sir.div` helper (where the check lives).
    let artifact = compile(&module).expect("compile");
    assert!(
        artifact.source.contains("__Sir.div(1, 0)"),
        "division must route through __Sir.div; got:\n{}",
        artifact.source
    );
    if let Some(stdout) = run_t2_module(&module, "div") {
        assert_eq!(stdout, "caught zde", "1/0 must be caught as ZeroDivisionError");
    }
}

#[test]
fn t2_array_fetch_oob_caught_by_indexerror_ts() {
    // `arr.fetch(100)` OOB raises IndexError (unlike `arr[100]`, which is nil).
    let module = ruby_to_semantic_ir::compile_source(
        "arr = [1, 2, 3]\nbegin\n  arr.fetch(100)\nrescue IndexError => e\n  puts \"caught ie\"\nend\n",
        "t2afetch",
    )
    .expect("lower ruby");
    if let Some(stdout) = run_t2_module(&module, "afetch") {
        assert_eq!(stdout, "caught ie", "arr.fetch(oob) must be caught as IndexError");
    }
}

#[test]
fn t2_hash_fetch_miss_caught_by_keyerror_ts() {
    // `h.fetch(missing)` raises KeyError (unlike `h[missing]`, which is nil).
    let module = ruby_to_semantic_ir::compile_source(
        "h = {\"a\" => 1}\nbegin\n  h.fetch(\"z\")\nrescue KeyError => e\n  puts \"caught ke\"\nend\n",
        "t2hfetch",
    )
    .expect("lower ruby");
    if let Some(stdout) = run_t2_module(&module, "hfetch") {
        assert_eq!(stdout, "caught ke", "h.fetch(miss) must be caught as KeyError");
    }
}

#[test]
fn t2_unknown_method_caught_by_nomethoderror_ts() {
    // `obj.undefined` raises NoMethodError (was a silent nil floor).
    let module = ruby_to_semantic_ir::compile_source(
        "x = 5\nbegin\n  x.no_such\nrescue NoMethodError => e\n  puts \"caught nme\"\nend\n",
        "t2nme",
    )
    .expect("lower ruby");
    if let Some(stdout) = run_t2_module(&module, "nme") {
        assert_eq!(stdout, "caught nme", "obj.undefined must be caught as NoMethodError");
    }
}

#[test]
fn t2_index_ops_still_return_nil_no_overraise_ts() {
    // Regression: plain `arr[oob]` and `hash[miss]` must STILL return nil — only
    // `.fetch` raises.  The Ruby parser has no `[]` index syntax, so we hand-build
    // the SIR `SeqIndex`/`MapGet` (the exact IR the index operators lower to) and
    // print each result's `nil?`, expecting `true` / `true`.
    let arr = Stmt::LetBinding {
        name: "arr".into(),
        sir_type: None,
        value: Expr::SeqLit {
            items: vec![
                Expr::IntLit { value: 1, span: sir_span() },
                Expr::IntLit { value: 2, span: sir_span() },
            ],
            span: sir_span(),
        },
        span: sir_span(),
    };
    // arr[100] — out of bounds → nil
    let seq_index = Expr::SeqIndex {
        seq: Box::new(Expr::VarRef { name: "arr".into(), scope: Scope::Local, span: sir_span() }),
        index: Box::new(Expr::IntLit { value: 100, span: sir_span() }),
        span: sir_span(),
    };
    let idx_nil = Expr::BuiltinCall {
        name: "__method__".into(),
        args: vec![seq_index, str_lit("nil?")],
        effects: EffectSet::PURE,
        span: sir_span(),
    };
    let h = Stmt::LetBinding {
        name: "h".into(),
        sir_type: None,
        value: Expr::MapLit { entries: vec![], span: sir_span() },
        span: sir_span(),
    };
    // h["z"] — missing key → nil
    let map_get = Expr::MapGet {
        map: Box::new(Expr::VarRef { name: "h".into(), scope: Scope::Local, span: sir_span() }),
        key: Box::new(str_lit("z")),
        span: sir_span(),
    };
    let hget_nil = Expr::BuiltinCall {
        name: "__method__".into(),
        args: vec![map_get, str_lit("nil?")],
        effects: EffectSet::PURE,
        span: sir_span(),
    };
    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![arr, print(idx_nil), h, print(hget_nil)],
            value: Expr::NilLit { span: sir_span() },
            span: sir_span(),
        },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sir_span(),
    };
    let module = Module {
        name: "t2nil".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Sequences,
            Feature::Maps,
            Feature::Strings,
            Feature::DynamicTyping,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![main],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("handbuilt")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: sir_span(),
    };
    if let Some(stdout) = run_t2_module(&module, "nilregress") {
        // Both index ops returned nil, so `.nil?` printed `true` for each.
        assert_eq!(
            stdout.replace("\r\n", "\n"),
            "true\ntrue",
            "arr[oob] and hash[miss] must still return nil (only .fetch raises)"
        );
    }
}

// ── MX3: Ruby mixins (include / extend) → TypeScript → node execution proof ───
//
// MX1 (merged) lowers `module M`, `include M`, and `extend M` to the
// `__def_method__` / `__include__` / `__extend__` builtins; MX3 makes the TS
// OOP runtime EXECUTE them.  These proofs lower REAL Ruby through
// `ruby-to-semantic-ir`, compile to TypeScript, and run under `node` — the same
// programs the reference (Python) backend proves.
//
// As with every node proof here, the workspace runtime cannot be resolved under
// bare `node`, so we swap the OOP import for a faithful inline `__SirOop` stub
// that TRANSCRIBES the real MX3 logic added to `@coding-adventures/sir-runtime-oop`:
//   • `includeModule` appends to the owner's include-order list;
//   • `extendModule` copies a module's instance methods into the owner's
//     class-method table;
//   • `resolveInstanceMethod` walks Ruby's MRO — class first, then its modules
//     most-recent-first (depth-first, `seen`-de-duplicated for diamonds), then
//     the superclass — so a class method SHADOWS a module method and a diamond
//     resolves once.
// Keeping the real MRO here means the proof exercises "the mixed-in method is
// found by the module-aware walk", not just that the emitted calls are present.

const SIR_OOP_MIXIN_STUB: &str = r#"const __SirOop = (() => {
  const SEP = "\x00";
  const key = (c, m) => c + SEP + m;
  const supers = new Map();
  const instanceMethods = new Map();
  const classMethods = new Map();
  const includedModules = new Map();
  const selfStack = [];
  const defaultSelf = { sirClass: "Object", ivars: new Map() };
  class SirInstance { constructor(c) { this.sirClass = c; this.ivars = new Map(); } }
  const curSelf = () => (selfStack.length ? selfStack[selfStack.length - 1] : defaultSelf);
  const superclassOf = (n) => (supers.has(n) ? supers.get(n) : null);
  const defineClass = (n, s) => { supers.set(n, s ?? null); };
  const defMethod = (c, m, fn) => { instanceMethods.set(key(c, m), fn); };
  const defClassMethod = (c, m, fn) => { classMethods.set(key(c, m), fn); };
  const ivarSet = (n, v) => { curSelf().ivars.set(n, v); return v; };
  const ivarGet = (n) => { const iv = curSelf().ivars; return iv.has(n) ? iv.get(n) : null; };
  const includeModule = (owner, mod) => {
    const list = includedModules.get(owner);
    if (list === undefined) includedModules.set(owner, [mod]);
    else if (!list.includes(mod)) list.push(mod);
  };
  const extendModule = (owner, mod) => {
    const prefix = mod + SEP;
    for (const [k, fn] of instanceMethods) {
      if (k.startsWith(prefix)) classMethods.set(key(owner, k.slice(prefix.length)), fn);
    }
  };
  const resolve = (className, methodName) => {
    const seen = new Set();
    const searchOwner = (owner) => {
      if (seen.has(owner)) return null;
      seen.add(owner);
      const own = instanceMethods.get(key(owner, methodName));
      if (own !== undefined) return own;
      const mods = includedModules.get(owner);
      if (mods !== undefined) {
        for (let i = mods.length - 1; i >= 0; i--) {
          const hit = searchOwner(mods[i]);
          if (hit !== null) return hit;
        }
      }
      return null;
    };
    let cur = className; const seenClasses = new Set();
    while (cur !== null && !seenClasses.has(cur)) {
      seenClasses.add(cur);
      const hit = searchOwner(cur);
      if (hit !== null) return hit;
      cur = superclassOf(cur);
    }
    return null;
  };
  const resolveClass = (className, methodName) => {
    let cur = className; const seen = new Set();
    while (cur !== null && !seen.has(cur)) {
      const fn = classMethods.get(key(cur, methodName));
      if (fn !== undefined) return fn;
      seen.add(cur); cur = superclassOf(cur);
    }
    return null;
  };
  const callNew = (c, ...args) => {
    const obj = new SirInstance(c);
    selfStack.push(obj);
    try { const init = resolve(c, "initialize"); if (init !== null) __Sir.apply(init, args); }
    finally { selfStack.pop(); }
    return obj;
  };
  const callMethod = (recv, name, ...args) => {
    if (recv instanceof SirInstance) {
      const fn = resolve(recv.sirClass, name);
      if (fn !== null) {
        selfStack.push(recv);
        try { return __Sir.apply(fn, args); } finally { selfStack.pop(); }
      }
    }
    return null;
  };
  const callClassMethod = (c, name, ...args) => {
    const fn = resolveClass(c, name);
    return fn === null ? null : __Sir.apply(fn, args);
  };
  return { defineClass, defMethod, defClassMethod, includeModule, extendModule,
           callNew, callMethod, callClassMethod, ivarSet, ivarGet };
})();
"#;

/// A `__Sir` stub carrying `Closure` + `apply` (for method closures) plus a
/// Ruby-flavoured `puts` (string+newline; the MX3 programs print via `puts`).
const SIR_CLOSURE_MIXIN_STUB: &str = r#"const __Sir = {
  Closure: class { constructor(fn) { this.fn = fn; } },
  apply: (c, args) => c.fn(...args),
  toDisplay: (v) => (v === null ? "nil" : String(v)),
  puts: (...args) => {
    if (args.length === 0) { process.stdout.write("\n"); return null; }
    for (const a of args) {
      const t = __Sir.toDisplay(a);
      process.stdout.write(t.endsWith("\n") ? t : t + "\n");
    }
    return null;
  },
};
"#;

fn ruby_mixin_ts_to_runnable_js(ts: &str) -> String {
    let mut js = ts.to_string();
    js = js.replace(
        "import * as __Sir from \"@coding-adventures/sir-runtime-core\";\n",
        SIR_CLOSURE_MIXIN_STUB,
    );
    js = js.replace(
        "import * as __SirOop from \"@coding-adventures/sir-runtime-oop\";\n",
        SIR_OOP_MIXIN_STUB,
    );
    js = js.replace(" as { [k: string]: __Sir.Val }", "");
    js = js.replace(": __Sir.Val[]", "");
    js = js.replace(": __Sir.Val", "");
    js
}

/// Compile a Ruby-lowered mixin module → TS → JS with the MX3 stub, run under
/// node, and return trimmed stdout.  `None` when node is unavailable.
fn run_mixin_source(src: &str, tag: &str) -> Option<String> {
    let module = ruby_to_semantic_ir::compile_source(src, tag).expect("lower ruby");
    let artifact = compile(&module).expect("compile to typescript");
    if !node_available() {
        eprintln!("note: `node` unavailable — skipping MX3 execution for `{tag}`");
        return None;
    }
    let js = ruby_mixin_ts_to_runnable_js(&artifact.source);
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_ts_mixin_{}_{}.js", tag, std::process::id()));
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
    let stdout = String::from_utf8_lossy(&output.stdout)
        .trim_end_matches(['\n', '\r'])
        .to_string();
    Some(stdout)
}

#[test]
fn mx3_included_module_method_is_callable_ts() {
    // A module instance method mixed into a class is found on an instance.
    let src = "module Greeter\n  def greet\n    \"hello\"\n  end\nend\n\
               class Person\n  include Greeter\nend\n\
               puts Person.new.greet\n";
    // Shape: the mixin directive maps to the runtime helper.
    let module = ruby_to_semantic_ir::compile_source(src, "mx_inc").expect("lower ruby");
    let artifact = compile(&module).expect("compile");
    assert!(
        artifact.source.contains("__SirOop.includeModule(\"Person\", \"Greeter\")"),
        "include must map to includeModule; got:\n{}",
        artifact.source
    );
    if let Some(stdout) = run_mixin_source(src, "mx_inc") {
        assert_eq!(stdout, "hello", "mixed-in module method must be callable");
    }
}

#[test]
fn mx3_class_method_shadows_module_method_ts() {
    // The class defines `greet` itself → class-first MRO shadows the module's.
    let src = "module Greeter\n  def greet\n    \"from module\"\n  end\nend\n\
               class Person\n  include Greeter\n  def greet\n    \"from class\"\n  end\nend\n\
               puts Person.new.greet\n";
    if let Some(stdout) = run_mixin_source(src, "mx_shadow") {
        assert_eq!(stdout, "from class", "class method must shadow the included module's");
    }
}

#[test]
fn mx3_most_recently_included_module_wins_ts() {
    // Two modules define the same method; the LAST included wins (reverse walk).
    let src = "module A\n  def who\n    \"A\"\n  end\nend\n\
               module B\n  def who\n    \"B\"\n  end\nend\n\
               class C\n  include A\n  include B\nend\n\
               puts C.new.who\n";
    if let Some(stdout) = run_mixin_source(src, "mx_recent") {
        assert_eq!(stdout, "B", "most recently included module wins the MRO");
    }
}

#[test]
fn mx3_diamond_include_resolves_once_ts() {
    // Diamond: C includes X and Y, both of which include Base.  Base#tag is
    // found exactly once (the `seen` set de-duplicates), and the program runs.
    let src = "module Base\n  def tag\n    \"base\"\n  end\nend\n\
               module X\n  include Base\nend\n\
               module Y\n  include Base\nend\n\
               class C\n  include X\n  include Y\nend\n\
               puts C.new.tag\n";
    if let Some(stdout) = run_mixin_source(src, "mx_diamond") {
        assert_eq!(stdout, "base", "diamond include must resolve the shared module once");
    }
}

#[test]
fn mx3_extend_makes_module_method_a_class_method_ts() {
    // `extend M` mixes M's instance methods as CLASS methods → `Widget.describe`.
    let src = "module Describable\n  def describe\n    \"a widget\"\n  end\nend\n\
               class Widget\n  extend Describable\nend\n\
               puts Widget.describe\n";
    let module = ruby_to_semantic_ir::compile_source(src, "mx_ext").expect("lower ruby");
    let artifact = compile(&module).expect("compile");
    assert!(
        artifact.source.contains("__SirOop.extendModule(\"Widget\", \"Describable\")"),
        "extend must map to extendModule; got:\n{}",
        artifact.source
    );
    if let Some(stdout) = run_mixin_source(src, "mx_ext") {
        assert_eq!(stdout, "a widget", "extend must make the module method a class method");
    }
}
