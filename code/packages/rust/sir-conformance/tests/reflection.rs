//! # Type-reflection conformance — `.class` across the backends
//!
//! Ruby answers `7.class == Integer`, `7.0.class == Float`, `"hi".class ==
//! String`. Each backend must reproduce the same class-NAME strings, so a
//! translated program that branches on a type reads identically everywhere.
//!
//! ## Where the backends stand (measured, not assumed)
//!
//! | backend    | `.class` |
//! |------------|----------|
//! | Python     | ✅ |
//! | Go         | ✅ (`_sir_ruby_class_name`) |
//! | JavaScript | ✅ **as of this change** — previously raised `NoMethodError` |
//! | Rust       | ❌ **panics at runtime** (exit 101) — tracked separately |
//! | C          | not yet emitted (skips) |
//! | Ruby       | ✅ (the reference; skips without a `ruby` toolchain) |
//!
//! Because the Rust arm still crashes, this is a **per-backend** guard in the
//! style of `division.rs`'s `python_division_is_ruby_floor_faithful` rather
//! than an all-backend frontier; it locks the arms that are closed and will
//! grow into a cross-backend assertion once Rust's `.class` is implemented.
//!
//! The JavaScript arm is the interesting one: JS numbers are all `f64`, so
//! `7` and `7.0` are the SAME value there. Distinguishing `Integer` from
//! `Float` is only possible because the backend now carries a tagged float —
//! before that, `7.0.class` was unanswerable in principle, not just missing.

use sir_conformance::{run_source, RunOutcome, Target};

/// `(ruby source, expected class name)` — the reflection surface every
/// backend should agree on.
const CASES: &[(&str, &str)] = &[
    ("puts(7.class)\n", "Integer"),
    ("puts(7.0.class)\n", "Float"),
    ("puts(\"hi\".class)\n", "String"),
    ("puts([1, 2].class)\n", "Array"),
    ("puts(nil.class)\n", "NilClass"),
    ("puts(true.class)\n", "TrueClass"),
    ("puts(false.class)\n", "FalseClass"),
];

/// Run `CASES` against one backend, asserting every case that actually runs.
/// `Skipped` (no toolchain) is inert rather than falsely green.
fn assert_class_names_on(target: Target, label: &str) -> usize {
    let mut ran = 0usize;
    for &(src, expected) in CASES {
        match run_source("reflection", src, target) {
            RunOutcome::Ran(out) => {
                assert_eq!(
                    out, expected,
                    "\n{label}: `{}` gave {out}, Ruby says {expected}\n",
                    src.trim()
                );
                ran += 1;
            }
            RunOutcome::Failed(msg) => panic!(
                "{label}: `{}` failed: {}",
                src.trim(),
                msg.lines().next().unwrap_or("")
            ),
            RunOutcome::Skipped(_) => {}
        }
    }
    ran
}

/// The JavaScript arm, **closed by this change**. `.class` previously raised
/// `NoMethodError` on every receiver (the backend implemented no type
/// reflection at all); it now reports the Ruby class name, including the
/// `Integer`/`Float` split its tagged floats make representable.
#[test]
fn javascript_class_reflection_is_ruby_faithful() {
    let ran = assert_class_names_on(Target::JavaScript, "JavaScript `.class`");
    if ran == 0 {
        eprintln!("note: `node` unavailable — JavaScript reflection not proved");
    }
}

/// The Go arm, already closed (`_sir_ruby_class_name`). Guards against a
/// regression and pins the exact class-name strings the JS arm mirrors.
#[test]
fn go_class_reflection_is_ruby_faithful() {
    let ran = assert_class_names_on(Target::Go, "Go `.class`");
    if ran == 0 {
        eprintln!("note: `go` unavailable — Go reflection not proved");
    }
}
