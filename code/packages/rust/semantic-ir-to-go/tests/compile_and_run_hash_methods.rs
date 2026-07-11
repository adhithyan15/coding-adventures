//! End-to-end proof for the **Ruby `Hash` method catalog** in the Go
//! backend — the newly-added `merge`, `to_a`, `dig`, `invert`, `delete`,
//! `store`/`[]=`, `clear`, and the block methods `each_key` / `each_value`
//! (plus the pre-existing `each_pair`) and the transforming block methods
//! `transform_values` / `transform_keys`, bringing the Go runtime to parity
//! with the Python/TS `sir-runtime-oop` catalogs.
//!
//! Like `compile_and_run_coll_methods.rs`, this hand-builds SIR modules that
//! exercise the catalog, emits Go, runs it under a real `go run`, and diffs
//! stdout against the values the Python/TS reference backends produce for the
//! SAME operations.  Symbol keys (`{a: 1}`) are used so the printed form
//! (`a`, from `Symbol.Name`) matches Ruby's surface.
//!
//! Gated on `go version`: a missing toolchain logs a skip rather than
//! reddening the build (mirrors the sibling exec tests).

use std::process::Command;

use semantic_ir::nodes::MapEntry;
use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Scope,
    Span, Stmt,
};
use semantic_ir_to_go::compile;

fn s() -> Span {
    Span::synthetic()
}

fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}

fn sym(name: &str) -> Expr {
    Expr::SymLit { name: name.into(), span: s() }
}

fn var_p(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Param, span: s() }
}

fn builtin(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}

/// `recv.meth(extra…)` → `BuiltinCall("__method__", [recv, "meth", …extra])`.
fn method(recv: Expr, name: &str, extra: Vec<Expr>) -> Expr {
    let mut args = vec![recv, Expr::StrLit { value: name.into(), span: s() }];
    args.extend(extra);
    builtin("__method__", args)
}

/// `{a: 1, b: 2, …}` from symbol-key / int-value pairs.
fn map_of(pairs: Vec<(&str, i64)>) -> Expr {
    Expr::MapLit {
        entries: pairs
            .into_iter()
            .map(|(k, v)| MapEntry { key: sym(k), value: ilit(v) })
            .collect(),
        span: s(),
    }
}

fn print_stmt(expr: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "print".into(),
            args: vec![expr],
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s(),
        },
        span: s(),
    }
}

fn closure(fn_name: &str) -> Expr {
    Expr::MakeClosure { fn_name: fn_name.into(), captures: vec![], span: s() }
}

/// One-parameter lambda `fn_name(x) -> body`, registered as a top-level fn.
fn lambda_fn(fn_name: &str, param: &str, body: Expr) -> Function {
    Function {
        name: fn_name.into(),
        params: vec![semantic_ir::Param {
            name: param.into(),
            kind: semantic_ir::ParamKind::Required,
            sir_type: None,
            default: None,
            span: s(),
        }],
        return_type: None,
        captures: vec![],
        body: Block { stmts: vec![], value: body, span: s() },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    }
}

fn manifest() -> FeatureManifest {
    FeatureManifest::from_features(&[
        Feature::Closures,
        Feature::Sequences,
        Feature::Maps,
        Feature::Strings,
        Feature::Symbols,
        Feature::MutableBindings,
        Feature::DynamicTyping,
    ])
}

fn program(functions: Vec<Function>) -> Module {
    Module {
        name: "hash_methods_demo".into(),
        manifest: manifest(),
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

/// The Hash catalog demo `main`.  Each printed line's expected value is what
/// the Python/TS reference runtime yields for the identical operation.
fn catalog_module() -> Module {
    // Block bodies: `each_value`/`each_pair` print, so accumulation is proven
    // by the printed lines they emit before the method's own return value.
    let print_x = lambda_fn(
        "__lam_print_x",
        "x",
        builtin("print", vec![var_p("x")]),
    );
    // `transform_values` replaces every value with the block result; a constant
    // body makes the expected hash trivially predictable ({a: 99, b: 99}).
    let const99 = lambda_fn("__lam_const99", "v", ilit(99));
    // `transform_keys` collapses BOTH keys onto the single symbol `:z`; Ruby
    // keeps the LAST colliding entry's value, so {a:1,b:2} → {z: 2}.
    let const_z = lambda_fn("__lam_const_z", "k", sym("z"));

    let stmts = vec![
        // {a:1}.merge({b:2}) → {a: 1, b: 2}   (fresh hash, other appended)
        print_stmt(method(map_of(vec![("a", 1)]), "merge", vec![map_of(vec![("b", 2)])])),
        // {a:1,b:2}.merge({b:9}) → {a: 1, b: 9}   (other wins on collision)
        print_stmt(method(
            map_of(vec![("a", 1), ("b", 2)]),
            "merge",
            vec![map_of(vec![("b", 9)])],
        )),
        // {a:1,b:2}.to_a → [[a, 1], [b, 2]]
        print_stmt(method(map_of(vec![("a", 1), ("b", 2)]), "to_a", vec![])),
        // {a:{b:1}}.dig(:a,:b) → 1   (nested)
        print_stmt(method(
            Expr::MapLit {
                entries: vec![MapEntry { key: sym("a"), value: map_of(vec![("b", 1)]) }],
                span: s(),
            },
            "dig",
            vec![sym("a"), sym("b")],
        )),
        // {a:{b:1}}.dig(:a,:z) → nil   (missing nested key)
        print_stmt(method(
            Expr::MapLit {
                entries: vec![MapEntry { key: sym("a"), value: map_of(vec![("b", 1)]) }],
                span: s(),
            },
            "dig",
            vec![sym("a"), sym("z")],
        )),
        // {a:1,b:2}.invert → {1: a, 2: b}
        print_stmt(method(map_of(vec![("a", 1), ("b", 2)]), "invert", vec![])),
        // {a:1,b:2}.delete(:a) → 1   (returns removed value)
        print_stmt(method(map_of(vec![("a", 1), ("b", 2)]), "delete", vec![sym("a")])),
        // {a:1}.delete(:z) → nil   (absent key)
        print_stmt(method(map_of(vec![("a", 1)]), "delete", vec![sym("z")])),
        // {a:1,b:2}.clear → {}
        print_stmt(method(map_of(vec![("a", 1), ("b", 2)]), "clear", vec![])),
        // {a:1}.store(:b, 2) → 2   (returns stored value)
        print_stmt(method(map_of(vec![("a", 1)]), "store", vec![sym("b"), ilit(2)])),
        // {a:1,b:2}.each_value { |v| print v } prints 1 then 2, returns self.
        //   → lines "1", "2", then the returned map "{a: 1, b: 2}"
        print_stmt(method(
            map_of(vec![("a", 1), ("b", 2)]),
            "each_value",
            vec![closure("__lam_print_x")],
        )),
        // {a:1,b:2}.each_key { |k| print k } prints a then b, returns self.
        //   → lines "a", "b", then the returned map "{a: 1, b: 2}"
        print_stmt(method(
            map_of(vec![("a", 1), ("b", 2)]),
            "each_key",
            vec![closure("__lam_print_x")],
        )),
        // {a:1,b:2}.transform_values { 99 } → {a: 99, b: 99}   (keys untouched)
        print_stmt(method(
            map_of(vec![("a", 1), ("b", 2)]),
            "transform_values",
            vec![closure("__lam_const99")],
        )),
        // {a:1,b:2}.transform_keys { :z } → {z: 2}   (collision → last value wins)
        print_stmt(method(
            map_of(vec![("a", 1), ("b", 2)]),
            "transform_keys",
            vec![closure("__lam_const_z")],
        )),
    ];

    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts, value: Expr::NilLit { span: s() }, span: s() },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    };

    program(vec![print_x, const99, const_z, main])
}

fn go_available() -> bool {
    Command::new("go")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_go(source: &str, tag: &str) -> std::process::Output {
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_go_hash_{tag}_{nonce}.go"));
    std::fs::write(&src_path, source).expect("write temp source");
    let out = Command::new("go")
        .arg("run")
        .arg(&src_path)
        .output()
        .expect("invoke go run");
    let _ = std::fs::remove_file(&src_path);
    out
}

#[test]
fn hash_methods_compile_and_run() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let artifact = compile(&catalog_module()).expect("module should compile to Go source");
    let run_out = run_go(&artifact.source, "catalog");
    if !run_out.status.success() {
        panic!(
            "emitted Go failed:\n--- stderr ---\n{}\n--- source ---\n{}",
            String::from_utf8_lossy(&run_out.stderr),
            artifact.source,
        );
    }
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        vec![
            "{a: 1, b: 2}",     // merge (append)
            "{a: 1, b: 9}",     // merge (collision → other wins)
            "[[a, 1], [b, 2]]", // to_a
            "1",                // dig nested hit
            "nil",              // dig nested miss
            "{1: a, 2: b}",     // invert
            "1",                // delete → removed value
            "nil",              // delete absent → nil
            "{}",               // clear
            "2",                // store → stored value
            "1",                // each_value yields 1
            "2",                // each_value yields 2
            "{a: 1, b: 2}",     // each_value returns self
            "a",                // each_key yields a
            "b",                // each_key yields b
            "{a: 1, b: 2}",     // each_key returns self
            "{a: 99, b: 99}",   // transform_values (keys untouched)
            "{z: 2}",           // transform_keys (collision → last value wins)
        ],
        "unexpected stdout:\n{stdout}"
    );
}
