//! Execution proof for Collections slice 1 (built-in String methods) on the C
//! backend — lower REAL Ruby source, emit C, compile with a real cc, run, assert
//! stdout.  Skips gracefully when no `cc` is present.
//!
//! A `"str".method` call lowers to `__method__(recv, "method")`; when `method`
//! is a built-in name (not a user-defined method) it routes to the runtime
//! `_sir_builtin_method` dispatcher, which type-checks the receiver and applies
//! the String implementation (or raises `NoMethodError` on a wrong-type receiver,
//! matching Ruby).

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
    let stem = format!("sirc_strm_{}_{}", std::process::id(), src.len());
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
fn string_length_upcase_downcase_reverse() {
    match run_ruby(
        "puts \"hello\".length\nputs \"hello\".upcase\nputs \"WORLD\".downcase\nputs \"abc\".reverse\n",
    ) {
        Some(out) => assert_eq!(out, "5\nHELLO\nworld\ncba\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn string_to_s_is_the_string_itself() {
    match run_ruby("puts \"hi\".to_s\n") {
        Some(out) => assert_eq!(out, "hi\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn string_empty_predicate_in_control_flow() {
    // `empty?` returns a bool; used in an `if` so the assertion doesn't depend on
    // the boolean *display* convention (a separate pre-existing frontend gap).
    match run_ruby(
        "if \"\".empty?\n  puts \"blank\"\nend\nif \"x\".empty?\n  puts \"never\"\nelse\n  puts \"filled\"\nend\n",
    ) {
        Some(out) => assert_eq!(out, "blank\nfilled\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn methods_chain() {
    // `upcase` returns a String, so another built-in method dispatches on it.
    match run_ruby("puts \"hello\".upcase.reverse\n") {
        Some(out) => assert_eq!(out, "OLLEH\n"),
        None => eprintln!("skip: no cc"),
    }
}
