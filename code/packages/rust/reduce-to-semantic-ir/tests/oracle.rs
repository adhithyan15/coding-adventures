//! Oracle/golden tests (HML01 §7): the SAME Reduce source, run through
//! **two independent implementations**, and diffed:
//!
//!   (a) `reduce-runtime` (`coding-adventures-reduce-runtime`) — this
//!       frontend's own sibling crate, which lowers to `symbolic-ir` and
//!       evaluates via `symbolic-vm`'s `SymbolicBackend` — the ground
//!       truth.
//!   (b) `reduce_to_semantic_ir::compile_source` → `semantic_ir::Module` →
//!       `semantic_ir_to_javascript::compile` → an actual `node` process.
//!
//! This is the direct Reduce sibling of
//! [`derive-to-semantic-ir`'s own `tests/oracle.rs`](../../derive-to-semantic-ir/tests/oracle.rs)
//! (itself the sibling of `j-to-semantic-ir`'s/`apl-to-semantic-ir`'s/
//! `matlab-to-semantic-ir`'s/`octave-to-semantic-ir`'s) — same overall
//! shape (`node_available` skip-not-fail guard, a `Case`/`CORPUS`, a
//! `ground_truth`/`compiled` pair, one looping `#[test]`, a `known_bug`
//! field for a disagreement rooted in a shared crate rather than this
//! frontend's own lowering) — completing HML01 §5's Stream B rollout note
//! for `reduce-to-semantic-ir` specifically (this PR updates that line —
//! see the spec; `wolfram-to-semantic-ir`/`macsyma-to-semantic-ir`/
//! `maple-to-semantic-ir` are unaffected by this PR and still have no
//! oracle file of their own).
//!
//! ## A harness-only "make it observable" step: wrapping every top-level
//! statement in `print`
//!
//! Exactly like `derive-to-semantic-ir/tests/oracle.rs`'s own identical
//! section (read that module doc first — this crate's design is a direct
//! retarget of Derive's, per `src/lower.rs`'s own module doc comment):
//! `reduce_to_semantic_ir::compile_source`'s lowering does **not** wrap a
//! bare top-level expression statement in the shared `"print"` builtin —
//! confirmed directly: `compile_source("1 + 2;\n", ..)`'s `main()` body is
//! a single bare `Stmt::ExprStmt` with no `print`/`console.log` anywhere in
//! the emitted JS module (`tests/e2e_node.rs`'s own module doc makes the
//! same "no `disp`-equivalent stdout" point). This is the same "every
//! statement always displays" worksheet convention Reduce shares with
//! Derive (MA08 §3's own `if`/`<<...>>`/assignment prose, and
//! `reduce-runtime::ReduceSession::feed`'s own doc comment: "every
//! statement produces one plain output line") — confirmed empirically
//! above (`ground_truth`'s own probe run): `coding_adventures_reduce_
//! runtime::eval` prints one line per statement, including `Assign`/
//! `Define`, with **no** numbered-input prefix at all (unlike Derive's own
//! `#n: `, per `reduce-runtime`'s own module doc: "Reduce's own session
//! transcript has no numbered-input convention"). So [`ground_truth`]
//! below needs no prefix-stripping step analogous to Derive's oracle
//! file's `strip_worksheet_index_prefix` — `reduce-runtime::eval`'s raw
//! output is already directly comparable, line for line, to what
//! [`wrap_top_level_in_print`]'s compiled side prints through
//! `console.log`.
//!
//! ## The dominant finding: the SIR23 JS backend does not evaluate
//! anything at all (found, NOT fixed here — a `semantic-ir-to-javascript`
//! gap, already documented, not re-discovered)
//!
//! Confirmed directly by hand-compiling and running representative shapes
//! through `node` (the exact same finding `derive-to-semantic-ir/tests/
//! oracle.rs` documents in full, and now further generalized in
//! `SIR23-symbolic-pattern-semantic-ir.md`'s own "Addendum — SIR23
//! symbolic evaluator + per-language display convention" section — see
//! that addendum and `derive-to-semantic-ir/CHANGELOG.md`'s `[0.1.1]`
//! entry for the complete write-up; this file only cites it, it does not
//! re-derive it): `Expr::SymApply` compiles, unconditionally, to
//! `__Sir.Symbolic.apply(head, [args])` — a **pure, inert term
//! constructor** — with no arithmetic folding, no comparison evaluation,
//! and no execution of the held `Assign`/`Define`/`If` forms at all. The
//! addendum confirms this is not Derive-specific: `derive-runtime`,
//! `reduce-runtime`, and `maple-runtime` all construct
//! `SymbolicBackend::new()` **completely unchanged**, so Reduce hits the
//! identical gap Derive did, for the identical reason. Concretely,
//! confirmed by hand-compiling and running each of these through `node`:
//!
//! - `2*3+4` compiles to a bare `Add(Mul(2, 3), 4)` term — never folds to
//!   `10`.
//! - `x := 5` / `x + 1` compiles to `Assign(x, 5)` then `Add(x, 1)` — the
//!   second statement's `x` is never substituted; it stays the raw symbol
//!   `x`, never `6`.
//! - `h(x) := x*x` / `h(5)` compiles to `Define(h, List(x), Mul(x, x))`
//!   then a bare, never-dispatched `h(5)` call term — never `25`.
//! - `5 > 3` compiles to `Greater(5, 3)` — never evaluates to `True`.
//! - `if 1 > 0 then 42 else 0` compiles to `If(Greater(1, 0), 42, 0)` — `If`
//!   is a HELD form on the ground-truth side (evaluates the condition,
//!   selects a branch); the compiled side never evaluates the condition or
//!   selects anything.
//!
//! This is a **shared-crate** gap (`semantic-ir-to-javascript` itself),
//! not a bug in this frontend's own lowering — `reduce_to_semantic_ir`
//! correctly emits the canonical `SymApply`/`SymSymbol` shapes MA08 §3
//! calls for (confirmed independently by `tests/test_lower.rs`'s ~59
//! passing shape assertions); the shared JS backend simply has no
//! evaluation semantics wired up for the SIR23 domain at all yet.
//!
//! ## A second, narrower finding layered on top: no per-language SIR23
//! display convention either (also found, NOT fixed here — same shared gap
//! Derive's oracle file documents)
//!
//! Independent of the evaluation gap above: even a term that WAS already
//! fully reduced would still print wrong for Reduce. `semantic-ir-to-
//! javascript`'s only SIR23 stringifier, `Symbolic.toDisplayString`
//! (`runtime.rs`), renders **every** compound term generically as
//! `head(args, ...)` — e.g. an unevaluated `Add(x, 1)` prints `"Add(x,
//! 1)"`, a `List(1, 2, 3)` prints `"List(1, 2, 3)"` — with no infix
//! `+`/`*`/`^` convention, no `{...}` curly-brace convention for `List`,
//! no `and`/`or`/`not`/`neq` lowercase-keyword convention, and no
//! case-bridging back to Reduce's own lowercase builtin surface spelling
//! (`reduce-runtime::printer::print_reduce` reverses ALL of these:
//! `Add(x,1)` → `"x + 1"`, `List(1,2,3)` → `"{1, 2, 3}"`, `First(l)` →
//! `"first(l)"`). Also a shared-crate gap, also `known_bug`, cited
//! alongside the evaluation gap above wherever a `CORPUS` entry's compiled
//! output would still disagree even under a hypothetical fix to the first
//! gap.
//!
//! ## A THIRD, Reduce-specific finding: several MA08 §3 heads have no
//! evaluation handler in `symbolic-vm` at all — already documented in MA08
//! §5 and `reduce-runtime`'s own module doc, and genuinely DIFFERENT from
//! the two shared-crate gaps above
//!
//! `CompoundExpression` (`<< ... >>`), `First`/`Second`/`Third`/`Rest`/
//! `Part`/`Append`/`Reverse` (the list accessors), and a non-folding
//! `Cons` have **no** handler in the shared `symbolic_vm::handlers::
//! build_handler_table` at all — confirmed both by `reduce-runtime`'s own
//! module doc comment and directly by this file's own `ground_truth` probe
//! run below (e.g. `first({1, 2, 3})` evaluates, via the REAL
//! `reduce-runtime`, to the literal unevaluated string `"first({1, 2,
//! 3})"` — not a bug in this oracle file, a confirmed, pre-existing,
//! disclosed gap in the shared symbolic engine itself, MA08 §5's own
//! corrected text). This is **not** the `semantic-ir-to-javascript`
//! evaluation gap described above — it is a **native-runtime** gap, one
//! layer further back in the pipeline, shared by `derive-runtime`'s
//! Wolfram/Macsyma siblings' own bespoke-`Backend` design point (MA08 §5:
//! "Macsyma's list functions and Wolfram's `CompoundExpression` are each
//! wired through a bespoke `Backend`... which is exactly what 'no custom
//! `Backend` at all' rules out building here"). Concretely: for these
//! heads, the ground-truth side is *already* unevaluated (nothing for the
//! compiled side to fail to keep up with) — the ONLY disagreement between
//! ground truth and compiled for these specific cases is the display
//! convention (gap two, above), not a missing evaluation on the compiled
//! side specifically. `CORPUS` entries hitting this third gap say so
//! explicitly, distinguishing it from the ordinary evaluation-gap
//! `known_bug` reason most entries below cite, per this task's own
//! instruction not to conflate distinct, already-documented gaps.
//!
//! One entry (`group_statement_executes_side_effects_in_order`) combines
//! finding one (the compiled side's `Assign` never binds `a`, so its
//! `a + 1` never evaluates to `2`) with finding three (neither side ever
//! collapses the outer `CompoundExpression` down to just its last
//! statement's value, since `symbolic-vm` has no handler for that head
//! either) — both already-documented, disjoint gaps, not a new one.
//!
//! ## Corpus
//!
//! Mirrors `derive-to-semantic-ir/tests/oracle.rs`'s own breadth target
//! (38 cases), adapted to MA08 §3's actual surface: ordinary operator
//! precedence and right-associative `^`/`**`; unary minus binding looser
//! than `^`; exact integer division vs. a genuine rational result;
//! assignment (plain and list-valued) read back by a later statement;
//! single- and multi-parameter procedure definition/call; `if`/`then`/
//! `else` (both branches, and the two-branch form's "false → `False`"
//! convention); every comparison/logic keyword (`= neq < <= > >= and or
//! not`, including a 3-term `and` chain exercising the n-ary logical-chain
//! fold); flat/singleton/elementwise-evaluated/empty list literals (MA08
//! §3's curly-brace list, D-5's Reduce analogue); list accessors
//! (`first`/`append`) that have no `symbolic-vm` handler at all (gap
//! three, above); cons (`.`), both the literal-list-folding shape (MA08
//! §3's only documented fold) and the non-folding shape onto a free
//! symbol; a group statement `<< ... >>` exercising in-order side effects;
//! a free-symbol additive-identity simplification; and bare integer/
//! float/symbol atoms (this subset's only `known_bug: None` cases, per
//! the evaluation-gap finding above). `DIF`/`INT`/trig calculus are
//! deliberately absent — MA08 §3's own table and `reduce-runtime::lower`'s
//! `standard_function` bridge table confirm Reduce's R-4 scope has **no**
//! calculus/trig bridging at all (unlike Derive), so there is nothing in
//! that area to test.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use coding_adventures_reduce_runtime::eval as reduce_eval;
use reduce_to_semantic_ir::compile_source;
use semantic_ir::{EffectSet, Expr, Module, Stmt};

/// Is a `node` binary on `PATH`? Mirrors `derive-to-semantic-ir/tests/
/// oracle.rs`'s own `node_available` (and every sibling oracle file's)
/// exactly: the test below skips (logs, does not fail) when it is not.
fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// One oracle corpus entry. Like `derive-to-semantic-ir`'s own `Case`,
/// `source` is the WHOLE program, byte-for-byte identical on both the
/// `ground_truth` and `compiled` sides.
struct Case {
    name: &'static str,
    source: &'static str,
    expected: &'static str,
    /// `None`: both `ground_truth` and `compiled` must equal `expected`.
    /// `Some(reason)`: only `ground_truth` is checked against `expected`;
    /// the `compiled`-side call is skipped entirely (not even invoked),
    /// with `reason` naming which documented gap (this file's module doc
    /// — the SIR23 evaluation gap, the SIR23 display-convention gap, or
    /// the Reduce-specific no-`symbolic-vm`-handler gap, or a combination)
    /// is responsible.
    known_bug: Option<&'static str>,
}

const CORPUS: &[Case] = &[
    // --- Bare atoms: the ONLY `known_bug: None` cases (see module doc's
    // "dominant finding" -- these need no evaluation at all, since the
    // source already denotes exactly one value). ---
    Case {
        name: "bare_integer_literal",
        source: "42;\n",
        expected: "42",
        known_bug: None,
    },
    Case {
        name: "bare_float_literal",
        source: "1.5;\n",
        expected: "1.5",
        known_bug: None,
    },
    // A whole-valued float still prints its trailing `.0` on BOTH sides:
    // `reduce-runtime::printer`'s `{v:?}` (Rust `Debug`) and the compiled
    // side's boxed-Float `floatToRubyString` (Ruby/Lisp convention) agree
    // here, unlike a bare unboxed JS number (`String(4.0) === "4"`) --
    // confirmed empirically, mirroring `derive-to-semantic-ir/tests/
    // oracle.rs`'s identical case.
    Case {
        name: "bare_whole_valued_float_keeps_trailing_dot_zero",
        source: "4.0;\n",
        expected: "4.0",
        known_bug: None,
    },
    Case {
        name: "bare_free_symbol",
        source: "foo;\n",
        expected: "foo",
        known_bug: None,
    },

    // --- Arithmetic: ordinary precedence, right-associative `^`/`**`,
    // unary minus binding looser than `^`. ---
    Case {
        name: "multiplication_binds_tighter_than_addition",
        source: "2*3+4;\n",
        expected: "10",
        // Flipped by the SIR23 addendum's item 1 (`semantic-ir-to-
        // javascript`'s `Symbolic.evalTerm` arithmetic/comparison/logic
        // scaffold): `Mul`/`Add` now fold numerically; confirmed by
        // running this exact case compiled-side.
        known_bug: None,
    },
    Case {
        name: "parens_override_precedence",
        source: "(2 + 3) * 4;\n",
        expected: "20",
        known_bug: None, // item 1: Add/Mul folding.
    },
    Case {
        name: "power_is_right_associative",
        source: "2^3^2;\n",
        expected: "512",
        known_bug: None, // item 1: Pow folding (right-associative shape already correct).
    },
    // `-2^2` -> `Neg(Pow(2, 2))` (unary minus binds LOOSER than `^`,
    // mirrors tests/test_lower.rs's unary_minus_binds_looser_than_power)
    // = Neg(4) = -4, not (-2)^2 = 4.
    Case {
        name: "unary_minus_binds_looser_than_power",
        source: "-2^2;\n",
        expected: "-4",
        // Flipped by item 1. Note this corrects the ORIGINAL known_bug
        // reason's own prediction, which assumed a folded `Neg` would
        // still need display-convention work ("even folded... would
        // print \"Neg(4)\", not \"-4\"") -- that assumption was wrong:
        // `Neg`'s numeric fold reduces straight to the PLAIN INTEGER
        // term `-4` (not a compound `Neg(4)` needing a prefix-minus
        // display rule), so `toDisplayString`'s existing "integer" case
        // already renders it correctly with no display work at all.
        known_bug: None,
    },
    Case {
        name: "exact_integer_division_folds_to_an_integer",
        source: "10 / 2;\n",
        expected: "5",
        known_bug: None, // item 1: Div folding (exact-integer collapse).
    },
    Case {
        name: "inexact_division_folds_to_a_rational",
        source: "1 / 3;\n",
        expected: "1/3",
        known_bug: None, // item 1: Div folding (exact-rational result).
    },
    // `x + 0` -> the additive-identity simplification (`x`), confirming a
    // free symbol stays symbolic through the shared handler on the ground
    // -truth side (mirrors reduce-runtime::tests::free_symbols_stay_
    // symbolic).
    Case {
        name: "additive_identity_simplifies_a_free_symbol",
        source: "x + 0;\n",
        expected: "x",
        // Flipped by item 1: `addHandler`'s ported identity-law
        // fallback (`x + 0 -> x`, `0 + x -> x`, mirroring
        // `handlers.rs::add_handler` exactly) now fires.
        known_bug: None,
    },
    Case {
        name: "negative_integer_literal",
        source: "-5;\n",
        expected: "-5",
        // Flipped by item 1 -- same correction as
        // `unary_minus_binds_looser_than_power` above: `Neg`'s numeric
        // fold produces the plain integer term `-5`, not a compound
        // `Neg(5)`, so no display-convention work was ever needed here
        // either, contrary to the original known_bug reason's guess.
        known_bug: None,
    },
    Case {
        name: "negation_of_a_free_symbol",
        source: "-x;\n",
        expected: "-x",
        known_bug: Some(
            "Evaluation/display gap (module doc): compiles to a bare Neg(x) term; there is nothing \
             to fold (x is free), but the display-convention gap alone means this prints \"Neg(x)\", \
             never the prefix surface \"-x\".",
        ),
    },

    // --- Assignment: read back by a LATER statement (mirrors
    // derive-to-semantic-ir's own variable_assignment_and_later_reference,
    // adapted to Reduce's own "every statement displays" convention --
    // MA08 §3/§5). ---
    Case {
        name: "variable_assignment_and_later_reference",
        source: "x := 5;\nx + 1;\n",
        expected: "5\n6",
        known_bug: Some(
            "Evaluation gap (module doc): Assign is a HELD form on the ground-truth side (binds x, \
             displays its value, 5); the compiled side's Assign(x, 5) is inert data -- no binding \
             ever happens -- so the SECOND statement's x is never substituted: it compiles to a bare \
             Add(x, 1) term, still referencing the raw, unbound symbol x, never 6.",
        ),
    },
    Case {
        name: "list_assignment_persists_across_statements",
        source: "v := {1, 2, 3};\nv;\n",
        expected: "{1, 2, 3}\n{1, 2, 3}",
        known_bug: Some(
            "Evaluation gap (module doc): the compiled side's Assign never binds v, so the second \
             statement compiles to the bare, still-unbound symbol v, not the list; and even a bound \
             List(1,2,3) would print via the display-convention gap as \"List(1, 2, 3)\", not \
             \"{1, 2, 3}\".",
        ),
    },

    // --- User-defined procedures: definition echoes the bare name (MA08
    // §3/§5's own worksheet convention -- confirmed against
    // reduce-runtime's own user_defined_operator_end_to_end test), then a
    // call dispatches. ---
    Case {
        name: "single_param_procedure_definition_and_call",
        source: "h(x) := x*x;\nh(5);\n",
        expected: "h\n25",
        known_bug: Some(
            "Evaluation gap (module doc): Define is a HELD form on the ground-truth side (registers \
             h, displays its bare name, \"h\"); the compiled side's Define(...) is inert data -- no \
             registration ever happens -- so the SECOND statement compiles to a bare, never-\
             dispatched h(5) call term, not 25.",
        ),
    },
    Case {
        name: "multi_param_procedure_definition_and_call",
        source: "g(a, b) := a + b;\ng(3, 4);\n",
        expected: "g\n7",
        known_bug: Some(
            "Evaluation gap (module doc): same root cause as single_param_procedure_definition_and_\
             call -- Define never registers g, so g(3, 4) never dispatches to 7.",
        ),
    },

    // --- IF: both branches, and the two-branch (no `else`) form's
    // "false -> False" convention (the held handler, MA08 §3/§5). ---
    Case {
        name: "if_true_branch_with_else",
        source: "if 1 > 0 then 42 else 0;\n",
        expected: "42",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare If(Greater(1, 0), 42, 0) term -- If is \
             a HELD form on the ground-truth side (evaluates the condition, then selects a branch); \
             the compiled side never evaluates the condition or selects anything.",
        ),
    },
    Case {
        name: "if_false_branch_with_else",
        source: "if 1 > 2 then 42 else 99;\n",
        expected: "99",
        known_bug: Some(
            "Evaluation gap (module doc): same root cause as if_true_branch_with_else -- compiles to \
             a bare If(Greater(1, 2), 42, 99) term, never selects a branch.",
        ),
    },
    Case {
        name: "if_true_branch_no_else",
        source: "if 1 > 0 then 42;\n",
        expected: "42",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare If(Greater(1, 0), 42) (2-arg) term, \
             never selects the then-branch.",
        ),
    },
    // Confirmed empirically against reduce-runtime directly (not assumed
    // from Derive's IF, which has no no-else form at all): a 2-arg `if`
    // whose condition fails evaluates to the symbol `False`, not to Nil or
    // an error -- `symbolic-vm`'s if_handler's own documented "2- or 3-arg
    // form" arity check (SIR23 addendum's "Environment / held-form
    // execution model" section).
    Case {
        name: "if_false_branch_no_else_yields_false",
        source: "if 1 > 2 then 42;\n",
        expected: "False",
        known_bug: Some(
            "Evaluation gap (module doc): compiles to a bare If(Greater(1, 2), 42) (2-arg) term, \
             never evaluates to the symbol False.",
        ),
    },

    // --- Comparisons: fold to the symbol True/False on the ground-truth
    // side (confirmed empirically -- NOT a JS boolean, NOT 1/0). ---
    Case {
        name: "comparison_true",
        source: "5 > 3;\n",
        expected: "True",
        known_bug: None, // item 1: comparison folding to the True symbol.
    },
    Case {
        name: "comparison_false",
        source: "3 > 5;\n",
        expected: "False",
        known_bug: None, // item 1: comparison folding to the False symbol.
    },
    Case {
        name: "less_equal_boundary_is_true",
        source: "3 <= 3;\n",
        expected: "True",
        known_bug: None, // item 1: comparison folding.
    },
    // `neq` is Reduce's own not-equal keyword (Derive has no equivalent
    // token at all -- MA08 §3).
    Case {
        name: "not_equal_is_true",
        source: "3 neq 4;\n",
        expected: "True",
        known_bug: None, // item 1: NotEqual comparison folding.
    },
    // `=` is Reduce's EQUATION operator (never assignment) and stays
    // symbolic when either side is a free variable -- both sides agree on
    // THIS shape already (nothing to evaluate away), but the compiled
    // side's printed NOTATION still disagrees (display-convention gap).
    Case {
        name: "equation_with_a_free_variable_stays_symbolic",
        source: "x = 4;\n",
        expected: "x = 4",
        known_bug: Some(
            "Display-convention gap only (module doc) -- this one needs no evaluation at all (x is \
             free, so Equal(x, 4) is already the fully-reduced ground-truth value); the compiled \
             side's generic, non-infix Symbolic.toDisplayString prints \"Equal(x, 4)\", never \
             Reduce's own infix \"x = 4\".",
        ),
    },

    // --- Logic keywords: and / or / not, including a 3-term `and` chain
    // (the n-ary logical_chain fold, mirrors tests/test_lower.rs's
    // logical_or_chain_folds_n_ary_not_nested_binary for `or`). ---
    Case {
        name: "and_short_circuits_to_true",
        source: "5 > 3 and 2 < 4;\n",
        expected: "True",
        known_bug: None, // item 1: comparison + And folding.
    },
    Case {
        name: "three_term_and_chain_folds_n_ary",
        source: "5 > 3 and 3 > 1 and 1 > 0;\n",
        expected: "True",
        known_bug: None, // item 1: comparison + n-ary And folding.
    },
    Case {
        name: "not_negates_a_true_comparison",
        source: "not (5 > 3);\n",
        expected: "False",
        known_bug: None, // item 1: comparison + Not folding.
    },

    // --- Lists: flat, singleton, elementwise-evaluated, and empty (MA08
    // §3's curly-brace `{a,b,c}` list, always flat -- no row/matrix shape,
    // MA08 §4). ---
    Case {
        name: "flat_list_literal",
        source: "{1, 2, 3};\n",
        expected: "{1, 2, 3}",
        known_bug: Some(
            "Display-convention gap (module doc): a flat List(1, 2, 3) needs no evaluation at all \
             (every element is already a literal), but the compiled side's generic \
             Symbolic.toDisplayString prints \"List(1, 2, 3)\", never Reduce's own curly-brace \
             surface \"{1, 2, 3}\".",
        ),
    },
    Case {
        name: "singleton_list_literal",
        source: "{5};\n",
        expected: "{5}",
        known_bug: Some(
            "Display-convention gap only (module doc): List(5) needs no evaluation; prints \
             \"List(5)\", not \"{5}\".",
        ),
    },
    Case {
        name: "list_of_expressions_evaluates_elementwise",
        source: "{1+1, 2*3, 2^3};\n",
        expected: "{2, 6, 8}",
        // UPDATED by item 1 (was: "Evaluation gap ... never {2, 6, 8}").
        // `List` has no handler of its own (by design, see the SIR23
        // addendum's own handler table -- "no handler needed at all"),
        // but `evalTerm`'s applicative-order argument evaluation still
        // folds each element for free: this now compiles to and
        // evaluates as `List(2, 6, 8)`, confirmed by direct inspection
        // of the emitted JS -- the elements are NOT unfolded anymore.
        // Only Reduce's own `{...}` curly-brace display convention
        // (item 4) is still missing, so this stays `known_bug`, now for
        // a purely display-convention reason.
        known_bug: Some(
            "Display-convention gap ONLY (module doc; corrected after item 1 landed) -- NOT an \
             evaluation gap anymore: evalTerm's applicative-order argument evaluation folds each \
             List element for free even though List itself has no handler, so this now compiles to \
             and evaluates as List(2, 6, 8) (confirmed by direct inspection of the emitted JS) -- \
             only the generic Symbolic.toDisplayString printing \"List(2, 6, 8)\" instead of \
             Reduce's own curly-brace surface \"{2, 6, 8}\" remains, which is item 4's job (a \
             per-language SIR23 display convention -- Reduce's own is not scoped for this rollout \
             either, see the addendum's own \"Scope boundary\" section, so this specific case waits \
             on a Reduce-oracle-driven display-convention item, not just Derive's item 4).",
        ),
    },
    Case {
        name: "empty_list_literal",
        source: "{};\n",
        expected: "{}",
        known_bug: Some(
            "Display-convention gap only (module doc): an empty List() needs no evaluation, but \
             prints \"List()\", not the curly-brace surface \"{}\".",
        ),
    },

    // --- List accessors: `first`/`append` (MA08 §3's list-accessor
    // surface). THE THIRD, Reduce-specific finding (module doc): these
    // have no evaluation handler in the shared symbolic-vm at all -- the
    // ground truth ITSELF stays unevaluated, confirmed empirically -- so
    // the only actual mismatch on the compiled side is display notation,
    // not a missing evaluation the compiled side alone is failing to do. ---
    Case {
        name: "first_of_a_list_has_no_shared_vm_handler",
        source: "first({1, 2, 3});\n",
        expected: "first({1, 2, 3})",
        known_bug: Some(
            "Reduce-specific gap (module doc's THIRD finding, MA08 §5/reduce-runtime's own module \
             doc -- NOT the semantic-ir-to-javascript evaluation gap): symbolic-vm's shared handler \
             table has no handler for First at all, so reduce-runtime itself (the ground truth) \
             already leaves this unevaluated -- confirmed empirically, eval() returns the literal \
             string \"first({1, 2, 3})\". The compiled side independently constructs the identical \
             First(List(1,2,3)) term (also never evaluated, for the ordinary semantic-ir-to-\
             javascript reason) but prints it via the generic display-convention gap as \
             \"First(List(1, 2, 3))\", not Reduce's own bridged-back-to-lowercase \
             \"first({1, 2, 3})\".",
        ),
    },
    Case {
        name: "append_of_two_lists_has_no_shared_vm_handler",
        source: "append({1}, {2});\n",
        expected: "append({1}, {2})",
        known_bug: Some(
            "Reduce-specific gap (module doc's THIRD finding): same root cause as \
             first_of_a_list_has_no_shared_vm_handler -- symbolic-vm has no Append handler, so the \
             ground truth already leaves this unevaluated (confirmed empirically); the compiled \
             side's independently-constructed, equally-unevaluated Append(List(1),List(2)) term \
             still disagrees on notation, printing \"Append(List(1), List(2))\", not \
             \"append({1}, {2})\".",
        ),
    },

    // --- Cons (`.`, MA08 §3): the literal-list-folding shape (the ONE
    // fold MA08 §3 documents -- happens at LOWERING time in BOTH
    // reduce-runtime and reduce-to-semantic-ir, per fold_cons -- so this
    // needs no evaluation at all, exactly like a flat list literal), and
    // the non-folding shape onto a free symbol (hits the THIRD,
    // Reduce-specific finding, like the list accessors above). ---
    Case {
        name: "cons_onto_a_literal_list_folds_at_lowering_time",
        source: "1 . {2, 3};\n",
        expected: "{1, 2, 3}",
        known_bug: Some(
            "Display-convention gap ONLY (module doc) -- NOT an evaluation gap: fold_cons folds `1 . \
             {2,3}` into a flat List(1,2,3) at LOWERING time, identically in reduce-runtime's own \
             lower.rs and this crate's src/lower.rs (confirmed: both crates' fold_cons share the same \
             documented logic) -- no Cons node, and no evaluation, is ever involved on either side. \
             The compiled side ends up with the exact same List(1, 2, 3) SymApply the flat_list_\
             literal case does; the only disagreement is the generic Symbolic.toDisplayString \
             printing \"List(1, 2, 3)\" instead of the curly-brace surface \"{1, 2, 3}\".",
        ),
    },
    Case {
        name: "cons_of_two_free_symbols_has_no_shared_vm_handler",
        source: "a . b;\n",
        expected: "a . b",
        known_bug: Some(
            "Reduce-specific gap (module doc's THIRD finding) -- b is not structurally a literal \
             List at lowering time, so fold_cons produces a bare Cons(a, b) term instead of folding \
             (MA08 §3's own disclosed gap in its own precedence table); symbolic-vm has no Cons \
             handler either, so the ground truth already leaves this unevaluated (confirmed \
             empirically: eval() returns the literal string \"a . b\"). The compiled side's \
             independently-constructed, equally-unevaluated Cons(a, b) term still disagrees on \
             notation, printing \"Cons(a, b)\", not the infix surface \"a . b\".",
        ),
    },

    // --- Group statement `<< s1; s2; ... >>` (MA08 §3's
    // CompoundExpression): exercises in-order side effects, combining
    // finding one (Assign never binds on the compiled side) with finding
    // three (CompoundExpression itself has no shared-vm handler on
    // EITHER side, so even the ground truth never collapses to just the
    // last statement's value -- mirrors reduce-runtime's own
    // group_statement_executes_side_effects_in_order test exactly). ---
    Case {
        name: "group_statement_evaluates_side_effects_in_order",
        source: "<< a := 1; a + 1 >>;\n",
        expected: "<< 1; 2 >>",
        known_bug: Some(
            "Two disjoint, already-documented gaps stack here (module doc). First finding: on the \
             compiled side, Assign(a, 1) never actually binds a, so the second sub-statement \
             compiles to a bare, still-unbound Add(a, 1), never 2 -- unlike the ground truth, where \
             the held Assign genuinely fires (confirmed empirically: a later a; call would read back \
             1). Third finding: CompoundExpression has no symbolic-vm handler on EITHER side, so \
             even the ground truth's own << 1; 2 >> never collapses to just its last statement's \
             value (2 alone) -- MA08 §3's own \"evaluates to its last statement's value\" prose does \
             not hold for the actual, shipped engine, already disclosed in reduce-runtime's module \
             doc and confirmed by its own identical test. On top of both, the compiled side's \
             generic display would print \"CompoundExpression(Assign(a, 1), Add(a, 1))\", not even \
             the << ... >> surface syntax.",
        ),
    },
];

/// Ground truth: run `source` through `reduce-runtime`'s own
/// [`reduce_eval`]. Unlike `derive-to-semantic-ir/tests/oracle.rs`'s own
/// `ground_truth` (which strips a `#n: ` worksheet-numbering prefix from
/// every line), Reduce's own session transcript has **no** numbered-input
/// convention at all (`reduce-runtime`'s own module doc: "unlike
/// `derive-runtime` or Wolfram's `In[n]:=`/`Out[n]=`, Reduce's own session
/// transcript has no numbered-input convention") -- confirmed directly by
/// this file's own probe run, so `reduce_eval`'s raw output (one plain
/// line per statement) is already directly comparable to the compiled
/// side's `console.log` output with no prefix-stripping step needed.
fn ground_truth(source: &str) -> String {
    let raw = reduce_eval(source)
        .unwrap_or_else(|e| panic!("reduce-runtime eval failed for {source:?}: {e}"));
    raw.trim_end_matches('\n').to_string()
}

/// Wrap every top-level statement's `expr` in the shared `"print"`
/// builtin — see this file's own module doc comment's "A harness-only
/// 'make it observable' step" section for the full rationale. Runs AFTER
/// `semantic_ir::validate` in [`compiled`] below, so validation itself
/// still exercises exactly what `reduce_to_semantic_ir::compile_source`
/// actually shipped, unmodified. Mirrors `derive-to-semantic-ir/tests/
/// oracle.rs`'s own `wrap_top_level_in_print` exactly.
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
/// `reduce_to_semantic_ir::compile_source`, `semantic_ir::validate`,
/// [`wrap_top_level_in_print`], `semantic_ir_to_javascript::compile`, and
/// an actual `node` process. Mirrors `derive-to-semantic-ir/tests/
/// oracle.rs`'s own `compiled` exactly, down to the
/// `OpenOptions::create_new(true)` temp-file handling (that file's own doc
/// comment explains why: `create_new` fails instead of silently following
/// an existing symlink planted at the shared, predictable system temp
/// path -- a deliberate security mitigation, kept exactly as-is here, not
/// weakened to `.create(true)`).
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
    path.push(format!("reduce_sir_oracle_{name}_{}.js", std::process::id()));
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
fn oracle_corpus_matches_native_reduce_runtime() {
    if !node_available() {
        eprintln!("skipping oracle_corpus_matches_native_reduce_runtime: `node` not available");
        return;
    }
    for case in CORPUS {
        let gt = ground_truth(case.source);
        assert_eq!(
            gt, case.expected,
            "{}: reduce-runtime itself disagrees with this corpus entry's own `expected` -- the \
             program or `expected` is wrong, fix the corpus rather than this assertion",
            case.name
        );

        match case.known_bug {
            None => {
                let got = compiled(case.name, case.source);
                assert_eq!(
                    got, case.expected,
                    "{}: reduce-to-semantic-ir -> semantic-ir-to-javascript -> node disagrees with \
                     the reduce-runtime ground truth ({gt:?}) -- see this file's module doc for the \
                     three documented, already-excluded gaps (no SIR23 evaluation at all; no \
                     per-language SIR23 display convention; several MA08 §3 heads with no \
                     symbolic-vm handler at all) before assuming this is a new one",
                    case.name
                );
            }
            Some(reason) => {
                // KNOWN BUG: the compiled-side assertion is deliberately
                // skipped (not even invoked) for this entry -- see this
                // file's module doc comment for why, and `reason` for
                // exactly which documented gap applies here.
                eprintln!(
                    "{}: skipping compiled-side assertion (KNOWN BUG, not fixed in this PR): {reason}",
                    case.name
                );
            }
        }
    }
}
