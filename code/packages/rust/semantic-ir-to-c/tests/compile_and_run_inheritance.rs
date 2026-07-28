//! Execution proof for OOP mirror slice 4 (inheritance + `super`) on the C
//! backend — lower REAL Ruby source, emit C, compile with a real cc, run, assert
//! stdout.  Skips gracefully when no `cc` is present.
//!
//! `class Dog < Animal` lowers to a `ClassDef` with a `superclass`, which emits a
//! `_sir_register_super("Dog", "Animal")` edge; instance dispatch then resolves a
//! method up the ancestry (`_sir_resolve_method`).  `super` lowers to
//! `__super__(method, definingClass, …args)` → `_sir_call_super`, which resolves
//! `method` from the SUPERCLASS of the defining class and applies it to the
//! current receiver.

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
    let stem = format!("sirc_inh_{}_{}", std::process::id(), src.len());
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
fn subclass_inherits_a_method() {
    // `Dog < Animal` defines no `legs`, so dispatch resolves it up the ancestry.
    match run_ruby(
        "class Animal\n  def legs\n    4\n  end\nend\n\
         class Dog < Animal\nend\n\
         puts Dog.new.legs\n",
    ) {
        Some(out) => assert_eq!(out, "4\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn super_calls_the_overridden_parent_method() {
    // `Derived#val` overrides `Base#val` and reaches it via `super`.
    match run_ruby(
        "class Base\n  def val\n    10\n  end\nend\n\
         class Derived < Base\n  def val\n    super + 5\n  end\nend\n\
         puts Derived.new.val\n",
    ) {
        Some(out) => assert_eq!(out, "15\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn super_forwards_an_argument() {
    // `super(x)` forwards its argument to the parent method.
    match run_ruby(
        "class Adder\n  def add(x)\n    x + 1\n  end\nend\n\
         class Doubler < Adder\n  def add(x)\n    super(x) + x\n  end\nend\n\
         puts Doubler.new.add(10)\n",
    ) {
        // Doubler#add(10): super(10) -> Adder#add(10) = 11; + x(10) = 21.
        Some(out) => assert_eq!(out, "21\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn super_climbs_a_three_level_chain() {
    // Each level's `super` climbs exactly one rung: C -> B -> A.
    match run_ruby(
        "class A\n  def v\n    1\n  end\nend\n\
         class B < A\n  def v\n    super + 10\n  end\nend\n\
         class C < B\n  def v\n    super + 100\n  end\nend\n\
         puts C.new.v\n",
    ) {
        // C#v: super -> B#v (super -> A#v = 1, +10 = 11), +100 = 111.
        Some(out) => assert_eq!(out, "111\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn ivars_persist_through_an_inherited_method() {
    // An inherited method runs with `self` bound to the subclass instance, so its
    // `@x` reads/writes the instance's own ivars (super does not rebind self).
    match run_ruby(
        "class Counter\n  def bump\n    @n = 5\n  end\n  def peek\n    @n\n  end\nend\n\
         class FancyCounter < Counter\nend\n\
         c = FancyCounter.new\nc.bump\nputs c.peek\n",
    ) {
        Some(out) => assert_eq!(out, "5\n"),
        None => eprintln!("skip: no cc"),
    }
}
