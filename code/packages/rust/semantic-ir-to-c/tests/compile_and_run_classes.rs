//! Execution proof for the OOP mirror slice 1 (classes + constants) on the C
//! backend — hand-build producer-agnostic modules, emit C, compile with a real
//! cc, run, assert stdout.  Skips gracefully when no `cc` is present.
//!
//! Slice 1 = the instance runtime foundation: an empty class (`class Foo; end`),
//! construction (`Foo.new` → a `SIR_INSTANCE` box printing `#<Foo>`), and the
//! entangled constants (`PI = 3` via a runtime const table).

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Scope, Span,
    Stmt, CURRENT_SIR_VERSION,
};

fn find_cc() -> Option<String> {
    if let Ok(cc) = std::env::var("SIR_CC") {
        if !cc.trim().is_empty() {
            return Some(cc);
        }
    }
    for cand in ["cc", "clang", "gcc"] {
        if Command::new(cand)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(cand.to_string());
        }
    }
    None
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn run(module: &Module) -> Option<String> {
    let cc = find_cc()?;
    let artifact = semantic_ir_to_c::compile(module).expect("C backend compile (no panic)");
    let dir = std::env::temp_dir();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stem = format!("sirc_cls_{}_{}", std::process::id(), n);
    let cpath: PathBuf = dir.join(format!("{stem}.c"));
    let exe: PathBuf = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::File::create(&cpath)
        .and_then(|mut f| f.write_all(artifact.source.as_bytes()))
        .expect("write .c");
    let out = Command::new(&cc)
        .args(["-std=c99", "-Wall", "-Werror=unused-variable", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm")  // Linux needs -lm to link floor/ceil/fabs (macOS libSystem folds it in)
        .output()
        .expect("spawn cc");
    assert!(
        out.status.success(),
        "compile failed:\n{}\n--- source ---\n{}",
        String::from_utf8_lossy(&out.stderr),
        artifact.source
    );
    let r = Command::new(&exe).output().expect("run");
    assert!(r.status.success(), "program exited non-zero");
    Some(String::from_utf8_lossy(&r.stdout).replace("\r\n", "\n"))
}

fn s() -> Span {
    Span::synthetic()
}
fn strlit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}
fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}
fn bc(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}
fn puts(arg: Expr) -> Stmt {
    Stmt::ExprStmt { expr: bc("puts", vec![arg]), span: s() }
}
fn local(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Local, span: s() }
}
fn module(stmts: Vec<Stmt>, feats: &[Feature]) -> Module {
    Module {
        name: "clsprog".into(),
        manifest: FeatureManifest::from_features(feats),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block { stmts, value: Expr::NilLit { span: s() }, span: s() },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        }],
        globals: vec![],
        metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
        span: s(),
    }
}

#[test]
fn empty_class_new_prints_the_instance() {
    // `class Foo; end; x = Foo.new; puts x` → `#<Foo>` (deterministic — no address).
    let m = module(
        vec![
            Stmt::ClassDef { name: "Foo".into(), superclass: None, body: vec![], span: s() },
            Stmt::LetBinding { name: "x".into(), sir_type: None, value: bc("__new__", vec![strlit("Foo")]), span: s() },
            puts(local("x")),
        ],
        &[Feature::Classes, Feature::Constants, Feature::Strings],
    );
    match run(&m) {
        Some(out) => assert_eq!(out, "#<Foo>\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn constant_set_and_get() {
    // `PI = 3; puts PI` → `3` (runtime const table).
    let m = module(
        vec![
            Stmt::Assign { name: "PI".into(), scope: Scope::Const, value: ilit(3), span: s() },
            puts(Expr::VarRef { name: "PI".into(), scope: Scope::Const, span: s() }),
        ],
        &[Feature::Constants, Feature::MutableBindings],
    );
    match run(&m) {
        Some(out) => assert_eq!(out, "3\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn two_instances_are_distinct_by_identity() {
    // Two `Foo.new`s are distinct objects: `==` is identity, so `a == b` is false
    // (printed via an int so the assertion is display-convention-independent).
    let m = module(
        vec![
            Stmt::ClassDef { name: "Foo".into(), superclass: None, body: vec![], span: s() },
            Stmt::LetBinding { name: "a".into(), sir_type: None, value: bc("__new__", vec![strlit("Foo")]), span: s() },
            Stmt::LetBinding { name: "b".into(), sir_type: None, value: bc("__new__", vec![strlit("Foo")]), span: s() },
            puts(bc("==", vec![local("a"), local("a")])),
            puts(bc("==", vec![local("a"), local("b")])),
        ],
        &[Feature::Classes, Feature::Constants, Feature::Strings],
    );
    match run(&m) {
        // `a==a` true, `a==b` false — Ruby display convention off (module not
        // tagged source_language=ruby), so booleans print `#t`/`#f`.
        Some(out) => assert_eq!(out, "#t\n#f\n"),
        None => eprintln!("skip: no cc"),
    }
}

// ---- rejection (deferred shapes rejected cleanly, never a panic) ----------

fn rejects(stmts: Vec<Stmt>, feats: &[Feature]) -> bool {
    semantic_ir_to_c::compile(&module(stmts, feats)).is_err()
}

#[test]
fn deferred_oop_shapes_are_rejected_cleanly() {
    // `__new__` with a constructor argument (needs `initialize`, a later slice).
    assert!(
        rejects(
            vec![puts(bc("__new__", vec![strlit("Foo"), ilit(1)]))],
            &[Feature::Classes, Feature::Constants, Feature::Strings]
        ),
        "__new__ with ctor args must be rejected"
    );
    // A MALFORMED `__def_method__` (no closure) — a well-formed one is now
    // supported (slice 2), but a missing closure must reject, not mis-emit.
    assert!(
        rejects(
            vec![
                Stmt::ClassDef { name: "Foo".into(), superclass: None, body: vec![], span: s() },
                puts(bc("__def_method__", vec![strlit("Foo"), strlit("m")])),
            ],
            &[Feature::Classes, Feature::Constants, Feature::Strings]
        ),
        "a malformed __def_method__ must be rejected"
    );
    // A `class << self` singleton (also observes Feature::Classes).
    assert!(
        rejects(
            vec![Stmt::SingletonClassDef { target: "self".into(), body: vec![], span: s() }],
            &[Feature::Classes]
        ),
        "singleton class must be rejected"
    );
    // A `__method__` dispatch to a method the module never registers is a
    // built-in method call (the Collections batch) — rejected cleanly.
    assert!(
        rejects(
            vec![puts(bc("__method__", vec![bc("__new__", vec![strlit("Foo")]), strlit("m")]))],
            &[Feature::Classes, Feature::Constants, Feature::Strings]
        ),
        "an unregistered (built-in) __method__ dispatch must be rejected"
    );
    // (A superclass is now SUPPORTED — OOP slice 4 registers `class Dog < Animal`
    // as a `_sir_register_super` edge — so it is no longer a rejected shape; see
    // `compile_and_run_inheritance.rs` for its execution proof.)
    // A non-empty class body (its content would be silently dropped by the
    // comment emit) — here a `Const`-assign body stmt (an accepted feature, so it
    // passes the manifest gate and must be caught by the scan, not dropped).
    assert!(
        rejects(
            vec![Stmt::ClassDef {
                name: "Foo".into(),
                superclass: None,
                body: vec![Stmt::Assign { name: "PI".into(), scope: Scope::Const, value: ilit(3), span: s() }],
                span: s(),
            }],
            &[Feature::Classes, Feature::Constants]
        ),
        "a non-empty class body must be rejected"
    );
}
