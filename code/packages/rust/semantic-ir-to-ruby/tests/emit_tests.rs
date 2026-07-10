//! Behavioural tests for the Ruby backend.
//!
//! Emit-shape assertions run everywhere (no `ruby` needed).  The end-to-end
//! tests lower real source, emit Ruby, and — when a `ruby` interpreter is on
//! `PATH` — run it and check stdout, skipping gracefully otherwise (the
//! toolchain-gated convention the conformance harness uses).

use semantic_ir_to_ruby::{compile, sanitize_ident};

/// Lower Ruby source → SIR → Ruby text.
fn ruby_to_ruby(src: &str) -> String {
    let module = ruby_to_semantic_ir::compile_source(src, "prog").expect("ruby lowering");
    compile(&module).expect("ruby emit").source
}

/// Lower Twig source → SIR → Ruby text.
fn twig_to_ruby(src: &str) -> String {
    let module = twig_to_semantic_ir::compile_source(src, "prog").expect("twig lowering");
    compile(&module).expect("ruby emit").source
}

/// Run emitted Ruby with a `ruby` interpreter if one is available; return its
/// stdout, or `None` to signal a skip.
fn run_ruby(source: &str) -> Option<String> {
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir();
    // Unique per (process, call) so parallel tests never share a temp file.
    let path = dir.join(format!(
        "sir_ruby_test_{}_{}.rb",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::File::create(&path)
        .ok()?
        .write_all(source.as_bytes())
        .ok()?;
    let out = std::process::Command::new("ruby").arg(&path).output().ok();
    let _ = std::fs::remove_file(&path);
    let out = out?;
    if !out.status.success() {
        panic!(
            "emitted ruby exited non-zero:\n{}\n--- source ---\n{source}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Some(
        String::from_utf8_lossy(&out.stdout)
            .replace("\r\n", "\n")
            .trim_end()
            .to_string(),
    )
}

// ── emit-shape (no interpreter needed) ──────────────────────────────────────

#[test]
fn arithmetic_precedence_shape() {
    let rb = ruby_to_ruby("puts 2 + 3 * 4");
    // Native operators, precedence preserved by parenthesisation.
    assert!(rb.contains("sir_puts((2 + (3 * 4)))"), "got:\n{rb}");
    assert!(rb.contains("def sir_user_main"), "renames main");
    assert!(rb.ends_with("sir_user_main\n"), "calls the entry:\n{rb}");
}

#[test]
fn if_is_a_native_expression() {
    let rb = ruby_to_ruby("def f(x)\n  if x > 0\n    10\n  else\n    0\n  end\nend\nputs f(3)");
    assert!(
        rb.contains("(if sir_truthy((x > 0)) then 10 else 0 end)"),
        "got:\n{rb}"
    );
}

#[test]
fn display_convention_follows_source_language() {
    // The mechanism: a module tagged source_language="ruby" renders booleans
    // the Ruby way (true/false); anything else keeps the Lisp #t/#f.  (The Ruby
    // frontend does not yet set the tag — a known cross-backend gap — so we set
    // it directly to exercise the backend's substitution.)
    let mut module = twig_to_semantic_ir::compile_source("(print 1)", "prog").unwrap();
    module.metadata.source_language = Some("ruby".into());
    assert!(compile(&module)
        .unwrap()
        .source
        .contains("SIR_DISPLAY_RUBY = true"));

    module.metadata.source_language = Some("twig".into());
    assert!(compile(&module)
        .unwrap()
        .source
        .contains("SIR_DISPLAY_RUBY = false"));
}

#[test]
fn deterministic_output() {
    let a = ruby_to_ruby("puts 2 + 3 * 4");
    let b = ruby_to_ruby("puts 2 + 3 * 4");
    assert_eq!(a, b, "emission must be byte-stable");
}

#[test]
fn string_hash_is_escaped_so_no_interpolation_can_fire() {
    // A literal `#` is escaped to `\#` so a crafted `#{...}` in source data can
    // never become a Ruby interpolation in the emitted literal.
    let rb = ruby_to_ruby(r##"puts "a#b""##);
    assert!(rb.contains("\"a\\#b\""), "the # should be escaped:\n{rb}");
    if let Some(out) = run_ruby(&rb) {
        assert_eq!(out, "a#b"); // and it still prints the literal text
    }
}

#[test]
fn sanitize_ident_handles_keywords_and_namespace() {
    assert_eq!(sanitize_ident("foo"), "foo");
    assert_eq!(sanitize_ident("end"), "end_"); // ruby keyword
    assert_eq!(sanitize_ident("class"), "class_");
    assert!(
        sanitize_ident("sir_x").starts_with("sir_x"),
        "runtime namespace guarded"
    );
    assert_eq!(sanitize_ident("Foo"), "_Foo"); // locals may not start uppercase
}

// ── end-to-end (skips when `ruby` is absent) ────────────────────────────────

#[test]
fn e2e_arithmetic() {
    let rb = ruby_to_ruby("puts 2 + 3 * 4");
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "14"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn e2e_recursion_via_method() {
    let rb = ruby_to_ruby(
        "def add(a, b)\n  a + b\nend\ndef triple(n)\n  add(add(n, n), n)\nend\nputs triple(7)",
    );
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "21"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn e2e_twig_closure_and_globals() {
    let rb = twig_to_ruby(
        "(define (adder n) (lambda (x) (+ x n))) (define add5 (adder 5)) (print (add5 3))",
    );
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "8"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn e2e_tail_if_both_branches() {
    let rb = ruby_to_ruby(
        "def classify(n)\n  if n == 0\n    \"zero\"\n  elsif n < 0\n    \"neg\"\n  else\n    \"pos\"\n  end\nend\nputs classify(0)\nputs classify(-5)\nputs classify(7)",
    );
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "zero\nneg\npos"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}
