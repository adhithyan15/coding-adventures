//! End-to-end proof for KW6 **KeywordParams** in the Go backend.
//!
//! Go has no native keyword arguments, so the backend performs **static
//! keyword→positional resolution**: a `DirectCall`'s callee signature is
//! statically known, so each `KeywordArg{ name, value }` is resolved to the
//! callee's declared parameter position at emit time, producing a plain
//! positional Go call.  An omitted *optional* keyword pads its slot with the
//! `_sir_missing` sentinel, and the callee's body prologue supplies the
//! default (identical to the SIR19 positional-default machinery — no runtime
//! library is added for keywords).
//!
//! DISCRIMINATING MODULE (the spec's `greet(greeting, name="world")`):
//!
//! ```text
//!   def greet(greeting:, name: "world")   // greeting: REQUIRED keyword
//!                                          // name:     OPTIONAL keyword, default "world"
//!     cons(greeting, cons(name, nil))     // returns the pair (greeting name)
//!   end
//!
//!   greet(greeting: "hi")               →  (hi world)   // name defaulted
//!   greet(greeting: "hi", name: "ada")  →  (hi ada)     // name supplied
//! ```
//!
//! The Go backend does not accept string interpolation (`StrConcat`), so —
//! unlike the Python/Rust reference backends that print the literal string
//! `hi, world` — this proof returns a **cons pair** of the two string params
//! and prints it, which the Go runtime formats as `(hi world)`.  The *content*
//! matches the reference (`greeting` = "hi", `name` = "world"/"ada"); only the
//! rendering differs, and it is fully discriminating: it pins **both**
//! keyword params to their resolved positions (greeting in `car`, name in
//! `cdr`) AND proves the omitted-optional default path (`(hi world)` vs
//! `(hi ada)`).
//!
//! The test gates on `go` being available (`go version`); a missing toolchain
//! logs a skip rather than reddening the build (mirrors the other
//! `compile_and_run_*` proofs in this crate).

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Param,
    ParamKind, Scope, Span, Stmt,
};
use semantic_ir_to_go::compile;

fn s() -> Span {
    Span::synthetic()
}

fn strlit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}

fn param_ref(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Param, span: s() }
}

fn kwarg(name: &str, value: Expr) -> Expr {
    Expr::KeywordArg { name: name.into(), value: Box::new(value), span: s() }
}

fn builtin(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall {
        name: name.into(),
        args,
        effects: EffectSet::PURE,
        span: s(),
    }
}

/// `print(expr)` as an effectful statement.
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

/// `def greet(greeting:, name: "world"); cons(greeting, cons(name, nil)); end`.
///
/// `greeting` is a REQUIRED keyword (`Keyword`, no default); `name` is an
/// OPTIONAL keyword (`Keyword`, default `"world"`).  The body returns the pair
/// `(greeting name)` so the printed value isolates exactly where each keyword
/// landed.
fn greet_fn() -> Function {
    let cons_name = builtin("cons", vec![param_ref("name"), Expr::NilLit { span: s() }]);
    let cons_pair = builtin("cons", vec![param_ref("greeting"), cons_name]);
    Function {
        name: "greet".into(),
        params: vec![
            Param {
                name: "greeting".into(),
                kind: ParamKind::Keyword,
                sir_type: None,
                default: None,
                span: s(),
            },
            Param {
                name: "name".into(),
                kind: ParamKind::Keyword,
                sir_type: None,
                default: Some(Box::new(strlit("world"))),
                span: s(),
            },
        ],
        return_type: None,
        captures: vec![],
        body: Block { stmts: vec![], value: cons_pair, span: s() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: s(),
    }
}

/// `main`: print `greet(greeting: "hi")` (→ `(hi world)`), then
/// print `greet(greeting: "hi", name: "ada")` (→ `(hi ada)`).
fn demo_module() -> Module {
    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![
                // greet(greeting: "hi") — optional keyword `name` omitted.
                print_stmt(Expr::DirectCall {
                    fn_name: "greet".into(),
                    args: vec![kwarg("greeting", strlit("hi"))],
                    effects: EffectSet::PURE,
                    span: s(),
                }),
                // greet(greeting: "hi", name: "ada") — both keywords supplied.
                print_stmt(Expr::DirectCall {
                    fn_name: "greet".into(),
                    args: vec![
                        kwarg("greeting", strlit("hi")),
                        kwarg("name", strlit("ada")),
                    ],
                    effects: EffectSet::PURE,
                    span: s(),
                }),
            ],
            value: Expr::NilLit { span: s() },
            span: s(),
        },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    };

    Module {
        name: "keyword_params_demo".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::KeywordParams,
            Feature::DefaultParams,
            // `cons`/pairs and string literals are observed features.
            Feature::Pairs,
            Feature::Strings,
            // Untyped params observe `DynamicTyping`; the validator requires
            // every observed feature to be declared.
            Feature::DynamicTyping,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![greet_fn(), main],
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

#[test]
fn keyword_params_compile_and_run() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }

    // 1. Emit.
    let artifact = compile(&demo_module()).expect("module should compile to Go source");

    // 2. Write to a unique temp file (`go run` requires a `.go` extension).
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_go_keyword_params_{nonce}.go"));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    // 3. Compile + run with `go run` (arg vector — no shell).
    let run_out = Command::new("go")
        .arg("run")
        .arg(&src_path)
        .output()
        .expect("invoke go run");

    if !run_out.status.success() {
        let stderr = String::from_utf8_lossy(&run_out.stderr);
        let _ = std::fs::remove_file(&src_path);
        panic!(
            "emitted Go failed to compile/run:\n--- stderr ---\n{stderr}\n--- source ---\n{}",
            artifact.source,
        );
    }

    // 4. Assert observable behaviour.  The FIRST call omitted the optional
    //    keyword `name`, so it defaulted to "world" → `(hi world)`.  The
    //    SECOND supplied it → `(hi ada)`.  Both prove `greeting` resolved to
    //    the first param and `name` to the second, by NAME.
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        vec!["(hi world)", "(hi ada)"],
        "unexpected program output; full stdout:\n{stdout}"
    );

    // 5. Best-effort cleanup.
    let _ = std::fs::remove_file(&src_path);
}
