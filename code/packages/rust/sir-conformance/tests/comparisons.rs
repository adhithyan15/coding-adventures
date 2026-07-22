//! # Comparison-operator conformance — `==`, `!=`, `<=`, `>=` on every backend
//!
//! The Ruby frontend lowers a comparison chain (`a == b`, `a != b`, `a <= b`,
//! `a >= b`) to operator-spelling builtins (`ruby_to_semantic_ir`'s
//! `lower_comparison_chain`). Only the C and Ruby backends lowered those
//! names; **Python, JavaScript, Go and Rust rejected them** — so even
//! `puts(1 == 1)` failed:
//!
//! | backend    | `puts(1 == 1)` was            | now |
//! |------------|-------------------------------|-----|
//! | Python     | `NameError: builtin '=='`     | ✅ `#t` — emitter maps `==`→`_sir_eq`, and `!=`/`<=`/`>=` to new `ne`/`le`/`ge` |
//! | JavaScript | `TypeError: unknown builtin ==`| ✅ `#t` — only `==` was unmapped; `!=`/`<=`/`>=` already routed |
//! | Go         | `panic: unknown builtin: ==`  | ✅ `#t` — emitter + runtime dispatch gain the four names |
//! | Rust       | emitted a call to a missing fn | ✅ `#t` — emitter maps them to new `ne`/`le`/`ge` helpers |
//! | C, Ruby    | (already worked)              | ✅ unchanged |
//!
//! `==` is a synonym for the `=` (structural equality) each backend already
//! had; `!=` is its exact negation; `<=`/`>=` are defined from the same
//! primitives as `<`/`>` (`a <= b ⟺ a < b or a == b`). The new `ne`/`le`/`ge`
//! runtime helpers mirror the C backend's `_sir_ne`/`_sir_le`/`_sir_ge`.
//!
//! **String ordering, too.** The cases cover integers, a cross int/float pair
//! (`1 == 1.0`), string equality, AND string *ordering* (`"a" < "b"`). The Go
//! runtime's `<`/`>`/`<=`/`>=` previously coerced through `_sir_as_float`,
//! which PANICS on a string — so `"a" < "b"` crashed rather than ordering.
//! This change gives Go a lexicographic string fast-path (via `_sir_cmp`),
//! matching Ruby, C, Rust and Python, so all six now agree on string order.
//!
//! **Composite equality.** `==`/`!=` on arrays are STRUCTURAL, not reference
//! identity — the end-to-end proof that the JavaScript backend's `eq`→`valEq`
//! change agrees cross-backend (JS used to answer `[1,2] == [1,2]` false). This
//! now runs on ALL SIX backends: the Ruby (0.4.0) and C (0.6.0) backends gained
//! the `sequences` feature, so an array-literal program lowers and asserts on
//! each — C was the last that skipped it.
//!
//! Booleans render in the harness's default (Lisp) convention, so the expected
//! strings are `#t` / `#f`.

use sir_conformance::{run_source, RunOutcome, Target};

/// `(ruby_expression, expected_display)` — each is `puts`-wrapped and run
/// through every backend. Expected strings are the Lisp-convention booleans
/// the conformance harness emits.
const CASES: &[(&str, &str)] = &[
    // `==` — equality, including cross int/float (Ruby `1 == 1.0` is true).
    ("1 == 1", "#t"),
    ("1 == 2", "#f"),
    ("1 == 1.0", "#t"),
    ("\"a\" == \"a\"", "#t"),
    ("\"a\" == \"b\"", "#f"),
    // `!=` — the exact negation of `==`.
    ("1 != 2", "#t"),
    ("1 != 1", "#f"),
    ("\"a\" != \"b\"", "#t"),
    ("\"a\" != \"a\"", "#f"),
    // `<=` — less-than-or-equal: strictly-less, equal, and strictly-greater.
    ("1 <= 2", "#t"),
    ("2 <= 2", "#t"),
    ("3 <= 2", "#f"),
    ("1 <= 1.0", "#t"), // int vs float compares by value
    // `>=` — the mirror of `<=`.
    ("2 >= 1", "#t"),
    ("2 >= 2", "#t"),
    ("1 >= 2", "#f"),
    // `<` / `>` — already worked for numbers; guard against a regression from
    // the new sibling arms landing in the same match.
    ("1 < 2", "#t"),
    ("2 > 1", "#t"),
    // String ORDERING — lexicographic on every backend (Go gained this here).
    ("\"a\" < \"b\"", "#t"),
    ("\"b\" < \"a\"", "#f"),
    ("\"a\" <= \"a\"", "#t"),
    ("\"b\" > \"a\"", "#t"),
    ("\"a\" >= \"a\"", "#t"),
    ("\"ab\" < \"b\"", "#t"), // prefix vs longer: 'a' < 'b'
    // COMPOSITE equality — `==`/`!=` are STRUCTURAL for arrays, not reference
    // identity. This is the end-to-end proof that the JavaScript `eq`→`valEq`
    // change agrees cross-backend: JS used to answer `[1,2] == [1,2]` false
    // (reference), the other backends true (structural). Now asserts on ALL SIX
    // backends — Ruby and C gained the `sequences` feature (C was last).
    ("[1, 2] == [1, 2]", "#t"),
    ("[1, 2] == [1, 3]", "#f"),
    ("[1, 2] != [1, 3]", "#t"),
    ("[1, 2] != [1, 2]", "#f"),
];

/// The frontier: every backend that *runs* a case must reproduce Ruby's
/// answer. Asserts on `Ran`; a `Failed` (an unknown-builtin crash — the very
/// bug this closes) fails the test naming the backend; `Skipped` (no toolchain)
/// is not asserted. Verified locally across Python, JavaScript, Go, Rust, C and
/// Ruby.
#[test]
fn comparison_operators_match_ruby_on_every_backend() {
    let mut ran = 0usize;
    for &(expr, expected) in CASES {
        let ruby = format!("puts({expr})\n");
        for &target in Target::all() {
            match run_source("comparisons", &ruby, target) {
                RunOutcome::Ran(out) => {
                    assert_eq!(
                        out,
                        expected,
                        "\nCOMPARISON FRONTIER: backend {} computed `{expr}` = {out:?}, Ruby = {expected:?}\n",
                        target.tag(),
                    );
                    ran += 1;
                }
                RunOutcome::Failed(msg) => panic!(
                    "COMPARISON FRONTIER: backend {} failed on `{expr}`: {}",
                    target.tag(),
                    msg.lines().next().unwrap_or("")
                ),
                RunOutcome::Skipped(_) => {}
            }
        }
    }
    assert!(ran > 0, "no backend toolchain available — the frontier proved nothing");
}
