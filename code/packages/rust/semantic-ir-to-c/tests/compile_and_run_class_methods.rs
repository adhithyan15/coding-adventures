//! Execution proof for OOP mirror slice 5 (class methods) on the C backend —
//! lower REAL Ruby source, emit C, compile with a real cc, run, assert stdout.
//! Skips gracefully when no `cc` is present.
//!
//! `def self.m` lowers to a hoisted function + `__def_class_method__` (registering
//! the closure in the SEPARATE class-method / singleton table); `Class.m(args)`
//! lowers to `__class_method__(StrLit(class), StrLit(method), …args)` →
//! `_sir_call_class_method`, an explicit table lookup that walks the ancestry
//! (class methods inherit) — never reflection.

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

fn run_ruby(src: &str) -> Option<String> {
    let cc = find_cc()?;
    let module = ruby_to_semantic_ir::compile_source(src, "prog").expect("ruby lowering");
    let art = semantic_ir_to_c::compile(&module).expect("C compile (no panic)");
    let scratch = sir_test_scratch::Scratch::new("sir_c_test_");
    let dir = scratch.path();
    // Hash the full source, not just its length -- two tests with equal-length
    // sources run as parallel threads in the SAME process (cargo test's
    // default), so a length-keyed stem collides and one test's compile/run
    // clobbers the other's temp file (a real, hit-in-CI flake).
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut hasher);
    // The hash alone still collides when two tests compile the SAME
    // source, so the stem also carries a per-call `unique_suffix()` (#18485).
    let stem = format!("sirc_cm_{}_{}", unique_suffix(), hasher.finish());
    let cpath = dir.join(format!("{stem}.c"));
    let exe = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    let _cleanup = RemoveOnDrop([cpath.clone(), exe.clone()]);
    sir_test_scratch::write_new(&cpath, &art.source).expect("write .c");
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
fn class_method_with_an_argument() {
    match run_ruby(
        "class Math2\n  def self.double(x)\n    x + x\n  end\nend\n\
         puts Math2.double(21)\n",
    ) {
        Some(out) => assert_eq!(out, "42\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn class_method_is_inherited_by_a_subclass() {
    // `Sub` defines no class method, so `Sub.double` resolves up the ancestry to
    // `Math2.double` (class methods inherit).
    match run_ruby(
        "class Math2\n  def self.double(x)\n    x + x\n  end\nend\n\
         class Sub < Math2\nend\n\
         puts Sub.double(50)\n",
    ) {
        Some(out) => assert_eq!(out, "100\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn class_and_instance_methods_of_the_same_name_coexist() {
    // A class method `tag` and an instance method `tag` live in SEPARATE tables,
    // so each dispatches to its own closure with no collision.
    match run_ruby(
        "class Foo\n  def self.tag\n    1\n  end\n  def tag\n    2\n  end\nend\n\
         puts Foo.tag\nf = Foo.new\nputs f.tag\n",
    ) {
        Some(out) => assert_eq!(out, "1\n2\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn class_method_calls_another_class_method() {
    // One class method dispatches to another on the same class.
    match run_ruby(
        "class Calc\n  def self.inc(x)\n    x + 1\n  end\n  def self.twice_inc(x)\n    \
         a = Calc.inc(x)\n    Calc.inc(a)\n  end\nend\n\
         puts Calc.twice_inc(10)\n",
    ) {
        // twice_inc(10): inc(10)=11, inc(11)=12.
        Some(out) => assert_eq!(out, "12\n"),
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
