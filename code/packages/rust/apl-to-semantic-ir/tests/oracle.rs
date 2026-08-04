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
//! `CORPUS` also covers all 5 non-`+` monadic scalar atoms (`- × ÷ ⌈ ⌊`) —
//! see "Three genuine bugs, now fixed" below for why they were absent when
//! this file was first written, and `monadic_negate_scalar`/
//! `monadic_negate_array`/`monadic_sign_positive`/`monadic_sign_negative`/
//! `monadic_sign_zero`/`monadic_reciprocal`/`monadic_ceiling`/
//! `monadic_floor` (further down `CORPUS`) for the cases themselves.
//! Monadic `+` (conjugate) remains without its own entry: a true
//! pass-through per this crate's own README table, trivial to the point of
//! not needing one.
//!
//! ## Three genuine bugs, now fixed (`semantic-ir-to-javascript` 0.43.0)
//!
//! While this file was first being written, every monadic use of `- × ÷ ⌈
//! ⌊` (every monadic scalar atom except `+`, a genuine no-op) was broken in
//! the compiled path, in one of three ways — discovered by hand-checking
//! the actual generated JavaScript and `node`'s stdout/stderr while scoping
//! this corpus, i.e. this oracle harness doing exactly the job HML01 §7
//! describes. None of the three was exercised by any of this crate's other
//! tests (`test_lower.rs`, `test_validator.rs`, `e2e_node.rs`, whose 9
//! `e2e_node.rs` cases all happen to use dyadic operators or the
//! SIR22-addendum operators exclusively) or mentioned in this crate's own
//! `README.md`/`CHANGELOG.md` prior to their discovery. All three lived in
//! `semantic-ir-to-javascript` (a separate, SHARED backend crate serving
//! many frontends), so were deliberately reported as follow-ups rather than
//! fixed in the PR that added this file — fixed in that crate's 0.43.0
//! (see its `CHANGELOG.md` for the full writeup); `CORPUS` now exercises
//! every case below directly.
//!
//! 1. **FIXED: monadic `-` (negate) on a bare SCALAR printed the right
//!    NUMBER with the WRONG GLYPH: ASCII `-5`, not APL's own high-minus
//!    `¯5`.** Root cause: the glyph decision was implicitly tied to
//!    whether a value happened to already be a genuine `NDArray` by the
//!    time it reached `print` — but a bare `-5` compiles to a bare
//!    `__Sir.neg(5)` call (APL has no dyadic-op wrapping for a monadic atom
//!    applied directly to a literal), so it never was one, and always fell
//!    through `formatSeen`'s ASCII `typeof v === "number"` branch. Fixed by
//!    moving the glyph decision into `formatSeen` itself, gated by a new
//!    per-module flag (`SIR_DISPLAY_APL_HIGH_MINUS`, mirroring the existing
//!    `SIR_DISPLAY_RUBY` boolean-spelling flag) rather than the value's own
//!    shape — necessary because a rank-0 `NDArray` turns out NOT to be
//!    unique to APL (`matlab-to-semantic-ir`'s `2 ^ 2` reaches the
//!    identical representation via `ElementwiseOp::Pow`, yet must keep
//!    printing ASCII `-4`), so only the source language can decide.
//!    `monadic_negate_scalar` below is the regression case.
//! 2. **FIXED: monadic `-` (negate) on a genuine ARRAY (rank ≥ 1) silently
//!    computed `NaN` instead of the correctly negated array.** `-1 2 ¯3`
//!    now gives `apl-runtime`'s correct `¯1 ¯2 3` on both sides. Root
//!    cause: `neg` always unwrapped through `numOf`, which only recognised
//!    a *rank-0* NDArray; a genuine rank-1 NDArray (what a stranded literal
//!    like `1 2 ¯3` lowers to) passed through unchanged, so native JS
//!    unary-minus coerced it to `NaN`. Fixed: `neg` now recognises a
//!    genuine NDArray of rank ≥ 1 and maps `-` elementwise into a NEW
//!    NDArray with the same shape. `monadic_negate_array` below is the
//!    regression case.
//! 3. **FIXED: monadic `× ÷ ⌈ ⌊` (sign/reciprocal/ceiling/floor) crashed
//!    with `TypeError: unknown builtin: <name>` for EVERY operand, scalar
//!    or array.** Root cause: this crate's own `src/lower.rs`/README
//!    documented `"sign"`/`"recip"`/`"ceil"`/`"floor"` as the intended
//!    `BuiltinCall` targets, but `semantic-ir-to-javascript` never
//!    registered any of the four in its `builtins` dispatch table (the
//!    generic `__Sir.callBuiltin` fallback every unrecognised builtin name
//!    routes through), so all four crashed unconditionally — a pure
//!    omission, not a subtler bug. Fixed: all four are now registered,
//!    ported 1:1 from `apl_runtime::eval::apply_monadic_scalar`/`apl_sign`.
//!    `monadic_sign_positive`/`monadic_sign_negative`/`monadic_sign_zero`/
//!    `monadic_reciprocal`/`monadic_reciprocal_zero`/`monadic_ceiling`/
//!    `monadic_floor` below are the regression cases — `monadic_sign_zero`
//!    and `monadic_reciprocal_zero` specifically pin down the `sign(0) ==
//!    0` (not `f64::signum()`'s `1`-for-`+0`) and `recip(0) == Infinity`
//!    (never an error) edge cases the ground-truth reference calls out
//!    explicitly.

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
    // --- Monadic scalar atoms (`- × ÷ ⌈ ⌊`), previously excluded entirely
    // -- see this file's module doc "Three genuine bugs, now fixed" section
    // for the full root-cause writeup of each bug these cases regression-
    // guard. (Monadic `+`, conjugate, is a true pass-through with no
    // interesting case of its own -- see this crate's own README table.)
    //
    // Bug #1 (wrong glyph) and bug #2 (wrong value, NaN) both traced back
    // to the SAME builtin, `neg` -- these two cases are its regression
    // guards. `-5` is a BARE scalar (no dyadic wrapping at all: APL has no
    // literal-only fast path, but a monadic atom applied directly to a
    // literal never goes through `ElementwiseOp` either), so it is the
    // case that used to print ASCII `-5` instead of `¯5`.
    Case {
        name: "monadic_negate_scalar",
        source: "-5\n",
        expected: "¯5",
    },
    // `1 2 ¯3` is a genuine rank-1 vector (stranded literal, wrapped in
    // `Ravel` per the 0.1.3 fix) -- negating it used to silently compute
    // `NaN` instead of the correctly negated `¯1 ¯2 3`.
    Case {
        name: "monadic_negate_array",
        source: "-1 2 ¯3\n",
        expected: "¯1 ¯2 3",
    },
    // Bug #3 (hard `TypeError: unknown builtin` crash) covered `sign`/
    // `recip`/`ceil`/`floor` -- one case per builtin below, plus the two
    // edge cases `apl_runtime::eval::apl_sign`/`apply_monadic_scalar`'s own
    // doc comments call out explicitly: `sign(0) == 0` (NOT
    // `f64::signum()`'s `1`-for-positive-zero convention) and
    // `recip(0) == Infinity` (never an error/`NaN`).
    Case {
        name: "monadic_sign_positive",
        source: "×5\n",
        expected: "1",
    },
    Case {
        name: "monadic_sign_negative",
        source: "×¯5\n",
        expected: "¯1",
    },
    Case {
        name: "monadic_sign_zero",
        source: "×0\n",
        expected: "0",
    },
    Case {
        name: "monadic_reciprocal",
        source: "÷4\n",
        expected: "0.25",
    },
    Case {
        name: "monadic_reciprocal_zero",
        source: "÷0\n",
        expected: "∞",
    },
    Case {
        name: "monadic_ceiling",
        source: "⌈3.2\n",
        expected: "4",
    },
    Case {
        name: "monadic_floor",
        source: "⌊3.8\n",
        expected: "3",
    },
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
