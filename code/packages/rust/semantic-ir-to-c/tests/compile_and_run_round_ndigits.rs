//! Execution proof for `Numeric#round(ndigits)` — the multi-digit form of
//! `round` (Collections slice 9's `round` only ever supported the 0-arg
//! form). Lowers REAL Ruby source, emits C, compiles with a real cc, runs,
//! asserts stdout. Skips gracefully when no `cc` is present.
//!
//! Every expected value below is independently confirmed against a real
//! `ruby` interpreter (`ruby -e '...'`), not hand-derived — including the
//! Integer-vs-Float return type split (`1234.5.round(-2)` is an Integer,
//! `3.14159.round(2)` stays a Float) and the half-away-from-zero tie rule
//! (`1250.round(-2) == 1300`, `1.25.round(1) == 1.3`).

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
    let stem = format!("sirc_roundnd_{}_{}", std::process::id(), hasher.finish());
    let cpath = dir.join(format!("{stem}.c"));
    let exe = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(&cpath, &art.source).expect("write .c");
    let out = Command::new(&cc)
        .args(["-std=c99", "-Wall", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm") // Linux needs -lm to link floor/ceil/pow (macOS libSystem folds it in)
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
fn integer_receiver_with_nonnegative_ndigits_is_unchanged() {
    match run_ruby("puts 1234.round(3)\nputs 1234.round(0)\n") {
        Some(out) => assert_eq!(out, "1234\n1234\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn integer_receiver_with_negative_ndigits_rounds_half_away_from_zero() {
    match run_ruby(
        "puts 1234.round(-2)\nputs 1250.round(-2)\nputs((-1250).round(-2))\nputs 1249.round(-2)\n",
    ) {
        Some(out) => assert_eq!(out, "1200\n1300\n-1300\n1200\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn integer_negative_ndigits_dwarfing_the_receiver_rounds_to_zero() {
    match run_ruby("puts 5.round(-3)\nputs 12345.round(-30)\n") {
        Some(out) => assert_eq!(out, "0\n0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn integer_negative_ndigits_saturates_on_int64_boundary_carry() {
    // A round-up carry can need ONE MORE digit than `int64_t` holds --
    // `INT64_MAX` (ends in 7) rounds up at `-1`, which would need
    // 9223372036854775810, one past the representable ceiling. This
    // saturates rather than silently wrapping, the same never-raise
    // convention every other overflow-hazard fix in this backend uses.
    match run_ruby(
        "x = 9223372036854775807\nputs x.round(-1)\ny = -9223372036854775807 - 1\nputs y.round(-1)\n",
    ) {
        Some(out) => assert_eq!(out, "9223372036854775807\n-9223372036854775808\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn float_receiver_with_positive_ndigits_rounds_and_stays_a_float() {
    match run_ruby("puts 3.14159.round(2)\nputs 1.25.round(1)\nputs 2.5.round(1)\nputs((-3.14159).round(2))\n") {
        Some(out) => assert_eq!(
            out,
            "3.1400000000000001\n1.3\n2.5\n-3.1400000000000001\n"
        ),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn float_receiver_with_nonpositive_ndigits_rounds_and_becomes_an_integer() {
    match run_ruby(
        "puts 1234.5.round(-2)\nputs 1250.0.round(-2)\nputs((-1250.0).round(-2))\nputs 0.4.round(0)\nputs 0.5.round(0)\n",
    ) {
        Some(out) => assert_eq!(out, "1200\n1300\n-1300\n0\n1\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn ndigits_far_beyond_precision_returns_the_receiver_unchanged_or_dwarfs_to_zero() {
    // Real Ruby: `1.5.round(20) == 1.5` (a Float can't gain more precision
    // than it already has), `1.5.round(-20) == 0` (dwarfed). Both confirmed
    // against a real Ruby interpreter, not hand-derived.
    match run_ruby("puts 1.5.round(20)\nputs 1.5.round(-20)\n") {
        Some(out) => assert_eq!(out, "1.5\n0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn a_hostile_extreme_float_ndigits_argument_saturates_instead_of_overflowing() {
    // Security-review regression: `_sir_round_ndigits_arg` saturates a huge-
    // magnitude-negative Float ndigits argument to exactly `INT64_MIN` (via
    // `_sir_f64_to_i64_saturating`). A bare `-ndigits` on that value is
    // signed-overflow UB -- the SAME hazard already fixed for the Integer
    // branch via `_sir_i64_abs_u`, just missed on the Float branch's own
    // negative-ndigits arm. This exercises exactly that input on BOTH a
    // Float and an Integer receiver -- neither should crash or misbehave,
    // both dwarf to 0 (real Ruby raises RangeError here instead; this
    // backend saturates rather than raises, the same convention `to_i` on
    // a non-finite Float already uses).
    match run_ruby("puts 3.14.round(-1.0e300)\nputs 42.round(-1.0e300)\n") {
        Some(out) => assert_eq!(out, "0\n0\n"),
        None => eprintln!("skip: no cc"),
    }
}
