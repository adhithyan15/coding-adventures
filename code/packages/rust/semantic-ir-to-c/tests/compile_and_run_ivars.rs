//! Execution proof for OOP mirror slice 3 (instance variables + `self`) on the C
//! backend — lower REAL Ruby source, emit C, compile with a real cc, run, assert
//! stdout.  Skips gracefully when no `cc` is present.
//!
//! An `@x` read/write lowers to a `Scope::Instance` `VarRef`/`Assign`, which the
//! emitter routes through `_sir_ivar_get`/`_sir_ivar_set` on `_sir_current_self`
//! — the receiver the dispatch bound.  The `@`-name is a QUOTED C string literal
//! (no injection).  A bare `self` lowers to `__self__` → `_sir_self()`.

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
    let stem = format!("sirc_ivar_{}_{}", std::process::id(), hasher.finish());
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
fn ivar_set_then_get() {
    // `@v` written by one method, read by another, on a stored instance.
    match run_ruby(
        "class Box\n  def set(v)\n    @v = v\n  end\n  def get\n    @v\n  end\nend\n\
         b = Box.new\nb.set(7)\nputs b.get\n",
    ) {
        Some(out) => assert_eq!(out, "7\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn two_ivars_summed() {
    // Two distinct `@`-names on one instance, combined in a third method.
    match run_ruby(
        "class Pair\n  def init\n    @a = 1\n    @b = 2\n  end\n  def sum\n    @a + @b\n  end\nend\n\
         p = Pair.new\np.init\nputs p.sum\n",
    ) {
        Some(out) => assert_eq!(out, "3\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn ivar_read_modify_write_persists() {
    // `@n = @n + 1` reads and writes the same ivar; state persists across calls.
    match run_ruby(
        "class Counter\n  def start\n    @n = 0\n  end\n  def inc\n    @n = @n + 1\n  end\n  \
         def val\n    @n\n  end\nend\n\
         c = Counter.new\nc.start\nc.inc\nc.inc\nc.inc\nputs c.val\n",
    ) {
        Some(out) => assert_eq!(out, "3\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn self_returns_receiver_for_chaining() {
    // A bare `self` (`__self__` → `_sir_self()`) returns the receiver, so a method
    // returning `self` can be chained into another dispatch.
    match run_ruby(
        "class Widget\n  def me\n    self\n  end\n  def size\n    9\n  end\nend\n\
         w = Widget.new\nputs w.me.size\n",
    ) {
        Some(out) => assert_eq!(out, "9\n"),
        None => eprintln!("skip: no cc"),
    }
}
