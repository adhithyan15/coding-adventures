//! Execution proof for OOP mirror slice 2 (instance methods) on the C backend —
//! lower REAL Ruby source, emit C, compile with a real cc, run, assert stdout.
//! Skips gracefully when no `cc` is present.
//!
//! A method-bearing class lowers to a hoisted function + `__def_method__`
//! (registering a `(class,method)` closure) + `__method__` (dispatching it). The
//! dispatch is an explicit table lookup — never reflection.

use std::process::Command;

fn find_cc() -> Option<String> {
    if let Ok(cc) = std::env::var("SIR_CC") {
        if !cc.trim().is_empty() {
            return Some(cc);
        }
    }
    ["cc", "clang", "gcc"]
        .iter()
        .find(|c| {
            Command::new(c)
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
        .map(|s| s.to_string())
}

/// Lower `src` (Ruby) → C, compile + run, return stdout (or None if no cc).
fn run_ruby(src: &str) -> Option<String> {
    let cc = find_cc()?;
    let module = ruby_to_semantic_ir::compile_source(src, "prog").expect("ruby lowering");
    let art = semantic_ir_to_c::compile(&module).expect("C compile (no panic)");
    let dir = std::env::temp_dir();
    // Hash the full source, not just its length -- two tests with equal-length
    // sources run as parallel threads in the SAME process (cargo test's
    // default), so a length-keyed stem collides and one test's compile/run
    // clobbers the other's temp file (a real, hit-in-CI flake).
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut hasher);
    // The hash alone still collides when two tests compile the SAME
    // source, so the stem also carries a per-call `unique_suffix()` (#18485).
    let stem = format!("sirc_meth_{}_{}", unique_suffix(), hasher.finish());
    let cpath = dir.join(format!("{stem}.c"));
    let exe = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    let _cleanup = RemoveOnDrop([cpath.clone(), exe.clone()]);
    std::fs::write(&cpath, &art.source).expect("write .c");
    let out = Command::new(&cc)
        .args(["-std=c99", "-Wall", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm")  // Linux needs -lm to link floor/ceil/fabs (macOS libSystem folds it in)
        .output()
        .expect("spawn cc");
    assert!(
        out.status.success(),
        "compile failed:\n{}\n--- source ---\n{}",
        String::from_utf8_lossy(&out.stderr),
        art.source
    );
    let r = Command::new(&exe).output().expect("run");
    assert!(r.status.success(), "program exited non-zero");
    Some(String::from_utf8_lossy(&r.stdout).replace("\r\n", "\n"))
}

#[test]
fn instance_method_call() {
    // `class Greeter; def greet; puts "hi"; end; end; Greeter.new.greet` → "hi".
    match run_ruby("class Greeter\n  def greet\n    puts \"hi\"\n  end\nend\nGreeter.new.greet\n") {
        Some(out) => assert_eq!(out, "hi\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn instance_method_with_args_and_return() {
    // A method with parameters and a return value used by the caller.
    match run_ruby("class Adder\n  def add(a, b)\n    a + b\n  end\nend\nputs Adder.new.add(2, 3)\n") {
        Some(out) => assert_eq!(out, "5\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn two_methods_on_one_class() {
    // Two methods on the same class, both dispatched on a stored instance.
    match run_ruby(
        "class Point\n  def x\n    10\n  end\n  def y\n    20\n  end\nend\n\
         p = Point.new\nputs p.x\nputs p.y\n",
    ) {
        Some(out) => assert_eq!(out, "10\n20\n"),
        None => eprintln!("skip: no cc"),
    }
}

/// A per-call unique part for this file's temp-file names.  The process id
/// keeps parallel test binaries apart, and the counter keeps apart the tests
/// of this binary, which run on parallel threads under one process id
/// (#18485).  A name built from the process id alone, or from it and a hash
/// of the source, let two tests that compile the same program share a file.
fn unique_suffix() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!("{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed))
}

/// Deletes a run's `.c` file and executable when the helper returns or
/// panics.  Each call names its files uniquely (see `unique_suffix` or the
/// counter above), so without this they would pile up in the temp directory,
/// one pair per test run.
struct RemoveOnDrop([std::path::PathBuf; 2]);

impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        for path in &self.0 {
            let _ = std::fs::remove_file(path);
        }
    }
}
