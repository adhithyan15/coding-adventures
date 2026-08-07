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
    let dir = std::env::temp_dir();
    // Hash the full source, not just its length -- two tests with equal-length
    // sources run as parallel threads in the SAME process (cargo test's
    // default), so a length-keyed stem collides and one test's compile/run
    // clobbers the other's temp file (a real, hit-in-CI flake).
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut hasher);
    let stem = format!("sirc_cm_{}_{}", std::process::id(), hasher.finish());
    let cpath = dir.join(format!("{stem}.c"));
    let exe = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
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
