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
//! | Rust       | ✅ **as of this change** — previously panicked at runtime (exit 101) |
//! | C          | not yet emitted (skips) |
//! | Ruby       | ✅ (the reference; skips without a `ruby` toolchain) |
//!
//! With the Rust arm now closed, the guard has grown from per-backend into a
//! **cross-backend** assertion: [`class_reflection_matches_ruby_on_every_backend`]
//! holds every backend that runs to the same class-name strings, exactly like
//! `division.rs`'s frontier. The per-backend tests remain as granular guards
//! that name the offending backend directly when one regresses.
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

/// The Rust arm, **closed by this change**. Rust already had the class-name
/// mapping (`ruby_class_name`) but used it ONLY to build a `NoMethodError`
/// message — `.class` itself was undispatched, so it raised, and the unrescued
/// raise surfaced as a process-killing panic (exit 101).
#[test]
fn rust_class_reflection_is_ruby_faithful() {
    let ran = assert_class_names_on(Target::Rust, "Rust `.class`");
    if ran == 0 {
        eprintln!("note: `rustc` unavailable — Rust reflection not proved");
    }
}

/// The frontier: **every** backend that runs a case must produce the same
/// Ruby class name. Now that Python, Go, JavaScript and Rust all answer, this
/// is a live cross-backend assertion — it fails, naming the backend, the day
/// one diverges. Backends without a toolchain (or that do not yet emit
/// reflection, e.g. C) report `Skipped` and are not asserted.
#[test]
fn class_reflection_matches_ruby_on_every_backend() {
    let mut ran = 0usize;
    for &(src, expected) in CASES {
        for &target in Target::all() {
            match run_source("reflection_all", src, target) {
                RunOutcome::Ran(out) => {
                    assert_eq!(
                        out, expected,
                        "\nREFLECTION FRONTIER: backend {} gave `{}` = {out}, Ruby says {expected}\n",
                        target.tag(),
                        src.trim(),
                    );
                    ran += 1;
                }
                // A backend that emits reflection but crashes on it is a
                // frontier failure, not a skip — surface it by name.
                RunOutcome::Failed(msg) => panic!(
                    "REFLECTION FRONTIER: backend {} failed on `{}`: {}",
                    target.tag(),
                    src.trim(),
                    msg.lines().next().unwrap_or("")
                ),
                RunOutcome::Skipped(_) => {}
            }
        }
    }
    assert!(ran > 0, "no backend toolchain available — reflection proved nothing");
}

/// `(ruby source, expected output)` — the `is_a?` family and `case/when
/// SomeClass`, whose class argument is a bare CONSTANT in the source.
const IS_A_CASES: &[(&str, &str)] = &[
    ("puts(7.is_a?(Integer))\n", "#t"),
    ("puts(7.is_a?(String))\n", "#f"),
    ("puts(7.kind_of?(Numeric))\n", "#t"),
    ("puts(7.instance_of?(Integer))\n", "#t"),
    ("puts(7.instance_of?(Numeric))\n", "#f"),
    ("case 7\nwhen Integer\n  puts(\"int\")\nend\n", "int"),
    ("case \"s\"\nwhen Integer\n  puts(\"int\")\nelse\n  puts(\"other\")\nend\n", "other"),
];

/// The `is_a?` frontier. A bare constant (`Integer`, `MyClass`) as the class
/// argument used to reach the backends as a `Const` reference, which only
/// Python could cope with: Go and Rust REJECTED the program at emit ("cannot
/// lower a constant reference"), and JavaScript emitted an undefined reference
/// that blew up at run time. Since `when SomeClass` lowers to `is_a?`, that
/// meant ordinary Ruby type-dispatch compiled on exactly one backend.
///
/// The frontend now lifts the constant to a `StrLit` of its NAME — the
/// convention `lower_class_pattern` already used — so no backend needs general
/// constant-reference support, and every backend's `is_a?` (which compares
/// class names) just works.
#[test]
fn is_a_and_case_when_match_ruby_on_every_backend() {
    let mut ran = 0usize;
    for &(src, expected) in IS_A_CASES {
        for &target in Target::all() {
            match run_source("is_a_frontier", src, target) {
                RunOutcome::Ran(out) => {
                    assert_eq!(
                        out, expected,
                        "\nIS_A FRONTIER: backend {} gave `{}` = {out}, Ruby says {expected}\n",
                        target.tag(),
                        src.trim(),
                    );
                    ran += 1;
                }
                RunOutcome::Failed(msg) => panic!(
                    "IS_A FRONTIER: backend {} failed on `{}`: {}",
                    target.tag(),
                    src.trim(),
                    msg.lines().next().unwrap_or("")
                ),
                RunOutcome::Skipped(_) => {}
            }
        }
    }
    assert!(ran > 0, "no backend toolchain available — is_a? proved nothing");
}

/// `(ruby source, expected output)` — reflection on a RESCUED EXCEPTION.
const EXCEPTION_CASES: &[(&str, &str)] = &[
    ("begin\n  raise ArgumentError, \"boom\"\nrescue ArgumentError => e\n  puts(e.class)\nend\n", "ArgumentError"),
    ("begin\n  raise ArgumentError, \"boom\"\nrescue ArgumentError => e\n  puts(e.is_a?(ArgumentError))\nend\n", "#t"),
    ("begin\n  raise ArgumentError, \"boom\"\nrescue ArgumentError => e\n  puts(e.is_a?(StandardError))\nend\n", "#t"),
    ("begin\n  raise ArgumentError, \"boom\"\nrescue ArgumentError => e\n  puts(e.is_a?(TypeError))\nend\n", "#f"),
    ("begin\n  raise ArgumentError, \"boom\"\nrescue ArgumentError => e\n  puts(e.message)\nend\n", "boom"),
    // `puts e` renders the MESSAGE (Ruby's `Exception#to_s`), which must stay
    // true even where the rescued value became a real exception OBJECT.
    ("begin\n  raise ArgumentError, \"boom\"\nrescue ArgumentError => e\n  puts(e)\nend\n", "boom"),
    // A BARE `raise` carries no message, and Ruby's default `#message` is then
    // the class name.  All four backends already agree on this, but each
    // arrives at it in a different place — Rust bakes it into `SirError.msg`
    // at construction, JavaScript into the `SirError` constructor, Python into
    // `args[0]`, and Go decides it at the `message` call site (its `Msg` stays
    // nil).  Four independent spellings of one rule is precisely what drifts
    // when one of them is next touched, so pin the agreement.
    ("begin\n  raise ArgumentError\nrescue ArgumentError => e\n  puts(e.message)\nend\n", "ArgumentError"),
    // NOT pinned here: `e == e`.  A rescued exception must of course equal
    // itself, and the Rust runtime's `value_eq_d` now says so (an
    // `Rc::ptr_eq` arm — without it a wildcard match would have answered
    // `false`, and a false `e == @last_error` fails open).  But `==` cannot
    // be exercised from Ruby source on every backend yet: the Ruby frontend
    // lowers `a == b` to `BuiltinCall("==")`, and only the Go, C and Ruby
    // backends lower that name — Python, JavaScript and Rust reject it as an
    // unknown builtin, even for `puts(1 == 1)`.  That is a *general*
    // operator-coverage gap, not an exception one, so it is fixed on its own
    // branch; a case here would only re-test it.
];

/// Reflection on a rescued exception, across the backends. Every one of these
/// was wrong somewhere before:
///
/// - **Rust** bound `e` to the message STRING, not an exception — `e.class`
///   said `String`, `is_a?` was false, `message` raised NoMethodError.
/// - **Python** had no `SirError` case in `class_of`, so `e.class` said
///   `Object` and `is_a?` was false.
/// - **`e.message` failed on ALL FOUR backends** — everyday Ruby
///   (`rescue => e; puts e.message`) simply did not work anywhere.
///
/// A false `e.is_a?(SomeError)` is the dangerous shape: `rescue => e; handle
/// if e.is_a?(Recoverable)` silently skips the handler.
#[test]
fn exception_reflection_matches_ruby_on_every_backend() {
    let mut ran = 0usize;
    for &(src, expected) in EXCEPTION_CASES {
        for &target in Target::all() {
            match run_source("exc_reflection", src, target) {
                RunOutcome::Ran(out) => {
                    assert_eq!(
                        out, expected,
                        "\nEXCEPTION REFLECTION: backend {} gave `{}` = {out}, Ruby says {expected}\n",
                        target.tag(),
                        src.lines().nth(3).unwrap_or(src).trim(),
                    );
                    ran += 1;
                }
                RunOutcome::Failed(msg) => panic!(
                    "EXCEPTION REFLECTION: backend {} failed on `{}`: {}",
                    target.tag(),
                    src.lines().nth(3).unwrap_or(src).trim(),
                    msg.lines().next().unwrap_or("")
                ),
                RunOutcome::Skipped(_) => {}
            }
        }
    }
    assert!(ran > 0, "no backend toolchain available — exception reflection proved nothing");
}
