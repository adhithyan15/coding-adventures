//! Execution proof for O4 user-defined-class OOP: hand-build a SIR module,
//! emit Go, run it with `go run`, and assert the observable behaviour matches
//! Ruby object semantics (method dispatch, `new`→`initialize`, inheritance +
//! `super`, `@ivar` through the current-self stack).
//!
//! The Go backend has NO native object model — the frontend HOISTS every
//! method to a detached top-level function, and the runtime recovers the
//! method↔class association at RUNTIME via explicit `(class, method)` map
//! tables (`runtime::RUNTIME`).  These cases prove the mapping end-to-end:
//!
//!   P1  `Dog.new("Rex").speak` — `initialize` stores `@name`, `speak` reads
//!       it back through the current-self stack → prints "Rex".
//!   P2  inheritance + `super` — `Cat < Animal`; `Animal#initialize` sets
//!       `@legs`, `Cat#initialize` calls `super` then a `describe` reads the
//!       parent-set ivar → prints "4" (discriminating: proves super ran AND
//!       the parent-set ivar is visible on the SAME self).
//!   SEC a class AND method named `constructor` — a source-derived name that
//!       is host-significant in JS engines — is JUST a map key here: it
//!       dispatches the USER method (a clean lookup), never host behaviour,
//!       and an UNregistered `__proto__` call hits the NoMethodError floor.
//!   CYC a cyclic ancestry (`A < B`, `B < A`) TERMINATES rather than looping:
//!       `A.new` with no `initialize` anywhere returns cleanly (exit 0).
//!
//! To avoid StrLit interpolation (which the Go backend rejects) every program
//! prints a single value — the ivar or a numeric marker — with no `#{}`.
//!
//! A missing `go` toolchain logs a skip rather than reddening the build
//! (mirrors `compile_and_run_exceptions.rs`).

use std::process::Command;

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Param, ParamKind,
    Scope, Span, Stmt,
};
use semantic_ir_to_go::compile;

fn s() -> Span {
    Span::synthetic()
}

fn str_lit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}

fn int_lit(n: i64) -> Expr {
    Expr::IntLit { value: n, span: s() }
}

fn param_ref(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Param, span: s() }
}

fn param(name: &str) -> Param {
    Param {
        name: name.into(),
        sir_type: None,
        kind: ParamKind::Required,
        default: None,
        span: s(),
    }
}

fn ivar_ref(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Instance, span: s() }
}

fn ivar_set(name: &str, value: Expr) -> Stmt {
    Stmt::Assign { name: name.into(), scope: Scope::Instance, value, span: s() }
}

fn builtin(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}

fn print_stmt(v: Expr) -> Stmt {
    Stmt::ExprStmt { expr: builtin("print", vec![v]), span: s() }
}

/// A hoisted method as a top-level function: `captures` are always empty
/// (methods use the self-stack, not captures), params are positional.
fn method_fn(fn_name: &str, params: Vec<Param>, body_stmts: Vec<Stmt>, value: Expr) -> Function {
    Function {
        name: fn_name.into(),
        params,
        return_type: None,
        captures: vec![],
        body: Block { stmts: body_stmts, value, span: s() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: s(),
    }
}

/// A `MakeClosure` over a hoisted method function (what `__def_method__`
/// receives as its third argument).
fn closure_of(fn_name: &str) -> Expr {
    Expr::MakeClosure { fn_name: fn_name.into(), captures: vec![], span: s() }
}

fn def_method(cls: &str, meth: &str, fn_name: &str) -> Stmt {
    Stmt::ExprStmt {
        expr: builtin(
            "__def_method__",
            vec![str_lit(cls), str_lit(meth), closure_of(fn_name)],
        ),
        span: s(),
    }
}

fn class_def(name: &str, superclass: Option<&str>, body: Vec<Stmt>) -> Stmt {
    Stmt::ClassDef {
        name: name.into(),
        superclass: superclass.map(|s| s.to_string()),
        body,
        span: s(),
    }
}

/// Assemble a module from a list of top-level functions + a `main` body.
fn module_from(
    name: &str,
    mut functions: Vec<Function>,
    main_stmts: Vec<Stmt>,
    features: &[Feature],
) -> Module {
    functions.push(method_fn(
        "main",
        vec![],
        main_stmts,
        Expr::NilLit { span: s() },
    ));
    let mut fs = vec![
        Feature::Classes,
        Feature::InstanceVars,
        Feature::Strings,
        Feature::MutableBindings,
        Feature::Closures,
        Feature::DynamicTyping,
    ];
    fs.extend_from_slice(features);
    Module {
        name: name.into(),
        manifest: FeatureManifest::from_features(&fs),
        imports: vec![],
        exports: vec![],
        functions,
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("test")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: s(),
    }
}

fn go_available() -> bool {
    Command::new("go")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Emit `m` to Go, run it, and return `(success, stdout)`.  A COMPILE error
/// (bad emit) panics loudly; a runtime non-zero exit returns the flag.
fn emit_and_run(m: &Module, nonce: &str) -> (bool, String) {
    let artifact = compile(m).expect("module should compile to Go source");
    let dir = std::env::temp_dir();
    let pid = std::process::id();
    let src_path = dir.join(format!("sir_go_oop_{pid}_{nonce}.go"));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");
    let run_out = Command::new("go")
        .arg("run")
        .arg(&src_path)
        .output()
        .expect("invoke go run");
    if !run_out.status.success() {
        let stderr = String::from_utf8_lossy(&run_out.stderr);
        if stderr.contains("cannot")
            || stderr.contains("syntax error")
            || stderr.contains("undefined:")
        {
            let _ = std::fs::remove_file(&src_path);
            panic!(
                "emitted Go failed to COMPILE:\n--- stderr ---\n{stderr}\n--- source ---\n{}",
                artifact.source
            );
        }
    }
    let stdout = String::from_utf8_lossy(&run_out.stdout).into_owned();
    let ok = run_out.status.success();
    let _ = std::fs::remove_file(&src_path);
    (ok, stdout)
}

/// P1 — `Dog.new("Rex").speak` prints "Rex".
///
/// ```ruby
/// class Dog
///   def initialize(name); @name = name; end
///   def speak; @name; end
/// end
/// print(Dog.new("Rex").speak)
/// ```
#[test]
fn p1_dog_initialize_and_speak() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    // def initialize(name); @name = name; end
    let init = method_fn(
        "Dog_initialize",
        vec![param("name")],
        vec![ivar_set("@name", param_ref("name"))],
        Expr::NilLit { span: s() },
    );
    // def speak; @name; end   (value position: read the ivar back)
    let speak = method_fn("Dog_speak", vec![], vec![], ivar_ref("@name"));

    let class = class_def(
        "Dog",
        None,
        vec![
            def_method("Dog", "initialize", "Dog_initialize"),
            def_method("Dog", "speak", "Dog_speak"),
        ],
    );
    // Dog.new("Rex").speak → __method__(__new__("Dog", "Rex"), "speak")
    let new_dog = builtin("__new__", vec![str_lit("Dog"), str_lit("Rex")]);
    let speak_call = builtin("__method__", vec![new_dog, str_lit("speak")]);
    let main = vec![class, print_stmt(speak_call)];

    let m = module_from("p1_dog", vec![init, speak], main, &[]);
    let (ok, out) = emit_and_run(&m, "p1");
    assert!(ok, "P1 should exit 0; stdout:\n{out}");
    assert_eq!(out.trim(), "Rex", "initialize-set @name must be readable through speak");
}

/// P2 — inheritance + `super`: `Cat < Animal`; the parent-set `@legs` is
/// visible on the same self after `super`.  Prints "4".
///
/// ```ruby
/// class Animal
///   def initialize; @legs = 4; end
/// end
/// class Cat < Animal
///   def initialize; super; end   # sets @legs via the parent on THIS self
///   def legs; @legs; end
/// end
/// print(Cat.new.legs)
/// ```
#[test]
fn p2_inheritance_and_super() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    // Animal#initialize: @legs = 4
    let animal_init = method_fn(
        "Animal_initialize",
        vec![],
        vec![ivar_set("@legs", int_lit(4))],
        Expr::NilLit { span: s() },
    );
    // Cat#initialize: super("initialize", "Cat")  — re-dispatch on same self
    let cat_init = method_fn(
        "Cat_initialize",
        vec![],
        vec![Stmt::ExprStmt {
            expr: builtin("__super__", vec![str_lit("initialize"), str_lit("Cat")]),
            span: s(),
        }],
        Expr::NilLit { span: s() },
    );
    // Cat#legs: @legs
    let cat_legs = method_fn("Cat_legs", vec![], vec![], ivar_ref("@legs"));

    let animal = class_def(
        "Animal",
        None,
        vec![def_method("Animal", "initialize", "Animal_initialize")],
    );
    let cat = class_def(
        "Cat",
        Some("Animal"),
        vec![
            def_method("Cat", "initialize", "Cat_initialize"),
            def_method("Cat", "legs", "Cat_legs"),
        ],
    );
    let new_cat = builtin("__new__", vec![str_lit("Cat")]);
    let legs_call = builtin("__method__", vec![new_cat, str_lit("legs")]);
    let main = vec![animal, cat, print_stmt(legs_call)];

    let m = module_from("p2_cat", vec![animal_init, cat_init, cat_legs], main, &[]);
    let (ok, out) = emit_and_run(&m, "p2");
    assert!(ok, "P2 should exit 0; stdout:\n{out}");
    assert_eq!(
        out.trim(),
        "4",
        "super must run the parent initialize on the SAME self, setting @legs"
    );
}

/// SECURITY — a class AND method named `constructor` dispatch the USER method
/// via a plain map lookup (never host/JS-engine behaviour), and an
/// UNregistered `__proto__` call hits the clean NoMethodError floor (non-zero
/// exit, no host effect).
#[test]
fn security_reserved_names_are_just_map_keys() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    // A user method literally named `constructor` returns a marker int.
    let ctor = method_fn("Weird_constructor", vec![], vec![], int_lit(99));
    let class = class_def(
        "constructor",
        None,
        vec![def_method("constructor", "constructor", "Weird_constructor")],
    );
    // new on a class named `constructor`, then call the `constructor` method.
    let obj = builtin("__new__", vec![str_lit("constructor")]);
    let call = builtin("__method__", vec![obj, str_lit("constructor")]);
    let main = vec![class, print_stmt(call)];

    let m = module_from("sec_ctor", vec![ctor], main, &[]);
    let (ok, out) = emit_and_run(&m, "sec1");
    assert!(ok, "reserved-name dispatch should run the USER method and exit 0; stdout:\n{out}");
    assert_eq!(out.trim(), "99", "the (class, method) map lookup — not host behaviour — must win");

    // A DIFFERENT program: call an UNregistered `__proto__` on a plain object
    // → the NoMethodError floor (non-zero exit), never a host `__proto__`.
    let obj2 = builtin("__new__", vec![str_lit("Plain")]);
    let bad = builtin("__method__", vec![obj2, str_lit("__proto__")]);
    let m2 = module_from(
        "sec_proto",
        vec![],
        vec![class_def("Plain", None, vec![]), print_stmt(bad)],
        &[],
    );
    let (ok2, out2) = emit_and_run(&m2, "sec2");
    assert!(!ok2, "an unknown method (`__proto__`) must hit the NoMethodError floor (non-zero exit); stdout:\n{out2}");
}

/// CYCLE — a cyclic ancestry (`A < B`, `B < A`) must TERMINATE.  `A.new` with
/// no `initialize` registered anywhere walks the cycle once (seen-guarded) and
/// returns the plain allocation, exiting 0 rather than looping forever.
#[test]
fn cyclic_ancestry_terminates() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let a = class_def("A", Some("B"), vec![]);
    let b = class_def("B", Some("A"), vec![]);
    // A.new — no initialize anywhere; the ancestry walk must not loop.
    let obj = builtin("__new__", vec![str_lit("A")]);
    // Print a marker AFTER the allocation to prove we got past it.
    let main = vec![
        a,
        b,
        Stmt::LetBinding { name: "x".into(), sir_type: None, value: obj, span: s() },
        print_stmt(int_lit(1)),
    ];
    let m = module_from("cyc", vec![], main, &[]);
    let (ok, out) = emit_and_run(&m, "cyc");
    assert!(ok, "cyclic ancestry must terminate and exit 0; stdout:\n{out}");
    assert_eq!(out.trim(), "1", "allocation past a cyclic hierarchy must complete");
}
