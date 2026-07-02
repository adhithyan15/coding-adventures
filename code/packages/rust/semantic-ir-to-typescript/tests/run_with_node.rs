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
