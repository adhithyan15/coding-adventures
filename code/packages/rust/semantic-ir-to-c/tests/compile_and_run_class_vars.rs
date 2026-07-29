//! Execution proof for OOP mirror slice 6 (class variables `@@x`) on the C
//! backend — lower REAL Ruby source, emit C, compile with a real cc, run, assert
//! stdout.  Skips gracefully when no `cc` is present.
//!
//! A class variable belongs to a CLASS (shared down its hierarchy): a class-body
//! `@@x = 0` initializer seeds `(class, @@x)` storage (`_sir_cvar_set_in`), and a
//! method body reads/writes it (`_sir_cvar_get`/`_sir_cvar_set`) resolving the
//! owning class from `_sir_current_class` (which dispatch binds to the receiver's
//! class for an instance method, or to the class for a class method).

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
    let stem = format!("sirc_cv_{}_{}", std::process::id(), src.len());
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
fn class_var_shared_between_class_and_instance_methods() {
    // `@@count` is seeded in the class body, bumped by a class method, and read
    // by an instance method — all resolve to the SAME storage.
    match run_ruby(
        "class Counter\n  @@count = 0\n  def self.bump\n    @@count = @@count + 1\n  end\n  \
         def peek\n    @@count\n  end\nend\n\
         Counter.bump\nCounter.bump\nc = Counter.new\nputs c.peek\n",
    ) {
        Some(out) => assert_eq!(out, "2\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn class_var_written_by_an_instance_method() {
    // An instance method reads-modifies-writes `@@total`; a second reads it back.
    match run_ruby(
        "class Box\n  @@total = 0\n  def add(n)\n    @@total = @@total + n\n  end\n  \
         def total\n    @@total\n  end\nend\n\
         b = Box.new\nb.add(3)\nb.add(4)\nputs b.total\n",
    ) {
        Some(out) => assert_eq!(out, "7\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn class_var_is_shared_down_the_hierarchy() {
    // A subclass instance sees the parent's `@@count` (one variable per
    // hierarchy), so bumping via the parent is visible through the subclass.
    match run_ruby(
        "class Counter\n  @@count = 0\n  def self.bump\n    @@count = @@count + 1\n  end\n  \
         def peek\n    @@count\n  end\nend\n\
         class Sub < Counter\nend\n\
         Counter.bump\nCounter.bump\ns = Sub.new\nputs s.peek\n",
    ) {
        Some(out) => assert_eq!(out, "2\n"),
        None => eprintln!("skip: no cc"),
    }
}
