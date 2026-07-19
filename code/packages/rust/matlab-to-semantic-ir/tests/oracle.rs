//! Oracle/golden tests (HML01 §7): the SAME MATLAB computation, run through
//! **two independent implementations**, and diffed:
//!
//!   (a) `matlab-runtime` — this frontend's own sibling crate, a
//!       tree-walking interpreter over `array-runtime` — the ground truth.
//!   (b) `matlab_to_semantic_ir::compile_source` → `semantic_ir::Module` →
//!       `semantic_ir_to_javascript::compile` → **an actual `node`
//!       process**.
//!
//! `wolfram-to-semantic-ir`/`macsyma-to-semantic-ir`'s own `tests/
//! e2e_node.rs` already prove (b) alone runs without crashing; this file is
//! the first test anywhere in the HML01 track that also runs (a) on the
//! same program and *diffs the two results* — the actual definition of
//! "oracle testing" per HML01 §7, previously unimplemented for every math
//! language (see that spec's §5 rollout note this PR updates).
//!
//! ## Why "the same source" is a `setup` + `final_expr` pair, not one string
//!
//! Naively, "run the same source through both paths" suggests one literal
//! string. Two independent discoveries made while building this harness
//! rule that out for MATLAB specifically — both confirmed empirically
//! (adding a throwaway probe test against `matlab-runtime` and reading the
//! generated JavaScript directly), not assumed:
//!
//! 1. **`matlab-runtime`'s `disp` builtin is a no-op.** Read
//!    `matlab-runtime/src/builtins.rs`'s `"disp" => { ... }` arm: it
//!    validates that an argument was passed and then returns an
//!    *unconditionally empty, invisible array*, discarding the argument
//!    entirely. `eval("disp(7)\n")` returns `"ans =\n\n\n\n"`, never `"7"`.
//!    Confirmed live: neither `matlab-runtime`'s own `#[cfg(test)]` module
//!    nor `matlab-repl`'s test suite ever exercises `disp` — every one of
//!    their assertions instead relies on MATLAB's *other* legitimate
//!    display convention: an unsuppressed (no trailing `;`) statement
//!    echoes `name = value` (or `ans = value` for a bare expression). That
//!    convention is real, idiomatic MATLAB (`>> 2 + 2` at a prompt shows
//!    `ans = 4`), and it is the one `matlab-runtime` actually implements
//!    correctly — so it is what this file uses to read the ground-truth
//!    value out of `matlab-runtime`. This is a genuine, confirmed bug in
//!    `matlab-runtime` (not something this PR touches or fixes — that
//!    crate's source is out of scope here); see the CHANGELOG entry this
//!    PR adds for the write-up.
//! 2. **`semantic_ir` has no representation of "implicit display" at
//!    all.** It is a narrow-waist IR shared by general-purpose languages
//!    that have no such convention; `matlab-to-semantic-ir`'s own
//!    `lower_statement_line` does not even look at the trailing `;` token.
//!    The *only* way to get an observable value out of the compiled JS ->
//!    `node` path is an explicit `disp(...)` call (which this frontend maps
//!    onto the shared SIR `print` builtin, and which the JS backend's
//!    `print` really does implement).
//!
//! So: `matlab-runtime`'s own explicit-display primitive doesn't work, and
//! the compiled path has no *implicit*-display primitive at all. Each
//! [`Case`] therefore stores one `setup` string (ordinary `;`-terminated
//! MATLAB statements, **byte-for-byte identical** on both sides — this is
//! the actual computation under test) plus one `final_expr` string (just an
//! expression, no statement terminator); [`ground_truth`] appends it bare
//! and unsuppressed (`matlab-runtime`'s own working convention) while
//! [`compiled`] wraps it in `disp(...)` (the compiled path's own working
//! convention). Only the "how do I observe this value" trailer differs,
//! using each side's own *actually-functioning* mechanism — the shared
//! computation is identical.
//!
//! ## Normalization
//!
//! [`normalize`] maps `"#t"`/`"true"` to `"1"` and `"#f"`/`"false"` to
//! `"0"`, and nothing else. This is needed for the `comparison` case:
//! MATLAB comparisons are `array-runtime` numeric logicals (`0.0`/`1.0`,
//! see `matlab-runtime/src/value.rs`'s doc comment — MATLAB has no separate
//! boolean type), so `matlab-runtime` prints `1`; but SIR comparison
//! builtins compile to native JS `<`/`>`/`===` (see `emit.rs`), which
//! produce real JS booleans, and the shared JS runtime's `format()` prints
//! those Scheme-style (`#t`/`#f`) by default. Same truth value, different
//! spelling by design convention (a general-purpose IR's boolean vs. a
//! MATLAB-specific "logicals are doubles" convention the frontend does not
//! yet re-encode) — exactly the class of difference the parent task
//! describes as safe to normalize, same as a trailing `.0000`. No other
//! substitution happens: a genuine value mismatch is never routed through
//! this function to make it pass.
//!
//! ## Corpus scope, and bugs found while scoping it (see CHANGELOG for the
//! full write-up of each)
//!
//! The corpus below is restricted to programs this frontend/backend pair
//! can *actually* execute correctly today. Getting there surfaced four
//! confirmed, previously-unknown bugs/gaps — each was a **discovery of
//! this oracle harness doing its job**, not a defect in the harness. Three
//! of the four have since been FIXED (a follow-up PR to the one that added
//! this file); one remains open:
//!
//! - **Integer-literal division floors instead of true-dividing** (still
//!   OPEN). MATLAB has no integer type — every number is a double, so
//!   `7 / 2` is always `3.5`. But `number_literal_expr` (`src/lower.rs`)
//!   lowers a decimal-point-free literal to `Expr::IntLit`, and the JS
//!   backend's shared `divide()` runtime helper (built for *Ruby's*
//!   `Integer#/`, which really does floor) floors whenever both operands
//!   are integer-valued — so the compiled path prints `3`. Excluded from
//!   `CORPUS`; not independently re-demonstrated as its own test here
//!   since it needs no `node` process to see (`grep -n "Math.floor" src/
//!   runtime.rs` in `semantic-ir-to-javascript` plus this file's own
//!   probe history is confirmation enough), but recorded in the CHANGELOG.
//!   Left open because a real fix needs a source-language-aware division
//!   convention (MATLAB has no integer type at all, unlike Ruby, whose
//!   `Integer#/`-flooring convention `divide()` was actually built for) —
//!   a bigger design decision than the fixes below, not a one-line change.
//! - **FIXED: unary minus on a power expression gave `NaN`, not the
//!   correct value.** `^`/`.^` *unconditionally* lower to the SIR22
//!   `ElementwiseOp::Pow` node (`try_power`, `src/lower.rs`) — unlike
//!   `+`/`-`/`*`, there is no literal-only scalar fast path for power at
//!   all. So even `2 ^ 2` (both literals) evaluates to an NDArray-shaped
//!   `{shape, data}` object, not a plain JS number, and unary minus's
//!   `neg` builtin (`emit.rs`) calls the runtime's `numOf` to unwrap its
//!   operand before negating — the exact same `numOf` gap the while-loop
//!   bug below turned out to hinge on. Fixing `numOf` (see that bug's
//!   entry) fixed this one too, for free, in the same commit: real MATLAB
//!   `-2 ^ 2 == -4` (unary binds looser than `^`, confirmed by
//!   `matlab-runtime`'s own `scalar_arithmetic_echoes_ans` test) now holds
//!   through the compiled path too.
//! - **FIXED: a `while` loop whose condition variable is also a
//!   non-literal arithmetic accumulator ran its body exactly once, not to
//!   convergence.** This was the most severe finding — a silent wrong
//!   *computation*, not just a wrong *display* — and still gets its own
//!   dedicated, always-informative regression test:
//!   [`while_loop_accumulator_converges_correctly`] below (renamed from
//!   `known_bug_while_loop_accumulator_terminates_after_one_iteration`),
//!   with the full root-cause writeup and the fix in its doc comment.
//!   Root cause, in one sentence: `semantic-ir-to-javascript`'s shared
//!   `numOf` helper (`runtime.rs`) unwrapped a tagged `SirFloat` box but
//!   not a rank-0 (scalar) NDArray, so a comparison against an
//!   NDArray-wrapped accumulator silently evaluated to `NaN < 10 ==
//!   false`; `numOf` now unwraps both.
//! - **FIXED: `matlab-to-semantic-ir` never declared `Feature::
//!   ShortCircuit`** for `&&`/`||`/`&`/`|` (`try_logical`, `src/lower.rs`
//!   had no `self.observed.add(Feature::ShortCircuit)` anywhere), so any
//!   MATLAB program using them failed `semantic_ir::validate()` outright
//!   with `"manifest does not declare feature short-circuit but module
//!   uses it"`. Confirmed via probe (`x > 3 && y > 5`); a one-line fix
//!   (`tests/test_validator.rs`'s
//!   `a_logical_and_program_validates_and_declares_short_circuit` is the
//!   regression test) — but such programs are still not in `CORPUS`
//!   below, since none of this crate's `&&`/`||` support has an oracle
//!   case here yet, just validator coverage.
//!
//! Two more constructs turned out to be impossible to oracle-test at all,
//! not just currently-buggy, because `matlab-runtime` itself cannot run
//! them:
//!
//! - **User-defined functions (`function ... end`).** `matlab-runtime`'s
//!   `eval.rs` statement dispatch has no `func_def` arm whatsoever (not
//!   even a partial one) — `eval("function r = seven()\n r = 7;\nend\n")`
//!   fails with `"unsupported statement 'func_def'"`. Meanwhile this
//!   crate's own `e2e_node.rs` already compiles and runs MATLAB function
//!   definitions successfully through the JS path. There is no ground
//!   truth to diff against until `matlab-runtime` grows function support.
//! - **Indexed assignment (`A(2) = 9;`).** `matlab-runtime`'s
//!   `eval_expr_or_assign` requires the assignment target to be a bare
//!   variable name (`lhs_name`) and errors with `"assignment target must
//!   be a variable"` for any indexed LHS — even though this is bog-standard
//!   MATLAB and this crate's own `e2e_node.rs` already round-trips it
//!   through the JS path (`indexed_assignment_mutates_in_place_and_reads_
//!   back_in_node`).
//!
//! Given all of the above, `CORPUS` sticks to: literal arithmetic
//! (`+ - *`, whose scalar fast path is unaffected by the bugs above),
//! comparisons feeding an `if`/`elseif`/`else` (never `disp`ed as a bare
//! `&&`/`||` combination), a `for` loop accumulator (immune to the
//! while-loop bug — see that test's doc comment for exactly why), and two
//! genuine array/matrix (SIR22) cases already proven executable by
//! `e2e_node.rs` (matrix multiplication, elementwise scalar broadcast),
//! now cross-checked against `matlab-runtime` for the first time.
//!
//! - **FIXED: bare-numeric-value truthiness disagreed with MATLAB's
//!   "logicals are doubles" convention.** MATLAB/Octave truthiness is
//!   "nonzero is true, zero is false" for ANY number, not just a comparison
//!   result — `~0` must be `1`, `~5` must be `0`. This frontend's
//!   `lower_unary` (`~`), `lower_if`, `lower_while`, and `try_logical`
//!   (`&&`/`||`) used to pass a bare numeric operand straight through to
//!   the shared JS backend's `truthy()` runtime helper, which implements
//!   SIR's OWN canonical truthiness instead (only `false`/`nil` are falsy —
//!   the Ruby/Lisp convention `ruby-to-semantic-ir` genuinely depends on).
//!   So `~0` compiled to `false` — backwards. `to_matlab_condition`
//!   (`src/lower.rs`) now wraps a bare-numeric boolean-context operand in
//!   an explicit `!= 0` SIR comparison at lowering time, leaving an
//!   already-boolean operand (a comparison, `~`, or `&&`/`||` result)
//!   unchanged. See `negation_of_bare_zero_is_true`,
//!   `negation_of_bare_nonzero_is_false`, `if_bare_zero_condition_is_false`,
//!   `if_bare_nonzero_condition_is_true`,
//!   `logical_and_short_circuits_false_on_bare_zero_operand`, and
//!   `logical_or_true_via_bare_nonzero_second_operand` below. (This gap was
//!   first documented — deliberately sidestepped, not fixed — by
//!   `octave-to-semantic-ir/tests/oracle.rs`'s `bang_negation_on_comparison`
//!   case; see that file's updated doc comment.)

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use coding_adventures_matlab_runtime::eval as matlab_eval;
use matlab_to_semantic_ir::compile_source;

/// Is a `node` binary on `PATH`? Every test below skips (logs, does not
/// fail) when it is not, mirroring `sir-conformance`'s per-toolchain
/// availability convention and this crate's own `e2e_node.rs`.
fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// One oracle corpus entry. `setup` is a run of ordinary `;`-terminated
/// MATLAB statements — the actual computation, shared byte-for-byte
/// between [`ground_truth`] and [`compiled`]. `final_expr` is a bare
/// expression (no terminator) naming the value being cross-checked.
/// `expected` is that value in already-[`normalize`]d form, independently
/// pinning down what "correct" means so a coincidental agreement between
/// two buggy implementations can't slip through unnoticed.
struct Case {
    name: &'static str,
    setup: &'static str,
    final_expr: &'static str,
    expected: &'static str,
}

const CORPUS: &[Case] = &[
    // Plain literal arithmetic, exercising `+`/`-`/`*` precedence
    // (multiplication binds tighter): 3 + 4*2 - 5 = 3 + 8 - 5 = 6.
    Case {
        name: "literal_arithmetic_precedence",
        setup: "x = 3 + 4 * 2 - 5;\n",
        final_expr: "x",
        expected: "6",
    },
    // Regression case for the (now-fixed) unary-minus-on-power bug
    // documented in this file's module doc: `^` always lowers to the
    // SIR22 `ElementwiseOp::Pow` node, even for two literals, so `2 ^ 2`
    // evaluated to an NDArray-shaped scalar and `neg` on it used to give
    // `NaN` instead of `-4` (unary minus binds looser than `^`, so this is
    // `-(2 ^ 2)`, not `(-2) ^ 2`). Fixed by the same `semantic-ir-to-
    // javascript` `numOf` change that fixed the while-loop bug below.
    Case {
        name: "unary_minus_on_power",
        setup: "",
        final_expr: "-2 ^ 2",
        expected: "-4",
    },
    // A bare comparison. See the module doc's "Normalization" section for
    // why `expected` is written in MATLAB's own 0/1 numeric-logical
    // spelling rather than a JS boolean literal.
    Case {
        name: "comparison",
        setup: "",
        final_expr: "5 > 3",
        expected: "1",
    },
    // if/else. `y` is deliberately pre-declared (`y = 0;`) before the
    // conditional: a first assignment made only inside one branch of an
    // `if`/`else` and read back afterward currently fails
    // `semantic_ir::validate()` with `var-ref ... references unknown name`
    // (confirmed via probe) -- a real scope-tracking gap in this crate's
    // own `lower_if`, out of scope to fix in this test-only PR, noted here
    // and in the CHANGELOG. Pre-declaring sidesteps it without changing
    // what the program computes.
    Case {
        name: "if_else",
        setup: "y = 0;\nx = 5;\nif x > 3\n  y = 1;\nelse\n  y = 2;\nend\n",
        final_expr: "y",
        expected: "1",
    },
    // elseif chain (same pre-declaration reasoning as `if_else` above).
    Case {
        name: "elseif_chain",
        setup: "r = 0;\nx = 5;\nif x > 10\n  r = 1;\nelseif x > 3\n  r = 2;\nelse\n  r = 3;\nend\n",
        final_expr: "r",
        expected: "2",
    },
    // for-loop accumulator: sum 1..5 = 15. `s` becomes an NDArray-shaped
    // scalar after its first `s = s + i` (a variable-involving, hence
    // array-domain, update -- see the module doc's bug list), so it is
    // read back via linear indexing (`s(1)`, valid MATLAB: a scalar is a
    // 1x1 array) rather than `disp`ed directly, exactly as this crate's
    // own `e2e_node.rs` already does for computed-array results (that
    // file's doc comment: "disp on a computed matrix has no
    // display-formatting story yet"). This loop is immune to the
    // while-loop bug documented below because its OWN termination test is
    // driven by the loop index `i` (always a plain number), never by the
    // accumulator `s`.
    Case {
        name: "for_loop_accumulator",
        setup: "s = 0;\nfor i = 1:5\n  s = s + i;\nend\n",
        final_expr: "s(1)",
        expected: "15",
    },
    // Real SIR22 array/matrix cases -- the actual point of Stream A.
    // [1 2; 3 4] * [1 2; 3 4] = [7 10; 15 22]; (1,1) (1-based) is 7. Same
    // source `e2e_node.rs`'s `matrix_multiplication_runs_in_node` already
    // proves executes on the JS side; here it is additionally checked
    // against `matlab-runtime`, which supports matrix literals, matmul,
    // and read-indexing natively.
    Case {
        name: "matrix_multiplication",
        setup: "A = [1 2; 3 4];\nB = A * A;\n",
        final_expr: "B(1, 1)",
        expected: "7",
    },
    // A .* 2: elementwise scale. A(2,2) is 4 before scaling, 8 after.
    Case {
        name: "elementwise_scalar_broadcast",
        setup: "A = [1 2; 3 4];\nB = A .* 2;\n",
        final_expr: "B(2, 2)",
        expected: "8",
    },
    // Regression corpus for the (now-fixed) bare-numeric-truthiness bug:
    // MATLAB has no separate boolean type -- "logicals are doubles", and
    // truthiness is simply "nonzero is true, zero is false" for ANY number,
    // not just a comparison result. `~0`/`~5` here negate a BARE numeric
    // literal directly (no comparison in sight), unlike the `comparison`
    // case above which negates a comparison RESULT (already a genuine JS
    // boolean either way a truthy-check reads it). Before the fix, this
    // frontend's `lower_unary`/`lower_if`/`lower_while`/`try_logical` passed
    // a bare numeric operand straight through to `BuiltinCall("not", ..)` /
    // `Expr::If` / `Stmt::While` / `Expr::LogicalAnd`/`LogicalOr` with no
    // MATLAB-truthiness recoding; the shared JS backend's `truthy()` then
    // read it under SIR's OWN canonical convention (only `false`/`nil` are
    // falsy -- see `semantic-ir-to-javascript/src/runtime.rs`), so `~0`
    // compiled to `false` (wrong; MATLAB's `~0` is `1`) while `matlab-
    // runtime`'s own ground-truth interpreter (`src/eval.rs`: `Some("~") =>
    // unary_map(&v, |x| if x == 0.0 { 1.0 } else { 0.0 })`) already got it
    // right. The fix lives in THIS frontend's lowering
    // (`to_matlab_condition`, `src/lower.rs`), not the shared runtime: a
    // bare-numeric operand reaching a boolean context is now wrapped in an
    // explicit `!= 0` SIR comparison at lowering time, so by the time any
    // backend's truthy-check sees it, it already IS a genuine boolean --
    // correct under either convention.
    Case {
        name: "negation_of_bare_zero_is_true",
        setup: "",
        final_expr: "~0",
        expected: "1",
    },
    Case {
        name: "negation_of_bare_nonzero_is_false",
        setup: "",
        final_expr: "~5",
        expected: "0",
    },
    // `if` on a BARE numeric condition (no comparison at all). Same
    // pre-declaration workaround as `if_else` above. Zero must NOT enter
    // the `if` branch; any nonzero number must.
    Case {
        name: "if_bare_zero_condition_is_false",
        setup: "y = 0;\nif 0\n  y = 1;\nelse\n  y = 2;\nend\n",
        final_expr: "y",
        expected: "2",
    },
    Case {
        name: "if_bare_nonzero_condition_is_true",
        setup: "y = 0;\nif 5\n  y = 1;\nelse\n  y = 2;\nend\n",
        final_expr: "y",
        expected: "1",
    },
    // Regression case for a second bug the first fix attempt introduced
    // (`/security-review` caught it before push): a bare `VarRef` holding a
    // STORED comparison result is never recognisably boolean by static
    // shape analysis alone, so a lowering-time-only fix that decides
    // "already boolean, skip the wrap" vs. "bare number, wrap in `!= 0`" by
    // inspecting the operand's immediate shape gets this case wrong --
    // `tf` gets wrapped in `tf != 0` regardless of its actual value, and
    // the JS runtime's strict-identity `!=` (`numOf(a) !== numOf(b)`) makes
    // `false != 0` unconditionally `true` (a `boolean` and a `number` are
    // never `===`), silently taking the `if` branch no matter what `tf`
    // holds. The fix (`to_matlab_condition` wrapping every operand in the
    // `matlab_truthy` runtime intrinsic, unconditionally, deciding
    // boolean-vs-number AT RUNTIME instead) makes this correct regardless
    // of the operand's static shape. `y` must become `2` (`tf` is `false`).
    Case {
        name: "if_condition_on_a_variable_holding_a_stored_false_comparison",
        setup: "y = 0;\ntf = (5 < 3);\nif tf\n  y = 1;\nelse\n  y = 2;\nend\n",
        final_expr: "y",
        expected: "2",
    },
    Case {
        name: "if_condition_on_a_variable_holding_a_stored_true_comparison",
        setup: "y = 0;\ntf = (5 > 3);\nif tf\n  y = 1;\nelse\n  y = 2;\nend\n",
        final_expr: "y",
        expected: "1",
    },
    // `&&`/`||` given a BARE-ZERO operand, observed through an `if` branch
    // decision rather than a `disp`ed raw value -- `disp`ing a `LogicalAnd`/
    // `LogicalOr` result directly is deliberately NOT done anywhere in this
    // corpus (see the module doc's corpus-scope note): this frontend's
    // short-circuit nodes return the Ruby-style "deciding operand" verbatim
    // (`emit.rs`), while `matlab-runtime` returns a coerced 0.0/1.0 double
    // (`eval.rs`'s `"&" | "&&"` arm) -- a representational difference
    // unrelated to this bug, already excluded from the corpus. Routing
    // through `if` sidesteps that mismatch entirely: only the BRANCH TAKEN
    // is observed, which is representation-independent. `0 && 1` must be
    // false (zero operand short-circuits); `0 || 5` must be true (the
    // nonzero second operand decides).
    Case {
        name: "logical_and_short_circuits_false_on_bare_zero_operand",
        setup: "y = 0;\nif 0 && 1\n  y = 1;\nelse\n  y = 2;\nend\n",
        final_expr: "y",
        expected: "2",
    },
    Case {
        name: "logical_or_true_via_bare_nonzero_second_operand",
        setup: "y = 0;\nif 0 || 5\n  y = 1;\nelse\n  y = 2;\nend\n",
        final_expr: "y",
        expected: "1",
    },
];

/// Ground truth: run `setup` followed by a *bare, unsuppressed*
/// `final_expr` through `matlab-runtime`, and pull the value out of its
/// `name = value` (or `ans = value`) echo -- mirroring the `scalar_in`
/// helper `matlab-runtime`'s own `#[cfg(test)]` module and `matlab-repl`'s
/// test suite both use throughout (see the module doc's point 1).
fn ground_truth(setup: &str, final_expr: &str) -> String {
    let src = format!("{setup}{final_expr}\n");
    let echo =
        matlab_eval(&src).unwrap_or_else(|e| panic!("matlab-runtime eval failed for {src:?}: {e}"));
    echo.rsplit('=').next().unwrap_or(&echo).trim().to_string()
}

/// Compiled path: run `setup` followed by `disp(final_expr);` -- SIR's only
/// observable-output primitive (see the module doc's point 2) -- through
/// `matlab_to_semantic_ir::compile_source`, `semantic_ir::validate`,
/// `semantic_ir_to_javascript::compile`, and an actual `node` process.
/// Mirrors `run_via_node` in this crate's own `tests/e2e_node.rs`, down to
/// the `OpenOptions::create_new(true)` temp-file handling (see that file's
/// doc comment for why: `create_new` fails instead of following an
/// existing symlink at the shared, predictable system temp path).
fn compiled(name: &str, setup: &str, final_expr: &str) -> String {
    let src = format!("{setup}disp({final_expr});\n");
    let module = compile_source(&src, "prog")
        .unwrap_or_else(|e| panic!("lowering failed for {name} ({src:?}): {e:?}"));
    let report = semantic_ir::validate(&module);
    assert!(
        report.is_ok(),
        "SIR validation failed for {name}: {:?}",
        report.issues
    );
    let artifact = semantic_ir_to_javascript::compile(&module)
        .unwrap_or_else(|e| panic!("backend emit failed for {name}: {e:?}"));

    let mut path = std::env::temp_dir();
    path.push(format!("matlab_sir_oracle_{name}_{}.js", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("create temp js (create_new, not following an existing symlink)");
    file.write_all(artifact.source.as_bytes())
        .expect("write temp js");
    drop(file);

    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);

    assert!(
        output.status.success(),
        "node failed for {name}: stderr=\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Normalize display-*spelling*-only differences -- see the module doc's
/// "Normalization" section. Never used to paper over a genuine value
/// mismatch: anything that isn't exactly `#t`/`true`/`#f`/`false` passes
/// through unchanged, so a real divergence still fails the comparison.
fn normalize(s: &str) -> String {
    match s {
        "#t" | "true" => "1".to_string(),
        "#f" | "false" => "0".to_string(),
        other => other.to_string(),
    }
}

#[test]
fn oracle_corpus_matches_native_matlab_runtime() {
    if !node_available() {
        eprintln!("skipping oracle_corpus_matches_native_matlab_runtime: `node` not available");
        return;
    }
    for case in CORPUS {
        let gt = normalize(&ground_truth(case.setup, case.final_expr));
        assert_eq!(
            gt, case.expected,
            "{}: matlab-runtime itself disagrees with this corpus entry's own `expected` -- \
             the program or `expected` is wrong, fix the corpus rather than this assertion",
            case.name
        );

        let got = normalize(&compiled(case.name, case.setup, case.final_expr));
        assert_eq!(
            got, case.expected,
            "{}: matlab-to-semantic-ir -> semantic-ir-to-javascript -> node disagrees with \
             the matlab-runtime ground truth ({gt:?}) -- see this file's module doc for known, \
             already-excluded bug classes before assuming this is a new one",
            case.name
        );
    }
}

/// FIXED (previously `known_bug_while_loop_accumulator_terminates_after_
/// one_iteration`): a MATLAB `while` loop whose condition variable is
/// *also* the target of a non-literal (variable-involving) arithmetic
/// update used to run its body exactly **once** through the compiled JS
/// path, instead of iterating to convergence.
///
/// Root cause, traced via the actual generated JavaScript
/// (`semantic_ir_to_javascript::compile` on `"n = 0;\nwhile n < 10\n  n =
/// n + 1;\nend\n"` emits, in `function main()`:
/// ```js
/// let n = 0;
/// while (__Sir.truthy(__Sir.lt(n, 10))) {
///   n = __Sir.Array.elementwise("Add", n, 1);
/// }
/// ```
/// ):
///
/// 1. `expr_is_known_scalar` (this crate's scalar/array disambiguation
///    heuristic, documented in `src/lower.rs`'s module doc and this
///    crate's own README) only ever treats a *literal*-derived expression
///    as provably scalar. `n + 1`, where `n` is a variable, is therefore
///    always lowered as an SIR22 `Expr::ElementwiseOp`, regardless of what
///    `n` actually holds at runtime. This part of the diagnosis was
///    correct and remains true after the fix below -- it is exactly what
///    still makes this a genuinely useful regression test, not merely a
///    coincidentally-passing one.
/// 2. `semantic-ir-to-javascript`'s `ElementwiseOp` codegen always returns
///    an NDArray-shaped `{ shape: number[], data: Float64Array }` object
///    (`runtime.rs`'s `ArrayRt.elementwise`), even for a logically-scalar
///    result. So after one iteration, `n` holds an *object*, not a JS
///    number. Also still true after the fix -- and fine, because...
/// 3. ...comparison builtins compile to thin runtime helpers that already
///    existed for a *different* reason (unwrapping a tagged `SirFloat` box
///    so `7.0 < 8` doesn't hit `NaN` via `ToPrimitive` on the box --
///    `emit.rs`: `"<" => Some("__Sir.lt")`), which in turn unwrap through
///    `runtime.rs`'s shared `numOf`. The ORIGINAL diagnosis (recorded in
///    this crate's CHANGELOG for the PR that found this bug) described
///    comparisons as bare native infix operators with "no array-aware
///    comparison helper" -- that was already stale by the time this bug
///    was investigated for a fix: the helper existed, it just didn't know
///    about NDArrays yet. `numOf` unwrapped a `SirFloat` box but returned
///    every other value (including a scalar NDArray) unchanged, so `n <
///    10` still coerced the NDArray object through `ToPrimitive` to `NaN`,
///    and `NaN < 10` is silently `false`.
///
/// **The fix** (`semantic-ir-to-javascript/src/runtime.rs`): `numOf` now
/// also unwraps a rank-0 (scalar) NDArray -- `x.shape.length === 0` --
/// to its sole `data[0]` element, alongside its existing `SirFloat` case.
/// `numOf` is the identity on anything it doesn't recognise and only
/// MATLAB/APL-style SIR22 frontends ever construct an NDArray, so this is
/// a no-op for every other language this backend serves and a real fix
/// for comparisons, negation, subtraction, and modulo against a
/// scalar-shaped array result -- see
/// `semantic-ir-to-javascript/tests/run_with_node.rs`'s
/// `numof_unwraps_scalar_ndarray_for_comparison_and_negation` for the
/// runtime-level regression test, which also confirms this incidentally
/// fixes the unary-minus-on-power bug documented in this file's module
/// doc (same root cause: `neg` calls `numOf` too).
///
/// Previously the loop body ran exactly once and then silently stopped --
/// no error, no validator issue, no crash, and the resulting `n = 1`
/// looked like a plausible (if wrong) answer in isolation: a silent wrong
/// *computation*, not merely a wrong *display* (contrast the
/// division/power-operator bugs documented in this file's module doc,
/// which only corrupt what gets printed). `for`-loop accumulators (see
/// `for_loop_accumulator` in `CORPUS` above) were never affected by the
/// underlying NDArray-wrapping in the first place -- a `for` loop's OWN
/// termination test is driven by the loop index (always a plain number,
/// incremented via native `+`, never `ElementwiseOp`), not by the
/// accumulator variable -- but any construct whose OWN loop/branch
/// condition reads a variable previously updated via non-literal
/// arithmetic would have hit this same failure mode; the fix is general
/// (in `numOf`, not in the while-loop's own codegen), so all such
/// constructs are covered, not just this one shape.
#[test]
fn while_loop_accumulator_converges_correctly() {
    if !node_available() {
        eprintln!("skipping while_loop_accumulator_converges_correctly: `node` not available");
        return;
    }
    let setup = "n = 0;\nwhile n < 10\n  n = n + 1;\nend\n";

    let gt = ground_truth(setup, "n");
    assert_eq!(
        gt, "10",
        "matlab-runtime ground truth for this loop changed -- update this test's premise"
    );

    let got = compiled("while_loop_accumulator_converges", setup, "n(1)");
    assert_eq!(
        got, "10",
        "the while-loop/NDArray-accumulator bug appears to have regressed -- \
         see this test's doc comment (and semantic-ir-to-javascript's `numOf`, \
         `runtime.rs`) for the fix this used to be a `known_bug_*` guard against"
    );
}
