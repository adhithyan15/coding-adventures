//! End-to-end proof for the **String-method dispatch** runtime in the Rust
//! backend, added for parity with the Python/TypeScript `sir-runtime-oop`
//! reference (the `_STRING_METHODS` / `stringMethod` catalog).
//!
//! Like the sibling exec-proof tests, each demo hand-builds a SIR module,
//! emits Rust via `compile`, compiles it with `rustc`, runs the binary, and
//! diffs stdout against the value the Python/TS reference produces for the
//! SAME SIR module.  Dispatch reaches the backend as the narrow-waist envelope
//! `BuiltinCall("__method__", [recv, StrLit("meth"), …args])` and the runtime
//! resolves it through an EXPLICIT `match` on the interned method name (never
//! reflection over a host method table — the C3 allowlist discipline).
//!
//! Cases (reference semantics):
//!   * `"hello".capitalize`      → `"Hello"`
//!   * `"café".capitalize`       → `"Café"`   (rune-aware, no char-boundary panic)
//!   * `"hi\n".chomp`            → `"hi"`
//!   * `"abc".chars`             → `["a", "b", "c"]`
//!   * `"a-b-c".split("-")`      → `["a", "b", "c"]`
//!   * `"foobar".sub("o","0")`   → `"f0obar"`  (first literal occurrence)
//!   * `"foobar".gsub("o","0")`  → `"f00bar"`  (all literal occurrences)
//!   * `"hello".index("l")`      → `2`
//!   * `"hello".index("z")`      → `nil`
//!   * `"hi".start_with?("h")`   → `#t`  (bool renders as `#t`/`#f`)
//!   * `"hi".end_with?("i")`     → `#t`
//!   * `"abc".reverse`           → `"cba"`
//!   * `"  x  ".lstrip`          → `"x  "`
//!   * `"foo".replace("bar")`    → `"bar"`
//!   * `"".empty?`               → `#t`
//!
//! `sub`/`gsub` are the LITERAL forms — the pattern is matched as a plain
//! substring (never a regex) and the replacement is inserted verbatim (no
//! `$&`/`\1` back-reference expansion), matching the reference exactly.
//!
//! If `rustc` (or a usable linker) is unavailable the test logs a skip rather
//! than failing; a missing host tool must never redden a build.  The host can
//! point the test at a working linker via `SIR_TEST_RUSTC_LINKER`.

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span,
    Stmt,
};
use semantic_ir_to_rust::compile;

fn s() -> Span {
    Span::synthetic()
}

fn slit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}

/// `recv.meth(args…)` — the `__method__` dispatch envelope.
fn method(recv: Expr, name: &str, mut args: Vec<Expr>) -> Expr {
    let mut all = vec![recv, slit(name)];
    all.append(&mut args);
    Expr::BuiltinCall { name: "__method__".into(), args: all, effects: EffectSet::PURE, span: s() }
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

fn demo_module(main_stmts: Vec<Stmt>) -> Module {
    let functions = vec![Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts: main_stmts, value: Expr::NilLit { span: s() }, span: s() },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    }];

    Module {
        name: "string_methods_demo".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Sequences,
            Feature::Strings,
            Feature::Symbols,
        ]),
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

fn full_demo() -> Module {
    let main_stmts = vec![
        // "hello".capitalize  → "Hello"
        print_stmt(method(slit("hello"), "capitalize", vec![])),
        // "café".capitalize  → "Café"  (rune-aware)
        print_stmt(method(slit("café"), "capitalize", vec![])),
        // "hi\n".chomp  → "hi"
        print_stmt(method(slit("hi\n"), "chomp", vec![])),
        // "abc".chars  → ["a", "b", "c"]
        print_stmt(method(slit("abc"), "chars", vec![])),
        // "a-b-c".split("-")  → ["a", "b", "c"]
        print_stmt(method(slit("a-b-c"), "split", vec![slit("-")])),
        // "foobar".sub("o","0")  → "f0obar"
        print_stmt(method(slit("foobar"), "sub", vec![slit("o"), slit("0")])),
        // "foobar".gsub("o","0")  → "f00bar"
        print_stmt(method(slit("foobar"), "gsub", vec![slit("o"), slit("0")])),
        // "hello".index("l")  → 2
        print_stmt(method(slit("hello"), "index", vec![slit("l")])),
        // "hello".index("z")  → nil
        print_stmt(method(slit("hello"), "index", vec![slit("z")])),
        // "hi".start_with?("h")  → #t
        print_stmt(method(slit("hi"), "start_with?", vec![slit("h")])),
        // "hi".end_with?("i")  → #t
        print_stmt(method(slit("hi"), "end_with?", vec![slit("i")])),
        // "abc".reverse  → "cba"
        print_stmt(method(slit("abc"), "reverse", vec![])),
        // "  x  ".lstrip  → "x  "
        print_stmt(method(slit("  x  "), "lstrip", vec![])),
        // "foo".replace("bar")  → "bar"
        print_stmt(method(slit("foo"), "replace", vec![slit("bar")])),
        // "".empty?  → #t
        print_stmt(method(slit(""), "empty?", vec![])),
        // ── char-set methods (v0.24.0): tr / count / delete / squeeze ──
        // "hello".tr("el", "ip")  → "hippo"
        print_stmt(method(slit("hello"), "tr", vec![slit("el"), slit("ip")])),
        // "hello".tr("l", "")  → "heo"  (empty `to` deletes)
        print_stmt(method(slit("hello"), "tr", vec![slit("l"), slit("")])),
        // "hello".count("lo")  → 3
        print_stmt(method(slit("hello"), "count", vec![slit("lo")])),
        // "hello".delete("aeiou")  → "hll"
        print_stmt(method(slit("hello"), "delete", vec![slit("aeiou")])),
        // "mississippi".squeeze  → "misisipi"
        print_stmt(method(slit("mississippi"), "squeeze", vec![])),
        // "aaabbbccc".squeeze("a")  → "abbbccc"
        print_stmt(method(slit("aaabbbccc"), "squeeze", vec![slit("a")])),
    ];

    demo_module(main_stmts)
}

fn rustc_available() -> bool {
    Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn string_methods_compile_and_run() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    let artifact = compile(&full_demo()).expect("module should compile to Rust source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_string_methods_{nonce}.rs"));
    let bin_path = dir.join(format!(
        "sir_string_methods_{nonce}{}",
        if cfg!(windows) { ".exe" } else { "" }
    ));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

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

    let run_out = Command::new(&bin_path).output().expect("run compiled binary");
    assert!(
        run_out.status.success(),
        "compiled binary exited non-zero:\n{}",
        String::from_utf8_lossy(&run_out.stderr),
    );
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(
        lines,
        vec![
            "Hello",         // capitalize
            "Café",          // capitalize (multibyte)
            "hi",            // chomp
            "[a, b, c]",     // chars
            "[a, b, c]",     // split("-")
            "f0obar",        // sub — first literal occurrence
            "f00bar",        // gsub — all literal occurrences
            "2",             // index("l")
            "nil",           // index("z")
            "#t",            // start_with?("h")
            "#t",            // end_with?("i")
            "cba",           // reverse
            "x  ",           // lstrip
            "bar",           // replace
            "#t",            // "".empty?
            "hippo",         // tr("el", "ip")
            "heo",           // tr("l", "") — empty `to` deletes
            "3",             // count("lo")
            "hll",           // delete("aeiou")
            "misisipi",      // squeeze (no arg)
            "abbbccc",       // squeeze("a")
        ],
        "unexpected program output; full stdout:\n{stdout}"
    );

    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}
