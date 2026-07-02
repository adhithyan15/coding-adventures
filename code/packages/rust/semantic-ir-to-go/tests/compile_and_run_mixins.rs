//! Execution proof for MX5 mixins (Ruby `module` + `include` / `extend`):
//! hand-build a SIR module, emit Go, run it with `go run`, and assert the
//! observable behaviour matches Ruby's mixin semantics — the module method is
//! found through the class's MRO, a class-defined method SHADOWS the module's,
//! a diamond include resolves ONCE, and `extend` makes a module method a CLASS
//! method callable as `Owner.method`.
//!
//! The Go backend has NO native module/mixin model — the frontend HOISTS every
//! module method to a detached top-level function and registers it via
//! `__def_method__("M", name, closure)` keyed by the MODULE name.  A directive
//! `__include__("C", "M")` records `M` on `C`'s included-module list, and the
//! runtime's method-resolution walk (`_sir_resolve_instance_method`) follows
//! Ruby's MRO: class → its included modules (reverse) → superclass → …  These
//! cases prove that end to end.
//!
//! A missing `go` toolchain logs a skip rather than reddening the build
//! (mirrors `compile_and_run_oop.rs`).

use std::process::Command;

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Param, Scope,
    Span, Stmt,
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

/// A hoisted method as a top-level function (no captures; methods use the
/// self-stack, not captures).
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

fn closure_of(fn_name: &str) -> Expr {
    Expr::MakeClosure { fn_name: fn_name.into(), captures: vec![], span: s() }
}

/// `__def_method__("Owner", "meth", <closure>)` — register `meth` on `Owner`
/// (a class OR a module name — same builtin either way).
fn def_method(owner: &str, meth: &str, fn_name: &str) -> Stmt {
    Stmt::ExprStmt {
        expr: builtin("__def_method__", vec![str_lit(owner), str_lit(meth), closure_of(fn_name)]),
        span: s(),
    }
}

fn include_stmt(owner: &str, module: &str) -> Stmt {
    Stmt::ExprStmt {
        expr: builtin("__include__", vec![str_lit(owner), str_lit(module)]),
        span: s(),
    }
}

fn extend_stmt(owner: &str, module: &str) -> Stmt {
    Stmt::ExprStmt {
        expr: builtin("__extend__", vec![str_lit(owner), str_lit(module)]),
        span: s(),
    }
}

fn class_def(name: &str, superclass: Option<&str>, body: Vec<Stmt>) -> Stmt {
    Stmt::ClassDef { name: name.into(), superclass: superclass.map(|s| s.to_string()), body, span: s() }
}

fn module_def(name: &str, body: Vec<Stmt>) -> Stmt {
    Stmt::ModuleDef { name: name.into(), body, span: s() }
}

/// Assemble a module from top-level functions + a `main` body.
fn module_from(name: &str, mut functions: Vec<Function>, main_stmts: Vec<Stmt>) -> Module {
    functions.push(method_fn("main", vec![], main_stmts, Expr::NilLit { span: s() }));
    let fs = vec![
        Feature::Classes,
        Feature::Modules,
        Feature::InstanceVars,
        Feature::Strings,
        Feature::MutableBindings,
        Feature::Closures,
        Feature::DynamicTyping,
    ];
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
    Command::new("go").arg("version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// Emit `m` to Go, run it, return `(success, stdout)`.  A COMPILE error panics
/// loudly; a runtime non-zero exit returns the flag.
fn emit_and_run(m: &Module, nonce: &str) -> (bool, String) {
    let artifact = compile(m).expect("module should compile to Go source");
    let dir = std::env::temp_dir();
    let pid = std::process::id();
    let src_path = dir.join(format!("sir_go_mixins_{pid}_{nonce}.go"));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");
    let run_out =
        Command::new("go").arg("run").arg(&src_path).output().expect("invoke go run");
    if !run_out.status.success() {
        let stderr = String::from_utf8_lossy(&run_out.stderr);
        if stderr.contains("cannot") || stderr.contains("syntax error") || stderr.contains("undefined:") {
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

/// MX-A — a module method is found through an including class's MRO.
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
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let hello = method_fn("Greet_hello", vec![], vec![], int_lit(1));
    let module = module_def("Greet", vec![def_method("Greet", "hello", "Greet_hello")]);
    let class = class_def("Person", None, vec![include_stmt("Person", "Greet")]);
    let new_p = builtin("__new__", vec![str_lit("Person")]);
    let call = builtin("__method__", vec![new_p, str_lit("hello")]);
    let main = vec![module, class, print_stmt(call)];

    let m = module_from("mx_a", vec![hello], main);
    let (ok, out) = emit_and_run(&m, "a");
    assert!(ok, "MX-A should exit 0; stdout:\n{out}");
    assert_eq!(out.trim(), "1", "a method from an included module must be reachable on an instance");
}

/// MX-B — a class-defined method SHADOWS the module's (class-first MRO).
///
/// ```ruby
/// module M
///   def kind; 1; end   # module says 1
/// end
/// class C
///   include M
///   def kind; 2; end   # class says 2 — class wins
/// end
/// print(C.new.kind)    # => 2
/// ```
#[test]
fn class_method_shadows_module_method() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let m_kind = method_fn("M_kind", vec![], vec![], int_lit(1));
    let c_kind = method_fn("C_kind", vec![], vec![], int_lit(2));
    let module = module_def("M", vec![def_method("M", "kind", "M_kind")]);
    let class = class_def(
        "C",
        None,
        vec![include_stmt("C", "M"), def_method("C", "kind", "C_kind")],
    );
    let new_c = builtin("__new__", vec![str_lit("C")]);
    let call = builtin("__method__", vec![new_c, str_lit("kind")]);
    let main = vec![module, class, print_stmt(call)];

    let m = module_from("mx_b", vec![m_kind, c_kind], main);
    let (ok, out) = emit_and_run(&m, "b");
    assert!(ok, "MX-B should exit 0; stdout:\n{out}");
    assert_eq!(out.trim(), "2", "the class's own method must shadow the included module's");
}

/// MX-C — a module method shadows the SUPERCLASS's (module precedes super in
/// the MRO), and a diamond include resolves ONCE (terminates).
///
/// ```ruby
/// module Shared
///   def tag; 10; end
/// end
/// class Base
///   def tag; 20; end          # superclass method
/// end
/// class Derived < Base
///   include Shared            # module beats the superclass
///   include Shared            # diamond / duplicate include — resolves once
/// end
/// print(Derived.new.tag)      # => 10
/// ```
#[test]
fn module_shadows_super_and_diamond_resolves_once() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let shared_tag = method_fn("Shared_tag", vec![], vec![], int_lit(10));
    let base_tag = method_fn("Base_tag", vec![], vec![], int_lit(20));
    let module = module_def("Shared", vec![def_method("Shared", "tag", "Shared_tag")]);
    let base = class_def("Base", None, vec![def_method("Base", "tag", "Base_tag")]);
    // Two includes of the same module (the diamond / duplicate) — the walk's
    // `seen` set must visit `Shared` once and terminate.
    let derived = class_def(
        "Derived",
        Some("Base"),
        vec![include_stmt("Derived", "Shared"), include_stmt("Derived", "Shared")],
    );
    let new_d = builtin("__new__", vec![str_lit("Derived")]);
    let call = builtin("__method__", vec![new_d, str_lit("tag")]);
    let main = vec![module, base, derived, print_stmt(call)];

    let m = module_from("mx_c", vec![shared_tag, base_tag], main);
    let (ok, out) = emit_and_run(&m, "c");
    assert!(ok, "MX-C should exit 0 (diamond must terminate); stdout:\n{out}");
    assert_eq!(out.trim(), "10", "an included module must shadow the superclass's method");
}

/// MX-D — `extend M` makes `M`'s instance method a CLASS method callable as
/// `Owner.method`.
///
/// ```ruby
/// module Counting
///   def total; 42; end
/// end
/// class Registry
///   extend Counting
/// end
/// print(Registry.total)   # => 42
/// ```
#[test]
fn extend_makes_module_method_a_class_method() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let total = method_fn("Counting_total", vec![], vec![], int_lit(42));
    let module = module_def("Counting", vec![def_method("Counting", "total", "Counting_total")]);
    let class = class_def("Registry", None, vec![extend_stmt("Registry", "Counting")]);
    // Registry.total — a class-method call on a Const receiver.
    let call = builtin("__class_method__", vec![str_lit("Registry"), str_lit("total")]);
    let main = vec![module, class, print_stmt(call)];

    let m = module_from("mx_d", vec![total], main);
    let (ok, out) = emit_and_run(&m, "d");
    assert!(ok, "MX-D should exit 0; stdout:\n{out}");
    assert_eq!(out.trim(), "42", "extend must expose the module method as a class method");
}

/// MX-E — a module can carry state through the current-self stack: a mixed-in
/// method reads an `@ivar` set by the including class's own method, proving the
/// module method runs on the SAME receiver (the self-stack is shared).
///
/// ```ruby
/// module Named
///   def name; @name; end
/// end
/// class Widget
///   include Named
///   def initialize; @name = 7; end
/// end
/// print(Widget.new.name)   # => 7
/// ```
#[test]
fn module_method_reads_including_class_ivar() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let name = method_fn("Named_name", vec![], vec![], ivar_ref("@name"));
    let init = method_fn(
        "Widget_initialize",
        vec![],
        vec![ivar_set("@name", int_lit(7))],
        Expr::NilLit { span: s() },
    );
    let module = module_def("Named", vec![def_method("Named", "name", "Named_name")]);
    let class = class_def(
        "Widget",
        None,
        vec![include_stmt("Widget", "Named"), def_method("Widget", "initialize", "Widget_initialize")],
    );
    let new_w = builtin("__new__", vec![str_lit("Widget")]);
    let call = builtin("__method__", vec![new_w, str_lit("name")]);
    let main = vec![module, class, print_stmt(call)];

    let m = module_from("mx_e", vec![name, init], main);
    let (ok, out) = emit_and_run(&m, "e");
    assert!(ok, "MX-E should exit 0; stdout:\n{out}");
    assert_eq!(out.trim(), "7", "a mixed-in method must run on the same self and see its @ivars");
}
