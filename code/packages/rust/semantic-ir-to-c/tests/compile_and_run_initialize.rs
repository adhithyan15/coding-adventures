//! Execution proof that `Foo.new` runs the class's `initialize` on the C
//! backend — lower REAL Ruby source, emit C, compile with a real cc, run,
//! assert stdout. Skips gracefully when no `cc` is present.
//!
//! Before this fix, `__new__` only allocated (`_sir_new_instance`) and never
//! invoked `initialize`, so every `@ivar` a constructor set was silently
//! nil — masked in earlier tests by `_sir_plus_v`'s nil-defaults-to-0
//! arithmetic coincidentally making `Counter`-style examples still print the
//! "right" answer for `@n = 0`-style starts. `_sir_call_new` (the fix) mirrors
//! the Go/Rust/Ruby backends: allocate, resolve `initialize` up the ancestry,
//! invoke it with the constructor args if found, return the object either way.

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
    // Hash the full source (not just its length) so two equal-length sources
    // running as parallel threads in the same process (cargo test's default)
    // don't collide on the same temp file — see compile_and_run_class_methods.rs.
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut hasher);
    let stem = format!("sirc_init_{}_{}", std::process::id(), hasher.finish());
    let cpath = dir.join(format!("{stem}.c"));
    let exe = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(&cpath, &art.source).expect("write .c");
    let out = Command::new(&cc)
        .args(["-std=c99", "-Wall", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm") // Linux needs -lm to link floor/ceil/fabs (macOS libSystem folds it in)
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
fn initialize_with_no_args_sets_an_ivar() {
    match run_ruby(
        "class Counter\n  def initialize\n    @n = 0\n  end\n  def inc\n    @n = @n + 1\n  end\n  \
         def value\n    @n\n  end\nend\n\
         c = Counter.new\nc.inc\nc.inc\nputs c.value\n",
    ) {
        Some(out) => assert_eq!(out, "2\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn initialize_receives_constructor_arguments() {
    // Before the fix, `Point.new(3, 4)` compiled (constructor args were
    // accepted at the SIR level) but silently dropped them -- `@x`/`@y`
    // stayed nil. Now `initialize`'s params bind the `.new` args.
    match run_ruby(
        "class Point\n  def initialize(x, y)\n    @x = x\n    @y = y\n  end\n  \
         def sum\n    @x + @y\n  end\nend\n\
         p = Point.new(3, 4)\nputs p.sum\n",
    ) {
        Some(out) => assert_eq!(out, "7\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn class_with_no_initialize_still_constructs() {
    // No `initialize` registered for `Empty` -- `_sir_call_new` must still
    // return a plain allocation (Ruby's default no-op `Object#initialize`),
    // not raise or leave the object unusable.
    match run_ruby(
        "class Empty\n  def tag\n    1\n  end\nend\n\
         e = Empty.new\nputs e.tag\n",
    ) {
        Some(out) => assert_eq!(out, "1\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn initialize_is_inherited_by_a_subclass_with_none_of_its_own() {
    // `Sub` defines no `initialize` of its own, so `Sub.new` must resolve
    // `Animal#initialize` up the ancestry (the same `_sir_resolve_method` walk
    // `_sir_call_method` already uses for ordinary dispatch) and run it on the
    // new `Sub` instance.
    match run_ruby(
        "class Animal\n  def initialize(name)\n    @name = name\n  end\n  \
         def label\n    @name\n  end\nend\n\
         class Dog < Animal\nend\n\
         d = Dog.new(\"Rex\")\nputs d.label\n",
    ) {
        Some(out) => assert_eq!(out, "Rex\n"),
        None => eprintln!("skip: no cc"),
    }
}
