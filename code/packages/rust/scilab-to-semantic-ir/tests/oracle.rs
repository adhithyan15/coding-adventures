//! Oracle/golden tests (HML01 §7): the SAME Scilab computation, run through
//! **two independent implementations**, and diffed:
//!
//!   (a) `scilab-runtime` (`coding-adventures-scilab-runtime`) — this
//!       frontend's own sibling crate: a tree-walking interpreter over
//!       `array-runtime` (MA-10d) — the ground truth.
//!   (b) `scilab_to_semantic_ir::compile_source` → `semantic_ir::Module` →
//!       `semantic_ir_to_javascript::compile` → an actual `node` process.
//!
//! Scilab is the LAST language in the whole HML01 track to get this test
//! suite — MATLAB, Octave, Wolfram, Macsyma, Maxima, APL, J, Reduce, Derive,
//! and Maple all already have one. Structurally this file is closer to
//! [`matlab-to-semantic-ir`'s own `tests/oracle.rs`](../../matlab-to-semantic-ir/tests/oracle.rs)/
//! [`octave-to-semantic-ir`'s](../../octave-to-semantic-ir/tests/oracle.rs)
//! than to the CAS-family files (Maple/Reduce/Derive/Wolfram/Macsyma):
//! Scilab shares MATLAB/Octave's imperative, array/matrix (SIR22) domain and
//! — confirmed directly below — the identical two ground-truth mechanics
//! that drove the MATLAB file's own `setup`/`final_expr` [`Case`] shape
//! rather than one bare `source` string. This file also adds one more field
//! neither MATLAB's nor Octave's own `Case` has: `known_bug`, mirroring
//! [`maple-to-semantic-ir`'s own convention](../../maple-to-semantic-ir/tests/oracle.rs)
//! (and `derive-to-semantic-ir`'s/`j-to-semantic-ir`'s identical mechanism)
//! for a genuinely documented, tracked divergence that still runs (and still
//! has its ground-truth side asserted) rather than being silently dropped
//! from `CORPUS` the way MATLAB's/Octave's own files exclude their
//! documented gaps entirely. This hybrid is a deliberate choice, not an
//! oversight: Scilab's ground-truth mechanics force the MATLAB-shaped
//! harness, but this crate turned out to have enough genuinely worth-tracking
//! findings (six, see below) that Maple's `known_bug` convention pulls its
//! own weight here too — every finding below is documented as data in
//! `CORPUS`, not just in this comment.
//!
//! ## Why `setup` + `final_expr`, not one `source` string (confirmed
//! directly, not assumed by analogy with MATLAB)
//!
//! 1. **`scilab-runtime`'s `disp` builtin is a no-op**, exactly like
//!    `matlab-runtime`'s. Read `scilab-runtime/src/builtins.rs`'s `"disp"`
//!    arm: it checks that one argument was passed and then unconditionally
//!    returns an empty, invisible array, discarding the argument entirely.
//!    Neither `scilab-runtime`'s own `#[cfg(test)]` module nor
//!    `scilab-to-semantic-ir`'s existing `tests/*.rs` ever reads a value back
//!    through it — every ground-truth assertion instead relies on Scilab's
//!    *other*, actually-working display convention: an unsuppressed (no
//!    trailing `;`) statement echoes `name = value` (or `ans = value` for a
//!    bare expression) — see `scilab-runtime/src/value.rs`'s `echo` function
//!    and `eval.rs`'s `eval_statement_line`. This is a confirmed,
//!    pre-existing bug in `scilab-runtime` (out of scope for this crate to
//!    fix — flagged here, not touched).
//! 2. **`semantic_ir` has no "implicit display" representation at all** —
//!    identical to the MATLAB file's own point 2. The only way to observe a
//!    value through the compiled → `node` path is an explicit `disp(...)`
//!    call, which this frontend's `lower_postfix` maps onto the shared SIR
//!    `print` builtin (`src/lower.rs`'s own doc comment, "Supported"
//!    section) — and unlike `scilab-runtime`'s own `disp`, THIS one actually
//!    works (confirmed directly: `tests/e2e_node.rs`'s
//!    `a_function_over_pure_literals_runs_in_node` et al. all read a value
//!    back through a bare `disp(...)` call successfully).
//!
//! So exactly like the MATLAB/Octave files: each [`Case`] stores one `setup`
//! string (ordinary `;`-terminated Scilab statements, byte-for-byte identical
//! on both sides) plus a `final_expr` (a bare expression, no terminator).
//! [`ground_truth`] appends `final_expr` bare and unsuppressed
//! (`scilab-runtime`'s own working convention); [`compiled`] wraps it in
//! `disp(...)` (the compiled path's own working convention, and — unlike
//! MATLAB's own `for_loop_accumulator` case, which had to read a computed
//! array back via linear indexing `s(1)` because `matlab-runtime`'s
//! contemporaneous JS backend had no display story for a computed NDArray —
//! confirmed directly here that `disp` on a NON-literal, already-computed
//! scalar (`s = s + i` accumulated across a loop) prints correctly with NO
//! indexing workaround needed: `tests/e2e_node.rs`'s own
//! `for_loop_accumulator_converges_in_node` already proves this via
//! `disp(total)` directly, and this file's own `for_loop_accumulator`/
//! `while_loop_accumulator` cases below confirm it again against
//! `scilab-runtime`).
//!
//! ## Normalization
//!
//! Identical rationale to the MATLAB/Octave files' own `normalize`: Scilab
//! comparisons/logicals are `array-runtime` numeric `0.0`/`1.0` values (no
//! separate boolean type — `scilab-runtime/src/value.rs`'s own module doc:
//! "Logicals are ordinary `0.0`/`1.0` numeric arrays"), so `scilab-runtime`
//! prints `1`/`0`; but the shared JS backend's `=`/`!=`/`<`/`<=`/`>`/`>=`
//! builtins compile to native JS `<`/`>`/`===`-style helpers (`emit.rs`:
//! `"=" => Some("__Sir.eq")`, etc.), which produce real JS booleans, and
//! `format()` renders those Scheme-style (`#t`/`#f`) by default. [`normalize`]
//! maps `"#t"`/`"true"` → `"1"` and `"#f"`/`"false"` → `"0"`, nothing else —
//! confirmed empirically for every comparison/logic/equality case below
//! (see the probe results cited in each case's own comment), never used to
//! paper over an actual value mismatch.
//!
//! ## Six findings, confirmed directly by running every case below through
//! both `scilab-runtime` and an actual `node` process (not assumed from
//! static analysis alone) — one of the six is a genuine BUG, NOT fixed here
//!
//! 1. **NOT a bug — already fixed upstream, unlike MATLAB's own history.**
//!    Every `if`/`elseif`/`select`/`while`/`for` construct in this crate's
//!    `lower.rs` already calls `hoist_assigned_names` (see that function's
//!    own extensive doc comment) before lowering its body, so a variable
//!    FIRST introduced inside a branch/loop is visible afterward with NO
//!    pre-declaration needed — unlike `matlab-to-semantic-ir`'s own
//!    `if_else`/`elseif_chain` cases, which still pre-declare (`y = 0;`)
//!    because that crate's own scope-tracking gap (documented in its oracle
//!    file's module doc) was never backported. Confirmed directly:
//!    `if_else_without_predeclaration`/`elseif_chain` below introduce `y`/`r`
//!    for the first time inside the `if`, with no prior `y = 0;`/`r = 0;`,
//!    and both sides agree. Not a new discovery (this crate's own
//!    `tests/e2e_node.rs` already demonstrates it), but confirmed here
//!    against `scilab-runtime` for the first time.
//! 2. **NOT a bug — the shared "bare-numeric/stored-comparison Scilab
//!    truthiness" fix (documented at length in `matlab-to-semantic-ir/tests/
//!    oracle.rs`'s own module doc) is already present, not something this
//!    crate had to rediscover.** `lower.rs`'s `to_scilab_condition` wraps
//!    every boolean-context operand in the shared `matlab_truthy` runtime
//!    intrinsic unconditionally, deciding boolean-vs-bare-number AT RUNTIME
//!    — so `~0`/`~5`, `if` on a bare zero/nonzero condition, `&&`/`||`/`&`/
//!    `|` on a bare-zero operand, and `if` on a variable holding a STORED
//!    comparison result (`tf = (5 > 3); if tf ...`) all agree with
//!    `scilab-runtime`'s ground truth once `normalize`d. Confirmed by every
//!    `negation_*`/`logical_*`/`stored_comparison_*` case below.
//! 3. **Display-convention gap, NOT new to this crate: the shared backend's
//!    Ruby-family `FloatLit` codegen boxes EVERY float literal through
//!    `__Sir.mkFloat(...)`** (`emit.rs`'s own doc comment: "leaves a
//!    non-integral [value] native... boxes an integral one"), so a
//!    whole-valued float literal like `4.0` prints with a spurious trailing
//!    `.0` on the compiled side (`format()`'s `SirFloat` branch calls
//!    `floatToRubyString`), while `scilab-runtime`'s own
//!    `array_runtime::fmt_num` — like `matlab-runtime`'s identical helper —
//!    explicitly drops the trailing `.0` for any integral-valued double
//!    (`"Integer-valued doubles print without a decimal point (3, not
//!    3.0)"`). **Genuinely new here**, though: neither the MATLAB nor the
//!    Octave oracle file's own `CORPUS` ever exercises a bare decimal-point
//!    float literal's *display* at all (both stick to integer literals
//!    throughout), so this specific SIR22-domain gap was never previously
//!    confirmed end-to-end anywhere in this repo — see
//!    `bare_whole_valued_float_literal_prints_a_spurious_trailing_dot_zero`
//!    below, the first case in this whole track to do so.
//! 4. **Display-convention gap, genuinely new to this crate: no per-language
//!    number formatting exists for `Infinity`/very-small-magnitude floats
//!    in the shared SIR22 codegen.** `%inf` prints `"inf"` on the
//!    `scilab-runtime` side (Rust's own `f64` `Display`) but `"Infinity"` on
//!    the compiled side (JS's `Number.prototype.toString`); `%eps`
//!    (`f64::EPSILON`, ~2.22e-16) prints as a full decimal expansion
//!    (`"0.0000000000000002220446049250313"`) on the Rust side but
//!    scientific notation (`"2.220446049250313e-16"`) on the JS side (V8
//!    switches to exponential notation below ~1e-6). Neither the MATLAB nor
//!    Octave oracle file's `CORPUS` tests `Inf`/`eps` display either, so
//!    this is a second genuinely-new-here display gap — see
//!    `percent_inf_display_format_diverges`/
//!    `percent_eps_display_format_diverges` below.
//! 5. **NOT new — the SAME still-OPEN "integer-literal division floors"
//!    bug `matlab-to-semantic-ir/tests/oracle.rs`'s own module doc already
//!    documents, confirmed to also affect Scilab** (inherited, not
//!    independently reintroduced): Scilab has no integer type either —
//!    every number is an `array-runtime` `f64` — so `1 / 3` is `0.333...`
//!    on the ground-truth side (`ops::div` is a plain float divide), but
//!    `number_literal_expr` (`src/lower.rs`, mirroring
//!    `matlab_to_semantic_ir::number_literal_expr` exactly) lowers a
//!    decimal-point-free literal to `Expr::IntLit`, and the shared JS
//!    backend's `divide()` runtime helper (`runtime.rs`, built for Ruby's
//!    `Integer#/`, which really does floor) floors whenever BOTH operands
//!    are integer-valued — so the compiled side prints `0`, not `0.333...`.
//!    See `integer_literal_division_floors_a_shared_backend_gap` below.
//! 6. **GENUINE BUG, confirmed directly by running actual `node` output —
//!    NOT a display-convention gap, NOT fixed in this PR.** `matmul(a, b)`
//!    in `semantic-ir-to-javascript/src/runtime.rs` reads `a.shape`/
//!    `b.shape` UNCONDITIONALLY (via its own `nrows`/`ncols` helpers) with
//!    NO `toArrayValue` normalization step first — unlike its sibling
//!    `elementwise(op, a, b)`, which explicitly calls `a = toArrayValue(a);
//!    b = toArrayValue(b);` before touching either operand specifically so
//!    "a plain `IntLit`/`FloatLit`/arithmetic result... never reaches
//!    `.data`/`.shape` and throws a `TypeError`" (that function's own doc
//!    comment). This crate's own scalar/array disambiguation heuristic
//!    (`expr_is_known_scalar`, mirroring `matlab_to_semantic_ir`'s
//!    identical one) can NEVER see through a variable binding — a bare
//!    `Expr::VarRef` is never "known scalar," even when the variable
//!    provably holds a plain number — so `build_multiplicative`'s `"*"` arm
//!    falls to its `else` branch (`Expr::MatMul`) for ANY `x * y` where
//!    NEITHER operand is a literal, regardless of what `x`/`y` actually
//!    hold at runtime. When `x`/`y` turn out to be plain scalars (not array
//!    literals), the emitted `__Sir.matmul(x, y)` call crashes at runtime:
//!    `TypeError: Cannot read properties of undefined (reading 'length')`
//!    at `nrows`, called from `matmul`. Confirmed three independent ways
//!    (all reproduced by hand, running the actual generated JS through
//!    `node`, not merely read from source): a bare top-level
//!    `x = 5; y = x * x;` (`scalar_variable_self_multiplication_crashes_the_
//!    compiled_path`), a function computing `y = x * x` for its own
//!    parameter `x` (`function_parameter_self_multiplication_crashes_the_
//!    compiled_path`), and a recursive factorial (`y = n * fact(n - 1)`) —
//!    all three crash identically. **This is almost certainly a
//!    pre-existing, previously-undiscovered gap in the SHARED
//!    `semantic-ir-to-javascript` crate** (not specific to this frontend's
//!    own lowering, which correctly mirrors `matlab_to_semantic_ir`'s own
//!    scalar-disambiguation heuristic verbatim) — reachable through
//!    `matlab-to-semantic-ir` too in principle (it shares the identical
//!    heuristic and the identical `matmul` codegen path), but never
//!    surfaced there because (a) `matlab-runtime` has no user-defined
//!    function support at all (so a function-parameter-based repro was
//!    never reachable), and (b) `matlab-to-semantic-ir/tests/oracle.rs`'s
//!    own `CORPUS` only ever multiplies two variables when they hold a
//!    genuine array LITERAL (`A = [1 2; 3 4]; B = A * A;` — always properly
//!    NDArray-shaped by construction, never a bare scalar), never a plain
//!    scalar variable times itself. Scilab's oracle harness is the first to
//!    exercise this exact shape, made possible specifically because
//!    `scilab-runtime` (unlike `matlab-runtime`) DOES support user-defined
//!    functions, giving this crate ground truth MATLAB's own oracle file
//!    could never have compared against even if it had tried. **Not fixed
//!    here per this task's own scope constraints** (this PR is oracle-TEST
//!    only; `semantic-ir-to-javascript` is explicitly out of bounds) — see
//!    `scalar_variable_self_multiplication_crashes_the_compiled_path`/
//!    `function_parameter_self_multiplication_crashes_the_compiled_path`
//!    below and this PR's own CHANGELOG entry; flagged for a dedicated
//!    follow-up task against `semantic-ir-to-javascript`'s `matmul`.
//!
//! ## Constructs confirmed impossible to oracle-test at all (not merely
//! excluded from `CORPUS` as "currently buggy") — mirrors the MATLAB oracle
//! file's own "two more constructs" section
//!
//! - **Indexed assignment (`A(2) = 9;`).** `scilab-runtime::eval::
//!   Interpreter::eval_expr_or_assign` requires the assignment target to be
//!   a bare variable name and errors `"assignment target must be a
//!   variable"` for any indexed LHS (confirmed directly: `A = [1 2 3];\nA(2)
//!   = 9;\n` fails with exactly that message) — even though this is
//!   ordinary Scilab and `scilab-to-semantic-ir`'s own `tests/e2e_node.rs`
//!   (`indexed_assignment_mutates_in_place_and_reads_back_in_node`) already
//!   round-trips it successfully through the JS path. There is no ground
//!   truth to diff against, exactly mirroring MATLAB's own identical,
//!   independently-confirmed gap in `matlab-runtime`.
//! - **`break`/`continue`, `$` (last-index), and multi-output functions.**
//!   `scilab-runtime` supports all three (see its own `#[cfg(test)]`
//!   module); this frontend's own `lower.rs` explicitly does NOT (each is a
//!   clean, disclosed `ScilabLowerError` — see that file's module doc,
//!   "Deliberately out of scope for v0.1.0"). Since `compile_source` never
//!   produces a `Module` for any of these, there is nothing for `compiled`
//!   below to even run — not a divergence to document, a scope cut already
//!   fully documented in `lower.rs` itself.
//!
//! ## No Scilab-specific builtin reaches the compiled path at all — a scope
//! note, not a new finding
//!
//! `scilab-runtime::builtins` implements `zeros`/`ones`/`eye`/`size`/
//! `length`/`numel`/`sum`/`mean`/`max`/`min`/`abs`/`sqrt`/`transpose` (as a
//! callable function, distinct from the `'` operator) in addition to `disp`
//! — but `scilab-to-semantic-ir`'s own `lower_postfix` recognises ONLY
//! `disp` as a builtin call target (confirmed directly: its own error
//! message for every other bare identifier reads "only `disp` is recognised
//! as a builtin in this cut"). So none of those other builtins can appear in
//! `CORPUS` below at all — calling any of them through `compile_source`
//! fails to lower, not merely to match. This is an already-documented,
//! deliberate v0.1.0 scope cut (`lower.rs`'s own module doc never lists them
//! as supported), not a new gap this file discovered.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use coding_adventures_scilab_runtime::eval as scilab_eval;
use scilab_to_semantic_ir::compile_source;

/// Is a `node` binary on `PATH`? Mirrors every sibling oracle file's
/// identical `node_available`: the test below skips (logs, does not fail)
/// when it is not.
fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// One oracle corpus entry. `setup` is a run of ordinary `;`-terminated
/// Scilab statements — the actual computation, shared byte-for-byte between
/// [`ground_truth`] and [`compiled`]. `final_expr` is a bare expression (no
/// terminator) naming the value being cross-checked. `expected` is that
/// value in already-[`normalize`]d form. `known_bug`: `None` means both
/// sides must equal `expected` exactly; `Some(reason)` means only the
/// ground-truth side is asserted (`compiled` is not even invoked) — see this
/// file's module doc comment's six numbered findings for what `reason`
/// refers to in each case, mirroring `maple-to-semantic-ir/tests/oracle.rs`'s
/// own `known_bug` convention and comparison-logic shape exactly.
struct Case {
    name: &'static str,
    setup: &'static str,
    final_expr: &'static str,
    expected: &'static str,
    known_bug: Option<&'static str>,
}

const CORPUS: &[Case] = &[
    // --- Plain literal arithmetic ------------------------------------------
    Case {
        name: "literal_arithmetic_precedence",
        setup: "x = 3 + 4 * 2 - 5;\n",
        final_expr: "x",
        expected: "6",
        known_bug: None,
    },
    Case {
        name: "nested_parens_override_precedence",
        setup: "",
        final_expr: "(2 + 3) * (4 - 1)",
        expected: "15",
        known_bug: None,
    },
    // Unary minus binds LOOSER than `^` (`-2^2` = `-(2^2)` = `-4`, not
    // `(-2)^2` = `4`) -- confirmed against `scilab-runtime`'s own
    // `scalar_arithmetic_echoes_ans` test premise, and against the compiled
    // path via the shared `numOf`-unwraps-a-rank-0-NDArray fix
    // `matlab-to-semantic-ir/tests/oracle.rs`'s module doc documents at
    // length (this crate inherits that fix for free -- confirmed here, not
    // re-derived).
    Case {
        name: "unary_minus_binds_looser_than_power",
        setup: "",
        final_expr: "-2 ^ 2",
        expected: "-4",
        known_bug: None,
    },
    Case {
        name: "exact_integer_division_folds_cleanly",
        setup: "",
        final_expr: "10 / 2",
        expected: "5",
        known_bug: None,
    },
    Case {
        name: "reassignment_of_a_known_local_takes_the_later_value",
        setup: "x = 1;\nx = 2;\n",
        final_expr: "x",
        expected: "2",
        known_bug: None,
    },
    Case {
        name: "scalar_variable_self_addition_computes_correctly",
        // Contrast case for finding six: `+` between two non-literal
        // operands takes the `ElementwiseOp` path (`build_additive`), whose
        // JS codegen (`ArrayRt.elementwise`) DOES call `toArrayValue` first
        // -- so this shape, unlike `*`, is perfectly safe. Included
        // specifically to show the bug is `*`-and-`MatMul`-specific, not a
        // blanket "any variable arithmetic crashes" problem.
        setup: "x = 5;\n",
        final_expr: "x + x",
        expected: "10",
        known_bug: None,
    },

    // --- Comparisons: fold to a real JS boolean on the compiled side, a
    // 0.0/1.0 numeric value on the ground-truth side -- see the module doc's
    // "Normalization" section. Includes both of Scilab's not-equal spellings
    // (MA10 §1 finding 6): `~=` and `<>`. ---
    Case {
        name: "comparison_greater_is_true",
        setup: "",
        final_expr: "5 > 3",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "comparison_greater_is_false",
        setup: "",
        final_expr: "3 > 5",
        expected: "0",
        known_bug: None,
    },
    Case {
        name: "equal_operator_is_true",
        setup: "",
        final_expr: "5 == 5",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "not_equal_tilde_spelling_is_true",
        setup: "",
        final_expr: "5 ~= 3",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "not_equal_angle_bracket_spelling_is_true",
        setup: "",
        final_expr: "5 <> 3",
        expected: "1",
        known_bug: None,
    },

    // --- Unary `~` and logical `&&`/`||`/`&`/`|` on BARE numeric operands
    // (no comparison in sight) -- exercises the shared `matlab_truthy`
    // runtime intrinsic `to_scilab_condition` wraps every boolean-context
    // operand in (finding two: already fixed upstream, not rediscovered
    // here). Short-circuit (`&&`/`||`) and elementwise (`&`/`|`) spellings
    // are deliberately NOT distinguished (this frontend's own `lower.rs`
    // module doc), so both are exercised. ---
    Case {
        name: "negation_of_bare_zero_is_true",
        setup: "",
        final_expr: "~0",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "negation_of_bare_nonzero_is_false",
        setup: "",
        final_expr: "~5",
        expected: "0",
        known_bug: None,
    },
    Case {
        name: "logical_single_amp_short_circuits_false_on_a_bare_zero_operand",
        setup: "y = 0;\nif 0 & 1\n  y = 1;\nelse\n  y = 2;\nend\n",
        final_expr: "y",
        expected: "2",
        known_bug: None,
    },
    Case {
        name: "logical_single_pipe_is_true_via_a_bare_nonzero_second_operand",
        setup: "y = 0;\nif 0 | 5\n  y = 1;\nelse\n  y = 2;\nend\n",
        final_expr: "y",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "logical_double_amp_and_is_true",
        setup: "y = 0;\nif 1 && 1\n  y = 1;\nelse\n  y = 2;\nend\n",
        final_expr: "y",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "logical_double_pipe_or_is_false_when_both_operands_are_zero",
        setup: "y = 0;\nif 0 || 0\n  y = 1;\nelse\n  y = 2;\nend\n",
        final_expr: "y",
        expected: "2",
        known_bug: None,
    },
    // `if` on a variable holding a STORED comparison result (not a bare
    // numeric literal, and not a comparison written directly in the `if`
    // header) -- exercises `matlab_truthy`'s runtime (not static-shape)
    // boolean-vs-number decision, mirroring `matlab-to-semantic-ir/tests/
    // oracle.rs`'s own regression pair for the identical reason.
    Case {
        name: "if_condition_on_a_variable_holding_a_stored_true_comparison",
        setup: "y = 0;\ntf = (5 > 3);\nif tf\n  y = 1;\nelse\n  y = 2;\nend\n",
        final_expr: "y",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "if_condition_on_a_variable_holding_a_stored_false_comparison",
        setup: "y = 0;\ntf = (5 < 3);\nif tf\n  y = 1;\nelse\n  y = 2;\nend\n",
        final_expr: "y",
        expected: "2",
        known_bug: None,
    },

    // --- if/elseif/else -- NO pre-declaration needed (finding one): this
    // crate's `hoist_assigned_names` already makes a branch-introduced
    // variable visible afterward, unlike matlab-to-semantic-ir's own,
    // still-unfixed scope-tracking gap its oracle file's `if_else`/
    // `elseif_chain` cases have to work around with `y = 0;`/`r = 0;`
    // pre-declarations. ---
    Case {
        name: "if_else_without_predeclaration",
        setup: "x = 5;\nif x > 3\n  y = 1;\nelse\n  y = 2;\nend\n",
        final_expr: "y",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "elseif_chain_without_predeclaration",
        setup: "x = 5;\nif x > 10\n  r = 1;\nelseif x > 3\n  r = 2;\nelse\n  r = 3;\nend\n",
        final_expr: "r",
        expected: "2",
        known_bug: None,
    },

    // --- Loops: for/while accumulators, read back via a direct `disp`, no
    // linear-indexing workaround needed (see this file's own module doc,
    // "Why setup + final_expr" section, for why this differs from MATLAB's
    // own `s(1)` workaround). ---
    Case {
        name: "for_loop_accumulator",
        setup: "s = 0;\nfor i = 1:5\n  s = s + i;\nend\n",
        final_expr: "s",
        expected: "15",
        known_bug: None,
    },
    Case {
        name: "while_loop_accumulator",
        setup: "n = 0;\nwhile n < 10\n  n = n + 1;\nend\n",
        final_expr: "n",
        expected: "10",
        known_bug: None,
    },

    // --- Matrices: literal construction, real matmul, elementwise scalar
    // broadcast, transpose -- all already proven to run in `node` by
    // `tests/e2e_node.rs`; here cross-checked against `scilab-runtime` for
    // the first time. ---
    Case {
        name: "matrix_multiplication_of_array_literals",
        // [1 2; 3 4] * [1 2; 3 4] = [7 10; 15 22]; (1,1) 1-based = 7. Both
        // operands come from a genuine `ArrayLit`, always properly
        // NDArray-shaped by construction -- contrast with finding six's bare
        // scalar-variable cases below, which crash precisely because they
        // are NOT array literals.
        setup: "A = [1 2; 3 4];\nB = A * A;\n",
        final_expr: "B(1, 1)",
        expected: "7",
        known_bug: None,
    },
    Case {
        name: "elementwise_scalar_broadcast",
        setup: "A = [1 2; 3 4];\nB = A .* 2;\n",
        final_expr: "B(2, 2)",
        expected: "8",
        known_bug: None,
    },
    Case {
        name: "range_and_transpose",
        setup: "A = [1 2; 3 4];\nB = A';\n",
        final_expr: "B(2, 1)",
        expected: "2",
        known_bug: None,
    },

    // --- select/case: Scilab's OWN multi-way conditional (MA10 §1 finding
    // 4), with no MATLAB/Octave `switch`/`otherwise` analogue anywhere else
    // in this repo -- desugars to a nested if-chain at lowering time (see
    // `lower.rs`'s own module doc, "select/case: desugared, no new SIR
    // node"). No pre-declaration needed here either (same hoisting
    // mechanism as if/elseif, finding one), except for the
    // no-match-no-else case, which must pre-declare specifically to prove
    // the untouched-value semantics (see that case's own comment). ---
    Case {
        name: "select_case_matches_the_first_equal_case",
        setup: "x = 2;\nselect x\n case 1\n  y = 10;\n case 2\n  y = 20;\n else\n  y = 0;\nend\n",
        final_expr: "y",
        expected: "20",
        known_bug: None,
    },
    Case {
        name: "select_case_falls_through_to_else_when_nothing_matches",
        setup: "x = 99;\nselect x\n case 1\n  y = 10;\n else\n  y = -1;\nend\n",
        final_expr: "y",
        expected: "-1",
        known_bug: None,
    },
    Case {
        name: "select_case_with_no_match_and_no_else_leaves_the_value_untouched",
        // `y` MUST be pre-declared here: this is the one select/case shape
        // where nothing in the construct assigns it at all (no case
        // matches, there is no `else`), so there is genuinely no value to
        // hoist a fresh binding FOR -- mirrors `scilab-runtime`'s own
        // `select_case_with_no_match_and_no_else_does_nothing` test premise
        // exactly.
        setup: "y = 7;\nx = 99;\nselect x\n case 1\n  y = 10;\nend\n",
        final_expr: "y",
        expected: "7",
        known_bug: None,
    },

    // --- User-defined functions: single return value, and recursion.
    // Genuinely NEW capability relative to MATLAB's/Octave's own oracle
    // files -- `matlab-runtime` has no `func_def` evaluator arm at all (that
    // file's own module doc: "There is no ground truth to diff against
    // until matlab-runtime grows function support"), but `scilab-runtime`
    // DOES support user-defined functions (multiple return values
    // included, though this frontend only lowers the single/zero-output
    // case) -- so Scilab's oracle harness can test something MATLAB's/
    // Octave's structurally cannot. Deliberately built around `+`/`-`/
    // comparisons, NOT `*` between two parameters -- see finding six: that
    // shape crashes the compiled path, and has its own dedicated cases
    // below instead of being silently avoided without comment. ---
    Case {
        name: "function_with_a_single_return_value",
        setup: "function y = increment(x)\n y = x + 1;\nendfunction\n",
        final_expr: "increment(5)",
        expected: "6",
        known_bug: None,
    },
    Case {
        name: "recursive_function_accumulates_via_addition",
        // sum(1..5) computed via recursion + addition (deliberately not
        // multiplication -- see finding six). 5+4+3+2+1+0 = 15.
        setup: "function y = sum_to(n)\n if n <= 0\n  y = 0;\n else\n  y = n + sum_to(n - 1);\n end\nendfunction\n",
        final_expr: "sum_to(5)",
        expected: "15",
        known_bug: None,
    },

    // --- Strings: assignment/display/equality only (MA10 §4's own scope
    // cut) -- `==`/`~=`/`<>` are the one operator family MA10 §4 keeps in
    // scope for strings (arithmetic/ordering over them is a documented,
    // deliberate rejection -- MA10 §1 finding 1, the whole reason this
    // language has its own frontend rather than reusing matlab-runtime's
    // MatValue). ---
    Case {
        name: "string_equality_is_true_for_identical_content",
        setup: "s = 'hello';\nt = 'hello';\n",
        final_expr: "s == t",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "string_equality_is_false_for_different_content",
        setup: "s = 'abc';\nt = 'xyz';\n",
        final_expr: "s == t",
        expected: "0",
        known_bug: None,
    },
    Case {
        name: "string_inequality_tilde_spelling",
        setup: "s = 'abc';\nt = 'xyz';\n",
        final_expr: "s ~= t",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "string_inequality_angle_bracket_spelling",
        setup: "s = 'abc';\nt = 'xyz';\n",
        final_expr: "s <> t",
        expected: "1",
        known_bug: None,
    },

    // --- The eight `%`-prefixed special constants (MA10 §3/§4):
    // constant-folded at lowering time, not a new SIR node (`lower.rs`'s own
    // module doc, "%-constants"). `%pi`/`%t`/`%f`/`%nan` all agree exactly;
    // `%inf`/`%eps` hit finding four (display-format only, see below). ---
    Case {
        name: "percent_pi_constant",
        setup: "",
        final_expr: "%pi",
        expected: "3.141592653589793",
        known_bug: None,
    },
    Case {
        name: "percent_t_is_the_numeric_value_one",
        setup: "",
        final_expr: "%t",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "percent_f_is_the_numeric_value_zero",
        setup: "",
        final_expr: "%f",
        expected: "0",
        known_bug: None,
    },
    Case {
        name: "percent_nan_constant",
        setup: "",
        final_expr: "%nan",
        expected: "NaN",
        known_bug: None,
    },

    // ======================================================================
    // Known-bug cases: `compiled` is deliberately NOT invoked for these (see
    // this file's module doc comment's six numbered findings, and
    // `maple-to-semantic-ir/tests/oracle.rs`'s identical `known_bug`
    // convention this mirrors). `expected` is still `scilab-runtime`'s own,
    // independently-asserted ground-truth value.
    // ======================================================================

    // Finding three: shared Ruby-family FloatLit boxing convention -- a
    // whole-valued float literal prints with a spurious trailing `.0` on the
    // compiled side. Genuinely new: neither the MATLAB nor Octave oracle
    // file's own CORPUS ever disp's a bare decimal-point float literal.
    Case {
        name: "bare_whole_valued_float_literal_prints_a_spurious_trailing_dot_zero",
        setup: "",
        final_expr: "4.0",
        expected: "4",
        known_bug: Some(
            "Finding three (module doc): emit.rs's FloatLit codegen unconditionally boxes every \
             float literal through __Sir.mkFloat(...) (the shared Ruby-family Integer-vs-Float \
             tagging convention), so a whole-valued literal like 4.0 renders via format()'s \
             SirFloat branch (floatToRubyString) as \"4.0\" on the compiled side -- but \
             scilab-runtime's own array_runtime::fmt_num (identical to matlab-runtime's) \
             explicitly drops the trailing .0 for any integral-valued double, printing \"4\". A \
             shared-crate display-convention gap, not this frontend's own lowering bug (a bare \
             4.0 literal needs no evaluation at all) -- confirmed to be genuinely new to this \
             oracle-testing exercise since neither the MATLAB nor Octave oracle CORPUS ever \
             disp's a decimal-point float literal directly.",
        ),
    },
    // Finding four: no per-language number formatting exists for Infinity /
    // very-small-magnitude floats anywhere in the shared SIR22 codegen.
    Case {
        name: "percent_inf_display_format_diverges",
        setup: "",
        final_expr: "%inf",
        expected: "inf",
        known_bug: Some(
            "Finding four (module doc): scilab-runtime's array_runtime::fmt_num renders an \
             infinite f64 via Rust's own Display (\"inf\"), but the compiled side's plain JS \
             number renders via Number.prototype.toString (\"Infinity\") -- a shared-crate \
             display-convention gap (no per-language number-formatting hook exists in the SIR22 \
             codegen), genuinely new to this oracle-testing exercise (neither the MATLAB nor \
             Octave oracle CORPUS tests %inf-equivalent display).",
        ),
    },
    Case {
        name: "percent_eps_display_format_diverges",
        setup: "",
        final_expr: "%eps",
        expected: "0.0000000000000002220446049250313",
        known_bug: Some(
            "Finding four (module doc): f64::EPSILON (~2.22e-16) renders as a full decimal \
             expansion via Rust's Display on the ground-truth side, but V8's Number.prototype.\
             toString switches to scientific notation below ~1e-6 (\"2.220446049250313e-16\") on \
             the compiled side -- the same shared no-per-language-number-formatting gap as \
             percent_inf_display_format_diverges, for a different magnitude regime.",
        ),
    },
    // Finding five: NOT new -- the same still-OPEN integer-literal-division-
    // floors bug matlab-to-semantic-ir/tests/oracle.rs's own module doc
    // documents, confirmed to also affect Scilab (inherited via the shared
    // divide() runtime helper and the identical number_literal_expr
    // int-vs-float lowering rule -- not independently reintroduced here).
    Case {
        name: "integer_literal_division_floors_a_shared_backend_gap",
        setup: "",
        final_expr: "1 / 3",
        expected: "0.3333333333333333",
        known_bug: Some(
            "Finding five (module doc) -- confirmed to affect Scilab too, NOT independently \
             reintroduced here: Scilab has no integer type either (array-runtime's f64 core is \
             always a true divide, ops::div), so ground truth is 0.333.... But number_literal_expr \
             lowers a decimal-point-free literal to Expr::IntLit (mirroring matlab_to_semantic_ir::\
             number_literal_expr exactly), and semantic-ir-to-javascript's shared divide() runtime \
             helper (built for Ruby's Integer#/, which really does floor) floors whenever BOTH \
             operands are integer-valued -- so the compiled side prints \"0\". Same still-open, \
             shared-crate gap matlab-to-semantic-ir/tests/oracle.rs's own module doc already \
             documents for MATLAB; this crate inherits it via the identical lowering rule and the \
             identical shared runtime helper, not via any bug of its own.",
        ),
    },
    // Finding six: GENUINE BUG (not a display-convention gap), confirmed by
    // actually running the generated JS through node -- see this file's
    // module doc comment for the full root-cause writeup. NOT fixed here
    // (semantic-ir-to-javascript is out of scope for this PR); flagged for
    // a dedicated follow-up task.
    Case {
        name: "scalar_variable_self_multiplication_crashes_the_compiled_path",
        setup: "x = 5;\n",
        final_expr: "x * x",
        expected: "25",
        known_bug: Some(
            "Finding six (module doc) -- GENUINE BUG, not a display-convention gap, NOT fixed in \
             this PR. `x * x` where x is a bare VarRef (never \"known scalar\" to this crate's own \
             expr_is_known_scalar heuristic, even though x provably holds the literal 5) falls to \
             build_multiplicative's `*` `else` branch, Expr::MatMul. semantic-ir-to-javascript's \
             matmul(a, b) runtime helper (runtime.rs) reads a.shape/b.shape unconditionally, with \
             NO toArrayValue normalization step first (unlike its sibling elementwise(), which \
             explicitly calls toArrayValue on both operands before touching either) -- so passing \
             two plain JS numbers crashes with `TypeError: Cannot read properties of undefined \
             (reading 'length')` at nrows, called from matmul. Confirmed directly by running the \
             actual generated JS through node, not read from source alone. This is almost \
             certainly reachable through matlab-to-semantic-ir too (identical heuristic, identical \
             matmul codegen), but was never previously discovered there because that oracle file's \
             own CORPUS only ever multiplies two variables holding a genuine array literal (always \
             properly NDArray-shaped), never a bare scalar. Flagged for a dedicated follow-up task \
             against semantic-ir-to-javascript's matmul; not touched here per this PR's own scope \
             constraints.",
        ),
    },
    Case {
        name: "function_parameter_self_multiplication_crashes_the_compiled_path",
        setup: "function y = square(x)\n y = x * x;\nendfunction\n",
        final_expr: "square(5)",
        expected: "25",
        known_bug: Some(
            "Finding six (module doc) -- same root cause as \
             scalar_variable_self_multiplication_crashes_the_compiled_path, reached instead through \
             a function PARAMETER (x is Scope::Param, not Scope::Local, but expr_is_known_scalar \
             treats every bare VarRef identically regardless of scope -- still never \"known \
             scalar\"). Confirmed directly via node: `TypeError: Cannot read properties of \
             undefined (reading 'length')` at nrows, called from matmul, called from the compiled \
             `square` function. This specific repro shape (a function computing x*x for its own \
             scalar parameter -- arguably one of the most ordinary possible Scilab/MATLAB function \
             bodies) was only reachable here because scilab-runtime, unlike matlab-runtime, \
             supports user-defined functions at all; not fixed in this PR.",
        ),
    },
];

/// Ground truth: run `setup` followed by a *bare, unsuppressed* `final_expr`
/// through `scilab-runtime`, and pull the value out of its `name = value`
/// (or `ans = value`) echo -- see this file's module doc comment's "Why
/// setup + final_expr" section, point 1, for why this is the only working
/// display convention on the ground-truth side. Mirrors `matlab-to-
/// semantic-ir/tests/oracle.rs`'s own `ground_truth` exactly.
fn ground_truth(setup: &str, final_expr: &str) -> String {
    let src = format!("{setup}{final_expr}\n");
    let echo =
        scilab_eval(&src).unwrap_or_else(|e| panic!("scilab-runtime eval failed for {src:?}: {e}"));
    echo.rsplit('=').next().unwrap_or(&echo).trim().to_string()
}

/// Compiled path: run `setup` followed by `disp(final_expr);` -- SIR's only
/// observable-output primitive (module doc point 2) -- through
/// `scilab_to_semantic_ir::compile_source`, `semantic_ir::validate`,
/// `semantic_ir_to_javascript::compile`, and an actual `node` process.
/// Mirrors `compiled` in `matlab-to-semantic-ir/tests/oracle.rs`, including
/// the `OpenOptions::create_new(true)` temp-file handling (that file's doc
/// comment: `create_new` fails instead of following an existing symlink
/// planted at the shared, predictable system temp path -- a deliberate
/// security mitigation, kept exactly as-is here).
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
    path.push(format!("scilab_sir_oracle_{name}_{}.js", std::process::id()));
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
/// through unchanged. Identical to `matlab-to-semantic-ir`'s/`octave-to-
/// semantic-ir`'s own `normalize`.
fn normalize(s: &str) -> String {
    match s {
        "#t" | "true" => "1".to_string(),
        "#f" | "false" => "0".to_string(),
        other => other.to_string(),
    }
}

#[test]
fn oracle_corpus_matches_native_scilab_runtime() {
    if !node_available() {
        eprintln!("skipping oracle_corpus_matches_native_scilab_runtime: `node` not available");
        return;
    }

    // Collect every mismatch rather than failing on the first one, so a
    // single run reports the full picture -- mirrors every sibling oracle
    // file's own "report every mismatch" convention (see this task's own
    // brief).
    let mut failures: Vec<String> = Vec::new();

    for case in CORPUS {
        let gt = normalize(&ground_truth(case.setup, case.final_expr));
        if gt != case.expected {
            failures.push(format!(
                "{}: scilab-runtime itself disagrees with this corpus entry's own `expected` \
                 (got {gt:?}, expected {:?}) -- the program or `expected` is wrong, fix the \
                 corpus rather than this assertion",
                case.name, case.expected
            ));
            continue;
        }

        match case.known_bug {
            None => {
                let got = normalize(&compiled(case.name, case.setup, case.final_expr));
                if got != case.expected {
                    failures.push(format!(
                        "{}: scilab-to-semantic-ir -> semantic-ir-to-javascript -> node disagrees \
                         with the scilab-runtime ground truth (got {got:?}, expected {:?}) -- see \
                         this file's module doc for the six documented, already-excluded findings \
                         before assuming this is a new one",
                        case.name, case.expected
                    ));
                }
            }
            Some(reason) => {
                // KNOWN BUG: the compiled-side assertion is deliberately
                // skipped (not even invoked) for this entry -- see this
                // file's module doc comment for why, and `reason` for
                // exactly which documented finding applies here. Mirrors
                // `maple-to-semantic-ir/tests/oracle.rs`'s identical
                // known_bug handling.
                eprintln!(
                    "{}: skipping compiled-side assertion (KNOWN BUG, not fixed in this PR): {reason}",
                    case.name
                );
            }
        }
    }

    assert!(
        failures.is_empty(),
        "oracle corpus mismatches ({} of {}):\n{}",
        failures.len(),
        CORPUS.len(),
        failures.join("\n")
    );
}
