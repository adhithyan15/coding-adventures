//! Oracle/golden tests (HML01 §7): the SAME Octave computation, run through
//! **two independent implementations**, and diffed:
//!
//!   (a) `octave-runtime` — this frontend's own sibling crate. It is itself
//!       a thin wrapper (`octavify` then delegate) over `matlab-runtime`, a
//!       tree-walking interpreter over `array-runtime` — the ground truth.
//!   (b) `octave_to_semantic_ir::compile_source` → `semantic_ir::Module` →
//!       `semantic_ir_to_javascript::compile` → **an actual `node`
//!       process**.
//!
//! This is the direct sibling of
//! [`matlab-to-semantic-ir`'s own `tests/oracle.rs`](../../matlab-to-semantic-ir/tests/oracle.rs)
//! — same shape, same `setup`/`final_expr`/`expected` [`Case`] structure,
//! same `ground_truth`/`compiled`/`normalize` trio, same
//! `OpenOptions::create_new(true)` temp-file handling — mirroring how
//! `octave-to-semantic-ir` itself (`src/lib.rs`) reuses
//! `matlab-to-semantic-ir` wholesale rather than reimplementing a second
//! frontend. **This file does not re-derive reasoning already established
//! there**: the "why `setup` + `final_expr`, not one string" rationale (the
//! `disp` no-op + no-implicit-display gap), the `#t`/`#f` normalization
//! rationale, the pre-declare-before-`if`/`else` scope-tracking workaround,
//! and the for-loop-immune-to-the-while-loop-bug reasoning are all
//! identical here and documented in full in that file's module doc comment
//! — read it first.
//!
//! ## What's actually new here: exercising Octave-only syntax
//!
//! `matlab-to-semantic-ir`'s own oracle corpus already covers plain MATLAB
//! arithmetic, comparisons, branching, loops, and array/matrix ops. Since
//! `octave-to-semantic-ir` is *only* `octave-runtime`'s `octavify` shim
//! (`coding_adventures_octave_runtime::octavify`) plus a straight delegation
//! to `matlab_to_semantic_ir::compile_source`, re-testing that same plain
//! MATLAB subset here would prove nothing this crate's `src/lib.rs`
//! `#[cfg(test)]` module (the shim-then-delegate wiring tests) doesn't
//! already prove structurally. So `CORPUS` below is deliberately restricted
//! to programs that exercise the ONE thing genuinely specific to Octave:
//! the surface forms `octavify` actually rewrites (see that function's own
//! doc comment / table in `octave-runtime/src/lib.rs`):
//!
//! | Octave form (this file's CORPUS)      | normalizes to (MATLAB) | exercised by |
//! |----------------------------------------|-------------------------|--------------|
//! | `# comment`                             | `% comment`             | `hash_comment_literal_arithmetic` |
//! | `!=`                                    | `~=`                    | `bang_equals_not_equal_comparison` |
//! | `!` (logical not)                       | `~`                     | `bang_negation_on_comparison` |
//! | `endif`                                 | `end`                   | `if_else_endif` |
//! | `endfor`                                | `end`                   | `for_loop_accumulator_endfor` |
//! | `endwhile`                              | `end`                   | `while_loop_accumulator_endwhile` |
//!
//! `endfunction`/`endswitch`/`end_try_catch`/`endparfor` are the remaining
//! three/four `endX` forms `octavify` normalizes but are **not** exercised
//! here: they gate on constructs (`function`, `switch`, `try`/`catch`,
//! `parfor`) that `matlab-to-semantic-ir` v0.1.0 either has no ground truth
//! for (`matlab-runtime` has no `func_def` evaluator arm — see the MATLAB
//! oracle file's own module doc) or does not lower at all (`switch` is an
//! explicit out-of-scope error, confirmed by this crate's own
//! `a_construct_outside_matlab_to_semantic_irs_scope_errors_cleanly` test;
//! `parfor` and `try`/`catch` are not in `matlab-to-semantic-ir`'s supported
//! construct list either). Octave's `++`/`--` and `do…until` are
//! *documented deferrals* `octavify` does not normalize at all (left
//! untouched, reported as an ordinary MATLAB parse error) — see this
//! crate's own `octave_only_do_until_is_not_normalized_and_errors` test —
//! so there is nothing to oracle-test there either.
//!
//! ## Confirmed empirically while building this file
//!
//! - **`octave-runtime`'s ground-truth convention is identical to
//!   `matlab-runtime`'s.** `coding_adventures_octave_runtime::eval` is
//!   `Interpreter::new().feed(source)` where `feed` is
//!   `self.inner.feed(&octavify(source))` over a
//!   `coding_adventures_matlab_runtime::Interpreter` — the exact same
//!   `Interpreter::feed` the MATLAB oracle file's `matlab_eval` calls, just
//!   with an `octavify` pre-pass. So the same two facts that file's module
//!   doc establishes for MATLAB — `disp` is an unconditional no-op, and the
//!   unsuppressed `name = value`/`ans = value` echo is the only working
//!   display path — hold here without any independent re-derivation needed;
//!   [`ground_truth`] below uses the identical echo-parsing convention.
//! - **The (now-fixed, per PR #8572) while-loop-accumulator and
//!   unary-minus-on-power bugs are confirmed fixed here too, not just for
//!   MATLAB.** `octave-to-semantic-ir` shares 100% of its lowering and
//!   codegen with `matlab-to-semantic-ir` (this crate's `src/lib.rs` has no
//!   `src/lower.rs` of its own at all), so any fix to
//!   `semantic-ir-to-javascript`'s shared `numOf` runtime helper applies
//!   unchanged here. `while_loop_accumulator_endwhile` below is a direct
//!   regression check of that fix, through Octave's own `endwhile` syntax
//!   rather than MATLAB's `end` — and it passes (see the test run this PR's
//!   commit message / CHANGELOG entry records), unlike what a pre-#8572
//!   version of this file would have shown.
//! - **The still-open integer-literal-division-floors bug and the missing
//!   `Feature::ShortCircuit` declaration (both documented as OPEN in the
//!   MATLAB oracle file's module doc) apply here unchanged**, for the same
//!   reason: shared lowering. `CORPUS` avoids both exactly as the MATLAB
//!   file's corpus does (no bare integer division, no `&&`/`||`/`&`/`|`).
//!
//! No new bug specific to the `octavify` shim itself was found: every
//! Octave-only construct in `CORPUS` below round-trips correctly against
//! `octave-runtime`'s own ground truth.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use coding_adventures_octave_runtime::eval as octave_eval;
use octave_to_semantic_ir::compile_source;

/// Is a `node` binary on `PATH`? Mirrors `matlab-to-semantic-ir/tests/
/// oracle.rs`'s `node_available` exactly: every test below skips (logs,
/// does not fail) when it is not.
fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// One oracle corpus entry. See `matlab-to-semantic-ir/tests/oracle.rs`'s
/// own [`Case`]-equivalent doc comment for the full rationale behind the
/// `setup`/`final_expr` split; unchanged here.
struct Case {
    name: &'static str,
    setup: &'static str,
    final_expr: &'static str,
    expected: &'static str,
}

const CORPUS: &[Case] = &[
    // Octave-only `#` comment (MATLAB uses `%`). Plain arithmetic precedence
    // underneath (3 + 4*2 - 5 = 6) just so the comment isn't the only thing
    // under test -- the point is that `octavify` strips it before the
    // MATLAB parser ever sees it, on BOTH the ground-truth and compiled
    // paths (each independently calls `octavify`/its equivalent).
    Case {
        name: "hash_comment_literal_arithmetic",
        setup: "x = 3 + 4 * 2 - 5; # octave-style comment, not matlab's %\n",
        final_expr: "x",
        expected: "6",
    },
    // Octave-only `!=` (MATLAB uses `~=`). `octavify` maps both to the same
    // internal comparison, so this is a genuine equivalence check, not a
    // syntax no-op: if `octavify` didn't rewrite `!=`, the MATLAB parser
    // would reject it outright and this test would fail loudly, not
    // silently.
    Case {
        name: "bang_equals_not_equal_comparison",
        setup: "",
        final_expr: "5 != 3",
        expected: "1",
    },
    // Octave-only `!` (logical not; MATLAB uses `~`), applied to a
    // PARENTHESIZED COMPARISON rather than a bare numeric variable. This
    // choice is deliberate, not incidental: SIR's shared `truthy()` runtime
    // helper (`semantic-ir-to-javascript/src/runtime.rs`) treats only
    // `false`/`nil` as falsy -- a Ruby/Lisp convention, unlike MATLAB's
    // "any nonzero number is true, 0 is false" logicals-are-doubles
    // convention. `~(x > 3)` negates a genuine native-JS boolean (comparison
    // builtins compile to `__Sir.lt`/`__Sir.gt`/etc., which return real
    // `true`/`false`, not a MATLAB-style 0.0/1.0 double) -- so `not`'s
    // `!__Sir.truthy(...)` (`emit.rs`) agrees with MATLAB/Octave semantics
    // here. Negating a bare numeric variable directly (e.g. `!x` with
    // `x = 0`) would NOT agree -- `truthy(0)` is `true` under SIR's
    // convention, so `not(0)` would wrongly give `false` where Octave gives
    // `1` (true) -- a real, confirmed frontend/backend semantic gap (this
    // crate inherits it from `matlab-to-semantic-ir`, which has no
    // MATLAB-logicals-aware truthiness recoding, the same class of gap as
    // the `comparison` case's `#t`/`#f`-vs-`1`/`0` spelling difference in
    // the MATLAB oracle file, except this one is NOT just a spelling
    // difference -- it is excluded from `CORPUS` for exactly that reason.
    Case {
        name: "bang_negation_on_comparison",
        setup: "x = 5;\n",
        final_expr: "!(x > 3)",
        expected: "0",
    },
    // if/else terminated by Octave's `endif` instead of bare `end`. Same
    // pre-declaration workaround as the MATLAB oracle file's own `if_else`
    // case (`y = 0;` before the conditional) for the identical
    // scope-tracking gap in `matlab-to-semantic-ir`'s `lower_if` -- inherited
    // unchanged since this crate shares that lowering wholesale.
    Case {
        name: "if_else_endif",
        setup: "y = 0;\nx = 5;\nif x > 3\n  y = 1;\nelse\n  y = 2;\nendif\n",
        final_expr: "y",
        expected: "1",
    },
    // for-loop accumulator (sum 1..5 = 15) terminated by Octave's `endfor`.
    // Read back via linear indexing (`s(1)`) for the same reason as the
    // MATLAB oracle file's `for_loop_accumulator`: `s` becomes an
    // NDArray-shaped scalar after its first variable-involving `s = s + i`
    // update.
    Case {
        name: "for_loop_accumulator_endfor",
        setup: "s = 0;\nfor i = 1:5\n  s = s + i;\nendfor\n",
        final_expr: "s(1)",
        expected: "15",
    },
    // while-loop accumulator (count 0 -> 10) terminated by Octave's
    // `endwhile`. This is the direct regression check for the (now-fixed,
    // PR #8572) `numOf`-doesn't-unwrap-a-scalar-NDArray bug documented at
    // length in the MATLAB oracle file's module doc and its own
    // `while_loop_accumulator_converges_correctly` test -- reproduced here
    // through Octave syntax specifically (not just re-running the MATLAB
    // case) to confirm the fix, which lives entirely in the shared
    // `semantic-ir-to-javascript` runtime, benefits this frontend too.
    Case {
        name: "while_loop_accumulator_endwhile",
        setup: "n = 0;\nwhile n < 10\n  n = n + 1;\nendwhile\n",
        final_expr: "n(1)",
        expected: "10",
    },
];

/// Ground truth: run `setup` followed by a *bare, unsuppressed* `final_expr`
/// through `octave-runtime`, and pull the value out of its `name = value`
/// (or `ans = value`) echo. Identical convention to (and, since
/// `octave-runtime::eval` delegates to the same `matlab_runtime::
/// Interpreter::feed` the MATLAB oracle file's own ground truth calls,
/// literally the same underlying mechanism as)
/// `matlab-to-semantic-ir/tests/oracle.rs`'s `ground_truth` -- see this
/// file's module doc for why that reuse is exact, not just similar.
fn ground_truth(setup: &str, final_expr: &str) -> String {
    let src = format!("{setup}{final_expr}\n");
    let echo =
        octave_eval(&src).unwrap_or_else(|e| panic!("octave-runtime eval failed for {src:?}: {e}"));
    echo.rsplit('=').next().unwrap_or(&echo).trim().to_string()
}

/// Compiled path: run `setup` followed by `disp(final_expr);` through
/// `octave_to_semantic_ir::compile_source` (which itself runs `octavify`
/// before delegating to `matlab_to_semantic_ir::compile_source` -- see
/// `src/lib.rs`), `semantic_ir::validate`, `semantic_ir_to_javascript::
/// compile`, and an actual `node` process. Mirrors `compiled` in
/// `matlab-to-semantic-ir/tests/oracle.rs`, including the
/// `OpenOptions::create_new(true)` temp-file handling (that file's doc
/// comment explains why: `create_new` fails instead of following an
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
    path.push(format!("octave_sir_oracle_{name}_{}.js", std::process::id()));
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

/// Normalize display-*spelling*-only differences -- identical to
/// `matlab-to-semantic-ir/tests/oracle.rs`'s `normalize` (see that file's
/// module doc "Normalization" section for the full `#t`/`#f`-vs-`1`/`0`
/// rationale, which applies here unchanged since Octave inherits MATLAB's
/// "logicals are doubles" convention through `octave-runtime`). Never used
/// to paper over a genuine value mismatch.
fn normalize(s: &str) -> String {
    match s {
        "#t" | "true" => "1".to_string(),
        "#f" | "false" => "0".to_string(),
        other => other.to_string(),
    }
}

#[test]
fn oracle_corpus_matches_native_octave_runtime() {
    if !node_available() {
        eprintln!("skipping oracle_corpus_matches_native_octave_runtime: `node` not available");
        return;
    }
    for case in CORPUS {
        let gt = normalize(&ground_truth(case.setup, case.final_expr));
        assert_eq!(
            gt, case.expected,
            "{}: octave-runtime itself disagrees with this corpus entry's own `expected` -- \
             the program or `expected` is wrong, fix the corpus rather than this assertion",
            case.name
        );

        let got = normalize(&compiled(case.name, case.setup, case.final_expr));
        assert_eq!(
            got, case.expected,
            "{}: octave-to-semantic-ir -> semantic-ir-to-javascript -> node disagrees with \
             the octave-runtime ground truth ({gt:?}) -- see this file's module doc for known, \
             already-excluded bug classes before assuming this is a new one",
            case.name
        );
    }
}
