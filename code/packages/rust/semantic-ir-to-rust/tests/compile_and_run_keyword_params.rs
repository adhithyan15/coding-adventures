//! End-to-end proof for **KeywordParams** (KW5) in the Rust backend.
//!
//! Rust has NO native keyword-argument syntax, so — per
//! `code/specs/sir-keyword-params.md` §4 — the backend resolves
//! keywords to a plain positional call **statically, at emit time**
//! (no runtime library).  The lib unit tests assert the emitted
//! *shape*; this test closes the loop: it hand-builds a SIR module with
//! an optional keyword parameter, emits Rust, compiles it with `rustc`,
//! runs the binary, and checks stdout.
//!
//! ### Discriminating program
//!
//! Modelled on the sibling default-params execution proof.  `greet`
//! returns its `name` keyword parameter, so the printed value IS the
//! resolved keyword — exposing whether resolution and the default fill
//! actually ran:
//!
//! ```text
//!   def greet(greeting, name: "world") -> name   // name is an OPTIONAL keyword
//!
//!   greet("hi")               // name omitted   ⇒ default "world"  → "world"
//!   greet("hi", name: "ada")  // name supplied   = "ada"           → "ada"
//! ```
//!
//! Why return `name` rather than the full `"hi, world"` interpolation
//! the spec's cross-backend reference program prints?  Building that
//! string needs `StrConcat` (SIR18 interpolation), a feature this Rust
//! backend does not accept — so, exactly as the default-params proof
//! returns `b` to expose the resolved default, we return `name` to
//! expose the resolved keyword.  The observable `"world"` / `"ada"`
//! outputs still match what the Python/TS reference prints for `name`
//! in the two calls, which is the load-bearing behaviour KW5 adds.
//!
//! `rustc` ships with every Rust toolchain, so this runs in CI.  A
//! missing `rustc` or linker logs a skip rather than reddening the
//! build (matching the sibling `compile_and_run_*` tests).

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Param,
    ParamKind, Scope, Span,
};
use semantic_ir_to_rust::compile;

fn s() -> Span {
    Span::synthetic()
}

fn slit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}

fn param_ref(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Param, span: s() }
}

/// `greet(arg0, name: value)` direct call.  `positional` is the
/// leading positional argument; `name_kw` is an optional supplied
/// keyword value (`None` omits the keyword).
fn greet_call(positional: Expr, name_kw: Option<Expr>) -> Expr {
    let mut args = vec![positional];
    if let Some(v) = name_kw {
        args.push(Expr::KeywordArg { name: "name".into(), value: Box::new(v), span: s() });
    }
    Expr::DirectCall { fn_name: "greet".into(), args, effects: EffectSet::PURE, span: s() }
}

/// `print(expr)` as an effectful statement.
fn print_stmt(expr: Expr) -> semantic_ir::Stmt {
    semantic_ir::Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "print".into(),
            args: vec![expr],
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s(),
        },
        span: s(),
    }
}

/// Build the discriminating module:
///   greet(greeting, name: "world") -> name
///   main: print(greet("hi")); print(greet("hi", name: "ada"))
fn demo_module() -> Module {
    let greet = Function {
        name: "greet".into(),
        params: vec![
            Param { name: "greeting".into(), kind: ParamKind::Required, sir_type: None, default: None, span: s() },
            // OPTIONAL keyword: kind == Keyword, default == Some("world").
            Param {
                name: "name".into(),
                kind: ParamKind::Keyword,
                sir_type: None,
                default: Some(Box::new(slit("world"))),
                span: s(),
            },
        ],
        return_type: None,
        captures: vec![],
        // greet returns `name` — so the printed value IS the resolved keyword.
        body: Block { stmts: vec![], value: param_ref("name"), span: s() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: s(),
    };

    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![
                // greet("hi")              → name defaulted "world" → "world"
                print_stmt(greet_call(slit("hi"), None)),
                // greet("hi", name: "ada") → name supplied "ada"    → "ada"
                print_stmt(greet_call(slit("hi"), Some(slit("ada")))),
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
            Feature::DynamicTyping,
            Feature::Strings,
            Feature::DefaultParams,
            Feature::KeywordParams,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![greet, main],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("test")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
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

#[test]
fn keyword_params_compile_and_run() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    // 1. Emit.
    let artifact = compile(&demo_module()).expect("module should compile to Rust source");

    // Sanity: the emitted source carries the positional-ized keyword
    // param + its default prologue, and the two resolved call shapes.
    assert!(
        artifact.source.contains("fn greet(greeting: __sir::Value, name: __sir::Value)"),
        "expected keyword param positional-ized in emitted source:\n{}",
        artifact.source
    );
    assert!(
        artifact.source.contains("let name = if __sir::is_missing(&name)"),
        "expected optional-keyword default prologue in emitted source:\n{}",
        artifact.source
    );
    // Omitted keyword → sentinel; supplied keyword → its value in position.
    assert!(
        artifact.source.contains(
            "greet(__sir::Value::Str(::std::rc::Rc::from(\"hi\")), __sir::missing())"
        ),
        "expected omitted-keyword call to pad the sentinel:\n{}",
        artifact.source
    );
    assert!(
        artifact.source.contains(
            "greet(__sir::Value::Str(::std::rc::Rc::from(\"hi\")), __sir::Value::Str(::std::rc::Rc::from(\"ada\")))"
        ),
        "expected supplied-keyword call resolved to positional:\n{}",
        artifact.source
    );

    // 2. Write the source to a unique temp file.
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_kwparams_{nonce}.rs"));
    let bin_path = dir.join(format!(
        "sir_kwparams_{nonce}{}",
        if cfg!(windows) { ".exe" } else { "" }
    ));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    // 3. Compile with rustc.  `--edition 2021` is required (raw idents +
    //    2018+ closure capture).  An absent default linker can be pointed
    //    at a working one via `SIR_TEST_RUSTC_LINKER` (e.g. `rust-lld`).
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
        // A missing linker is a host issue, not a codegen defect — skip.
        if stderr.contains("linker")
            && (stderr.contains("not found") || stderr.contains("No such file"))
        {
            eprintln!("skipping: no usable linker on host\n{stderr}");
            let _ = std::fs::remove_file(&src_path);
            return;
        }
        panic!(
            "emitted Rust failed to compile:\n--- stderr ---\n{stderr}\n--- source ---\n{}",
            artifact.source,
        );
    }

    // 4. Run the binary and capture stdout.
    let run_out = Command::new(&bin_path).output().expect("run compiled binary");
    assert!(
        run_out.status.success(),
        "compiled binary exited non-zero:\n{}",
        String::from_utf8_lossy(&run_out.stderr),
    );
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    // 5. Assert the program's observable behaviour:
    //    greet("hi")              → name defaulted → "world"
    //    greet("hi", name: "ada") → name supplied  → "ada"
    assert_eq!(
        lines,
        vec!["world", "ada"],
        "unexpected program output; full stdout:\n{stdout}"
    );

    // 6. Best-effort cleanup.
    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}
