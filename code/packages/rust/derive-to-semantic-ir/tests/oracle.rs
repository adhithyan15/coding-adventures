//! Oracle/golden tests (HML01 §7): the SAME Derive source, run through
//! **two independent implementations**, and diffed:
//!
//!   (a) `derive-runtime` (`coding-adventures-derive-runtime`) — this
//!       frontend's own sibling crate, which lowers to `symbolic-ir` and
//!       evaluates via `symbolic-vm`'s `SymbolicBackend` — the ground
//!       truth.
//!   (b) `derive_to_semantic_ir::compile_source` → `semantic_ir::Module` →
//!       `semantic_ir_to_javascript::compile` → **an actual `node`**
//!       process.
//!
//! This is the direct Derive sibling of
//! [`j-to-semantic-ir`'s own `tests/oracle.rs`](../../j-to-semantic-ir/tests/oracle.rs)
//! (itself the sibling of `apl-to-semantic-ir`'s/`matlab-to-semantic-ir`'s/
//! `octave-to-semantic-ir`'s) — same overall shape (`node_available`
//! skip-not-fail guard, a `Case`/`CORPUS`, a `ground_truth`/`compiled`
//! pair, one looping `#[test]`, a `known_bug` field for a disagreement
//! rooted in the SHARED `semantic-ir-to-javascript` crate rather than this
//! frontend's own lowering) — completing HML01 §5's "a true oracle diff
//! [for Stream B] remains the one open item" note for `derive-to-semantic-ir`
//! specifically (this PR updates that line — see the spec; the other four
//! Stream B frontends, Wolfram/Macsyma/Reduce/Maple, are unaffected by this
//! PR and still have no oracle file of their own).
//!
//! ## A harness-only "make it observable" step neither J's nor APL's
//! oracle file needed: wrapping every top-level statement in `print`
//!
//! J's/APL's own lowering already wraps a bare top-level expression in the
//! shared `"print"` builtin *unconditionally*, as a pre-existing part of
//! those crates' own shipped lowering (see `j-to-semantic-ir/tests/
//! oracle.rs`'s module doc, point 1) — so their compiled JS already prints
//! something on its own, and their oracle files needed no extra plumbing
//! to make a value observable.
//!
//! `derive-to-semantic-ir`'s own lowering does **not** do this — and, per
//! `tests/e2e_node.rs`'s own module doc comment, this is a *documented,
//! deliberate* design point, not an oversight: "a Derive program never
//! produces a host-language computed value through this pipeline, so
//! there is no `disp`-equivalent stdout to assert on." Confirmed directly:
//! `compile_source("1 + 2\n", ..)`'s `main()` body is a single bare
//! `__Sir.Symbolic.apply(...)` expression-statement with no `print`/
//! `console.log` anywhere in the emitted module — the built term is
//! computed and immediately discarded, exactly like any other
//! side-effect-free JS expression statement.
//!
//! An oracle test, by its very nature, needs an observable value on BOTH
//! sides to diff — so [`wrap_top_level_in_print`] below performs a small,
//! test-local transformation: after `compile_source` (and after
//! `semantic_ir::validate`, so what gets validated is exactly what
//! shipped, unmodified) it walks the already-built [`semantic_ir::Module`]
//! and wraps each top-level `Stmt::ExprStmt`'s `expr` in
//! `Expr::BuiltinCall("print", [expr])` — mirroring the *shape*
//! `j-to-semantic-ir::lower::lower_top_level_statement`'s 1-child arm
//! already produces, using only `semantic_ir`'s own public `Module`/
//! `Stmt`/`Expr` types (the same types `tests/test_lower.rs` already
//! constructs directly). This is deliberately done **here, in the test
//! harness**, not in `src/lower.rs`: it does not change
//! `derive_to_semantic_ir::compile_source`'s actual shipped behavior for
//! any other caller (still print-free, exactly as documented), it needs
//! no update to `tests/test_lower.rs`'s ~40 existing shape-assertion
//! tests (which all assert the RAW, unwrapped `Expr` shape), and Derive's
//! own "every statement always displays, no suppression syntax at all"
//! convention (`derive-runtime::DeriveSession::feed`'s own doc comment) is
//! a property of the *language*, applied uniformly here to every
//! statement — unlike J, which only wraps a bare *non-assignment*
//! statement (J's own assignment is silent).
//!
//! ## The dominant finding: the SIR23 JS backend does not evaluate
//! anything at all (found, NOT fixed here — a `semantic-ir-to-javascript`
//! gap)
//!
//! Even with `print` wrapping restored, building this corpus found that
//! `semantic-ir-to-javascript`'s SIR23 codegen has **no evaluator or
//! simplifier of any kind** — confirmed by direct inspection of the
//! emitted JS for every representative shape below. `Expr::SymApply`
//! compiles, unconditionally, to `__Sir.Symbolic.apply(head, [args])` — a
//! **pure, inert term constructor** — with no arithmetic folding, no
//! comparison evaluation, no calculus, and (most consequentially) no
//! execution of the held `Assign`/`Define`/`If` forms at all: an
//! `Assign`/`Define` never actually binds anything in the compiled JS, so
//! a *later* reference to the same name in the SAME compiled program still
//! refers to the raw, never-bound symbol. Concretely, confirmed by
//! hand-compiling and running each of these through `node`:
//!
//! - `1 + 2*3` compiles to a bare `Add(1, Mul(2, 3))` term — never folds
//!   to `7`.
//! - `x := 5` / `x + 1` compiles to `Assign(x, 5)` then `Add(x, 1)` — the
//!   second statement's `x` is never substituted; it stays the raw symbol
//!   `x`, never `6`.
//! - `DIF(x^2, x)` compiles to `D(Pow(x, 2), x)` — never differentiates to
//!   `2*x`.
//! - `5 > 3` compiles to `Greater(5, 3)` — never evaluates to `True`.
//! - `F(x) := x*x` / `F(5)` compiles to `Define(F, List(x), Mul(x, x))`
//!   then a bare `F(5)` call term — never dispatches to `25`.
//!
//! This is a **shared-crate** gap (`semantic-ir-to-javascript` itself),
//! not a bug in this frontend's own lowering — `derive_to_semantic_ir`
//! correctly emits the canonical `SymApply`/`SymSymbol` shapes MA07 §3
//! calls for (confirmed independently by `tests/test_lower.rs`'s ~40
//! passing shape assertions); the shared JS backend simply has no
//! evaluation semantics wired up for the SIR23 domain at all yet (its
//! `SymReplaceAll`/`SymReplaceRepeated` arms DO implement real
//! pattern-rewriting — see `runtime.rs`'s `Symbolic.replaceAll`/
//! `replaceRepeated` — but Derive's grammar has no rewrite-rule syntax to
//! ever emit those nodes at all, per MA07 §4, so this crate can never
//! reach that machinery either). Per this task's own explicit scope
//! discipline (a shared-crate bug is documented via `known_bug`, not
//! patched in this frontend's PR), every `CORPUS` entry below whose
//! ground-truth result requires ANY evaluation beyond "the source text
//! already denotes one atom" is marked accordingly. This is also, in
//! effect, the reason `wolfram-to-semantic-ir` and `macsyma-to-semantic-ir`
//! — Stream B's other two shipped frontends — have no `tests/oracle.rs` of
//! their own yet either: the same gap blocks a meaningful value-diff for
//! any symbolic-domain frontend, not just Derive.
//!
//! ## A second, narrower finding layered on top: no per-language SIR23
//! display convention either (also found, NOT fixed here)
//!
//! Independent of the evaluation gap above: even a term that WAS already
//! fully reduced would still print wrong for Derive. `semantic-ir-to-
//! javascript`'s only SIR23 stringifier, `Symbolic.toDisplayString`
//! (`runtime.rs`), renders **every** compound term generically as
//! `head(args, ...)` — e.g. an unevaluated `Add(x, 1)` prints `"Add(x,
//! 1)"`, a `List(1, 2, 3)` prints `"List(1, 2, 3)"`, a `Neg(x)` prints
//! `"Neg(x)"` — with no infix `+`/`*`/`^` convention, no `[...]`/`[...;...]`
//! bracket convention for `List`, no prefix `-`/`NOT` convention, and no
//! case-bridging back to Derive's own UPPERCASE builtin surface spelling
//! (`derive-runtime::printer::print_derive` reverses ALL of these:
//! `Add(x,1)` → `"x + 1"`, `List(1,2,3)` → `"[1, 2, 3]"`, `Neg(x)` →
//! `"-x"`, an unresolved `Cos(x)` → `"COS(x)"`). Unlike the SIR22 array
//! domain's `ArrayRt.fmtNum`/`display` (which already has per-language
//! flags — `SIR_DISPLAY_APL_HIGH_MINUS`, `SIR_DISPLAY_J_UNDERSCORE`), the
//! SIR23 domain has no such mechanism for ANY source language yet. Also a
//! shared-crate gap, also `known_bug`, cited alongside the evaluation gap
//! above wherever a `CORPUS` entry's compiled output would still disagree
//! even under a hypothetical fix to the first gap.
//!
//! ## Corpus
//!
//! Mirrors `j-to-semantic-ir/tests/oracle.rs`'s own breadth target,
//! adapted to MA07 §3's actual surface: ordinary (non-J/APL) operator
//! precedence and right-associative `^`; unary minus binding looser than
//! `^`; exact integer division vs. a genuine rational result; assignment
//! read back by a later statement; single- and multi-parameter
//! user-defined function definition/call; `DIF`/`INT` via the shared
//! calculus handlers (including the "differentiate, then call at a point"
//! worksheet idiom `derive-runtime`'s own test suite uses); `IF`'s two
//! branches; every comparison/logic keyword (`= <= < > >= AND OR NOT`,
//! including a 3-term `AND` chain exercising the n-ary logical-chain
//! fold); vectors and matrices as D-5 structural `List` data (flat,
//! singleton, elementwise-evaluated, 2×2, and 3-row/1-column shapes); a
//! free-symbol additive-identity simplification; and bare integer/float/
//! symbol atoms (this subset's only `known_bug: None` cases, per the
//! evaluation-gap finding above).

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use coding_adventures_derive_runtime::eval as derive_eval;
use derive_to_semantic_ir::compile_source;
use semantic_ir::{EffectSet, Expr, Module, Stmt};

/// Is a `node` binary on `PATH`? Mirrors `j-to-semantic-ir/tests/
/// oracle.rs`'s own `node_available` (and every sibling oracle file's)
/// exactly: the test below skips (logs, does not fail) when it is not.
fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// One oracle corpus entry. Like `j-to-semantic-ir`'s own `Case`, `source`
/// is the WHOLE program, byte-for-byte identical on both the
/// `ground_truth` and `compiled` sides.
struct Case {
    name: &'static str,
    source: &'static str,
    expected: &'static str,
    /// `None`: both `ground_truth` and `compiled` must equal `expected`.
    /// `Some(reason)`: only `ground_truth` is checked against `expected`;
    /// the `compiled`-side call is skipped entirely (not even invoked),
    /// with `reason` naming which documented shared-crate gap (this
    /// file's module doc — the evaluation gap, the display-convention
    /// gap, or both) is responsible.
    known_bug: Option<&'static str>,
}

const CORPUS: &[Case] = &[
    // --- Bare atoms: the ONLY `known_bug: None` cases (see module doc's
    // "dominant finding" -- these need no evaluation at all, since the
    // source already denotes exactly one value). ---
    Case {
        name: "bare_integer_literal",
        source: "42\n",
        expected: "42",
        known_bug: None,
    },
    Case {
        name: "bare_float_literal",
        source: "1.5\n",
        expected: "1.5",
        known_bug: None,
    },
    // A whole-valued float still prints its trailing `.0` on BOTH sides:
    // `derive-runtime::printer`'s `{v:?}` (Rust `Debug`) and the compiled
    // side's boxed-Float `floatToRubyString` (Ruby/Lisp convention) agree
    // here, unlike a bare unboxed JS number (`String(4.0) === "4"`).
    Case {
        name: "bare_whole_valued_float_keeps_trailing_dot_zero",
        source: "4.0\n",
        expected: "4.0",
        known_bug: None,
    },
    Case {
        name: "bare_free_symbol",
        source: "foo\n",
        expected: "foo",
        known_bug: None,
    },

    // --- Arithmetic: ordinary precedence (NOT J/APL's right-to-left),
    // right-associative `^`, unary minus binding looser than `^`. ---
    Case {
        name: "multiplication_binds_tighter_than_addition",
        source: "2*3+4\n",
        expected: "10",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare Add(Mul(2, 3), 4) term, never folds \
             to 10 -- semantic-ir-to-javascript's SIR23 codegen never evaluates a SymApply, it only \
             constructs one.",
        ),
    },
    Case {
        name: "parens_override_precedence",
        source: "(2 + 3) * 4\n",
        expected: "20",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare Mul(Add(2, 3), 4) term, never folds \
             to 20.",
        ),
    },
    Case {
        name: "power_is_right_associative",
        source: "2^3^2\n",
        expected: "512",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare Pow(2, Pow(3, 2)) term (the shape IS \
             right-associative, matching tests/test_lower.rs's power_is_right_associative -- only \
             the numeric fold to 512 is missing), never evaluates.",
        ),
    },
    // `-2^2` -> `Neg(Pow(2, 2))` (unary minus binds LOOSER than `^`,
    // mirrors tests/test_lower.rs's unary_minus_binds_looser_than_power)
    // = Neg(4) = -4, not (-2)^2 = 4.
    Case {
        name: "unary_minus_binds_looser_than_power",
        source: "-2^2\n",
        expected: "-4",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare Neg(Pow(2, 2)) term (the shape IS \
             correct -- unary minus binds looser than Pow -- only the fold to -4 is missing); even \
             folded, the display-convention gap (module doc) would print \"Neg(4)\", not \"-4\".",
        ),
    },
    Case {
        name: "exact_integer_division_folds_to_an_integer",
        source: "10 / 2\n",
        expected: "5",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare Div(10, 2) term, never folds to the \
             integer 5.",
        ),
    },
    Case {
        name: "inexact_division_folds_to_a_rational",
        source: "1 / 3\n",
        expected: "1/3",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare Div(1, 3) term, never folds to the \
             rational 1/3.",
        ),
    },
    // `x + 0` -> the additive-identity simplification (`x`), confirming a
    // free symbol stays symbolic through the shared handler on the ground
    // -truth side (derive-runtime::tests::free_symbols_stay_symbolic).
    Case {
        name: "additive_identity_simplifies_a_free_symbol",
        source: "x + 0\n",
        expected: "x",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare Add(x, 0) term; the additive-identity \
             simplification (x + 0 -> x) never runs, so it never reduces to the bare symbol x.",
        ),
    },
    Case {
        name: "negative_integer_literal",
        source: "-5\n",
        expected: "-5",
        known_bug: Some(
            "Evaluation gap (module doc): -5 lowers to Neg(5) on BOTH sides (derive-runtime's own \
             lower_unary is identically unconditional, per that crate's src/lower.rs) -- the ground \
             truth folds Neg(5) -> -5 at EVAL time; the compiled side never evaluates, so it stays \
             Neg(5), and even folded the display-convention gap would print \"Neg(5)\" verbatim, not \
             the surface \"-5\".",
        ),
    },
    Case {
        name: "negation_of_a_free_symbol",
        source: "-x\n",
        expected: "-x",
        known_bug: Some(
            "Evaluation/display gap (module doc): compiles to a bare Neg(x) term; there is nothing \
             to fold (x is free), but the display-convention gap alone means this prints \"Neg(x)\", \
             never the prefix surface \"-x\".",
        ),
    },

    // --- Assignment: read back by a LATER statement (mirrors
    // j-to-semantic-ir's own variable_assignment_and_later_reference,
    // adapted to Derive's own "every statement displays" convention --
    // MA07 §5 -- unlike J's silent assignment). ---
    Case {
        name: "variable_assignment_and_later_reference",
        source: "x := 5\nx + 1\n",
        expected: "5\n6",
        known_bug: Some(
            "Evaluation gap (module doc): Assign is a HELD form on the ground-truth side (binds x, \
             displays its value, 5); the compiled side's Assign(x, 5) is inert data -- no binding \
             ever happens -- so the SECOND statement's x is never substituted: it compiles to a bare \
             Add(x, 1) term, still referencing the raw, unbound symbol x, never 6.",
        ),
    },
    Case {
        name: "vector_assignment_persists_across_statements",
        source: "v := [1, 2, 3]\nv\n",
        expected: "[1, 2, 3]\n[1, 2, 3]",
        known_bug: Some(
            "Evaluation gap (module doc): the compiled side's Assign never binds v, so the second \
             statement compiles to the bare, still-unbound symbol v, not the list; and even a bound \
             List(1,2,3) would print via the display-convention gap as \"List(1, 2, 3)\", not \
             \"[1, 2, 3]\".",
        ),
    },

    // --- User-defined functions: definition echoes the bare name (MA07
    // §5's own worksheet convention -- confirmed against derive-runtime's
    // own user_defined_function_end_to_end test), then a call dispatches. ---
    Case {
        name: "single_param_function_definition_and_call",
        source: "F(x) := x*x\nF(5)\n",
        expected: "F\n25",
        known_bug: Some(
            "Evaluation gap (module doc): Define is a HELD form on the ground-truth side (registers \
             F, displays its bare name, \"F\"); the compiled side's Define(...) is inert data -- no \
             registration ever happens -- so the SECOND statement compiles to a bare, never-\
             dispatched F(5) call term, not 25.",
        ),
    },
    Case {
        name: "multi_param_function_definition_and_call",
        source: "G(a, b) := a + b\nG(3, 4)\n",
        expected: "G\n7",
        known_bug: Some(
            "Evaluation gap (module doc): same root cause as single_param_function_definition_and_\
             call -- Define never registers G, so G(3, 4) never dispatches to 7.",
        ),
    },

    // --- DIF / INT: the shared calculus handlers (D-4, MA07 §2/§5). ---
    Case {
        name: "dif_differentiates_a_power",
        source: "DIF(x^2, x)\n",
        expected: "2*x",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare D(Pow(x, 2), x) term -- confirmed by \
             direct inspection of the emitted JS -- differentiation never runs, so it never reduces \
             to 2*x; even if it did, the display-convention gap would print \"Mul(2, x)\", not \
             \"2*x\".",
        ),
    },
    Case {
        name: "dif_of_sin_gives_cos",
        source: "DIF(SIN(x), x)\n",
        expected: "COS(x)",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare D(Sin(x), x) term, never differentiates \
             to Cos(x); even folded, the display-convention gap has no case-bridge back to Derive's \
             UPPERCASE surface convention, so it would print \"Cos(x)\", not \"COS(x)\".",
        ),
    },
    // Mirrors derive-runtime's own `a_small_derive_program_evaluates_end_
    // to_end` test exactly: a function whose body differentiates a
    // DIFFERENT free variable (t) than its own formal parameter (y), so
    // substituting the call argument at y never touches t, and DIF(SIN(t),
    // t) evaluates via the shared handler regardless of the call argument.
    Case {
        name: "a_worksheet_program_defines_then_differentiates",
        source: "H(y) := DIF(SIN(t), t)\nH(0)\n",
        expected: "H\nCOS(t)",
        known_bug: Some(
            "Evaluation gap (module doc): Define never registers H, so H(0) never dispatches and \
             never differentiates -- it compiles to a bare, never-called H(0) term.",
        ),
    },
    Case {
        name: "int_integrates_a_symbol",
        source: "INT(x, x)\n",
        expected: "1/2*x^2",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare Integrate(x, x) term, never integrates \
             to 1/2*x^2.",
        ),
    },

    // --- IF: both branches (the held handler, MA07 §5). ---
    Case {
        name: "if_true_branch",
        source: "IF(1 > 0, 42, 0)\n",
        expected: "42",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare If(Greater(1, 0), 42, 0) term -- IF is \
             a HELD form on the ground-truth side (evaluates the condition, then selects a branch); \
             the compiled side never evaluates the condition or selects anything.",
        ),
    },
    Case {
        name: "if_false_branch",
        source: "IF(1 > 2, 42, 99)\n",
        expected: "99",
        known_bug: Some(
            "Evaluation gap (module doc): same root cause as if_true_branch -- compiles to a bare \
             If(Greater(1, 2), 42, 99) term, never selects a branch.",
        ),
    },

    // --- Comparisons: fold to the symbol True/False on the ground-truth
    // side (confirmed empirically -- NOT a JS boolean, NOT 1/0). ---
    Case {
        name: "comparison_true",
        source: "5 > 3\n",
        expected: "True",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare Greater(5, 3) term, never evaluates to \
             the symbol True.",
        ),
    },
    Case {
        name: "comparison_false",
        source: "3 > 5\n",
        expected: "False",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare Greater(3, 5) term, never evaluates to \
             the symbol False.",
        ),
    },
    Case {
        name: "less_equal_boundary_is_true",
        source: "3 <= 3\n",
        expected: "True",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare LessEqual(3, 3) term, never evaluates.",
        ),
    },
    // `=` is Derive's EQUATION operator (never assignment) and stays
    // symbolic when either side is a free variable -- both sides agree on
    // THIS shape already (nothing to evaluate away), but the compiled
    // side's printed NOTATION still disagrees (display-convention gap).
    Case {
        name: "equation_with_a_free_variable_stays_symbolic",
        source: "x = 4\n",
        expected: "x = 4",
        known_bug: Some(
            "Display-convention gap only (module doc) -- this one needs no evaluation at all (x is \
             free, so Equal(x, 4) is already the fully-reduced ground-truth value); the compiled \
             side's generic, non-infix Symbolic.toDisplayString prints \"Equal(x, 4)\", never \
             Derive's own infix \"x = 4\".",
        ),
    },

    // --- Logic keywords: AND / OR / NOT, including a 3-term AND chain
    // (the n-ary logical_chain fold, mirrors tests/test_lower.rs's
    // logical_or_chain_folds_n_ary_not_nested_binary). ---
    Case {
        name: "and_or_short_circuit_to_true",
        source: "5 > 3 AND 2 < 4\n",
        expected: "True",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare And(Greater(5, 3), Less(2, 4)) term, \
             never evaluates to True.",
        ),
    },
    Case {
        name: "three_term_and_chain_folds_n_ary",
        source: "5 > 3 AND 3 > 1 AND 1 > 0\n",
        expected: "True",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare, flat 3-operand And(Greater(5,3), \
             Greater(3,1), Greater(1,0)) term (the n-ary fold shape IS correct, per tests/test_\
             lower.rs's identical logical_or_chain_folds_n_ary_not_nested_binary check for OR) -- \
             only the evaluation to True is missing.",
        ),
    },
    Case {
        name: "not_negates_a_true_comparison",
        source: "NOT (5 > 3)\n",
        expected: "False",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare Not(Greater(5, 3)) term, never \
             evaluates to False.",
        ),
    },

    // --- SIN / COS / SQRT of a literal argument (the base elementary-
    // function handlers, D-4). ---
    Case {
        name: "sin_of_zero",
        source: "SIN(0)\n",
        expected: "0",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare Sin(0) call term, never evaluates to 0.",
        ),
    },
    Case {
        name: "cos_of_zero",
        source: "COS(0)\n",
        expected: "1",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare Cos(0) call term, never evaluates to 1.",
        ),
    },
    Case {
        name: "sqrt_of_a_perfect_square",
        source: "SQRT(4)\n",
        expected: "2",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare Sqrt(4) call term, never evaluates to \
             2.",
        ),
    },

    // --- Vectors / matrices: D-5 structural List data (MA07 §2/§3). ---
    Case {
        name: "flat_vector_literal",
        source: "[1, 2, 3]\n",
        expected: "[1, 2, 3]",
        known_bug: Some(
            "Display-convention gap (module doc): a flat List(1, 2, 3) needs no evaluation at all \
             (every element is already a literal), but the compiled side's generic \
             Symbolic.toDisplayString prints \"List(1, 2, 3)\", never Derive's own bracket surface \
             \"[1, 2, 3]\".",
        ),
    },
    Case {
        name: "singleton_vector_literal",
        source: "[5]\n",
        expected: "[5]",
        known_bug: Some(
            "Display-convention gap only (module doc): List(5) needs no evaluation; prints \
             \"List(5)\", not \"[5]\".",
        ),
    },
    Case {
        name: "vector_of_expressions_evaluates_elementwise",
        source: "[1+1, 2*3, 2^3]\n",
        expected: "[2, 6, 8]",
        known_bug: Some(
            "Evaluation gap (module doc): each element is itself an unfolded Add/Mul/Pow term inside \
             the List, so this compiles to a bare List(Add(1,1), Mul(2,3), Pow(2,3)) term -- \
             confirmed by direct inspection of the emitted JS -- never [2, 6, 8].",
        ),
    },
    Case {
        name: "two_by_two_matrix_literal",
        source: "[1, 2; 3, 4]\n",
        expected: "[1, 2; 3, 4]",
        known_bug: Some(
            "Display-convention gap only (module doc): List(List(1,2), List(3,4)) needs no \
             evaluation (a matrix of literals), but prints \"List(List(1, 2), List(3, 4))\", never \
             the \";\"-separated row surface \"[1, 2; 3, 4]\".",
        ),
    },
    Case {
        name: "three_row_one_column_matrix_literal",
        source: "[1; 2; 3]\n",
        expected: "[1; 2; 3]",
        known_bug: Some(
            "Display-convention gap only (module doc): a three-row matrix of literals needs no \
             evaluation, but prints generically, not with Derive's own \";\"-separated row surface.",
        ),
    },
];

/// Ground truth: run `source` through `derive-runtime`'s own
/// [`derive_eval`], then strip its `#n: ` worksheet-numbering prefix from
/// every line (MA07 §5's own numbered-history convention — Derive, unlike
/// J, has no statement-suppression syntax at all, so EVERY statement gets
/// one of these lines, including `Assign`/`Define`). The compiled side
/// (via [`wrap_top_level_in_print`]) prints one bare, unnumbered line per
/// statement through `console.log`, so stripping the prefix here is what
/// makes the two sides directly, textually comparable.
fn ground_truth(source: &str) -> String {
    let raw = derive_eval(source)
        .unwrap_or_else(|e| panic!("derive-runtime eval failed for {source:?}: {e}"));
    raw.lines()
        .map(strip_worksheet_index_prefix)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Strip a leading `#<digits>: ` worksheet-index prefix from one line of
/// [`coding_adventures_derive_runtime::eval`]'s output, if present.
/// Validates the prefix shape exactly (a `#`, then only ASCII digits, then
/// `: `) rather than splitting on the first `": "` anywhere in the line —
/// none of this corpus's own printed values happen to contain a literal
/// `": "` substring, but validating the prefix shape explicitly means this
/// helper is correct regardless, not merely lucky.
fn strip_worksheet_index_prefix(line: &str) -> &str {
    let Some(after_hash) = line.strip_prefix('#') else {
        return line;
    };
    match after_hash.find(": ") {
        Some(pos) if !after_hash[..pos].is_empty() && after_hash[..pos].bytes().all(|b| b.is_ascii_digit()) => {
            &after_hash[pos + 2..]
        }
        _ => line,
    }
}

/// Wrap every top-level statement's `expr` in the shared `"print"`
/// builtin — see this file's own module doc comment's "A harness-only
/// 'make it observable' step" section for the full rationale. Runs AFTER
/// `semantic_ir::validate` in [`compiled`] below, so validation itself
/// still exercises exactly what `derive_to_semantic_ir::compile_source`
/// actually shipped, unmodified.
fn wrap_top_level_in_print(module: &mut Module) {
    for f in &mut module.functions {
        if f.name != "main" {
            continue;
        }
        for stmt in &mut f.body.stmts {
            if let Stmt::ExprStmt { expr, span } = stmt {
                let inner = std::mem::replace(expr, Expr::NilLit { span: span.clone() });
                *expr = Expr::BuiltinCall {
                    name: "print".to_string(),
                    args: vec![inner],
                    effects: EffectSet::PURE,
                    span: span.clone(),
                };
            }
        }
    }
}

/// Compiled path: run `source` (unchanged) through
/// `derive_to_semantic_ir::compile_source`, `semantic_ir::validate`,
/// [`wrap_top_level_in_print`], `semantic_ir_to_javascript::compile`, and
/// an actual `node` process. Mirrors `j-to-semantic-ir/tests/oracle.rs`'s
/// own `compiled` exactly, down to the `OpenOptions::create_new(true)`
/// temp-file handling (that file's own doc comment explains why:
/// `create_new` fails instead of silently following an existing symlink
/// planted at the shared, predictable system temp path).
fn compiled(name: &str, source: &str) -> String {
    let mut module = compile_source(source, "prog")
        .unwrap_or_else(|e| panic!("lowering failed for {name} ({source:?}): {e:?}"));
    let report = semantic_ir::validate(&module);
    assert!(
        report.is_ok(),
        "SIR validation failed for {name}: {:?}",
        report.issues
    );
    wrap_top_level_in_print(&mut module);
    let artifact = semantic_ir_to_javascript::compile(&module)
        .unwrap_or_else(|e| panic!("backend emit failed for {name}: {e:?}"));

    let mut path = std::env::temp_dir();
    path.push(format!("derive_sir_oracle_{name}_{}.js", std::process::id()));
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
fn oracle_corpus_matches_native_derive_runtime() {
    if !node_available() {
        eprintln!("skipping oracle_corpus_matches_native_derive_runtime: `node` not available");
        return;
    }
    for case in CORPUS {
        let gt = ground_truth(case.source);
        assert_eq!(
            gt, case.expected,
            "{}: derive-runtime itself disagrees with this corpus entry's own `expected` -- the \
             program or `expected` is wrong, fix the corpus rather than this assertion",
            case.name
        );

        match case.known_bug {
            None => {
                let got = compiled(case.name, case.source);
                assert_eq!(
                    got, case.expected,
                    "{}: derive-to-semantic-ir -> semantic-ir-to-javascript -> node disagrees with \
                     the derive-runtime ground truth ({gt:?}) -- see this file's module doc for the \
                     two documented, already-excluded shared-crate gaps (no SIR23 evaluation at all; \
                     no per-language SIR23 display convention) before assuming this is a new one",
                    case.name
                );
            }
            Some(reason) => {
                // KNOWN BUG: the compiled-side assertion is deliberately
                // skipped (not even invoked) for this entry -- see this
                // file's module doc comment for why, and `reason` for
                // exactly which documented shared-crate gap applies here.
                eprintln!(
                    "{}: skipping compiled-side assertion (KNOWN BUG, not fixed in this PR): {reason}",
                    case.name
                );
            }
        }
    }
}
