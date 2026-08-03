//! Execution proof for SIR18 string interpolation (`"a#{x}b"`) on the C
//! backend — lower REAL Ruby source, emit C, compile with a real cc, run,
//! assert stdout. Skips gracefully when no `cc` is present.
//!
//! The Ruby frontend already lowers `"a#{x}b"` to `Expr::StrConcat { parts }`
//! for every backend (Python/TypeScript already emit it); the C backend
//! rejected `Feature::StringInterpolation` outright until this PR. Each part
//! renders through a new `_sir_display_str` runtime helper (Ruby's `to_s`
//! style — a string-returning parallel to the `puts`/`print` FILE*-writing
//! `_sir_fmt`) and the parts concatenate with `_sir_cat`.

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
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut hasher);
    let stem = format!("sirc_interp_{}_{}", std::process::id(), hasher.finish());
    let cpath = dir.join(format!("{stem}.c"));
    let exe = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(&cpath, &art.source).expect("write .c");
    let out = Command::new(&cc)
        .args(["-std=c99", "-Wall", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm")
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
fn interpolates_a_local_variable() {
    match run_ruby("x = 5\nputs \"value is #{x}\"\n") {
        Some(out) => assert_eq!(out, "value is 5\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn interpolates_an_expression() {
    match run_ruby("x = 5\nputs \"next is #{x + 1}\"\n") {
        Some(out) => assert_eq!(out, "next is 6\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn interpolates_multiple_segments_in_order() {
    match run_ruby("a = 1\nb = 2\nputs \"#{a} + #{b} = #{a + b}\"\n") {
        Some(out) => assert_eq!(out, "1 + 2 = 3\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn interpolates_a_string_value_without_extra_quoting() {
    match run_ruby("name = \"world\"\nputs \"hello #{name}\"\n") {
        Some(out) => assert_eq!(out, "hello world\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn interpolates_an_array_value() {
    match run_ruby("a = [1, 2, 3]\nputs \"list: #{a}\"\n") {
        Some(out) => assert_eq!(out, "list: [1, 2, 3]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn interpolates_a_nil_value() {
    // Matches this backend's existing `puts nil` -> "nil" convention (see
    // e.g. compile_and_run_index_bracket.rs) -- `_sir_display_str` and
    // `_sir_fmt` render SIR_NIL identically.
    match run_ruby("x = nil\nputs \"got [#{x}]\"\n") {
        Some(out) => assert_eq!(out, "got [nil]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn interpolation_inside_a_method_body_uses_a_parameter() {
    // A compound part (a method call through __method__) reaches the hoisted
    // emit_assign path, not just the simple emit_expr path.
    match run_ruby(
        "class Greeter\n  def hello(name)\n    \"hi #{name}!\"\n  end\nend\n\
         puts Greeter.new.hello(\"Ada\")\n",
    ) {
        Some(out) => assert_eq!(out, "hi Ada!\n"),
        None => eprintln!("skip: no cc"),
    }
}
