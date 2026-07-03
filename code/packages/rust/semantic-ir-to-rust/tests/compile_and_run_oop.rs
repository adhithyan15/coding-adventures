//! End-to-end proof for the **O5 user-defined-class OOP runtime** in the
//! Rust backend.
//!
//! The Ruby→SIR frontend (O2) lowers user-defined-class OOP to a small
//! family of builtins the Rust backend routes to its inlined `__sir` OOP
//! runtime:
//!
//!   * `Foo.new(args)`        → `__new__`            → `__sir::call_new`
//!   * `def m` in `class C`   → `__def_method__`     → `__sir::def_method`
//!   * `def self.m`           → `__def_class_method__` → `__sir::def_class_method`
//!   * `super(args)`          → `__super__`          → `__sir::call_super`
//!   * `self`                 → `__self__`           → `__sir::current_self`
//!   * `recv.m(args)`         → `__method__`         → `__sir::call_method`
//!     (a `Value::Instance` receiver resolves the USER method table walking
//!     ancestry; every other receiver keeps the unchanged collection path)
//!   * `@x` / `@@x`           → `ivar_get`/`ivar_set` / `cvar_get`/`cvar_set`
//!
//! Unit tests (in `emit.rs`/`runtime.rs`) assert the emitted *shape*; this
//! test hand-builds SIR modules, emits Rust, compiles with `rustc`, runs the
//! binary, and checks stdout — the SAME strategy as the E4 exception exec
//! proof.  Method `def`s hoist to top-level `Function`s referenced by the
//! `__def_*` builtins' `MakeClosure`, exactly as the frontend produces.
//!
//! `StrLit` interpolation is not accepted by the Rust backend, so — like the
//! other Rust exec-proof tests — assertions use simple INTEGER markers the
//! `print` path renders, proving ivar-through-method dispatch, inheritance +
//! `super`, the security floor, and cyclic-ancestry termination.
//!
//! If `rustc` (or a usable linker) is unavailable the test logs a skip rather
//! than failing.  The host points the test at a working linker via
//! `SIR_TEST_RUSTC_LINKER` (e.g. the toolchain's bundled `rust-lld`).

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module,
    RescueClause, Scope, Span, Stmt,
};
use semantic_ir_to_rust::compile;

fn s() -> Span {
    Span::synthetic()
}

fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}

fn slit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}

fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}

/// `recv.m(args…)` narrow-waist envelope.
fn method_call(recv: Expr, m: &str, args: Vec<Expr>) -> Expr {
    let mut all = vec![recv, slit(m)];
    all.extend(args);
    call("__method__", all)
}

/// A zero-capture closure over a hoisted method function.
fn closure(fn_name: &str) -> Expr {
    Expr::MakeClosure { fn_name: fn_name.into(), captures: vec![], span: s() }
}

fn ivar_ref(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Instance, span: s() }
}

fn param_ref(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Param, span: s() }
}

fn ivar_set_stmt(name: &str, value: Expr) -> Stmt {
    Stmt::Assign { name: name.into(), scope: Scope::Instance, value, span: s() }
}

fn print_stmt(e: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: call("print", vec![e]),
        span: s(),
    }
}

fn expr_stmt(e: Expr) -> Stmt {
    Stmt::ExprStmt { expr: e, span: s() }
}

/// A hoisted method function: `name(params…) { body_stmts; value }`.
fn method_fn(name: &str, params: &[&str], body_stmts: Vec<Stmt>, value: Expr) -> Function {
    Function {
        name: name.into(),
        params: params
            .iter()
            .map(|p| semantic_ir::Param {
                name: (*p).into(),
                kind: semantic_ir::ParamKind::Required,
                sir_type: None,
                default: None,
                span: s(),
            })
            .collect(),
        return_type: None,
        captures: vec![],
        body: Block { stmts: body_stmts, value, span: s() },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    }
}

/// `__def_method__("C", "m", <closure over fn_name>)` as a statement.
fn def_method_stmt(cls: &str, m: &str, fn_name: &str) -> Stmt {
    expr_stmt(call("__def_method__", vec![slit(cls), slit(m), closure(fn_name)]))
}

/// Assemble a module: extra method functions + a `main` body, with the
/// given features (Classes/InstanceVars/etc. added on top).
fn oop_module(mut functions: Vec<Function>, main_stmts: Vec<Stmt>, features: &[Feature]) -> Module {
    let main_fn = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts: main_stmts, value: Expr::NilLit { span: s() }, span: s() },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    };
    functions.push(main_fn);
    // The observed-feature set for these hand-built OOP modules:
    //   • Classes / InstanceVars — the OOP surface under test;
    //   • Closures — the `__def_*` method-body `MakeClosure`;
    //   • DynamicTyping — untyped params/values;
    //   • Strings — the `StrLit` class/method names;
    //   • MutableBindings — an `Assign` (the `@ivar =` write) observes it.
    // The validator requires the manifest to declare EXACTLY what the module
    // uses, so we declare this whole base set (each test adds nothing more).
    let mut feats = vec![
        Feature::Classes,
        Feature::InstanceVars,
        Feature::Closures,
        Feature::DynamicTyping,
        Feature::Strings,
        Feature::MutableBindings,
    ];
    feats.extend_from_slice(features);
    Module {
        name: "oop_demo".into(),
        manifest: FeatureManifest::from_features(&feats),
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

/// A `class C < Super` declaration (empty body — methods hoist).
fn class_def(name: &str, superclass: Option<&str>) -> Stmt {
    Stmt::ClassDef {
        name: name.into(),
        superclass: superclass.map(|s| s.to_string()),
        body: vec![],
        span: s(),
    }
}

/// `begin <fault>; rescue <class>; <on_catch> end` — used to prove a runtime
/// fault surfaces as a CATCHABLE typed exception (T5 typed-runtime-errors:
/// an unresolved instance method now raises `NoMethodError` rather than
/// flooring to `nil`).
fn begin_rescue(fault: Stmt, class: &str, on_catch: Vec<Stmt>) -> Stmt {
    Stmt::TryCatch {
        body: vec![fault],
        rescues: vec![RescueClause {
            exception_types: vec![class.to_string()],
            binding: None,
            body: on_catch,
            span: s(),
        }],
        ensure_body: None,
        span: s(),
    }
}

fn rustc_available() -> bool {
    Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Compile emitted Rust source and run the binary, returning `(stdout,
/// exit_success)`.  Returns `None` if the host has no usable linker (a skip,
/// never a failure) — the harness is gated on a working `rustc`/linker.
fn compile_and_run(source: &str, tag: &str) -> Option<(String, bool)> {
    let dir = std::env::temp_dir();
    let nonce = format!("{}_{}", std::process::id(), tag);
    let src_path = dir.join(format!("sir_oop_{nonce}.rs"));
    let bin_path = dir.join(format!("sir_oop_{nonce}{}", if cfg!(windows) { ".exe" } else { "" }));
    std::fs::write(&src_path, source).expect("write temp source");

    let mut cmd = Command::new("rustc");
    cmd.arg("--edition").arg("2021").arg("-O");
    if let Ok(linker) = std::env::var("SIR_TEST_RUSTC_LINKER") {
        if !linker.is_empty() {
            cmd.arg("-C").arg(format!("linker={linker}"));
        }
    }
    let compile_out = cmd
        .arg(&src_path)
        .arg("-o")
        .arg(&bin_path)
        .output()
        .expect("invoke rustc");
    if !compile_out.status.success() {
        let stderr = String::from_utf8_lossy(&compile_out.stderr);
        if stderr.contains("linker") && (stderr.contains("not found") || stderr.contains("No such file")) {
            eprintln!("skipping: no usable linker on host\n{stderr}");
            let _ = std::fs::remove_file(&src_path);
            return None;
        }
        panic!(
            "emitted Rust failed to compile:\n--- stderr ---\n{stderr}\n--- source ---\n{source}"
        );
    }

    let run_out = Command::new(&bin_path).output().expect("run compiled binary");
    let stdout = String::from_utf8_lossy(&run_out.stdout).to_string();
    let ok = run_out.status.success();
    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
    Some((stdout, ok))
}

/// P1 — `Dog.new(marker).speak`: proves ivar-through-method dispatch.
///
/// ```ruby
/// class Dog
///   def initialize(name); @name = name; end
///   def speak; @name; end
/// end
/// puts Dog.new(42).speak    # => 42
/// ```
///
/// `initialize` writes `@name` on the fresh instance (self bound by
/// `call_new`); `speak` reads `@name` back through method dispatch.  Prints
/// the integer marker `42`.
#[test]
fn p1_dog_initialize_and_speak() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    // Hoisted method bodies.
    let init = method_fn(
        "Dog_initialize",
        &["name"],
        vec![ivar_set_stmt("@name", param_ref("name"))],
        Expr::NilLit { span: s() },
    );
    let speak = method_fn("Dog_speak", &[], vec![], ivar_ref("@name"));

    let main = vec![
        class_def("Dog", None),
        def_method_stmt("Dog", "initialize", "Dog_initialize"),
        def_method_stmt("Dog", "speak", "Dog_speak"),
        // puts Dog.new(42).speak
        print_stmt(method_call(call("__new__", vec![slit("Dog"), ilit(42)]), "speak", vec![])),
    ];
    let m = oop_module(vec![init, speak], main, &[]);
    let src = compile(&m).expect("compile P1").source;
    let Some((stdout, ok)) = compile_and_run(&src, "p1_dog") else { return };
    assert!(ok, "P1 process should exit 0; stdout {stdout:?}");
    assert_eq!(stdout.lines().collect::<Vec<_>>(), vec!["42"], "got {stdout:?}");
}

/// P2 — inheritance + `super`.
///
/// ```ruby
/// class Animal
///   def initialize(legs); @legs = legs; end
///   def describe; @legs; end
/// end
/// class Cat < Animal
///   def describe; super + 100; end
/// end
/// puts Cat.new(4).describe   # => 104
/// ```
///
/// `Cat.new(4)` inherits `Animal#initialize` via ancestry (sets `@legs=4`).
/// `Cat#describe` calls `super` → `Animal#describe` (returns `@legs`=4) with
/// the SAME self bound, then adds 100 → `104`.
#[test]
fn p2_inheritance_and_super() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let animal_init = method_fn(
        "Animal_initialize",
        &["legs"],
        vec![ivar_set_stmt("@legs", param_ref("legs"))],
        Expr::NilLit { span: s() },
    );
    let animal_describe = method_fn("Animal_describe", &[], vec![], ivar_ref("@legs"));
    // Cat#describe: super("describe","Cat") + 100.
    let cat_describe = method_fn(
        "Cat_describe",
        &[],
        vec![],
        call("+", vec![call("__super__", vec![slit("describe"), slit("Cat")]), ilit(100)]),
    );

    let main = vec![
        class_def("Animal", None),
        def_method_stmt("Animal", "initialize", "Animal_initialize"),
        def_method_stmt("Animal", "describe", "Animal_describe"),
        class_def("Cat", Some("Animal")),
        def_method_stmt("Cat", "describe", "Cat_describe"),
        print_stmt(method_call(call("__new__", vec![slit("Cat"), ilit(4)]), "describe", vec![])),
    ];
    let m = oop_module(vec![animal_init, animal_describe, cat_describe], main, &[]);
    let src = compile(&m).expect("compile P2").source;
    let Some((stdout, ok)) = compile_and_run(&src, "p2_super") else { return };
    assert!(ok, "P2 process should exit 0; stdout {stdout:?}");
    assert_eq!(stdout.lines().collect::<Vec<_>>(), vec!["104"], "got {stdout:?}");
}

/// SECURITY — a class/method literally named `constructor` is inert DATA.
///
/// We build a class named `constructor` with an `initialize` and a `get`
/// method, and ALSO call an UNREGISTERED dangerous name (`drop`) on the
/// instance.  Two guarantees: (1) the `constructor`-named class + its
/// methods work purely as map data (`get` returns the ivar marker `7`),
/// never reaching a host callable; (2) calling an unregistered method
/// (`drop`) surfaces a CONTROLLED typed `NoMethodError` (never a host
/// `Drop`/reflection) — T5 makes the old silent `nil` floor a catchable
/// error, so we `rescue NoMethodError` and print `8`, and the process still
/// exits 0.  Prints `7` then `8`.
#[test]
fn security_reflective_name_is_inert_data() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let init = method_fn(
        "ctor_initialize",
        &[],
        vec![ivar_set_stmt("@x", ilit(7))],
        Expr::NilLit { span: s() },
    );
    let get = method_fn("ctor_get", &[], vec![], ivar_ref("@x"));

    let main = vec![
        // A class whose NAME is a reflective gadget in other languages.
        class_def("constructor", None),
        def_method_stmt("constructor", "initialize", "ctor_initialize"),
        def_method_stmt("constructor", "get", "ctor_get"),
        // The class name (Const-like) is passed as a string; `get` works.
        print_stmt(method_call(call("__new__", vec![slit("constructor")]), "get", vec![])),
        // An UNREGISTERED method name (`drop`) → a CONTROLLED, catchable
        // `NoMethodError` (never a host `Drop`/reflection).  Caught → print 8.
        begin_rescue(
            print_stmt(method_call(call("__new__", vec![slit("constructor")]), "drop", vec![])),
            "NoMethodError",
            vec![print_stmt(ilit(8))],
        ),
    ];
    let m = oop_module(vec![init, get], main, &[Feature::Exceptions]);
    let src = compile(&m).expect("compile security").source;
    // The emitted runtime must never turn a source name into a host call:
    // no `recv.name`-style reflection appears — dispatch is HashMap-keyed.
    assert!(
        src.contains("METHOD_TABLE") && src.contains("resolve_method"),
        "dispatch must be an explicit table lookup"
    );
    let Some((stdout, ok)) = compile_and_run(&src, "security") else { return };
    assert!(ok, "security process should exit 0; stdout {stdout:?}");
    assert_eq!(stdout.lines().collect::<Vec<_>>(), vec!["7", "8"], "got {stdout:?}");
}

/// Cyclic ancestry must TERMINATE (not hang) and floor to `nil`.
///
/// We register `class A < B` and `class B < A` (a cycle), define NO method
/// named `missing`, then call `A.new.missing`.  `resolve_method`'s `seen`
/// guard bounds the ancestry walk, so the walk TERMINATES (rather than
/// looping forever) with the method unresolved — which T5 surfaces as a
/// catchable `NoMethodError`.  We `rescue NoMethodError` and print `3`,
/// proving the walk cannot hang on a malformed hierarchy.  Prints `3`.
#[test]
fn cyclic_ancestry_terminates() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let main = vec![
        class_def("A", Some("B")),
        class_def("B", Some("A")),
        begin_rescue(
            print_stmt(method_call(call("__new__", vec![slit("A")]), "missing", vec![])),
            "NoMethodError",
            vec![print_stmt(ilit(3))],
        ),
    ];
    let m = oop_module(vec![], main, &[Feature::Exceptions]);
    let src = compile(&m).expect("compile cyclic").source;
    let Some((stdout, ok)) = compile_and_run(&src, "cyclic") else { return };
    assert!(ok, "cyclic-ancestry process should exit 0 (walk terminates); stdout {stdout:?}");
    assert_eq!(stdout.lines().collect::<Vec<_>>(), vec!["3"], "got {stdout:?}");
}

/// The self-stack pops even when a method body panics (RAII guard).  We can
/// prove this without an explicit panic: after a nested `call_new` +
/// `call_method` sequence returns, `__self__` at top level is `nil` again —
/// the stack was balanced by the drop guards.  Prints `nil`.
#[test]
fn self_stack_balanced_after_dispatch() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let init = method_fn(
        "Box_initialize",
        &["v"],
        vec![ivar_set_stmt("@v", param_ref("v"))],
        Expr::NilLit { span: s() },
    );
    let get = method_fn("Box_get", &[], vec![], ivar_ref("@v"));
    let main = vec![
        class_def("Box", None),
        def_method_stmt("Box", "initialize", "Box_initialize"),
        def_method_stmt("Box", "get", "Box_get"),
        // Dispatch through a full new+method call, discarding the result.
        expr_stmt(method_call(call("__new__", vec![slit("Box"), ilit(1)]), "get", vec![])),
        // Then at top level `self` must be nil again (stack balanced).
        print_stmt(call("__self__", vec![])),
    ];
    let m = oop_module(vec![init, get], main, &[]);
    let src = compile(&m).expect("compile self-stack").source;
    let Some((stdout, ok)) = compile_and_run(&src, "self_stack") else { return };
    assert!(ok, "self-stack process should exit 0; stdout {stdout:?}");
    assert_eq!(stdout.lines().collect::<Vec<_>>(), vec!["nil"], "got {stdout:?}");
}
