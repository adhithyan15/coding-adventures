//! Execution proof for Collections slice 10 (Symbol + universal Object/Bool
//! methods) on the C backend — lower REAL Ruby source, emit C, compile with
//! a real cc, run, assert stdout. Skips gracefully when no `cc` is present.

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
    let stem = format!("sirc_sym10_{}_{}", std::process::id(), hasher.finish());
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
fn symbol_to_s_length_size_empty() {
    match run_ruby("puts :hello.to_s\nputs :hello.length\nputs :hello.size\nputs :\"\".empty?\n") {
        Some(out) => assert_eq!(out, "hello\n5\n5\n#t\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn symbol_upcase_downcase_return_a_fresh_symbol_not_a_string() {
    // `:foo.upcase == :FOO` -- a Symbol, not a String; proven by round-
    // tripping through `to_sym` (identity on a Symbol) and `to_s`.
    match run_ruby("x = :foo.upcase\nputs x\nputs x.to_sym.to_s\n") {
        Some(out) => assert_eq!(out, "FOO\nFOO\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn symbol_inspect_prefixes_a_colon() {
    match run_ruby("puts :hello.inspect\n") {
        Some(out) => assert_eq!(out, ":hello\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn symbol_to_sym_is_the_identity() {
    match run_ruby("x = :hello\nputs x.to_sym.equal?(x)\n") {
        Some(out) => assert_eq!(out, "#t\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn nil_p_across_receiver_types() {
    match run_ruby("puts nil.nil?\nputs 0.nil?\nputs \"\".nil?\nputs false.nil?\n") {
        Some(out) => assert_eq!(out, "#t\n#f\n#f\n#f\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn itself_returns_the_receiver() {
    match run_ruby("puts 5.itself\nputs \"hi\".itself\n") {
        Some(out) => assert_eq!(out, "5\nhi\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn frozen_p_is_true_for_immutable_primitives_false_for_mutable_ones() {
    match run_ruby(
        "puts nil.frozen?\nputs true.frozen?\nputs 5.frozen?\nputs 3.5.frozen?\nputs :sym.frozen?\n\
         puts \"str\".frozen?\nputs [1].frozen?\n",
    ) {
        Some(out) => assert_eq!(out, "#t\n#t\n#t\n#t\n#t\n#f\n#f\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn equal_p_is_pointer_identity_not_structural_equality() {
    // Two SEPARATELY built arrays with equal content are NOT `equal?`
    // (structural equality is `==`, unaffected by this slice); the SAME
    // array through an alias IS.
    match run_ruby(
        "a = [1, 2]\nb = [1, 2]\nputs a.equal?(b)\nc = a\nputs a.equal?(c)\n",
    ) {
        Some(out) => assert_eq!(out, "#f\n#t\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn equal_p_on_scalars_and_symbols_is_value_identity() {
    // Scalars/Symbols have no separate identity from value in Ruby (and
    // Symbols are interned, so this reduces to the same pointer-identity
    // check as the heap-boxed case above).
    match run_ruby("puts 5.equal?(5)\nputs :sym.equal?(:sym)\nputs nil.equal?(nil)\n") {
        Some(out) => assert_eq!(out, "#t\n#t\n#t\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn equal_p_returns_false_across_different_types() {
    match run_ruby("puts 5.equal?(\"5\")\nputs nil.equal?(false)\n") {
        Some(out) => assert_eq!(out, "#f\n#f\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn bool_and_or_xor_are_eager_and_ruby_truthiness_coercing() {
    // `true & nil == false`, `false | 0 == true` -- eager (not `&&`/`||`
    // short-circuit, which the frontend lowers to `If`, never a method
    // call), and every non-nil/non-false operand coerces to truthy
    // (`0`/`""` included -- Ruby's rule, unlike Python's).
    match run_ruby(
        "puts(true.&(nil))\nputs(false.|(0))\nputs(true.^(true))\nputs(true.^(false))\n",
    ) {
        Some(out) => assert_eq!(out, "#f\n#t\n#f\n#t\n"),
        None => eprintln!("skip: no cc"),
    }
}
