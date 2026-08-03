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
        .arg("-lm")  // Linux needs -lm to link floor/ceil/fabs (macOS libSystem folds it in)
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
fn gcd_and_digits_on_int64_min_do_not_hit_negation_ub() {
    // Regression: `gcd`/`digits` used to negate a NEGATIVE `int64_t` with a
    // bare unary `-` to get its magnitude. `-INT64_MIN` is signed-overflow
    // UB (no positive `int64_t` can hold 2^63) -- verified to actually give
    // build-dependent WRONG answers (and, under some optimization levels, a
    // hardware trap) before the fix, not just a theoretical concern.
    // `-9223372036854775807 - 1` constructs INT64_MIN without a literal that
    // itself overflows i64::MAX during lexing (`-9223372036854775808` as a
    // single token doesn't parse: the positive digit run overflows first).
    match run_ruby(
        "x = -9223372036854775807 - 1\nputs x.gcd(6)\nputs x.digits.length\n",
    ) {
        Some(out) => assert_eq!(out, "2\n19\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn floor_ceil_round_saturate_on_non_finite_or_out_of_range_floats() {
    // Regression: a bare `(int64_t)double_value` cast is UB once the value
    // is non-finite or exceeds int64 range -- verified platform-dependent
    // (arm64 saturates, x86 gives a different "integer indefinite" value for
    // the SAME input). Guarded to saturate deterministically to
    // INT64_MAX/INT64_MIN/0 regardless of target, matching this runtime's
    // other numeric conversions' never-raise floor.
    match run_ruby(
        "puts((1.0 / 0.0).floor)\n\
         puts((-1.0 / 0.0).ceil)\n\
         puts((1.0 / 0.0 * 0.0).round)\n\
         puts(1.0e300.floor)\n",
    ) {
        Some(out) => assert_eq!(
            out,
            "9223372036854775807\n-9223372036854775808\n0\n9223372036854775807\n"
        ),
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

#[test]
fn divmod_by_negative_one_does_not_overflow_on_int64_min() {
    // Regression: `INT64_MIN / -1` is signed-overflow UB in C -- and unlike
    // the OTHER divmod overflow below, this one is UB in the DIVISION
    // ITSELF, not just an intermediate product. Special-cased since it
    // divides evenly.
    match run_ruby("x = -9223372036854775807 - 1\nputs(x.divmod(-1))\n") {
        Some(out) => assert_eq!(out, "[-9223372036854775808, 0]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn divmod_near_int64_min_does_not_overflow_computing_the_remainder() {
    // Regression: an EARLIER version computed the floored remainder as
    // `a - q * b`. For `a = INT64_MIN, b = 3`, the floored quotient is
    // `-3074457345618258603`, and `q * b` ALONE overflows `int64_t` by 1 --
    // even though the final remainder (1) fits trivially. Fixed by adjusting
    // the TRUNCATING remainder (`a % b`) directly instead of multiplying.
    match run_ruby("x = -9223372036854775807 - 1\nputs(x.divmod(3))\n") {
        Some(out) => assert_eq!(out, "[-3074457345618258603, 1]\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn gcd_with_a_zero_operand_saturates_instead_of_overflowing() {
    // Regression: `0.gcd(x) == x.abs` in Ruby, and `|INT64_MIN|` is exactly
    // 2^63 -- one past `INT64_MAX` (2^63-1). This runtime has no bignum to
    // hold the true value, so it saturates to `INT64_MAX` rather than
    // narrowing-overflowing into `(int64_t)` (which silently wrapped to
    // INT64_MIN before the fix).
    match run_ruby("x = -9223372036854775807 - 1\nputs(0.gcd(x))\n") {
        Some(out) => assert_eq!(out, "9223372036854775807\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn upto_downto_at_the_int64_boundary_do_not_overflow_the_counter() {
    // Regression: `for (; i <= n; i++)` increments `i` past `n` even on the
    // LAST iteration before the loop test re-runs and stops it -- UB the
    // moment `i == INT64_MAX` (`upto`) or `i == INT64_MIN` (`downto`).
    // `INT64_MAX.upto(INT64_MAX)` / `INT64_MIN.downto(INT64_MIN)` are each
    // exactly one iteration and must not crash or hang.
    match run_ruby(
        "x = 9223372036854775807\nx.upto(x) { |i| puts i }\n\
         y = -9223372036854775807 - 1\ny.downto(y) { |i| puts i }\n",
    ) {
        Some(out) => assert_eq!(out, "9223372036854775807\n-9223372036854775808\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn step_stops_rather_than_overflowing_when_the_stride_would_cross_int64_max() {
    // Regression: `v += st` overflows once `v` is close enough to
    // `INT64_MAX` that adding `st` would cross it. There is no next
    // in-range value to visit, so the fix stops the iteration there instead
    // of computing the (UB) overflowing sum.
    match run_ruby(
        "x = 9223372036854775806\nx.step(9223372036854775807, 5) { |i| puts i }\nputs \"done\"\n",
    ) {
        Some(out) => assert_eq!(out, "9223372036854775806\ndone\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn abs_and_pred_on_int64_min_saturate_instead_of_overflowing() {
    // Round-3 regression: `abs`/`pred` had the SAME `-INT64_MIN`/`INT64_MIN
    // - 1` overflow hazard already fixed for `gcd`/`digits` and `divmod`,
    // just not swept to these two call sites in the earlier rounds.
    // `abs(INT64_MIN)` used to return a NEGATIVE number (`-INT64_MIN`
    // wrapped); `pred(INT64_MIN)` used to wrap to `INT64_MAX`, the opposite
    // end of the range. Both now saturate at the representable floor/ceiling.
    match run_ruby("x = -9223372036854775807 - 1\nputs(x.abs)\nputs(x.pred)\n") {
        Some(out) => assert_eq!(out, "9223372036854775807\n-9223372036854775808\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn to_i_on_a_non_finite_float_saturates_instead_of_overflowing() {
    // Round-3 regression: widening `to_i` to accept a numeric receiver
    // (this slice) routed a Float through the pre-existing generic
    // `_sir_to_i`, whose `(int64_t)` cast is UB for a non-finite or
    // out-of-range Float -- the SAME hazard `floor`/`ceil`/`round` already
    // guard against. Now routes through the same saturating helper.
    match run_ruby(
        "puts((1.0 / 0.0).to_i)\nputs((-1.0 / 0.0).to_i)\nputs((1.0 / 0.0 * 0.0).to_i)\n",
    ) {
        Some(out) => assert_eq!(
            out,
            "9223372036854775807\n-9223372036854775808\n0\n"
        ),
        None => eprintln!("skip: no cc"),
    }
}
