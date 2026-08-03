//! Execution proof for OOP mirror slice 7 (modules / mixins) on the C backend —
//! lower REAL Ruby source, emit C, compile with a real cc, run, assert stdout.
//! Skips gracefully when no `cc` is present.  This is the FINAL OOP slice; with
//! it the C backend covers the full class/module surface (6-backend OOP parity).
//!
//! A module's methods are registered like a class's (`__def_method__`, keyed on
//! the module name), so a mixin needs no new method storage — only a record of
//! which modules a class mixes in.  `include M` (`__include__`) folds M's
//! instance methods into the class's INSTANCE-method resolution; `extend M`
//! (`__extend__`) folds them into the class's CLASS-method resolution.

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
    let stem = format!("sirc_mod_{}_{}", std::process::id(), hasher.finish());
    let cpath = dir.join(format!("{stem}.c"));
    let exe = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(&cpath, &art.source).expect("write .c");
    let out = Command::new(&cc)
        .args(["-std=c99", "-Wall", "-o"])
        .arg(&exe)
        .arg(&cpath)
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
fn include_mixes_module_instance_methods_into_a_class() {
    // `Person` includes `Greet`, so `Person.new.hi` resolves `hi` in the module.
    match run_ruby(
        "module Greet\n  def hi\n    42\n  end\nend\n\
         class Person\n  include Greet\nend\n\
         puts Person.new.hi\n",
    ) {
        Some(out) => assert_eq!(out, "42\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn extend_mixes_module_methods_as_class_methods() {
    // `Widget` extends `Cls`, so the module's instance method `tag` is callable
    // as the CLASS method `Widget.tag`.
    match run_ruby(
        "module Cls\n  def tag\n    7\n  end\nend\n\
         class Widget\n  extend Cls\nend\n\
         puts Widget.tag\n",
    ) {
        Some(out) => assert_eq!(out, "7\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn included_method_sees_the_receiver_ivars() {
    // A mixed-in method runs with `self` bound to the including instance, so its
    // `@n` reads/writes that instance's own ivars.
    match run_ruby(
        "module Counter\n  def peek\n    @n\n  end\nend\n\
         class Box\n  include Counter\n  def set\n    @n = 5\n  end\nend\n\
         b = Box.new\nb.set\nputs b.peek\n",
    ) {
        Some(out) => assert_eq!(out, "5\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn a_class_method_wins_over_an_own_instance_method_of_the_same_name() {
    // `include` does not shadow the class's own method: `Person` defines `hi`
    // itself, so its own method takes precedence over the module's.
    match run_ruby(
        "module Greet\n  def hi\n    1\n  end\nend\n\
         class Person\n  include Greet\n  def hi\n    2\n  end\nend\n\
         puts Person.new.hi\n",
    ) {
        Some(out) => assert_eq!(out, "2\n"),
        None => eprintln!("skip: no cc"),
    }
}
