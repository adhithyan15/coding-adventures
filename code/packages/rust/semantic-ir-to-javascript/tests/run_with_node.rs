//! End-to-end integration test: Twig source → SIR → JavaScript → `node`.
//!
//! The whole point of this backend is *self-contained* JavaScript that
//! runs as-is.  Unit tests prove the emitted *shape*; this test proves
//! the emitted *behaviour* by actually executing the artifact under
//! Node.js and comparing stdout.
//!
//! Node is optional at test time.  When `node --version` does not
//! succeed (CI image without Node, locked-down sandbox, …) the test
//! degrades to the syntactic checks and skips execution, printing a
//! note rather than failing — mirroring the spec's "without `node`, the
//! syntactic tests still verify the output shape".

use std::path::PathBuf;
use std::process::Command;

use semantic_ir_to_javascript::compile;

/// Is a working `node` on PATH?
fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Compile Twig `src` to a `.js` file in a unique temp path, run it with
/// `node`, and return its stdout (trailing newline trimmed).  Returns
/// `None` when Node is unavailable (caller should skip the assertion).
fn emit_and_run(src: &str, module_name: &str, tag: &str) -> Option<String> {
    let module = twig_to_semantic_ir::compile_source(src, module_name).expect("lower twig");
    let artifact = compile(&module).expect("compile to javascript");

    if !node_available() {
        eprintln!("note: `node` unavailable — skipping execution for `{tag}`");
        return None;
    }

    // Unique path per process so parallel test runs never collide.
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_js_{}_{}_{}.js", tag, std::process::id(), module_name));
    std::fs::write(&path, &artifact.source).expect("write temp js");

    let output = Command::new("node")
        .arg(&path)
        .output()
        .expect("spawn node");

    // Best-effort cleanup; a leftover temp file is harmless.
    let _ = std::fs::remove_file(&path);

    assert!(
        output.status.success(),
        "node exited non-zero for `{tag}`:\nstdout: {}\nstderr: {}\nsource:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        artifact.source,
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    Some(stdout.trim_end_matches(['\n', '\r']).to_string())
}

#[test]
fn add_program_prints_three() {
    // (define (add a b) (+ a b)) ; (print (add 1 2)) → 3
    let out = emit_and_run("(define (add a b) (+ a b))\n(print (add 1 2))", "addprog", "add");
    if let Some(stdout) = out {
        assert_eq!(stdout, "3", "expected 3 from add(1, 2)");
    }
}

#[test]
fn factorial_program_prints_120() {
    let src = "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1)))))\n(print (fact 5))";
    let out = emit_and_run(src, "factprog", "fact");
    if let Some(stdout) = out {
        assert_eq!(stdout, "120", "expected 5! = 120");
    }
}

#[test]
fn closure_adder_program_prints_eight() {
    // A higher-order program: `adder` returns a closure capturing `n`;
    // `add5` is the closure; `(add5 3)` applies it → 8.  Exercises
    // MakeClosure (capture), the global init, and applyClosure together.
    let src =
        "(define (adder n) (lambda (x) (+ x n)))\n(define add5 (adder 5))\n(print (add5 3))";
    let out = emit_and_run(src, "closprog", "closure");
    if let Some(stdout) = out {
        assert_eq!(stdout, "8", "expected add5(3) = 8");
    }
}
