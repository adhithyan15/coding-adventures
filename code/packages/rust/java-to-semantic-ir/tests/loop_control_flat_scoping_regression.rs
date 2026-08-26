//! Regression test for a real, `/security-review`-caught non-termination
//! bug in task #64's own `Stmt::Break`/`Stmt::Continue` support work:
//! `lower_do_while_statement`/`lower_for_statement_inner`'s synthetic
//! guard-flag names must never collide with a real Java local, under any
//! backend's scoping rules.
//!
//! An earlier version of this same fix pass generated the flag as a plain
//! `__do_while_N`/`__for_first_N` with no collision check at all against
//! real Java locals. The flag's own reference lives inside the loop's
//! *condition* expression (moved there by this same fix pass, to keep a
//! `continue` from skipping it), and several backends — Python and Ruby
//! among them — compile a SIR condition/body pair with FLAT scoping: no
//! new scope opens for either, so a body-declared local sharing the
//! flag's exact name silently shadows it. For `do`-`while`, that re-arms
//! the "always enter" flag every iteration, so the real condition is
//! never evaluated again — an unconditional infinite loop.
//!
//! A second attempted fix used `#` (illegal in a Java identifier, JLS
//! §3.8) in the flag's own name, reasoning that made it unforgeable by
//! real Java source. A further `/security-review` round proved that
//! false: every flat-scoping backend's `sanitize_ident` escapes `#` into
//! an ordinary, legal-identifier string a real Java program CAN declare
//! directly (e.g. Python's hex-escape turns `__do_while#0` into
//! `___do_while_230`), reproducing the identical collision through the
//! escaped form instead.
//!
//! The actual fix (`fresh_flag_name`, shared by both desugarings) drops
//! any attempt at an "unforgeable" name and instead picks a plain,
//! escape-free candidate, then checks it directly against both the
//! ambient scope (`lookup_local_with_frame`) and everything the loop's
//! own lowered body declares (`DeclaredNameCollector`, riding
//! `semantic_ir`'s shared `Visitor`), retrying the next counter value on
//! any collision — see `lower_do_while_statement`'s and `fresh_flag_
//! name`'s own doc comments for the full story.
//!
//! This test proves the fix holds against the ACTUAL Python backend (the
//! one the original finding reproduced against), not just against
//! `semantic-ir::validate()` or the JavaScript backend `loop_control_
//! java_execution.rs` otherwise exercises — with a hard wall-clock
//! timeout, so a regression FAILS this test cleanly rather than hanging
//! the whole suite (and CI) forever.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use java_to_semantic_ir::compile_source;
use semantic_ir::Stmt;

fn python_available() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run `python3 <path>` (with `PYTHONPATH` set to `pythonpath`) and wait
/// up to `timeout` for it to exit, polling rather than blocking on
/// `Command::output()` (which has no timeout at all) — a regression here
/// must be observed as a clean test failure, not an indefinite hang.
/// Kills the child and returns `None` on timeout.
fn run_with_timeout(
    path: &std::path::Path,
    pythonpath: &std::ffi::OsStr,
    timeout: Duration,
) -> Option<std::process::Output> {
    let mut child = Command::new("python3")
        .arg(path)
        .env("PYTHONPATH", pythonpath)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn python3");
    let start = Instant::now();
    loop {
        if let Some(_status) = child.try_wait().expect("poll child status") {
            return Some(child.wait_with_output().expect("collect child output"));
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn run_via_python_bounded(name: &str, java_src: &str) -> String {
    let mut module = compile_source(java_src, "prog").expect("lowering should succeed");
    let report = semantic_ir::validate(&module);
    assert!(
        report.is_ok(),
        "SIR validation failed for {name}: {:?}",
        report.issues
    );
    let main = module
        .functions
        .iter_mut()
        .find(|f| f.name == "main")
        .expect("expected a synthesized `main` function");
    match main.body.stmts.pop() {
        Some(Stmt::ExprStmt { expr, .. }) => main.body.value = expr,
        Some(other) => panic!(
            "expected `main`'s last statement to be a bare expression statement, got {other:?}"
        ),
        None => panic!("`main` has no statements to observe"),
    }
    main.name = "probe".to_string();

    let artifact = semantic_ir_to_python::compile(&module).expect("backend emit should succeed");

    let mut path = std::env::temp_dir();
    path.push(format!(
        "java_sir_flat_scoping_regress_{name}_{}.py",
        std::process::id()
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("create temp py (create_new, not following an existing symlink)");
    use std::io::Write as _;
    file.write_all(artifact.source.as_bytes())
        .expect("write temp py");
    writeln!(file, "print(probe())").expect("write print epilogue");
    drop(file);

    let py_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../python");
    let pythonpath = std::env::join_paths([
        py_root.join("sir-runtime-core/src"),
        py_root.join("sir-runtime-pairs/src"),
        py_root.join("sir-runtime-oop/src"),
        py_root.join("sir-runtime-range/src"),
        py_root.join("sir-runtime-regex/src"),
        py_root.join("sir-runtime-exceptions/src"),
    ])
    .expect("join PYTHONPATH");

    let output = run_with_timeout(&path, &pythonpath, Duration::from_secs(15));
    let _ = std::fs::remove_file(&path);

    let output = match output {
        Some(o) => o,
        None => panic!(
            "python3 did not exit within 15s for {name} -- this is the exact non-termination \
             regression this test exists to catch, not a flaky timeout; see this file's own \
             module doc comment"
        ),
    };

    assert!(
        output.status.success(),
        "python3 failed for {name}: stderr=\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn wrap(body: &str) -> String {
    format!("class Main {{ public static void main(String[] args) {{ {body} }} }}")
}

#[test]
fn do_while_flag_name_does_not_hang_on_python_flat_scoping_even_with_a_shadowing_local() {
    if !python_available() {
        eprintln!("skipping: `python3` not available");
        return;
    }
    // A body-declared local matching the flag's own first-candidate
    // name exactly. Before `fresh_flag_name`'s collision check existed,
    // this scenario hung Python's own interpreter forever (flat `while`/
    // condition scoping means the body's own assignment IS the flag).
    let src = wrap(concat!(
        "int y = 0; ",
        "do { ",
        "  boolean __do_while_0 = true; ",
        "  y = y + 1; ",
        "} while (y < 3); ",
        "y;"
    ));
    assert_eq!(
        run_via_python_bounded("do_while_flat_scoping", &src),
        "3"
    );
}

#[test]
fn classic_for_flag_name_does_not_hang_on_python_flat_scoping_even_with_a_shadowing_local() {
    if !python_available() {
        eprintln!("skipping: `python3` not available");
        return;
    }
    let src = wrap(concat!(
        "int sum = 0; ",
        "for (int i = 0; i < 3; i++) { ",
        "  boolean __for_first_0 = true; ",
        "  sum = sum + i; ",
        "} ",
        "sum;"
    ));
    // 0 + 1 + 2 = 3.
    assert_eq!(
        run_via_python_bounded("classic_for_flat_scoping", &src),
        "3"
    );
}
