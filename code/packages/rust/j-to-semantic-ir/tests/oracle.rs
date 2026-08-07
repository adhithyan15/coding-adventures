//! Oracle/golden tests (HML01 §7): the SAME J source, run through **two
//! independent implementations**, and diffed:
//!
//!   (a) `j-runtime` (`coding-adventures-j-runtime`) — this frontend's own
//!       sibling crate, a tree-walking interpreter over `array-runtime` —
//!       the ground truth.
//!   (b) `j_to_semantic_ir::compile_source` → `semantic_ir::Module` →
//!       `semantic_ir_to_javascript::compile` → **an actual `node`
//!       process**.
//!
//! This is the direct J sibling of
//! [`apl-to-semantic-ir`'s own `tests/oracle.rs`](../../apl-to-semantic-ir/tests/oracle.rs)
//! (itself the sibling of `matlab-to-semantic-ir`'s/`octave-to-semantic-ir`'s)
//! — same overall shape (`node_available` skip-not-fail guard, a
//! `Case`/`CORPUS`, a `ground_truth`/`compiled` pair, one looping `#[test]`)
//! — completing HML01 §5's "J's own oracle tests remain an open follow-on
//! item" note (this PR updates that line — see the spec).
//!
//! ## Same simplifications as APL, confirmed empirically for J too
//!
//! 1. **No `setup`/`final_expr` split.** `j-runtime`'s own module doc
//!    (`eval.rs`) states plainly: "Assignment is silent; a bare
//!    (non-assignment) statement auto-prints its result — mirrors
//!    `apl-runtime`'s own real-session convention exactly", and
//!    `j-to-semantic-ir`'s own lowering wraps a bare top-level `noun_expr`
//!    in the shared `"print"` builtin unconditionally
//!    (`src/lower.rs::lower_top_level_statement`'s 1-child arm) — exactly
//!    like APL. So [`Case`] here is just `name` + `source` (one full
//!    program, identical on both sides) + `expected`, plus one addition
//!    neither APL's file nor the MATLAB/Octave template needs: `known_bug`
//!    (see below).
//! 2. **No `normalize()`.** J's 12 shared scalar atoms map onto the same
//!    `ElementwiseOpKind` comparisons APL uses, which convert straight back
//!    to a `1.0`/`0.0` float before reaching `display` on both sides (see
//!    `apl-to-semantic-ir/tests/oracle.rs`'s own "normalize()" section for
//!    the full reasoning, which transfers unchanged) — `5>3` prints `"1"`
//!    on both sides with no spelling gap to paper over.
//!
//! ## A THIRD thing this file needs that neither APL's nor MATLAB/Octave's
//! oracle files needed: `known_bug`
//!
//! Building this corpus found real, reproducible disagreements between
//! `j-runtime` and the compiled-then-`node` path that are **not** bugs in
//! this crate or in `j-runtime` (verified by root-causing each one — see
//! below) but in the SHARED `semantic-ir-to-javascript` backend crate,
//! consumed by many other frontends (MATLAB, Octave, Wolfram, Derive,
//! Reduce, Maple, Scilab, etc.). Per this task's own explicit discipline
//! (mirroring how APL's oracle file originally shipped its own three bugs
//! excluded-not-fixed, before a later, separate PR fixed them in
//! `semantic-ir-to-javascript` 0.43.0), a shared-crate bug is documented
//! here and left for a follow-up PR, not patched in this one. [`Case`]
//! therefore carries one more field, `known_bug: Option<&'static str>`:
//! `None` for every ordinary entry (both `ground_truth` and `compiled` must
//! agree with `expected`), or `Some(reason)` for an entry where
//! `ground_truth` is still asserted against `expected` (so a wrong corpus
//! value is still caught) but the `compiled`-side assertion is skipped
//! entirely (matching the task's own "temporarily comment out just that
//! one assertion" instruction, implemented as a data-driven skip rather
//! than literal commented-out code, so the reason travels with the case
//! instead of living only in a comment).
//!
//! ### Bug A: no J-specific display convention at all in
//! `semantic-ir-to-javascript` (glyph for negative numbers AND infinity)
//!
//! `emit.rs`'s `display_apl_high_minus` flag is `m.metadata.source_language
//! .as_deref() == Some("apl")` — there is no equivalent check for `"j"`
//! anywhere in that crate. Two consequences, both confirmed by hand-running
//! the generated JavaScript:
//! - A **bare/boxed scalar** negative number or `Infinity` reaches
//!   `formatSeen`'s `SIR_DISPLAY_APL_HIGH_MINUS`-gated branch, which is
//!   `false` for a J-sourced module, so it falls through to plain
//!   JavaScript stringification: `String(-5)` → `"-5"` (ASCII, not J's own
//!   leading underscore `_5`; confirmed: `-5` alone), `String(Infinity)` →
//!   `"Infinity"` (not J's own lowercase `inf`; confirmed: `%0`, monadic
//!   reciprocal of zero).
//! - A **genuine `NDArray`** (rank ≥ 1, or a rank-0 value that happens to
//!   already be boxed as one) reaches `ArrayRt.fmtNum`, which
//!   *unconditionally* renders APL's own high-minus `¯` for any negative
//!   value with **no flag check of any kind** — confirmed: `-1 2 _3`
//!   (monadic negate of a genuine rank-1 vector) prints `¯1 ¯2 3`, and
//!   `-/+\1 2 3` (reduce-of-scan, a rank-0 `NDArray` by the time it reaches
//!   `print`) prints `¯8`. Neither ASCII `-`/`Infinity` nor APL's `¯` is J's
//!   own convention (`j_runtime::value::fmt_num`: leading underscore `_`,
//!   `inf`/`_inf` for infinities — see that module's own doc comment for
//!   why J can't reuse APL's `¯` at all: ASCII-only, MA06 §4).
//!
//! This is **not** something `j-to-semantic-ir`'s own lowering can route
//! around the way [`lower_term`](../src/lower.rs)'s `Ravel`-wrap or
//! [`Lowerer::zero_base_index`](../src/lower.rs)'s `- 1` wrap do (see this
//! file's "Two genuine bugs, found and fixed HERE" section below for those)
//! — this is a pure **display**-time decision baked into
//! `semantic-ir-to-javascript`'s own inlined runtime string, with no SIR
//! node or value-level workaround available from the frontend side. Fixing
//! it needs a THIRD per-module display flag in that shared crate (e.g.
//! `SIR_DISPLAY_J_UNDERSCORE`, mirroring `SIR_DISPLAY_APL_HIGH_MINUS`'s own
//! existing pattern) plus a matching `ArrayRt.fmtNum`/`formatSeen` branch —
//! exactly the kind of shared-crate change this task's own instructions say
//! must NOT be made in this PR. Every `CORPUS` entry below whose `expected`
//! value contains a negative number or `inf` therefore carries a
//! `known_bug` note citing this section.
//!
//! ### Bug B: `semantic-ir-to-javascript` never registered J's two new
//! builtins (`tally`, `replicate`, `exp`)
//!
//! `j-to-semantic-ir`'s own `src/lower.rs`/`README.md`/`CHANGELOG.md`
//! document `#`'s monadic form as `BuiltinCall("tally", ..)`, `#`'s dyadic
//! form as `BuiltinCall("replicate", ..)`, and `^`'s monadic form as
//! `BuiltinCall("exp", ..)` — but `semantic-ir-to-javascript`'s `builtins`
//! dispatch table (the generic `__Sir.callBuiltin` fallback every
//! unrecognised name routes through) never gained entries for any of the
//! three, so all three crash **unconditionally**, for every operand:
//! `node` exits with `TypeError: unknown builtin: tally` (or `replicate`,
//! or `exp`) every time. Confirmed by hand-running the generated
//! JavaScript for `#1 2 3`, `2 0 3#1 2 3`, and `^0`. This is the exact same
//! *class* of bug as APL's own historical bug #3 (`sign`/`recip`/`ceil`/
//! `floor` never registered) — a pure omission in the shared crate, not a
//! subtler logic error — except unlike that one (fixed in
//! `semantic-ir-to-javascript` 0.43.0 before this file was written), these
//! three names are new to J and have never been registered at all. Dyadic
//! `^` (power) is unaffected — it reuses `ElementwiseOpKind::Pow`, already
//! implemented for MATLAB's `.^` (`dyadic_pow` below passes cleanly).
//!
//! ## Two genuine bugs, found AND FIXED here (in `j-to-semantic-ir`'s own
//! `src/lower.rs`, not the shared crate)
//!
//! Building this corpus also found two bugs genuinely local to this
//! crate's own lowering — both fixed directly in this PR, per this task's
//! own instructions ("if the bug is in `j-to-semantic-ir`'s OWN lowering
//! ... FIX it directly"):
//!
//! 1. **Stranded literals of 2+ numbers were never `Ravel`-wrapped**,
//!    unlike `apl-to-semantic-ir`'s own identical construct (that crate's
//!    0.1.3 fix). `semantic-ir`'s own `ArrayLit` doc comment is explicit
//!    that a 1-row literal is a genuinely rank-2 `[1, n]` value under this
//!    IR's column-major convention, not a rank-1 `[n]` vector — so a bare
//!    `Expr::ArrayLit { rows: vec![row], .. }` was the WRONG shape for a J
//!    stranded literal (`2 2`, `1 2 3`, …), and any op that validates its
//!    operand is rank ≤ 1 (dyadic `$`'s shape argument, dyadic `i.`'s
//!    haystack) rejected it outright: confirmed empirically,
//!    `2 2$1 2 3 4` (reshape whose *shape* argument, `2 2`, is a stranded
//!    literal) crashed the compiled path with `reshape: shape argument
//!    must be a scalar or vector (got rank 2)`, even though this exact
//!    program round-trips correctly through `j-runtime`. Fixed in
//!    `src/lower.rs::Lowerer::lower_term`, mirroring
//!    `apl-to-semantic-ir::Lowerer::lower_term`'s identical `Expr::Ravel`
//!    wrap exactly. `printed_matrix_two_by_two`, `shape_of_a_reshaped_
//!    matrix`, `reshape_cycles_a_shorter_source`, and
//!    `dyadic_index_of_is_zero_based_with_tally_sentinel` below all
//!    regression-guard this (each was a hard `node` crash before the fix,
//!    confirmed by re-running this exact corpus against the pre-fix
//!    lowering).
//! 2. **Monadic/dyadic `i.` reused APL's `Expr::IndexGenerator`/
//!    `Expr::IndexOf` nodes directly, silently inheriting APL's 1-based
//!    convention and `len + 1`-not-found sentinel** — genuinely wrong
//!    values for J, whose `i.` is 0-based with a plain-tally not-found
//!    sentinel (MA06 §1 bullet 3, this crate's single most
//!    safety-critical distinction from APL — see `j_runtime::builtins`'
//!    own doc comments for `index_generator`/`index_of`). Confirmed
//!    empirically: `i.5` compiled to `1 2 3 4 5` (APL's 1-based iota),
//!    not `j-runtime`'s own `0 1 2 3 4`. Unlike Bug A/B above, this one
//!    **can** be corrected entirely from this crate's own lowering, with
//!    no shared-crate change: `apl_value - 1` is an exact identity for
//!    BOTH J conventions (found: `apl_value` is the 1-based position
//!    `k + 1` for J's 0-based `k`, so `apl_value - 1 == k`; not found:
//!    `apl_value` is `len + 1`, so `apl_value - 1 == len`, exactly J's own
//!    tally sentinel) — see `src/lower.rs::Lowerer::zero_base_index`'s own
//!    doc comment for the full proof. `index_generator_is_zero_based` and
//!    `dyadic_index_of_is_zero_based_with_tally_sentinel` below regression-
//!    guard this.
//!
//! ## Corpus
//!
//! Mirrors `apl-to-semantic-ir/tests/oracle.rs`'s own breadth target: J's
//! signature right-to-left/no-precedence evaluation; a true/false
//! comparison pair; a scalar-vector broadcast; an assignment read back by a
//! later statement; a printed matrix (2-D display); reduce (`+/`) and scan
//! (`+\`) via the shared SIR22-addendum kernels; every SIR22/addendum
//! `Expr` variant this crate's own `src/lower.rs` can emit
//! (`ElementwiseOp`, `ArrayLit`+`Ravel`, `Shape`, `Reshape`,
//! `IndexGenerator`, `IndexOf`, `Ravel`, `Catenate`, `Reduce`, `Scan`,
//! `BuiltinCall` for every one of `neg`/`sign`/`recip`/`floor`/`ceil`/
//! `tally`/`replicate`/`exp`/`print`); the `@` compose conjunction; **at
//! least one hook and one fork** (J's genuinely novel trains feature, MA06
//! §3 — the single most J-specific thing to verify, absent from APL
//! entirely) — this corpus has two hooks, two verb-left forks, and one
//! leading-noun fork; and J's own leading-underscore negative-literal
//! spelling (`_5`, confirmed against `j_runtime::value::fmt_num`, distinct
//! from APL's high-minus `¯` and from a bare ASCII `-`).

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use coding_adventures_j_runtime::eval as j_eval;
use j_to_semantic_ir::compile_source;

/// Is a `node` binary on `PATH`? Mirrors `apl-to-semantic-ir/tests/
/// oracle.rs`'s own `node_available` (and every sibling oracle file's)
/// exactly: the test below skips (logs, does not fail) when it is not.
fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// One oracle corpus entry. Like `apl-to-semantic-ir`'s own `Case`,
/// `source` is the WHOLE program, byte-for-byte identical on both the
/// `ground_truth` and `compiled` sides (see this file's module doc
/// comment, point 1, for why J needs no `setup`/`final_expr` split any
/// more than APL does). `known_bug` is the one addition beyond that
/// template — see this file's own "A THIRD thing this file needs" section.
struct Case {
    name: &'static str,
    source: &'static str,
    expected: &'static str,
    /// `None`: both `ground_truth` and `compiled` must equal `expected`.
    /// `Some(reason)`: only `ground_truth` is checked against `expected`;
    /// the `compiled`-side call is skipped entirely (not even invoked),
    /// with `reason` naming which documented shared-crate bug (this file's
    /// module doc, "Bug A"/"Bug B") is responsible.
    known_bug: Option<&'static str>,
}

const CORPUS: &[Case] = &[
    // --- Base-cut breadth (mirrors apl-to-semantic-ir/tests/oracle.rs) ---
    //
    // J's signature semantics: no operator precedence, strictly
    // right-to-left. `2*3+4` is `2*(3+4) = 14`, NOT `(2*3)+4 = 10` --
    // matching `j-runtime`'s own `right_to_left_evaluation_has_no_
    // precedence` test.
    Case {
        name: "right_to_left_no_operator_precedence",
        source: "2*3+4\n",
        expected: "14",
        known_bug: None,
    },
    // A true and a false comparison -- both sides agree on the bare
    // numeric spelling (`1`/`0`), never a JS-native boolean (see this
    // file's module doc comment's "No normalize()" point).
    Case {
        name: "comparison_true",
        source: "5>3\n",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "comparison_false",
        source: "3>5\n",
        expected: "0",
        known_bug: None,
    },
    // Scalar-vector broadcast: `2*1 2 3` doubles each element. `1 2 3`
    // lowers to a genuine rank-1 vector via the `Ravel`-wrap fix (this
    // file's module doc comment, "genuine bugs found AND FIXED here" #1),
    // so this also incidentally confirms that fix from the ground-truth
    // side.
    Case {
        name: "scalar_vector_broadcast",
        source: "2*1 2 3\n",
        expected: "2 4 6",
        known_bug: None,
    },
    // Assignment is silent; a LATER bare statement reads the bound name
    // back and auto-prints.
    Case {
        name: "variable_assignment_and_later_reference",
        source: "A=.3\nA+4\n",
        expected: "7",
        known_bug: None,
    },
    // A printed MATRIX (2-D display), not just a vector line. The shape
    // argument `2 2` is itself a stranded literal -- this is the exact
    // program that crashed the compiled path with `reshape: shape
    // argument must be a scalar or vector (got rank 2)` before this PR's
    // `Ravel`-wrap fix (this file's module doc comment, bug #1).
    Case {
        name: "printed_matrix_two_by_two",
        source: "2 2$1 2 3 4\n",
        expected: "1 2\n3 4",
        known_bug: None,
    },

    // --- Reduce (+/) and scan (+\), the shared SIR22-addendum kernels ---
    Case {
        name: "reduce_add",
        source: "+/1 2 3 4\n",
        expected: "10",
        known_bug: None,
    },
    Case {
        name: "scan_running_sum",
        source: "+\\1 2 3\n",
        expected: "1 3 6",
        known_bug: None,
    },
    // Reduce-of-scan: `-/+\1 2 3` = `-/[1,3,6]` = `(1-3)-6` = `-8`. By the
    // time this reaches `print` it is a rank-0 `NDArray`, so it hits
    // `ArrayRt.fmtNum`'s unconditional high-minus branch -- see this
    // file's module doc comment, "Bug A".
    Case {
        name: "scan_then_reduce",
        source: "-/+\\1 2 3\n",
        expected: "_8",
        known_bug: Some(
            "Bug A (display convention): the reduce result is a rank-0 NDArray by the time it \
             reaches print, so semantic-ir-to-javascript's ArrayRt.fmtNum renders APL's \
             unconditional high-minus glyph (\"\u{af}8\") -- there is no J-specific display flag \
             in that shared crate at all, so neither this nor ASCII \"-8\" is available; J's own \
             convention is a leading underscore, \"_8\".",
        ),
    },

    // --- `@` compose (atop) -- both cases give a POSITIVE result, so
    // neither exercises Bug A; these are clean, both-sides-pass regression
    // cases for the compose formula itself.
    Case {
        name: "compose_monadic_double_negate",
        source: "-@-5\n",
        expected: "5",
        known_bug: None,
    },
    Case {
        name: "compose_dyadic_negate_of_difference",
        source: "3-@-4\n",
        expected: "1",
        known_bug: None,
    },

    // --- Trains: hooks and forks (MA06 §3) -- J's genuinely novel
    // feature, no APL precedent at all (this file's module doc comment).
    //
    // Hook, monadic: `(+ *) y = y + (*y)`. For y=5: 5 + sign(5) = 6.
    Case {
        name: "hook_monadic",
        source: "(+*)5\n",
        expected: "6",
        known_bug: None,
    },
    // Hook, dyadic: `x (+ *) y = x + (*y)` -- `*` (sign) applies
    // monadically to y alone. For x=3, y=5: 3 + sign(5) = 4.
    Case {
        name: "hook_dyadic",
        source: "3(+*)5\n",
        expected: "4",
        known_bug: None,
    },
    // Verb-left fork, monadic: `(+ * -) y = (+y) * (-y)`. For y=5:
    // 5 * (-5) = -25 -- a genuine negative RESULT (not just an
    // intermediate), so this hits Bug A on the compiled side.
    Case {
        name: "fork_monadic_verb_left",
        source: "(+*-)5\n",
        expected: "_25",
        known_bug: Some(
            "Bug A (display convention): (+*-)5 = 5 * (-5) = -25, a genuine negative scalar -- \
             see the corpus-wide note at the top of this file. The train-folding itself is \
             correct (the VALUE -25 is right); only the printed glyph is wrong (\"\u{af}25\", not \"_25\").",
        ),
    },
    // Verb-left fork, dyadic: `x (+ * -) y = (x+y) * (x-y)`. For x=3, y=5:
    // 8 * (-2) = -16 -- same Bug A.
    Case {
        name: "fork_dyadic_verb_left",
        source: "3(+*-)5\n",
        expected: "_16",
        known_bug: Some(
            "Bug A (display convention): 3(+*-)5 = (3+5) * (3-5) = 8 * -2 = -16, a genuine \
             negative scalar -- see the corpus-wide note at the top of this file.",
        ),
    },
    // Leading-noun fork, monadic: `(5 + -) y = 5 + (-y)`. For y=3:
    // 5 + (-3) = 2 -- POSITIVE, so this one is a clean both-sides-pass
    // regression case for the leading-noun-fork formula itself, free of
    // Bug A.
    Case {
        name: "leading_noun_fork_monadic",
        source: "(5 + -)3\n",
        expected: "2",
        known_bug: None,
    },

    // --- J's own leading-underscore negative-literal spelling ---
    //
    // `_5` alone is a single NUMBER token (leading underscore in the
    // mantissa -- j.tokens SECTION 4), not monadic `-` applied to `5` --
    // confirmed against `j_runtime::builtins`'s own parse convention.
    // Required breadth item: "negative-literal spelling (_5, J's leading-
    // underscore convention, unlike APL's high-minus or plain numerics)".
    Case {
        name: "negative_literal_spelling",
        source: "_5\n",
        expected: "_5",
        known_bug: Some(
            "Bug A (display convention): a bare IntLit(-5) reaches formatSeen's bare-number \
             branch, gated by SIR_DISPLAY_APL_HIGH_MINUS which is false for a J-sourced module \
             (that flag only ever checks source_language == \"apl\"), so it falls through to \
             plain JS stringification (\"-5\") instead of J's own leading-underscore \"_5\".",
        ),
    },

    // --- Monadic scalar atoms (`- * % <. >.`) -- mirrors
    // apl-to-semantic-ir/tests/oracle.rs's identical breadth, minus the
    // "Three genuine bugs" story (those were already fixed in
    // semantic-ir-to-javascript 0.43.0 before this crate's own lowering
    // was written, so neg/sign/recip/ceil/floor all compute the correct
    // VALUE here -- only the display GLYPH is wrong for J, per Bug A,
    // whenever the value is actually negative).
    Case {
        name: "monadic_negate_scalar",
        source: "-5\n",
        expected: "_5",
        known_bug: Some("Bug A (display convention) -- same root cause as negative_literal_spelling above."),
    },
    // `1 2 _3` is a genuine rank-1 vector (stranded literal, Ravel-wrapped
    // per this PR's fix #1) -- negating it now correctly computes
    // `_1 _2 3` (values right, per apl-to-semantic-ir's own historical
    // bug #2 fix in semantic-ir-to-javascript 0.43.0), but the compiled
    // side still renders APL's high-minus glyph unconditionally (Bug A).
    Case {
        name: "monadic_negate_array",
        source: "-1 2 _3\n",
        expected: "_1 _2 3",
        known_bug: Some(
            "Bug A (display convention): the negated array is a genuine rank-1 NDArray, so \
             ArrayRt.fmtNum renders APL's unconditional high-minus glyph (\"\u{af}1 \u{af}2 3\") \
             for the negative elements, not J's own leading underscore.",
        ),
    },
    Case {
        name: "monadic_sign_positive",
        source: "*5\n",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "monadic_sign_zero",
        source: "*0\n",
        expected: "0",
        known_bug: None,
    },
    Case {
        name: "monadic_sign_negative",
        source: "*_5\n",
        expected: "_1",
        known_bug: Some("Bug A (display convention) -- same root cause as monadic_negate_scalar above."),
    },
    Case {
        name: "monadic_reciprocal",
        source: "%4\n",
        expected: "0.25",
        known_bug: None,
    },
    // `j_runtime::builtins`'s own doc comment calls this edge case out
    // explicitly: `recip(0) == Infinity`, never an error. `j-runtime`
    // renders it as J's own lowercase `inf`; the compiled side reaches the
    // SAME Bug A branch as a negative number does (formatSeen's
    // SIR_DISPLAY_APL_HIGH_MINUS-gated bare-number path also decides the
    // infinity spelling), so it prints JavaScript's native `"Infinity"`
    // instead.
    Case {
        name: "monadic_reciprocal_zero",
        source: "%0\n",
        expected: "inf",
        known_bug: Some(
            "Bug A (display convention): reciprocal of 0 is a bare, non-finite JS number; \
             formatSeen's SIR_DISPLAY_APL_HIGH_MINUS-gated branch (false for J) falls through to \
             plain JS stringification (\"Infinity\") instead of J's own lowercase \"inf\" -- the \
             SAME missing-J-flag root cause as the negative-number cases above, just the \
             infinity-spelling half of it rather than the sign-glyph half.",
        ),
    },
    Case {
        name: "monadic_ceiling",
        source: ">.3.2\n",
        expected: "4",
        known_bug: None,
    },
    Case {
        name: "monadic_floor",
        source: "<.3.8\n",
        expected: "3",
        known_bug: None,
    },

    // --- $ / i. / , (shared SIR22-addendum nodes with APL) ---
    //
    // Monadic `$` (Shape) of a dyadic `$` (Reshape) result -- exercises
    // BOTH nodes in one program, and (since the Reshape's own shape
    // argument, `2 3`, is a stranded literal) regression-guards this PR's
    // `Ravel`-wrap fix #1 exactly like `printed_matrix_two_by_two` above.
    Case {
        name: "shape_of_a_reshaped_matrix",
        source: "$2 3$1 2 3 4 5 6\n",
        expected: "2 3",
        known_bug: None,
    },
    // Dyadic `$` (Reshape) CYCLING a shorter source to fill a larger
    // target -- `1 2` repeats to `1 2 1 / 2 1 2`.
    Case {
        name: "reshape_cycles_a_shorter_source",
        source: "2 3$1 2\n",
        expected: "1 2 1\n2 1 2",
        known_bug: None,
    },
    // Monadic `i.` (IndexGenerator) -- 0-based, `i.5` is `0 1 2 3 4`, NEVER
    // APL's 1-based `1 2 3 4 5`. This is the exact regression case for
    // this PR's fix #2 (`Lowerer::zero_base_index`) -- confirmed to
    // compile to APL's own 1-based iota before that fix.
    Case {
        name: "index_generator_is_zero_based",
        source: "i.5\n",
        expected: "0 1 2 3 4",
        known_bug: None,
    },
    // Dyadic `i.` (IndexOf) -- 0-based positions with a plain-tally
    // not-found sentinel: search `[20, 99, 10]` in `[10, 20, 30]`. `20` is
    // at 0-based index 1; `99` is not found (tally = 3, NOT APL's
    // `len + 1` = 4); `10` is at 0-based index 0. The haystack `10 20 30`
    // is itself a stranded literal, so this ALSO regression-guards fix #1
    // (it crashed the compiled path with `indexOf: left argument must be
    // a scalar or vector (got rank 2)` before that fix).
    Case {
        name: "dyadic_index_of_is_zero_based_with_tally_sentinel",
        source: "10 20 30 i.20 99 10\n",
        expected: "1 3 0",
        known_bug: None,
    },
    // Monadic `,` (Ravel) of a reshaped matrix: flatten `[[1,2,3],[4,5,6]]`
    // row-major back to `1 2 3 4 5 6`.
    Case {
        name: "ravel_of_a_reshaped_matrix",
        source: ",(2 3$1 2 3 4 5 6)\n",
        expected: "1 2 3 4 5 6",
        known_bug: None,
    },
    // Dyadic `,` (Catenate): `1 2,3 4` end-to-end catenates two vectors.
    Case {
        name: "dyadic_catenate",
        source: "1 2,3 4\n",
        expected: "1 2 3 4",
        known_bug: None,
    },

    // --- `#` and `^` -- genuinely new primitives, no APL analogue at all
    // (this crate's own module doc comment's "Two new primitives"
    // section). All three of `tally`/`replicate`/`exp` hit Bug B (the
    // shared crate never registered these builtin names at all, so `node`
    // crashes with `TypeError: unknown builtin: <name>` for every operand
    // -- not a wrong VALUE, a hard crash).
    Case {
        name: "monadic_tally",
        source: "#1 2 3\n",
        expected: "3",
        known_bug: Some(
            "Bug B (missing builtin): semantic-ir-to-javascript's builtin dispatch table has no \
             \"tally\" entry at all -- node exits with `TypeError: unknown builtin: tally` for \
             every operand, not merely a wrong value.",
        ),
    },
    Case {
        name: "dyadic_replicate",
        source: "2 0 3#1 2 3\n",
        expected: "1 1 3 3 3",
        known_bug: Some(
            "Bug B (missing builtin): semantic-ir-to-javascript's builtin dispatch table has no \
             \"replicate\" entry at all -- node exits with `TypeError: unknown builtin: \
             replicate` for every operand.",
        ),
    },
    Case {
        name: "monadic_exp",
        source: "^0\n",
        expected: "1",
        known_bug: Some(
            "Bug B (missing builtin): semantic-ir-to-javascript's builtin dispatch table has no \
             \"exp\" entry at all -- node exits with `TypeError: unknown builtin: exp` for every \
             operand.",
        ),
    },
    // Dyadic `^` (Pow) is UNAFFECTED by Bug B -- it reuses
    // `ElementwiseOpKind::Pow`, already implemented in
    // semantic-ir-to-javascript for MATLAB's `.^`, so this is a clean,
    // both-sides-pass case.
    Case {
        name: "dyadic_pow",
        source: "2^3\n",
        expected: "8",
        known_bug: None,
    },
];

/// Ground truth: run `source` through `j-runtime`'s own [`j_eval`], which
/// already returns exactly the auto-print output of the program's bare
/// top-level expression(s) (see this file's module doc comment, point 1)
/// -- no echo-parsing needed, unlike the MATLAB/Octave oracle files' `name
/// = value` convention.
fn ground_truth(source: &str) -> String {
    j_eval(source)
        .unwrap_or_else(|e| panic!("j-runtime eval failed for {source:?}: {e}"))
        .trim()
        .to_string()
}

/// Compiled path: run `source` (unchanged) through
/// `j_to_semantic_ir::compile_source`, `semantic_ir::validate`,
/// `semantic_ir_to_javascript::compile`, and an actual `node` process.
/// Mirrors `apl-to-semantic-ir/tests/oracle.rs`'s own `compiled` exactly,
/// down to the `OpenOptions::create_new(true)` temp-file handling (that
/// file's own doc comment explains why: `create_new` fails instead of
/// silently following an existing symlink planted at the shared,
/// predictable system temp path).
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
    path.push(format!("j_sir_oracle_{name}_{}.js", std::process::id()));
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
fn oracle_corpus_matches_native_j_runtime() {
    if !node_available() {
        eprintln!("skipping oracle_corpus_matches_native_j_runtime: `node` not available");
        return;
    }
    for case in CORPUS {
        let gt = ground_truth(case.source);
        assert_eq!(
            gt, case.expected,
            "{}: j-runtime itself disagrees with this corpus entry's own `expected` -- \
             the program or `expected` is wrong, fix the corpus rather than this assertion",
            case.name
        );

        match case.known_bug {
            None => {
                let got = compiled(case.name, case.source);
                assert_eq!(
                    got, case.expected,
                    "{}: j-to-semantic-ir -> semantic-ir-to-javascript -> node disagrees with \
                     the j-runtime ground truth ({gt:?}) -- see this file's module doc for the \
                     two documented, already-excluded shared-crate bug classes (Bug A: display \
                     convention; Bug B: missing tally/replicate/exp builtins) before assuming \
                     this is a new one",
                    case.name
                );
            }
            Some(reason) => {
                // KNOWN BUG: the compiled-side assertion is deliberately
                // skipped (not even invoked) for this entry -- see this
                // file's module doc comment's "A THIRD thing this file
                // needs" section for why, and `reason` for exactly which
                // documented shared-crate bug applies here.
                eprintln!(
                    "{}: skipping compiled-side assertion (KNOWN BUG, not fixed in this PR): {reason}",
                    case.name
                );
            }
        }
    }
}
