//! Oracle/golden tests (HML01 §7): the SAME APL source, run through **two
//! independent implementations**, and diffed:
//!
//!   (a) `apl-runtime` (`coding-adventures-apl-runtime`) — this frontend's
//!       own sibling crate, a tree-walking interpreter over `array-runtime`
//!       — the ground truth.
//!   (b) `apl_to_semantic_ir::compile_source` → `semantic_ir::Module` →
//!       `semantic_ir_to_javascript::compile` → **an actual `node`
//!       process**.
//!
//! This is the direct APL sibling of
//! [`matlab-to-semantic-ir`'s own `tests/oracle.rs`](../../matlab-to-semantic-ir/tests/oracle.rs)
//! (and its `octave-to-semantic-ir` cousin) — same overall shape
//! (`node_available` skip-not-fail guard, a `Case`/`CORPUS`, a
//! `ground_truth`/`compiled` pair, one looping `#[test]`), completing
//! HML01 §5's "APL/J's own oracle tests remain open follow-on items" note
//! for APL specifically (this PR updates that line — see the spec).
//!
//! ## Two ways this file is SIMPLER than its MATLAB/Octave siblings
//! (confirmed empirically, not assumed from the task description)
//!
//! 1. **No `setup`/`final_expr` split.** The MATLAB oracle file needs one
//!    because `matlab-runtime`'s `disp` builtin is a no-op and
//!    `semantic_ir` has no "implicit display" node — so the ground-truth
//!    side reads an unsuppressed `ans = value` echo while the compiled side
//!    needs an explicit `disp(...)` call, and the two conventions can't
//!    share one literal string. Neither problem exists here: APL auto-prints
//!    a bare (non-assignment) top-level expression **natively**, on BOTH
//!    sides — `apl_runtime::eval` returns exactly that auto-print output
//!    (see that crate's module doc, and its own `assignment_is_silent_bare_
//!    expression_prints` test), and `apl-to-semantic-ir`'s lowering wraps a
//!    bare top-level `value_expr` in the shared `"print"` builtin
//!    unconditionally (`src/lower.rs`'s "Auto-print, not MATLAB-style
//!    suppression" section) — exactly what `tests/e2e_node.rs` already
//!    relies on for every one of its 9 tests. So [`Case`] here is just
//!    `name` + `source` (one full program, identical on both sides,
//!    ending in a bare printing expression) + `expected`.
//! 2. **No `normalize()`.** Verified, not assumed: every [`CORPUS`] entry
//!    below is asserted equal to `expected` from BOTH sides, and while
//!    drafting this file an extra `assert_eq!(ground_truth(..), compiled(..))`
//!    (comparing the two raw outputs to each other, before either is even
//!    compared to `expected`) was added temporarily to every case and
//!    passed byte-for-byte every time — including the high-minus glyph
//!    (`¯8`, not `-8`) and multi-line matrix layout — then removed once
//!    confirmed, since it added no signal beyond the two `expected`
//!    comparisons already there. Two independent reasons this holds, unlike
//!    MATLAB (whose oracle file needs `#t`/`true` → `1` normalization for
//!    JS-native comparison booleans):
//!    - **Display formatting is a literal 1:1 port.** `apl-runtime`'s own
//!      `value::display` (high-minus `¯`, bare value, space-separated
//!      vector, right-aligned matrix) and `semantic-ir-to-javascript`'s
//!      `runtime.rs` `display`/`fmtNum` (confirmed via
//!      `RUNTIME.contains("return x < 0 ? \"¯\" + body : body;")` in that
//!      crate's own tests) are the same formatting rules transcribed twice,
//!      by design (`tests/e2e_node.rs`'s module doc, point 1).
//!    - **APL comparisons never surface a JS-native boolean.** Unlike
//!      MATLAB (whose comparisons compile to native JS `<`/`>`/`===`, so
//!      `format()` prints Scheme-style `#t`/`#f`), APL's 6 comparison atoms
//!      are `ElementwiseOpKind` variants (`Eq`/`Ne`/`Lt`/`Le`/`Ge`/`Gt`)
//!      whose codegen (`semantic-ir-to-javascript/src/runtime.rs`'s
//!      `ArrayRt.elementwise` dispatch, e.g. `case "Gt": return b2f(a > b);`)
//!      converts the JS boolean straight back to a `1.0`/`0.0` float before
//!      it ever reaches `display` — matching `apl_runtime`'s own
//!      `array_runtime::ops`, which represents APL's "no separate boolean
//!      type" convention as plain float data from the start. So `5>3`
//!      prints `"1"` on both sides with no spelling gap to paper over.
//!
//! ## Corpus
//!
//! The first 9 entries are the exact same programs `tests/e2e_node.rs`
//! already proves compile-and-run-in-`node` correctly, reused verbatim here
//! — this file's actual job is cross-checking those same 9 expected values
//! against `apl-runtime`'s independent tree-walking evaluator, which
//! `e2e_node.rs` has no way to do (it only has the compiled side). Two more
//! entries (`dyadic_index_of`, `dyadic_catenate`) complete oracle coverage
//! of all 9 SIR22-"addendum" node kinds: `e2e_node.rs`'s 9 tests exercise
//! only 7 of the 9 (`Reduce`/`Scan`/`OuterProduct`/`Shape`/`Reshape`/
//! `IndexGenerator`/`Ravel`) — `IndexOf` (dyadic `⍳`) and `Catenate`
//! (dyadic `,`) had codegen (`emit.rs`'s `Expr::IndexOf`/`Expr::Catenate`
//! arms) but no test anywhere actually running them through `node`, until
//! now. The remaining entries are base-cut (non-addendum) breadth: APL's
//! signature right-to-left/no-precedence evaluation, a true/false
//! comparison pair, a scalar-vector broadcast, an assignment read back by a
//! later statement, and a printed matrix (2-D display, not just a vector
//! line).
//!
//! Deliberately absent from `CORPUS`: any monadic use of `- × ÷ ⌈ ⌊`. Every
//! one of the 5 (every monadic scalar atom except `+`, a genuine no-op) is
//! broken in the compiled path today, in one of three ways — see "Bugs
//! found" below. `CORPUS` sticks to what round-trips correctly: all 12
//! dyadic scalar atoms (via plain `ElementwiseOp`, unaffected — see
//! `comparison_true`/`false`, `right_to_left_no_operator_precedence`,
//! `scalar_vector_broadcast` above) and monadic `+` (a true pass-through,
//! per this crate's own README table, so trivial to the point of not
//! needing its own corpus entry).
//!
//! ## Three genuine, previously-undiscovered bugs found while building this
//! harness — EXCLUDED from `CORPUS`, not fixed here (out of scope: fixing
//! any of them needs a change to `semantic-ir-to-javascript`, a separate
//! crate)
//!
//! None of the three is exercised by any of this crate's existing tests
//! (`test_lower.rs`, `test_validator.rs`, `e2e_node.rs`) or mentioned in
//! `apl-to-semantic-ir`'s own `README.md`/`CHANGELOG.md` — all three were
//! only ever found by hand-checking the actual generated JavaScript and
//! `node`'s stdout/stderr while scoping this corpus (specifically, while
//! trying to add monadic-atom coverage per this task's own suggestion),
//! i.e. this oracle harness doing exactly the job HML01 §7 describes. All
//! three trace back to the same underlying story: **this frontend's
//! monadic-scalar-atom lowering (`- × ÷ ⌈ ⌊`) was designed and documented
//! (0.1.0's README/CHANGELOG) but never actually exercised end to end
//! through a real backend + `node`** — `e2e_node.rs`'s 9 tests all happen
//! to use dyadic operators or the SIR22-addendum operators exclusively.
//!
//! 1. **Monadic `-` (negate) on a bare SCALAR prints the right NUMBER with
//!    the WRONG GLYPH: ASCII `-5`, not APL's own high-minus `¯5`.** `-5`
//!    gives `apl-runtime`'s correct `¯5`, but the compiled path prints
//!    `-5` (confirmed live: `monadic_negate_scalar` was in an earlier
//!    draft of `CORPUS` as a presumed-working case and failed this exact
//!    way). Root cause, traced through `runtime.rs`: `emit.rs` compiles
//!    monadic `-` to `__Sir.neg(x)`, and `neg` (for an unboxed operand)
//!    returns a *plain native JS number* (`isFloat(x) ? mkFloat(-numOf(x))
//!    : -numOf(x)` — a bare integer stays a bare integer). `print`'s
//!    `formatSeen` dispatch checks `typeof v === "number"` (returning
//!    `String(v)`, i.e. ASCII formatting) *before* it ever reaches the
//!    NDArray branch that calls `ArrayRt.display` (APL's own high-minus
//!    convention — see the "No `normalize()`" section above). So ANY bare
//!    (non-NDArray-boxed) JS number printed via APL's auto-print path gets
//!    the wrong glyph for a negative value — this is not specific to `neg`,
//!    it would affect any expression whose SIR-level value is a raw scalar
//!    rather than a genuine (even rank-0) NDArray, but `neg` is the only
//!    monadic atom that reaches `print` with a value at all (the other
//!    four crash first — see #3 below) so it's the only one this shows up
//!    on today. The value is numerically correct; only the printed
//!    spelling is wrong, which is nonetheless a real bug given this
//!    crate's whole point is reproducing APL's OWN console convention
//!    (`src/value.rs`'s own `negative_numbers_use_high_minus_not_ascii`
//!    test asserts exactly this glyph, just on the `apl-runtime` side).
//! 2. **Monadic `-` (negate) on a genuine ARRAY (rank ≥ 1) silently
//!    computes `NaN` instead of the correctly negated array** — a wrong
//!    VALUE, not just a wrong glyph. `-1 2 ¯3` (negate the 3-element vector
//!    `1 2 ¯3`) gives `apl-runtime`'s correct `¯1 ¯2 3`, but the compiled
//!    path prints `NaN`. Root cause: `neg`'s `numOf` (same file) only
//!    unwraps a boxed `SirFloat` or a *rank-0* (`x.shape.length === 0`)
//!    NDArray — a genuine rank-1 NDArray (exactly what a stranded literal
//!    like `1 2 ¯3` lowers to, per the `Ravel`-wrapping fix in 0.1.3) fails
//!    both checks and passes through `numOf` unchanged, so `neg` computes
//!    native JS unary-minus on a plain object, which coerces to `NaN`.
//!    Same failure *class* as the (now-fixed) MATLAB oracle file's
//!    while-loop/unary-minus-on-power bug — `numOf` not recognizing an
//!    NDArray shape it should have — but a different, still-open instance:
//!    that fix only taught `numOf` to unwrap a *rank-0* NDArray; it was
//!    never taught anything about a genuine rank-≥1 array, and doing so
//!    wouldn't even be the right fix for `neg` specifically — negating a
//!    real array needs to produce a new array (elementwise), not coerce to
//!    a single unwrapped number the way a comparison/subtraction against a
//!    scalar-shaped array correctly does.
//! 3. **Monadic `× ÷ ⌈ ⌊` (sign/reciprocal/ceiling/floor) crash with
//!    `TypeError: unknown builtin: <name>` for EVERY operand, scalar or
//!    array — not a wrong-value/wrong-glyph bug like #1/#2, a hard runtime
//!    crash.** Confirmed live for all four (`×5`, `÷4`, `⌈3.2`, `⌊3.8` each
//!    threw `TypeError: unknown builtin: sign` / `recip` / `ceil` / `floor`
//!    respectively). Root cause: `apl-to-semantic-ir`'s own README/
//!    `src/lower.rs` documents these exact four names as the intended
//!    `BuiltinCall` targets for monadic `× ÷ ⌈ ⌊` — but
//!    `semantic-ir-to-javascript` never actually implements any of them.
//!    `emit.rs`'s `emit_builtin_call` has a fixed 1-arg match (`"not"`,
//!    `"matlab_truthy"`, `"neg"`, `"len"`) and a fixed named-helper table
//!    (`print`/`puts`/`cons`/`car`/`cdr`/`pair?`/`null?`/`number?`/
//!    `symbol?`) — neither lists any of the four, so all four fall through
//!    to the generic `__Sir.callBuiltin(name, args)` path (documented in
//!    that same file as the correct behavior for "a new builtin [that]
//!    needs no backend change to *run*" — except here it was never added
//!    to `runtime.rs`'s `builtins` dispatch table either, so the fallback
//!    itself throws instead of running). `runtime.rs` does have `"floor"`/
//!    `"ceil"` cases, but they live in a completely different mechanism — a
//!    Ruby-style `__method__` dispatch switch keyed on method name
//!    (`7.5.floor`), never reachable from a bare top-level `BuiltinCall`
//!    the way APL's monadic atoms emit one. `"sign"`/`"recip"` do not exist
//!    ANYWHERE in either file. This looks like a pure omission — these
//!    four builtins were designed and documented (this crate's own 0.1.0
//!    CHANGELOG/README) but never actually given backend implementations.
//!
//! All three are reported in this PR's summary as follow-up items (and
//! flagged as a spawned background task) rather than fixed inline, per
//! this task's explicit scope boundary. Net effect: monadic `- × ÷ ⌈ ⌊`
//! (5 of APL's 6 monadic-capable atoms) are ALL broken end to end through
//! the compiled path today, in three different ways — only monadic `+`
//! (the no-op) is unaffected.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use apl_to_semantic_ir::compile_source;
use coding_adventures_apl_runtime::eval as apl_eval;

/// Is a `node` binary on `PATH`? Mirrors `tests/e2e_node.rs`'s own
/// `node_available` (and every sibling oracle file's) exactly: the test
/// below skips (logs, does not fail) when it is not.
fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// One oracle corpus entry. Unlike `matlab-to-semantic-ir`/
/// `octave-to-semantic-ir`'s `Case` (which splits `setup`/`final_expr`),
/// `source` is the WHOLE program, byte-for-byte identical on both the
/// `ground_truth` and `compiled` sides — see this file's module doc,
/// point 1, for why APL needs no such split. `expected` independently
/// pins down what "correct" means, so a coincidental agreement between two
/// buggy implementations can't slip through unnoticed.
struct Case {
    name: &'static str,
    source: &'static str,
    expected: &'static str,
}

const CORPUS: &[Case] = &[
    // --- The 9 SIR22-addendum programs `tests/e2e_node.rs` already proves
    // compile-and-run-correctly through `node` -- reused verbatim; this
    // file's job is cross-checking the SAME 9 expected values against
    // `apl-runtime`'s independent evaluator for the first time. See that
    // file's own doc comments for the full per-program rationale
    // (prefix/order subtleties, the ArrayLit-rank workarounds, etc.) --
    // not re-derived here.
    Case {
        name: "reduce_add",
        source: "+/1 2 3 4\n",
        expected: "10",
    },
    Case {
        name: "reduce_max",
        source: "⌈/3 1 4 1 5\n",
        expected: "5",
    },
    Case {
        name: "scan_then_reduce",
        source: "-/+\\1 2 3\n",
        expected: "¯8",
    },
    Case {
        name: "outer_product_of_two_iota_vectors",
        source: "+/,(⍳2)∘.×(⍳3)\n",
        expected: "18",
    },
    Case {
        name: "outer_product_of_two_bare_stranded_literals",
        source: "+/,1 2∘.×3 4\n",
        expected: "21",
    },
    Case {
        name: "shape_of_a_reshaped_matrix",
        source: "⍴(,2 3)⍴⍳6\n",
        expected: "2 3",
    },
    Case {
        name: "reshape_with_bare_stranded_literal_shape_and_target",
        source: "⍴2 3⍴1 2 3 4 5 6\n",
        expected: "2 3",
    },
    Case {
        name: "index_generator",
        source: "⍳5\n",
        expected: "1 2 3 4 5",
    },
    Case {
        name: "ravel_of_a_reshaped_matrix",
        source: ",(,2 3)⍴⍳6\n",
        expected: "1 2 3 4 5 6",
    },
    // --- Two more addendum node kinds `e2e_node.rs` never exercised:
    // dyadic `⍳` (IndexOf) and dyadic `,` (Catenate). Completes oracle
    // coverage of all 9 SIR22-addendum node kinds.
    //
    // `10 20 30⍳20 99 10`: search each of [20, 99, 10] in [10, 20, 30]
    // (1-based). 20 is at position 2; 99 is not present, so `IndexOf`'s
    // documented "not found" convention (length + 1 = 4) applies; 10 is at
    // position 1. Matches `apl-runtime`'s own `dyadic_iota_is_index_of`
    // test exactly (same source, same expected shape).
    Case {
        name: "dyadic_index_of",
        source: "10 20 30⍳20 99 10\n",
        expected: "2 4 1",
    },
    // `1 2,3 4`: catenate two vectors end to end.
    Case {
        name: "dyadic_catenate",
        source: "1 2,3 4\n",
        expected: "1 2 3 4",
    },
    // --- Base-cut (non-addendum) breadth.
    //
    // APL's signature semantics: no operator precedence, strictly
    // right-to-left. `2×3+4` is `2×(3+4) = 14`, NOT `(2×3)+4 = 10` --
    // matching `apl-runtime`'s own `right_to_left_evaluation_has_no_
    // precedence` test and this crate's module doc.
    Case {
        name: "right_to_left_no_operator_precedence",
        source: "2×3+4\n",
        expected: "14",
    },
    // A true and a false comparison. See this file's module doc
    // "normalize()" section for why both sides agree on the bare numeric
    // spelling (`1`/`0`), not a JS boolean -- no normalization needed.
    Case {
        name: "comparison_true",
        source: "5>3\n",
        expected: "1",
    },
    Case {
        name: "comparison_false",
        source: "3>5\n",
        expected: "0",
    },
    // Scalar-vector broadcast: `2×1 2 3` doubles each element. `1 2 3`
    // lowers to a genuine rank-1 vector (via the `Ravel`-wrapping fix, see
    // `CHANGELOG.md`'s 0.1.3 entry), so this also incidentally confirms
    // that fix from the ground-truth side, not just the IR-shape/`node`
    // side `test_lower.rs`/`e2e_node.rs` already cover.
    Case {
        name: "scalar_vector_broadcast",
        source: "2×1 2 3\n",
        expected: "2 4 6",
    },
    // Assignment is silent; a LATER bare statement reads the bound name
    // back and auto-prints. Exercises `Lowerer`'s ordinary (non-chained)
    // assignment path across two program lines, not just a single
    // expression.
    Case {
        name: "variable_assignment_and_later_reference",
        source: "A←3\nA+4\n",
        expected: "7",
    },
    // A printed MATRIX (2-D display), not just a vector line -- exercises
    // `display`'s row-per-line, right-aligned-per-column convention
    // end to end (both sides), not just the vector/scalar cases every
    // other entry above happens to hit.
    Case {
        name: "printed_matrix_two_by_two",
        source: "2 2⍴1 2 3 4\n",
        expected: "1 2\n3 4",
    },
    // No monadic `- × ÷ ⌈ ⌊` entry: every one of the 5 non-`+` monadic
    // scalar atoms is broken in the compiled path today -- see this file's
    // module doc "Bugs found" section (#1 wrong glyph, #2 wrong value, #3
    // hard crash).
];

/// Ground truth: run `source` through `apl-runtime`'s own `eval`, which
/// already returns exactly the auto-print output of the program's one
/// bare top-level expression (see this file's module doc, point 1) --
/// no echo-parsing needed, unlike the MATLAB/Octave oracle files' `name =
/// value` convention.
fn ground_truth(source: &str) -> String {
    apl_eval(source)
        .unwrap_or_else(|e| panic!("apl-runtime eval failed for {source:?}: {e}"))
        .trim()
        .to_string()
}

/// Compiled path: run `source` (unchanged -- no `disp`-equivalent wrapping
/// needed, see this file's module doc point 1) through
/// `apl_to_semantic_ir::compile_source`, `semantic_ir::validate`,
/// `semantic_ir_to_javascript::compile`, and an actual `node` process.
/// Mirrors `run_via_node` in this crate's own `tests/e2e_node.rs`, down to
/// the `OpenOptions::create_new(true)` temp-file handling (see that file's
/// doc comment for why: `create_new` fails instead of following an
/// existing symlink at the shared, predictable system temp path).
fn compiled(name: &str, source: &str) -> String {
    let module = compile_source(source, "prog")
        .unwrap_or_else(|e| panic!("lowering failed for {name} ({source:?}): {e:?}"));
    let report = semantic_ir::validate(&module);
    assert!(
        report.is_ok(),
        "SIR validation failed for {name}: {:?}",
        report.issues
    );
    let artifact = semantic_ir_to_javascript::compile(&module)
        .unwrap_or_else(|e| panic!("backend emit failed for {name}: {e:?}"));

    let mut path = std::env::temp_dir();
    path.push(format!("apl_sir_oracle_{name}_{}.js", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("create temp js (create_new, not following an existing symlink)");
    file.write_all(artifact.source.as_bytes())
        .expect("write temp js");
    drop(file);

    let output = Command::new("node")
        .arg(&path)
        .output()
        .expect("spawn node");
    let _ = std::fs::remove_file(&path);

    assert!(
        output.status.success(),
        "node failed for {name}: stderr=\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn oracle_corpus_matches_native_apl_runtime() {
    if !node_available() {
        eprintln!("skipping oracle_corpus_matches_native_apl_runtime: `node` not available");
        return;
    }
    for case in CORPUS {
        let gt = ground_truth(case.source);
        assert_eq!(
            gt, case.expected,
            "{}: apl-runtime itself disagrees with this corpus entry's own `expected` -- \
             the program or `expected` is wrong, fix the corpus rather than this assertion",
            case.name
        );

        let got = compiled(case.name, case.source);
        assert_eq!(
            got, case.expected,
            "{}: apl-to-semantic-ir -> semantic-ir-to-javascript -> node disagrees with \
             the apl-runtime ground truth ({gt:?}) -- see this file's module doc for the three \
             known, already-excluded bug classes before assuming this is a new one",
            case.name
        );
    }
}
