//! Execution proof for **MX6 mixins** (Ruby `module` + `include` / `extend`)
//! in the Rust backend.
//!
//! The Rust backend has NO native module/mixin model — the MX1 frontend HOISTS
//! every module method to a detached top-level function and registers it via
//! `__def_method__("M", name, closure)` keyed by the MODULE name.  A directive
//! `__include__("C", "M")` records `M` on `C`'s included-module list, and the
//! runtime's MRO-aware resolver (`resolve_instance_method`) follows Ruby's
//! Method Resolution Order: class → its included modules (reverse) →
//! superclass → …  `__extend__("C", "M")` copies `M`'s instance methods into
//! `C`'s class-method table, so they become callable as `C.method` (routed by
//! `__class_method__` → `call_class_method`).  These cases prove that end to
//! end, matching Ruby (and the four already-merged backends):
//!
//!   (a) an included module's instance method is reachable on an instance;
//!   (b) a class's own method SHADOWS an included module's;
//!   (c) a module method shadows the SUPERCLASS's, and a diamond include
//!       resolves ONCE (terminates);
//!   (d) `extend M` makes `M`'s method a CLASS method (`Owner.method`);
//!   (e) a SELF-including module terminates — the resolver's `seen` set
//!       prevents an infinite loop; the unresolved method raises a catchable
//!       `NoMethodError` (proving termination, no hang).
//!
//! Unit tests (in `emit.rs`/`runtime.rs`) assert the emitted *shape*; this
//! test hand-builds SIR modules, emits Rust, compiles with `rustc`, runs the
//! binary, and checks stdout — the SAME strategy as the O5 OOP exec proof.
//! Assertions use INTEGER markers the `print` path renders (the Rust backend
//! does not accept `StrLit` interpolation).
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

fn ivar_set_stmt(name: &str, value: Expr) -> Stmt {
    Stmt::Assign { name: name.into(), scope: Scope::Instance, value, span: s() }
}

fn print_stmt(e: Expr) -> Stmt {
    Stmt::ExprStmt { expr: call("print", vec![e]), span: s() }
}

fn expr_stmt(e: Expr) -> Stmt {
    Stmt::ExprStmt { expr: e, span: s() }
}

/// A hoisted method function: `name(params…) { body_stmts; value }`.
fn method_fn(name: &str, body_stmts: Vec<Stmt>, value: Expr) -> Function {
    Function {
        name: name.into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts: body_stmts, value, span: s() },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    }
}

/// `__def_method__("Owner", "m", <closure>)` — register `m` on `Owner`
/// (a class OR a module name — same builtin either way).
fn def_method_stmt(owner: &str, m: &str, fn_name: &str) -> Stmt {
    expr_stmt(call("__def_method__", vec![slit(owner), slit(m), closure(fn_name)]))
}

fn include_stmt(owner: &str, module: &str) -> Stmt {
    expr_stmt(call("__include__", vec![slit(owner), slit(module)]))
}

fn extend_stmt(owner: &str, module: &str) -> Stmt {
    expr_stmt(call("__extend__", vec![slit(owner), slit(module)]))
}

fn class_def(name: &str, superclass: Option<&str>) -> Stmt {
    Stmt::ClassDef {
        name: name.into(),
        superclass: superclass.map(|s| s.to_string()),
        body: vec![],
        span: s(),
    }
}

fn module_def(name: &str) -> Stmt {
    Stmt::ModuleDef { name: name.into(), body: vec![], span: s() }
}

/// `begin <fault>; rescue <class>; <on_catch> end` — proves an unresolved
/// class-method surfaces as a CATCHABLE typed `NoMethodError` (case e:
/// termination), rather than hanging on a self-including module.
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

/// Assemble a module: extra method functions + a `main` body.
fn mixin_module(mut functions: Vec<Function>, main_stmts: Vec<Stmt>, extra: &[Feature]) -> Module {
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
    let mut feats = vec![
        Feature::Classes,
        Feature::Modules,
        Feature::InstanceVars,
        Feature::Closures,
        Feature::DynamicTyping,
        Feature::Strings,
        Feature::MutableBindings,
    ];
    feats.extend_from_slice(extra);
    Module {
        name: "mixin_demo".into(),
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

fn rustc_available() -> bool {
    Command::new("rustc").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// Compile emitted Rust source and run the binary, returning `(stdout,
/// exit_success)`.  Returns `None` if the host has no usable linker (a skip,
/// never a failure).
fn compile_and_run(source: &str, tag: &str) -> Option<(String, bool)> {
    let dir = std::env::temp_dir();
    let nonce = format!("{}_{}", std::process::id(), tag);
    let src_path = dir.join(format!("sir_mixins_{nonce}.rs"));
    let bin_path =
        dir.join(format!("sir_mixins_{nonce}{}", if cfg!(windows) { ".exe" } else { "" }));
    std::fs::write(&src_path, source).expect("write temp source");

    let mut cmd = Command::new("rustc");
    cmd.arg("--edition").arg("2021").arg("-O");
    if let Ok(linker) = std::env::var("SIR_TEST_RUSTC_LINKER") {
        if !linker.is_empty() {
            cmd.arg("-C").arg(format!("linker={linker}"));
        }
    }
    let compile_out =
        cmd.arg(&src_path).arg("-o").arg(&bin_path).output().expect("invoke rustc");
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

/// (a) A module's instance method is found through an including class's MRO.
///
/// ```ruby
/// module Greet
///   def hello; 1; end
/// end
/// class Person
///   include Greet
/// end
/// print(Person.new.hello)   # => 1
/// ```
#[test]
fn included_module_method_is_callable() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let hello = method_fn("Greet_hello", vec![], ilit(1));
    let main = vec![
        module_def("Greet"),
        def_method_stmt("Greet", "hello", "Greet_hello"),
        class_def("Person", None),
        include_stmt("Person", "Greet"),
        print_stmt(method_call(call("__new__", vec![slit("Person")]), "hello", vec![])),
    ];
    let m = mixin_module(vec![hello], main, &[]);
    let src = compile(&m).expect("compile to Rust").source;
    let Some((out, ok)) = compile_and_run(&src, "a") else { return };
    assert!(ok, "case (a) should exit 0; stdout:\n{out}");
    assert_eq!(out.trim(), "1", "a method from an included module must be reachable on an instance");
}

/// (b) A class-defined method SHADOWS the module's (class-first MRO).
///
/// ```ruby
/// module M; def kind; 1; end; end
/// class C; include M; def kind; 2; end; end
/// print(C.new.kind)   # => 2
/// ```
#[test]
fn class_method_shadows_module_method() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let m_kind = method_fn("M_kind", vec![], ilit(1));
    let c_kind = method_fn("C_kind", vec![], ilit(2));
    let main = vec![
        module_def("M"),
        def_method_stmt("M", "kind", "M_kind"),
        class_def("C", None),
        include_stmt("C", "M"),
        def_method_stmt("C", "kind", "C_kind"),
        print_stmt(method_call(call("__new__", vec![slit("C")]), "kind", vec![])),
    ];
    let m = mixin_module(vec![m_kind, c_kind], main, &[]);
    let src = compile(&m).expect("compile to Rust").source;
    let Some((out, ok)) = compile_and_run(&src, "b") else { return };
    assert!(ok, "case (b) should exit 0; stdout:\n{out}");
    assert_eq!(out.trim(), "2", "the class's own method must shadow the included module's");
}

/// (c) A module method shadows the SUPERCLASS's (module precedes super in the
/// MRO), and a DIAMOND include (the same module included twice) resolves ONCE
/// — the walk's `seen` set must visit it once and terminate.
///
/// ```ruby
/// module Shared; def tag; 10; end; end
/// class Base; def tag; 20; end; end
/// class Derived < Base
///   include Shared
///   include Shared    # diamond / duplicate — resolves once
/// end
/// print(Derived.new.tag)   # => 10
/// ```
#[test]
fn module_shadows_super_and_diamond_resolves_once() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let shared_tag = method_fn("Shared_tag", vec![], ilit(10));
    let base_tag = method_fn("Base_tag", vec![], ilit(20));
    let main = vec![
        module_def("Shared"),
        def_method_stmt("Shared", "tag", "Shared_tag"),
        class_def("Base", None),
        def_method_stmt("Base", "tag", "Base_tag"),
        class_def("Derived", Some("Base")),
        include_stmt("Derived", "Shared"),
        include_stmt("Derived", "Shared"),
        print_stmt(method_call(call("__new__", vec![slit("Derived")]), "tag", vec![])),
    ];
    let m = mixin_module(vec![shared_tag, base_tag], main, &[]);
    let src = compile(&m).expect("compile to Rust").source;
    let Some((out, ok)) = compile_and_run(&src, "c") else { return };
    assert!(ok, "case (c) should exit 0 (diamond must terminate); stdout:\n{out}");
    assert_eq!(out.trim(), "10", "an included module must shadow the superclass's method");
}

/// (d) `extend M` makes `M`'s instance method a CLASS method callable as
/// `Owner.method`.
///
/// ```ruby
/// module Counting; def total; 42; end; end
/// class Registry; extend Counting; end
/// print(Registry.total)   # => 42
/// ```
#[test]
fn extend_makes_module_method_a_class_method() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let total = method_fn("Counting_total", vec![], ilit(42));
    let call_cm = call("__class_method__", vec![slit("Registry"), slit("total")]);
    let main = vec![
        module_def("Counting"),
        def_method_stmt("Counting", "total", "Counting_total"),
        class_def("Registry", None),
        extend_stmt("Registry", "Counting"),
        print_stmt(call_cm),
    ];
    let m = mixin_module(vec![total], main, &[]);
    let src = compile(&m).expect("compile to Rust").source;
    let Some((out, ok)) = compile_and_run(&src, "d") else { return };
    assert!(ok, "case (d) should exit 0; stdout:\n{out}");
    assert_eq!(out.trim(), "42", "extend must expose the module method as a class method");
}

/// (e) A SELF-including module TERMINATES: resolving an undefined method on a
/// module that includes itself must not loop forever — the resolver's `seen`
/// set caps the walk, and the unresolved method raises a catchable
/// `NoMethodError`.  We prove termination by rescuing that error and printing
/// a marker; a hang would time the test out instead.
///
/// ```ruby
/// module Loopy
///   include Loopy          # self-include
/// end
/// class Host; include Loopy; end
/// begin
///   Host.new.nope          # undefined → NoMethodError (must terminate)
/// rescue NoMethodError
///   print(99)
/// end
/// ```
#[test]
fn self_including_module_terminates() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let fault = expr_stmt(method_call(call("__new__", vec![slit("Host")]), "nope", vec![]));
    let main = vec![
        module_def("Loopy"),
        include_stmt("Loopy", "Loopy"), // self-include
        class_def("Host", None),
        include_stmt("Host", "Loopy"),
        begin_rescue(fault, "NoMethodError", vec![print_stmt(ilit(99))]),
    ];
    // The rescue makes the module use `Feature::Exceptions`.
    let m = mixin_module(vec![], main, &[Feature::Exceptions]);
    let src = compile(&m).expect("compile to Rust").source;
    let Some((out, ok)) = compile_and_run(&src, "e") else { return };
    assert!(ok, "case (e) should exit 0 (must terminate, not hang); stdout:\n{out}");
    assert_eq!(out.trim(), "99", "a self-including module must terminate and raise NoMethodError");
}

/// Bonus (mirrors the Go MX-E case): a mixed-in method reads an `@ivar` set by
/// the including class's own `initialize`, proving the module method runs on
/// the SAME receiver (the shared self-stack).
///
/// ```ruby
/// module Named; def name; @name; end; end
/// class Widget; include Named; def initialize; @name = 7; end; end
/// print(Widget.new.name)   # => 7
/// ```
#[test]
fn module_method_reads_including_class_ivar() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let name = method_fn("Named_name", vec![], ivar_ref("@name"));
    let init = method_fn(
        "Widget_initialize",
        vec![ivar_set_stmt("@name", ilit(7))],
        Expr::NilLit { span: s() },
    );
    let main = vec![
        module_def("Named"),
        def_method_stmt("Named", "name", "Named_name"),
        class_def("Widget", None),
        include_stmt("Widget", "Named"),
        def_method_stmt("Widget", "initialize", "Widget_initialize"),
        print_stmt(method_call(call("__new__", vec![slit("Widget")]), "name", vec![])),
    ];
    let m = mixin_module(vec![name, init], main, &[]);
    let src = compile(&m).expect("compile to Rust").source;
    let Some((out, ok)) = compile_and_run(&src, "f") else { return };
    assert!(ok, "bonus case should exit 0; stdout:\n{out}");
    assert_eq!(out.trim(), "7", "a mixed-in method must run on the same self and see its @ivars");
}
