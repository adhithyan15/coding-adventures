//! Execution proof for Collections slice 9 (Numeric methods) on the C
//! backend — lower REAL Ruby source, emit C, compile with a real cc, run,
//! assert stdout. Skips gracefully when no `cc` is present.
//!
//! Semantics are matched against the Python/TS `sir-runtime-oop` reference
//! catalog. `even?`/`odd?`/`pred` are Integer-only here (true Ruby), unlike
//! that reference's looser dynamic typing.

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
    let stem = format!("sirc_num9_{}_{}", std::process::id(), hasher.finish());
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
fn abs_works_on_int_and_float() {
    // `puts (-5).abs` (space before the paren) hits a pre-existing frontend
    // parsing quirk unrelated to this slice (Ruby's command-call-with-a-
    // parenthesized-argument grouping) -- `puts(...)` with the call's own
    // parens sidesteps it, same workaround used throughout this file.
    match run_ruby("puts((-5).abs)\nputs((-2.5).abs)\n") {
        Some(out) => assert_eq!(out, "5\n2.5\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn even_and_odd_predicates() {
    match run_ruby("puts 4.even?\nputs 4.odd?\nputs 5.even?\nputs 5.odd?\n") {
        Some(out) => assert_eq!(out, "#t\n#f\n#f\n#t\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn zero_positive_negative_predicates() {
    match run_ruby("puts 0.zero?\nputs 5.positive?\nputs((-5).negative?)\n") {
        Some(out) => assert_eq!(out, "#t\n#t\n#t\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn pred_decrements_an_integer() {
    match run_ruby("puts 5.pred\n") {
        Some(out) => assert_eq!(out, "4\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn floor_ceil_round_on_a_float() {
    match run_ruby("puts 3.7.floor\nputs 3.2.ceil\nputs 3.5.round\nputs((-3.5).round)\n") {
        Some(out) => assert_eq!(out, "3\n4\n4\n-4\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn floor_ceil_round_on_an_integer_are_identity() {
    match run_ruby("puts 3.floor\nputs 3.ceil\nputs 3.round\n") {
        Some(out) => assert_eq!(out, "3\n3\n3\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn divmod_floors_the_quotient_and_signs_the_remainder_like_the_divisor() {
    match run_ruby("puts 7.divmod(3)\nputs((-7).divmod(3))\n") {
        Some(out) => assert_eq!(out, "[2, 1]\n[-3, 2]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn divmod_by_zero_raises_zero_division_error() {
    match run_ruby(
        "begin\n  puts 1.divmod(0)\nrescue ZeroDivisionError => e\n  puts \"caught\"\nend\n",
    ) {
        Some(out) => assert_eq!(out, "caught\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn fdiv_never_raises_on_a_zero_divisor() {
    match run_ruby("puts 1.fdiv(0)\nputs((-1).fdiv(0))\nputs 0.fdiv(0)\n") {
        Some(out) => assert_eq!(out, "Infinity\n-Infinity\nNaN\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn clamp_bounds_a_value_into_a_range() {
    match run_ruby("puts 10.clamp(1, 5)\nputs((-10).clamp(1, 5))\nputs 3.clamp(1, 5)\n") {
        Some(out) => assert_eq!(out, "5\n1\n3\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn between_checks_an_inclusive_range() {
    match run_ruby("puts 3.between?(1, 5)\nputs 10.between?(1, 5)\n") {
        Some(out) => assert_eq!(out, "#t\n#f\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn gcd_computes_the_greatest_common_divisor() {
    match run_ruby("puts 12.gcd(18)\nputs((-12).gcd(18))\n") {
        Some(out) => assert_eq!(out, "6\n6\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn digits_returns_least_significant_digit_first() {
    match run_ruby("puts 123.digits\nputs 0.digits\n") {
        Some(out) => assert_eq!(out, "[3, 2, 1]\n[0]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn times_yields_zero_through_n_minus_one() {
    match run_ruby("3.times { |i| puts i }\n") {
        Some(out) => assert_eq!(out, "0\n1\n2\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn upto_and_downto_are_inclusive() {
    match run_ruby("1.upto(3) { |i| puts i }\n3.downto(1) { |i| puts i }\n") {
        Some(out) => assert_eq!(out, "1\n2\n3\n3\n2\n1\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn step_with_default_and_explicit_stride() {
    match run_ruby("1.step(5) { |i| puts i }\n1.step(10, 3) { |i| puts i }\n") {
        Some(out) => assert_eq!(out, "1\n2\n3\n4\n5\n1\n4\n7\n10\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn step_with_zero_stride_is_a_safe_no_op_not_a_hang() {
    // The DoS-safety guard: a zero stride would never cross `limit` in a naive
    // implementation. `run` asserts the process exits 0 -- a hang would
    // timeout the test rather than return non-zero, so this proves it
    // terminates, not just that it "looks right."
    match run_ruby("1.step(5, 0) { |i| puts i }\nputs \"done\"\n") {
        Some(out) => assert_eq!(out, "done\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn to_i_and_to_f_widen_to_numeric_receivers() {
    match run_ruby("puts 3.5.to_i\nputs 3.to_f\n") {
        Some(out) => assert_eq!(out, "3\n3.0\n"),
        None => eprintln!("skip: no cc"),
    }
}
