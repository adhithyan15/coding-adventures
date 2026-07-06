//! End-to-end proof for the **M6 universal metaprogramming surface** in the
//! Rust backend — Ruby's `Kernel`/`Object` reflection primitives:
//!
//!   * `send`/`__send__`/`public_send` — the first arg NAMES a method; dispatch
//!     RE-ENTERS `call_method` with that name + the remaining args.  The dynamic
//!     name indexes the SAME closed catalogs a direct `recv.meth` call uses, so
//!     an unknown name raises the SAME typed `NoMethodError` — never reflection
//!     on the source-derived string (the [[dynamic-dispatch-rce]] lesson).
//!   * `tap { |x| … }`               — yield the receiver, return the RECEIVER.
//!   * `then`/`yield_self { |x| … }` — yield the receiver, return the BLOCK
//!     result (block-less → the receiver).
//!   * `respond_to?(:m)`             — true iff dispatch resolves `m` (honest:
//!     consults the same catalog/table a real call walks).
//!   * boolean `&`/`|`/`^` on a `true`/`false` receiver — Ruby's EAGER logical
//!     operators.
//!
//! Parity-fill: this surface is already merged in the Python + TypeScript
//! backends (`sir-runtime-oop`); the Rust runtime ports the SAME behaviour so
//! a metaprogramming program produces identical output on every backend.
//!
//! Like the other Rust exec-proofs, this hand-builds SIR modules, emits Rust,
//! compiles with `rustc`, runs the binary, and checks stdout.  `StrLit`
//! interpolation is unsupported, so assertions use INTEGER / bare-string
//! markers the `print` path renders.  A missing `rustc`/linker is a SKIP.

use std::process::Command;

use semantic_ir::{
    Block, CaptureValue, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata,
    Module, Param, ParamKind, RescueClause, Scope, Span, Stmt,
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

fn symlit(v: &str) -> Expr {
    Expr::SymLit { name: v.into(), span: s() }
}

fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}

fn blit(v: bool) -> Expr {
    Expr::BoolLit { value: v, span: s() }
}

fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}

/// `recv.meth(args…)` — the `__method__` dispatch envelope.
fn method(recv: Expr, name: &str, mut args: Vec<Expr>) -> Expr {
    let mut all = vec![recv, slit(name)];
    all.append(&mut args);
    Expr::BuiltinCall { name: "__method__".into(), args: all, effects: EffectSet::PURE, span: s() }
}

/// A no-capture block closure over a top-level block function.
fn block(fn_name: &str) -> Expr {
    Expr::MakeClosure { fn_name: fn_name.into(), captures: Vec::<CaptureValue>::new(), span: s() }
}

fn param(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Param, span: s() }
}

fn print_stmt(expr: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: call("print", vec![expr]),
        span: s(),
    }
}

fn expr_stmt(e: Expr) -> Stmt {
    Stmt::ExprStmt { expr: e, span: s() }
}

/// A top-level block/method body function: `name(params…) { body; value }`.
fn body_fn(name: &str, params: &[&str], body: Vec<Stmt>, value: Expr) -> Function {
    Function {
        name: name.into(),
        params: params
            .iter()
            .map(|p| Param {
                name: (*p).into(),
                kind: ParamKind::Required,
                sir_type: None,
                default: None,
                span: s(),
            })
            .collect(),
        return_type: None,
        captures: vec![],
        body: Block { stmts: body, value, span: s() },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    }
}

/// Assemble a module: a `main` body plus the referenced block/method functions.
fn demo_module(main_stmts: Vec<Stmt>, mut fns: Vec<Function>, features: &[Feature]) -> Module {
    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts: main_stmts, value: Expr::NilLit { span: s() }, span: s() },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    };
    fns.insert(0, main);
    let mut feats = vec![
        Feature::Sequences,
        Feature::Strings,
        Feature::Symbols,
        Feature::Closures,
        Feature::DynamicTyping,
    ];
    feats.extend_from_slice(features);
    Module {
        name: "metaprog_demo".into(),
        manifest: FeatureManifest::from_features(&feats),
        imports: vec![],
        exports: vec![],
        functions: fns,
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("test")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: s(),
    }
}

/// `begin <fault>; rescue <class>; <on_catch> end`.
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

/// Compile emitted Rust and run the binary → `(stdout, exit_success)`.  `None`
/// on a missing linker (skip).  Mirrors the other Rust exec-proofs.
fn compile_and_run(source: &str, tag: &str) -> Option<(String, bool)> {
    let dir = std::env::temp_dir();
    let nonce = format!("{}_{}", std::process::id(), tag);
    let src_path = dir.join(format!("sir_m6_{nonce}.rs"));
    let bin_path = dir.join(format!("sir_m6_{nonce}{}", if cfg!(windows) { ".exe" } else { "" }));
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
        panic!("emitted Rust failed to compile:\n--- stderr ---\n{stderr}\n--- source ---\n{source}");
    }

    let run_out = Command::new(&bin_path).output().expect("run compiled binary");
    let stdout = String::from_utf8_lossy(&run_out.stdout).to_string();
    let ok = run_out.status.success();
    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
    Some((stdout, ok))
}

/// `send(:meth, args…)` dispatches through the SAME closed catalog a direct
/// call uses — for both a primitive and a collection receiver.
///
/// ```ruby
/// puts "hello".send(:upcase)          # => HELLO
/// puts [1, 2, 3].send(:length)        # => 3
/// puts [3, 1, 2].__send__(:sort)      # => [1, 2, 3]
/// ```
#[test]
fn send_dispatches_through_catalog() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let main = vec![
        // "hello".send(:upcase) → HELLO
        print_stmt(method(slit("hello"), "send", vec![symlit("upcase")])),
        // [1,2,3].send(:length) → 3
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3)]), "send", vec![symlit("length")])),
        // [3,1,2].__send__(:sort) → [1, 2, 3]
        print_stmt(method(seq(vec![ilit(3), ilit(1), ilit(2)]), "__send__", vec![symlit("sort")])),
    ];
    let m = demo_module(main, vec![], &[]);
    let out = compile(&m).expect("compile send").source;
    // Security witness: `send` re-enters the explicit `call_method` (no
    // reflective host-name dispatch appears in the emitted runtime).
    assert!(out.contains("fn call_method"), "dispatch must be the explicit call_method");
    let Some((stdout, ok)) = compile_and_run(&out, "send") else { return };
    assert!(ok, "send process should exit 0; stdout {stdout:?}");
    assert_eq!(stdout.lines().collect::<Vec<_>>(), vec!["HELLO", "3", "[1, 2, 3]"], "got {stdout:?}");
}

/// An UNKNOWN `send` target raises a typed `NoMethodError` — the SAME boundary
/// a direct unknown call hits (no silent nil, no reflection).
///
/// ```ruby
/// begin
///   "x".send(:bogus_xyz)
/// rescue NoMethodError
///   puts 9
/// end                                 # => 9
/// ```
#[test]
fn send_unknown_raises_no_method_error() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let main = vec![begin_rescue(
        print_stmt(method(slit("x"), "send", vec![symlit("bogus_xyz")])),
        "NoMethodError",
        vec![print_stmt(ilit(9))],
    )];
    let m = demo_module(main, vec![], &[Feature::Exceptions]);
    let out = compile(&m).expect("compile send-unknown").source;
    let Some((stdout, ok)) = compile_and_run(&out, "send_unknown") else { return };
    assert!(ok, "send-unknown process should exit 0 (caught); stdout {stdout:?}");
    assert_eq!(stdout.lines().collect::<Vec<_>>(), vec!["9"], "got {stdout:?}");
}

/// `tap` runs the block for its side effect and returns the RECEIVER;
/// `then`/`yield_self` return the BLOCK RESULT.
///
/// ```ruby
/// puts 5.tap { |x| puts x }           # => 5 then 5   (block side-effect, then recv)
/// puts 10.then { |x| x + 1 }          # => 11         (block result)
/// puts 10.yield_self { |x| x + 2 }    # => 12
/// ```
#[test]
fn tap_returns_receiver_then_returns_block_result() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let fns = vec![
        // tap block: { |x| puts x }  (side effect, value discarded)
        body_fn("__blk_puts", &["x"], vec![print_stmt(param("x"))], Expr::NilLit { span: s() }),
        // then block: { |x| x + 1 }
        body_fn("__blk_inc", &["x"], vec![], call("+", vec![param("x"), ilit(1)])),
        // yield_self block: { |x| x + 2 }
        body_fn("__blk_inc2", &["x"], vec![], call("+", vec![param("x"), ilit(2)])),
    ];
    let main = vec![
        // 5.tap { |x| puts x }  → prints 5 (block), then the tap value 5.
        print_stmt(method(ilit(5), "tap", vec![block("__blk_puts")])),
        // 10.then { |x| x + 1 }  → 11
        print_stmt(method(ilit(10), "then", vec![block("__blk_inc")])),
        // 10.yield_self { |x| x + 2 }  → 12
        print_stmt(method(ilit(10), "yield_self", vec![block("__blk_inc2")])),
    ];
    let m = demo_module(main, fns, &[]);
    let out = compile(&m).expect("compile tap/then").source;
    let Some((stdout, ok)) = compile_and_run(&out, "tap_then") else { return };
    assert!(ok, "tap/then process should exit 0; stdout {stdout:?}");
    // tap prints the block's `5` first, then the tap expression's own `5`.
    assert_eq!(stdout.lines().collect::<Vec<_>>(), vec!["5", "5", "11", "12"], "got {stdout:?}");
}

/// `respond_to?` is honest: `true` for a catalog method, `false` for an
/// out-of-catalog name (which a real call would `NoMethodError` on).
///
/// ```ruby
/// puts "x".respond_to?(:upcase)       # => true
/// puts "x".respond_to?(:bogus_xyz)    # => false
/// puts [1].respond_to?(:map)          # => true
/// ```
#[test]
fn respond_to_reports_catalog_membership() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let main = vec![
        print_stmt(method(slit("x"), "respond_to?", vec![symlit("upcase")])),
        print_stmt(method(slit("x"), "respond_to?", vec![symlit("bogus_xyz")])),
        print_stmt(method(seq(vec![ilit(1)]), "respond_to?", vec![symlit("map")])),
    ];
    let m = demo_module(main, vec![], &[]);
    let out = compile(&m).expect("compile respond_to?").source;
    let Some((stdout, ok)) = compile_and_run(&out, "respond_to") else { return };
    assert!(ok, "respond_to? process should exit 0; stdout {stdout:?}");
    // The Rust `format` renders booleans as `#t`/`#f` (the runtime's display
    // form), so a `true`/`false` prints `#t`/`#f`.
    assert_eq!(stdout.lines().collect::<Vec<_>>(), vec!["#t", "#f", "#t"], "got {stdout:?}");
}

/// Boolean `&`/`|`/`^` are Ruby's EAGER logical operators on a bool receiver.
///
/// ```ruby
/// puts(true & false)                  # => false
/// puts(false | true)                  # => true
/// puts(true ^ true)                   # => false
/// ```
#[test]
fn boolean_operators_on_bool_receiver() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let main = vec![
        print_stmt(method(blit(true), "&", vec![blit(false)])),
        print_stmt(method(blit(false), "|", vec![blit(true)])),
        print_stmt(method(blit(true), "^", vec![blit(true)])),
    ];
    let m = demo_module(main, vec![], &[]);
    let out = compile(&m).expect("compile bool-ops").source;
    let Some((stdout, ok)) = compile_and_run(&out, "bool_ops") else { return };
    assert!(ok, "bool-ops process should exit 0; stdout {stdout:?}");
    assert_eq!(stdout.lines().collect::<Vec<_>>(), vec!["#f", "#t", "#f"], "got {stdout:?}");
}

/// `send` on a USER INSTANCE routes through the user method table (the same
/// ancestry walk a direct call takes), and `respond_to?` reports a user method.
///
/// ```ruby
/// class Box
///   def initialize(v); @v = v; end
///   def get; @v; end
/// end
/// b = Box.new(42)
/// puts b.send(:get)                   # => 42
/// puts b.respond_to?(:get)            # => true
/// puts b.respond_to?(:missing_xyz)    # => false
/// ```
#[test]
fn send_and_respond_to_on_user_instance() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let init = body_fn(
        "Box_initialize",
        &["v"],
        vec![Stmt::Assign {
            name: "@v".into(),
            scope: Scope::Instance,
            value: param("v"),
            span: s(),
        }],
        Expr::NilLit { span: s() },
    );
    let get = body_fn("Box_get", &[], vec![], Expr::VarRef {
        name: "@v".into(),
        scope: Scope::Instance,
        span: s(),
    });
    let def_method = |cls: &str, m: &str, fnn: &str| {
        expr_stmt(call("__def_method__", vec![slit(cls), slit(m), block(fnn)]))
    };
    let main = vec![
        Stmt::ClassDef { name: "Box".into(), superclass: None, body: vec![], span: s() },
        def_method("Box", "initialize", "Box_initialize"),
        def_method("Box", "get", "Box_get"),
        // Box.new(42).send(:get) → 42  (routes through the user method table)
        print_stmt(method(call("__new__", vec![slit("Box"), ilit(42)]), "send", vec![symlit("get")])),
        // Box.new(42).respond_to?(:get) → true
        print_stmt(method(call("__new__", vec![slit("Box"), ilit(1)]), "respond_to?", vec![symlit("get")])),
        // Box.new(42).respond_to?(:missing_xyz) → false
        print_stmt(method(call("__new__", vec![slit("Box"), ilit(1)]), "respond_to?", vec![symlit("missing_xyz")])),
    ];
    let m = demo_module(
        main,
        vec![init, get],
        &[Feature::Classes, Feature::InstanceVars, Feature::MutableBindings],
    );
    let out = compile(&m).expect("compile instance send").source;
    let Some((stdout, ok)) = compile_and_run(&out, "instance_send") else { return };
    assert!(ok, "instance-send process should exit 0; stdout {stdout:?}");
    assert_eq!(stdout.lines().collect::<Vec<_>>(), vec!["42", "#t", "#f"], "got {stdout:?}");
}
