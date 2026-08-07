//! Oracle/golden tests (HML01 §7): the SAME Maple source, run through
//! **two independent implementations**, and diffed:
//!
//!   (a) `maple-runtime` (`coding-adventures-maple-runtime`) — this
//!       frontend's own sibling crate, which lowers to `symbolic-ir` and
//!       evaluates via `symbolic-vm`'s `SymbolicBackend` — the ground
//!       truth.
//!   (b) `maple_to_semantic_ir::compile_source` → `semantic_ir::Module`
//!       → `semantic_ir_to_javascript::compile` → an actual `node`
//!       process.
//!
//! This is the direct Maple sibling of
//! [`reduce-to-semantic-ir`'s own `tests/oracle.rs`](../../reduce-to-semantic-ir/tests/oracle.rs)
//! (itself the sibling of `derive-to-semantic-ir`'s/`j-to-semantic-ir`'s/
//! `apl-to-semantic-ir`'s/`matlab-to-semantic-ir`'s/`octave-to-semantic-ir`'s)
//! — same overall shape (`node_available` skip-not-fail guard, a
//! `Case`/`CORPUS`, a `ground_truth`/`compiled` pair, one looping
//! `#[test]`, a `known_bug` field for a disagreement rooted in a shared
//! crate rather than this frontend's own lowering) — completing HML01
//! §5's Stream B rollout note for `maple-to-semantic-ir` specifically
//! (this PR updates that line — see the spec; this is the LAST of the
//! five SIR23 frontends to get its own oracle file).
//!
//! ## A harness-only "make it observable" step: wrapping every top-level
//! statement in `print`
//!
//! Exactly like `reduce-to-semantic-ir/tests/oracle.rs`'s own identical
//! section (read that module doc first — this crate's harness is a
//! direct retarget of Reduce's/Derive's): `maple_to_semantic_ir::
//! compile_source`'s lowering does **not** wrap a bare top-level
//! expression statement in the shared `"print"` builtin — confirmed
//! directly: `compile_source("1 + 2;\n", ..)`'s `main()` body is a
//! single bare `Stmt::ExprStmt` with no `print`/`console.log` anywhere in
//! the emitted JS module (`tests/e2e_node.rs`'s own module doc makes the
//! same "no `disp`-equivalent stdout" point). This is the same
//! "every statement always displays" worksheet convention Maple shares
//! with Reduce and Derive (MA09 §3's own `;`-vs-`:` statement-separator
//! row, and `maple-runtime::MapleSession::feed`'s own doc comment: every
//! `;`-terminated statement produces one plain output line) — confirmed
//! empirically below ([`ground_truth`]'s own probe run): `coding_
//! adventures_maple_runtime::eval` prints one line per *displayed*
//! statement, with **no** numbered-input prefix at all (MA09 §5:
//! "matching [Reduce]'s own unnumbered `reduce-repl`") — so
//! [`ground_truth`] below needs no prefix-stripping step analogous to
//! Derive's oracle file's `strip_worksheet_index_prefix`, exactly like
//! Reduce's own oracle file.
//!
//! ## Finding one (shared, already documented elsewhere): the SIR23 JS
//! backend has real arithmetic/comparison/logic folding (item 1 of 4),
//! but NOT held-form execution, calculus, or a per-language display
//! convention (items 2-4)
//!
//! `semantic-ir-to-javascript` 0.49.0 (`Symbolic.evalTerm`, that crate's
//! own `CHANGELOG.md` `[0.49.0]` entry, found by `derive-to-semantic-ir`'s
//! and `reduce-to-semantic-ir`'s own oracle files, NOT re-discovered
//! here) folds `Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg`/`Inv`/`Abs`, the six
//! comparisons (to the `True`/`False` **symbol**, never a JS boolean),
//! and `And`/`Or`/`Not` (n-ary). `HELD_HEADS = {"Assign", "Define",
//! "If"}` is declared but wired to NO handler — a held head's args are
//! passed through byte-for-byte unevaluated, and even the accumulated
//! term simply gets rebuilt from the (untouched) head/args, so
//! `Assign(x, 5)` never binds `x`, `Define(f, [x], x*x)` never registers
//! `f`, and `If(cond, then, else)` never selects a branch (confirmed by
//! hand-compiling and running each shape through `node`, mirroring the
//! sibling oracle files' identical confirmation). `D`/`Integrate`
//! (`diff`/`int`'s canonical heads) are not in `HANDLERS` at all, so
//! calculus stays unevaluated too. And the sole SIR23 stringifier,
//! `Symbolic.toDisplayString`, still renders every compound term
//! generically as `head(args, ...)` — no infix `+`/`*`/`^`, no
//! `[...]`/`{...}` bracket convention, no `and`/`or`/`not` lowercase
//! convention, and (see finding two below) no case-bridging for the
//! `True`/`False` symbol. These are the SAME two shared-crate gaps
//! (SIR23 evaluation scope, SIR23 display convention) `derive-to-
//! semantic-ir`'s and `reduce-to-semantic-ir`'s own oracle files already
//! document in full — cited here, not re-derived.
//!
//! ## Finding two — GENUINELY NEW, Maple-specific: a `True`/`False`
//! CASE mismatch that neither Reduce's nor Derive's own oracle file ever
//! hit
//!
//! Every comparison/logical-operator handler in `semantic-ir-to-
//! javascript`'s `runtime.rs` (`comparisonHandler`, `andHandler`,
//! `orHandler`, `notHandler`) folds to the literal symbol `symTerm
//! ("True")`/`symTerm("False")` — **capitalized**, matching Wolfram's
//! own convention (`toDisplayString`'s `"symbol"` case is a bare
//! `return node.name`, with **no** per-language case-bridging anywhere
//! in the SIR23 domain — verified directly: grepping this crate's own
//! `runtime.rs` for `SIR_DISPLAY_` finds that mechanism wired up ONLY
//! for the unrelated SIR16/SIR22 array domain — APL's/J's/Ruby's own
//! flags — never for SIR23 at all). `reduce-runtime::printer`'s and
//! `derive-runtime`'s own printers *also* render `True`/`False`
//! capitalized (their own native, real-language convention already
//! agrees with the JS backend's hardcoded spelling), which is exactly
//! why `reduce-to-semantic-ir/tests/oracle.rs`'s comparison/logic cases
//! could flip to `known_bug: None` once item 1 landed. **Maple is
//! different**: MA09 §3 documents `true`/`false` as real Maple's own
//! *lowercase* boolean surface (the `type/truefalseFAIL` Help page), and
//! `maple-runtime::printer::render` explicitly bridges the shared
//! backend's `True`/`False` symbol back to lowercase (`IRNode::Symbol(s)
//! if s == "True" => "true"`, confirmed directly against that file). So
//! even a comparison/logic case that folds *identically* on both sides
//! (finding one's fix covers this) still disagrees on the printed
//! **case**: ground truth prints `"true"`/`"false"`, the compiled side's
//! generic, non-language-aware `toDisplayString` prints `"True"`/
//! `"False"`. This is a genuinely new sub-case of the shared
//! display-convention gap (finding one's second half), not a new crate
//! bug — but it is a strictly BIGGER set of already-would-be-`known_bug:
//! None` cases than either sibling oracle file hit, confirmed by running
//! every comparison/logic case below and observing the capitalization
//! mismatch directly (not assumed from the static analysis above alone).
//!
//! ## Finding three — Maple-specific, but NOT a bug: `Set` (MA09 §5)
//! folds its elements "for free," exactly like `List`, but has no
//! display convention either
//!
//! `Set` (the new head `{a, b, c}` lowers to, MA09 §3/§5) has no
//! `symbolic-vm` handler on the ground-truth side and no `HANDLERS` entry
//! on the compiled side either — but on BOTH sides, an unmatched head's
//! arguments still evaluate in applicative order before the (missing)
//! handler lookup fails and the term is rebuilt from the now-folded args
//! (`evalApply`'s "no handler matched -> rebuild from evaluated head +
//! evaluated args" fallthrough; `maple-runtime`'s own doc comment
//! confirms the identical behavior natively: "`Set` is not a held head,
//! so its elements evaluate... but the call itself stays structurally
//! correct-but-unevaluated"). So `{1+1, 2*3}` folds its elements to
//! `{2, 6}` on BOTH sides — the only actual disagreement is bracket
//! notation (`Set(2, 6)` vs. `{2, 6}`), the same "already-evaluated,
//! display-only" shape `List` hits, not a deeper evaluation gap the way
//! Reduce's `first`/`append`/`Cons`-onto-a-free-symbol cases are (those
//! have no shared handler AND leave the ground truth itself unevaluated
//! — `Set` here always resolves its arguments on the ground-truth side
//! too, confirmed directly by `maple-runtime`'s own `set_literal_
//! evaluates_its_elements_but_stays_structurally_unresolved` test).
//!
//! ## Corpus
//!
//! Chosen to exercise Maple's own distinctive surface (MA09 §3), not
//! generic filler already covered by the other four CAS-family oracle
//! files: bare integer/float/symbol/boolean atoms (the boolean cases are
//! where finding two bites, even though every OTHER sibling oracle file's
//! equivalent case is `known_bug: None`); ordinary operator precedence,
//! right-associative `^` (no `**` synonym, unlike Reduce's), unary minus
//! binding looser than `^`; exact integer vs. genuine-rational division;
//! an additive-identity simplification; every comparison INCLUDING
//! Maple's own `<>` not-equal spelling (neither Reduce's `neq` keyword
//! nor Wolfram's `!=`); `and`/`or`/`not` (a 3-term `and` chain exercising
//! the n-ary fold); `:=` assignment and a later read-back (held-form
//! gap); the arrow-operator `Define` (`f := x -> e` / `f := (x, y) -> e`,
//! MA09 §1's OWN documented trap — NOT the `f(x) := e` remember-table
//! spelling Reduce's/Derive's own general-definition idiom would suggest,
//! since that spelling is excluded from this subset's grammar entirely);
//! `if`/`elif`/`else`/`end if` (the right-folded elif chain genuinely new
//! relative to Reduce's simpler 2-or-3-child `if`, plus the unresolved-
//! condition case that reconstructs Maple's own `if...then...else...end
//! if` surface on the ground-truth side); flat/singleton/empty/
//! elementwise-evaluated `[...]` list literals; flat/empty/elementwise-
//! evaluated `{...}` SET literals (finding three, MA09's own genuinely
//! new-to-this-repo aggregate type, kept textually and semantically
//! DISTINCT from the list cases, per MA09 §1's own "same brackets,
//! different family conventions" warning); and `diff`/`int` (MA09's own
//! lowercase calculus bridge, evaluated on the ground-truth side via the
//! shared `D`/`Integrate` handlers, unevaluated on the compiled side per
//! finding one).

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use coding_adventures_maple_runtime::eval as maple_eval;
use maple_to_semantic_ir::compile_source;
use semantic_ir::{EffectSet, Expr, Module, Stmt};

/// Is a `node` binary on `PATH`? Mirrors every sibling oracle file's
/// identical `node_available`: the test below skips (logs, does not
/// fail) when it is not.
fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// One oracle corpus entry. `source` is the WHOLE program, byte-for-byte
/// identical on both the `ground_truth` and `compiled` sides.
struct Case {
    name: &'static str,
    source: &'static str,
    expected: &'static str,
    /// `None`: both `ground_truth` and `compiled` must equal `expected`.
    /// `Some(reason)`: only `ground_truth` is checked against `expected`;
    /// the `compiled`-side call is skipped entirely (not even invoked),
    /// with `reason` naming which documented gap (this file's module doc
    /// — findings one, two, or three, or a combination) is responsible.
    known_bug: Option<&'static str>,
}

const CORPUS: &[Case] = &[
    // --- Bare atoms: numbers/symbols need no evaluation at all, so they
    // are `known_bug: None`; the boolean literals hit finding two (the
    // Maple-specific True/False case mismatch) even though they too need
    // no evaluation. ---
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
    // A whole-valued float still prints its trailing `.0` on BOTH sides
    // (the shared boxed-float codegen, confirmed by `derive-to-semantic-
    // ir`'s and `reduce-to-semantic-ir`'s own identical cases) -- unlike
    // a bare unboxed JS number (`String(4.0) === "4"`).
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
    Case {
        name: "bare_boolean_true_literal",
        source: "true;\n",
        expected: "true",
        known_bug: Some(
            "Finding two (module doc): a bare boolean literal needs no evaluation on either side, \
             but maple-runtime::printer bridges the shared backend's True symbol to Maple's own \
             lowercase surface (\"true\"), while the compiled side's generic, non-language-aware \
             Symbolic.toDisplayString prints the raw symbol name unchanged (\"True\") -- a case \
             mismatch, not an evaluation gap.",
        ),
    },
    Case {
        name: "bare_boolean_false_literal",
        source: "false;\n",
        expected: "false",
        known_bug: Some(
            "Finding two (module doc): same case-mismatch root cause as bare_boolean_true_literal -- \
             ground truth prints \"false\", compiled prints \"False\".",
        ),
    },

    // --- Arithmetic: ordinary precedence, right-associative `^` (no `**`
    // synonym in this grammar, unlike Reduce's), unary minus binding
    // looser than `^`. ---
    Case {
        name: "multiplication_binds_tighter_than_addition",
        source: "2*3+4;\n",
        expected: "10",
        known_bug: None, // Finding one: Add/Mul folding.
    },
    Case {
        name: "parens_override_precedence",
        source: "(2 + 3) * 4;\n",
        expected: "20",
        known_bug: None, // Finding one: Add/Mul folding.
    },
    Case {
        name: "power_is_right_associative",
        source: "2^3^2;\n",
        expected: "512",
        known_bug: None, // Finding one: Pow folding (right-associative shape already correct).
    },
    // `-2^2` -> `Neg(Pow(2, 2))` (unary minus binds LOOSER than `^`,
    // mirrors tests/test_lower.rs's unary_minus_binds_looser_than_power)
    // = Neg(4) = -4, not (-2)^2 = 4.
    Case {
        name: "unary_minus_binds_looser_than_power",
        source: "-2^2;\n",
        expected: "-4",
        known_bug: None, // Finding one: Pow then Neg numeric folding.
    },
    Case {
        name: "exact_integer_division_folds_to_an_integer",
        source: "10 / 2;\n",
        expected: "5",
        known_bug: None, // Finding one: Div folding (exact-integer collapse).
    },
    Case {
        name: "inexact_division_folds_to_a_rational",
        source: "1 / 3;\n",
        expected: "1/3",
        known_bug: None, // Finding one: Div folding (exact-rational result).
    },
    // `x + 0` -> the additive-identity simplification (`x`), confirming a
    // free symbol stays symbolic through the shared handler on the
    // ground-truth side (mirrors maple-runtime::tests::
    // free_symbols_stay_symbolic).
    Case {
        name: "additive_identity_simplifies_a_free_symbol",
        source: "x + 0;\n",
        expected: "x",
        known_bug: None, // Finding one: addHandler's ported identity-law fallback.
    },
    Case {
        name: "negative_integer_literal",
        source: "-5;\n",
        expected: "-5",
        // Neg's numeric fold produces the plain integer term -5 directly
        // (never a compound Neg(5) term needing a prefix-minus display
        // rule), so no display-convention work is needed either.
        known_bug: None,
    },
    Case {
        name: "negation_of_a_free_symbol",
        source: "-x;\n",
        expected: "-x",
        known_bug: Some(
            "Finding one (module doc): compiles to a bare Neg(x) term; there is nothing to fold (x \
             is free, and there is no free-symbol identity law for Neg), but the display-convention \
             gap alone means this prints \"Neg(x)\", never the prefix surface \"-x\".",
        ),
    },

    // --- Comparisons: fold to the True/False SYMBOL on the ground-truth
    // side, then get bridged to Maple's own lowercase surface by
    // maple-runtime::printer -- every one of these hits finding two, even
    // though the underlying fold itself is correct on the compiled side
    // too (finding one). Includes Maple's own `<>` not-equal spelling
    // (neither Reduce's `neq` keyword nor Wolfram's `!=`). ---
    Case {
        name: "comparison_true",
        source: "5 > 3;\n",
        expected: "true",
        known_bug: Some(
            "Finding two (module doc): Greater(5, 3) folds to the True symbol on BOTH sides \
             (finding one's comparisonHandler port), but ground truth prints the Maple-bridged \
             lowercase \"true\" while the compiled side's generic toDisplayString prints the raw \
             symbol name \"True\" -- a case mismatch, not a folding gap.",
        ),
    },
    Case {
        name: "comparison_false",
        source: "3 > 5;\n",
        expected: "false",
        known_bug: Some(
            "Finding two (module doc): same case-mismatch root cause as comparison_true -- folds to \
             False on both sides, prints \"false\" vs \"False\".",
        ),
    },
    Case {
        name: "less_equal_boundary_is_true",
        source: "3 <= 3;\n",
        expected: "true",
        known_bug: Some("Finding two (module doc): case mismatch, same root cause as comparison_true."),
    },
    // `<>` is Maple's own not-equal spelling (MA09 §3) -- distinct from
    // Reduce's `neq` keyword and Wolfram's `!=`.
    Case {
        name: "not_equal_operator_is_true",
        source: "1 <> 2;\n",
        expected: "true",
        known_bug: Some("Finding two (module doc): case mismatch, same root cause as comparison_true."),
    },
    Case {
        name: "not_equal_operator_is_false",
        source: "1 <> 1;\n",
        expected: "false",
        known_bug: Some("Finding two (module doc): case mismatch, same root cause as comparison_false."),
    },
    // `=` is Maple's EQUATION operator (never assignment -- `:=` alone
    // owns that role) and stays symbolic when either side is a free
    // variable.
    Case {
        name: "equation_with_a_free_variable_stays_symbolic",
        source: "x = 4;\n",
        expected: "x = 4",
        known_bug: Some(
            "Display-convention gap (finding one's second half, module doc) -- this one needs no \
             evaluation at all (x is free, so Equal(x, 4) is already the fully-reduced ground-truth \
             value); the compiled side's generic, non-infix Symbolic.toDisplayString prints \
             \"Equal(x, 4)\", never Maple's own infix \"x = 4\".",
        ),
    },

    // --- Logic keywords: and / or / not, including a 3-term `and` chain
    // (the n-ary logical_chain fold) -- every case here folds identically
    // on both sides (finding one) but still hits finding two's case
    // mismatch on the final True/False symbol. ---
    Case {
        name: "and_short_circuits_to_true",
        source: "5 > 3 and 2 < 4;\n",
        expected: "true",
        known_bug: Some("Finding two (module doc): comparison + And folding agree; case mismatch on the result."),
    },
    Case {
        name: "three_term_and_chain_folds_n_ary",
        source: "5 > 3 and 3 > 1 and 1 > 0;\n",
        expected: "true",
        known_bug: Some(
            "Finding two (module doc): comparison + n-ary And folding agree (finding one); case \
             mismatch on the folded True symbol's printed spelling.",
        ),
    },
    Case {
        name: "not_negates_a_true_comparison",
        source: "not (5 > 3);\n",
        expected: "false",
        known_bug: Some("Finding two (module doc): comparison + Not folding agree; case mismatch on the result."),
    },

    // --- Assignment: read back by a LATER statement (held-form gap,
    // finding one). ---
    Case {
        name: "variable_assignment_and_later_reference",
        source: "x := 5;\nx + 1;\n",
        expected: "5\n6",
        known_bug: Some(
            "Finding one (module doc): Assign is a HELD form on the ground-truth side (binds x, \
             displays its value, 5); the compiled side's Assign(x, 5) is inert data -- no binding \
             ever happens -- so the SECOND statement's x is never substituted: it compiles to a bare \
             Add(x, 1) term, still referencing the raw, unbound symbol x, never 6.",
        ),
    },
    Case {
        name: "list_assignment_persists_across_statements",
        source: "v := [1, 2, 3];\nv;\n",
        expected: "[1, 2, 3]\n[1, 2, 3]",
        known_bug: Some(
            "Finding one (module doc): the compiled side's Assign never binds v, so the second \
             statement compiles to the bare, still-unbound symbol v, not the list; and even a bound \
             List(1,2,3) would print via the display-convention gap as \"List(1, 2, 3)\", not \
             \"[1, 2, 3]\".",
        ),
    },

    // --- The arrow-operator `Define` (MA09 §1/§3's own documented trap:
    // this is Maple's GENERAL function-definition spelling -- `f(x) :=
    // expr` is excluded from this subset's grammar entirely, since in
    // real Maple it means something narrower, a remember-table patch). ---
    Case {
        name: "single_param_arrow_function_definition_and_call",
        source: "f := x -> x*x;\nf(5);\n",
        expected: "f\n25",
        known_bug: Some(
            "Finding one (module doc): Define is a HELD form on the ground-truth side (registers f, \
             displays its bare name, \"f\" -- symbolic-vm's define_handler returns Symbol(name)); the \
             compiled side's Define(...) is inert data -- no registration ever happens -- so the \
             SECOND statement compiles to a bare, never-dispatched f(5) call term, not 25.",
        ),
    },
    Case {
        name: "multi_param_arrow_function_definition_and_call",
        source: "g := (x, y) -> x + y;\ng(2, 3);\n",
        expected: "g\n5",
        known_bug: Some(
            "Finding one (module doc): same root cause as single_param_arrow_function_definition_and_\
             call -- Define never registers g, so g(2, 3) never dispatches to 5.",
        ),
    },

    // --- IF: both branches, the no-`else` "unresolved -> false" surface
    // convention (lowercase, per finding two -- confirmed directly by
    // maple-runtime's own if_with_no_else_returns_false_on_a_failing_
    // condition test), the elif right-fold, and the unresolved-condition
    // case where the ground truth reconstructs Maple's own if/then/
    // else/end-if surface. ---
    Case {
        name: "if_true_branch_with_else",
        source: "if 1 > 0 then 42 else 0 end if;\n",
        expected: "42",
        known_bug: Some(
            "Finding one (module doc): compiles to a bare If(Greater(1, 0), 42, 0) term -- If is a \
             HELD form on the ground-truth side (evaluates the condition, then selects a branch); the \
             compiled side never evaluates the condition or selects anything.",
        ),
    },
    Case {
        name: "if_false_branch_with_else",
        source: "if 1 > 2 then 42 else 99 end if;\n",
        expected: "99",
        known_bug: Some(
            "Finding one (module doc): same root cause as if_true_branch_with_else -- compiles to a \
             bare If(Greater(1, 2), 42, 99) term, never selects a branch.",
        ),
    },
    Case {
        name: "if_false_branch_no_else_yields_lowercase_false",
        source: "if 0 > 1 then 42 end if;\n",
        expected: "false",
        known_bug: Some(
            "Finding one AND two (module doc): compiles to a bare If(Greater(0, 1), 42) (2-arg) term \
             -- never selects a branch (finding one) -- and even if it did resolve to the False \
             symbol, the generic toDisplayString would print \"False\", not Maple's own lowercase \
             \"false\" (finding two).",
        ),
    },
    Case {
        name: "elif_chain_evaluates_the_first_true_branch",
        source: "if false then 1 elif true then 2 else 3 end if;\n",
        expected: "2",
        known_bug: Some(
            "Finding one (module doc): the elif chain desugars to a nested If(False, 1, If(True, 2, \
             3)) on both sides (this frontend's own lowering is independently verified against \
             tests/test_lower.rs), but If is held on the compiled side -- neither the outer nor the \
             inner condition is ever evaluated, so this never selects branch 2.",
        ),
    },
    // The ground truth reconstructs Maple's OWN if/then/else/end-if
    // surface via maple-runtime::printer::render_if when the condition
    // stays symbolic (x is free) -- confirmed directly by maple-runtime's
    // own if_unresolved_on_a_free_variable_prints_back_as_if_syntax test.
    // The compiled side's held If passes its ORIGINAL, untouched args
    // through (not even the condition is evaluated, unlike a fully
    // free-standing Greater(x, 0) elsewhere in this corpus), so this
    // stacks a display-convention gap (generic head(args) instead of
    // if/then/else/end-if) on top of finding one -- not a new gap, but
    // worth its own case since the ground-truth side does real,
    // non-trivial reconstruction work here.
    Case {
        name: "if_with_unresolved_condition_reconstructs_maple_surface",
        source: "if x > 0 then 1 else -1 end if;\n",
        expected: "if x > 0 then 1 else -1 end if",
        known_bug: Some(
            "Finding one (module doc): If is held on the compiled side, so its args (Greater(x, 0), \
             1, Neg(1)) are passed through completely untouched -- not even one level of recursion \
             into the condition -- and then rendered generically as \"If(Greater(x, 0), 1, Neg(1))\", \
             nothing like maple-runtime::printer::render_if's reconstructed \"if x > 0 then 1 else -1 \
             end if\" surface.",
        ),
    },

    // --- Lists: flat, singleton, empty, and elementwise-evaluated (MA09
    // §3's ordered, square-bracket `[a, b, c]` -- unlike Derive's own use
    // of the SAME brackets for a vector, MA09 §1's own "same brackets, \
    // different meaning" warning). ---
    Case {
        name: "flat_list_literal",
        source: "[1, 2, 3];\n",
        expected: "[1, 2, 3]",
        known_bug: Some(
            "Display-convention gap (module doc, finding one's second half): a flat List(1, 2, 3) \
             needs no evaluation at all (every element is already a literal), but the compiled side's \
             generic Symbolic.toDisplayString prints \"List(1, 2, 3)\", never Maple's own \
             square-bracket surface \"[1, 2, 3]\".",
        ),
    },
    Case {
        name: "singleton_list_literal",
        source: "[5];\n",
        expected: "[5]",
        known_bug: Some(
            "Display-convention gap only (module doc): List(5) needs no evaluation; prints \
             \"List(5)\", not \"[5]\".",
        ),
    },
    Case {
        name: "empty_list_literal",
        source: "[];\n",
        expected: "[]",
        known_bug: Some(
            "Display-convention gap only (module doc): an empty List() needs no evaluation, but \
             prints \"List()\", not the bracket surface \"[]\".",
        ),
    },
    Case {
        name: "list_of_expressions_evaluates_elementwise",
        source: "[1+1, 2*3, 2^3];\n",
        expected: "[2, 6, 8]",
        // NOT an evaluation gap: List has no handler of its own, but
        // evalTerm's applicative-order argument evaluation still folds
        // each element for free (finding one), confirmed by direct
        // inspection of the emitted JS -- only the bracket display
        // convention is missing.
        known_bug: Some(
            "Display-convention gap ONLY (module doc) -- NOT an evaluation gap: evalTerm's \
             applicative-order argument evaluation folds each List element for free even though List \
             itself has no handler, so this compiles to and evaluates as List(2, 6, 8); only the \
             generic Symbolic.toDisplayString printing \"List(2, 6, 8)\" instead of Maple's own \
             bracket surface \"[2, 6, 8]\" remains.",
        ),
    },

    // --- Sets (finding three, module doc): MA09's own genuinely new-to-
    // this-repo aggregate -- unordered, curly-brace `{a, b, c}` -- unlike
    // Reduce's own use of the SAME brackets for a LIST (MA09 §1's "same
    // brackets, different meaning" warning, the other direction). No
    // `symbolic-vm` handler and no `HANDLERS` entry on EITHER side, but
    // elements still fold on BOTH sides via applicative-order argument
    // evaluation -- confirmed directly against maple-runtime's own
    // set_literal_evaluates_its_elements_but_stays_structurally_unresolved
    // test, which is why these are display-only known_bug cases, not
    // evaluation-gap ones (contrast with Reduce's first/append, which
    // have no shared handler AND leave the ground truth itself
    // unevaluated). ---
    Case {
        name: "flat_set_literal_keeps_duplicates",
        source: "{1, 1, 2};\n",
        expected: "{1, 1, 2}",
        known_bug: Some(
            "Finding three (module doc): Set(1, 1, 2) needs no evaluation (every element is already \
             a literal, and this subset's Set never deduplicates -- real Maple's dedup semantics \
             aren't enforced at evaluation time, confirmed by maple-runtime's own identical test), but \
             the compiled side's generic toDisplayString prints \"Set(1, 1, 2)\", never Maple's own \
             curly-brace surface \"{1, 1, 2}\".",
        ),
    },
    Case {
        name: "empty_set_literal",
        source: "{};\n",
        expected: "{}",
        known_bug: Some(
            "Finding three (module doc): an empty Set() needs no evaluation, but prints \"Set()\", \
             not the curly-brace surface \"{}\".",
        ),
    },
    Case {
        name: "set_of_expressions_evaluates_elementwise",
        source: "{1+1, 2*3};\n",
        expected: "{2, 6}",
        known_bug: Some(
            "Finding three (module doc) -- NOT an evaluation gap: Set has no handler on either side, \
             but applicative-order argument evaluation folds each element for free on BOTH the \
             ground-truth side (confirmed directly by maple-runtime's own set_literal_evaluates_its_\
             elements_but_stays_structurally_unresolved test) and the compiled side (finding one's \
             evalApply fallthrough); only the generic toDisplayString printing \"Set(2, 6)\" instead \
             of Maple's own curly-brace surface \"{2, 6}\" remains.",
        ),
    },
    // Lists and sets of the SAME elements print differently in real
    // Maple (MA09 §1) -- confirmed this genuinely distinguishing case is
    // still a real Maple property the ground truth honors, even though
    // the compiled side's generic printer collapses BOTH to the
    // indistinguishable-looking (but head-labeled) `List(...)`/`Set(...)`
    // shape.
    Case {
        name: "list_and_set_of_the_same_elements_print_differently",
        source: "[1, 2];\n",
        expected: "[1, 2]",
        known_bug: Some(
            "Display-convention gap only (module doc): confirms List's own bracket surface is \
             distinct from Set's (see flat_set_literal_keeps_duplicates's \"{1, 1, 2}\" for the Set \
             analogue of this same element shape) -- the compiled side's generic printer would render \
             both as indistinguishable head(args) shapes but for the (still-different) head name.",
        ),
    },

    // --- diff/int: MA09's own lowercase calculus bridge to the shared
    // D/Integrate handlers (finding one: D/Integrate are not in the
    // compiled side's HANDLERS map at all, so calculus stays unevaluated
    // there, even though the shared symbolic-vm engine really
    // differentiates/integrates on the ground-truth side). ---
    Case {
        name: "diff_evaluates_via_the_shared_derivative_handler",
        source: "diff(x^2, x);\n",
        expected: "2*x",
        known_bug: Some(
            "Finding one (module doc): symbolic-vm's real D handler differentiates x^2 with respect \
             to x on the ground-truth side (confirmed directly by maple-runtime's own identical \
             test); semantic-ir-to-javascript's HANDLERS map has no D entry at all (calculus is \
             SIR23 addendum item 3, not yet landed), so the compiled side's D(Pow(x, 2), x) term \
             stays completely unevaluated, printed generically as \"D(Pow(x, 2), x)\", never \"2*x\".",
        ),
    },
    Case {
        name: "int_evaluates_via_the_shared_integrate_handler",
        source: "int(x, x);\n",
        expected: "1/2*x^2",
        known_bug: Some(
            "Finding one (module doc): same root cause as diff_evaluates_via_the_shared_derivative_\
             handler -- symbolic-vm's real Integrate handler integrates x with respect to x on the \
             ground-truth side (confirmed directly by maple-runtime's own identical test); the \
             compiled side's Integrate(x, x) term stays completely unevaluated (no HANDLERS entry), \
             printed generically as \"Integrate(x, x)\", never \"1/2*x^2\".",
        ),
    },
];

/// Ground truth: run `source` through `maple-runtime`'s own
/// [`maple_eval`]. Like `reduce-to-semantic-ir/tests/oracle.rs`'s own
/// `ground_truth` (and unlike Derive's own numbered-worksheet
/// convention), Maple's own session transcript has **no** numbered-input
/// convention at all (MA09 §5: "matching [Reduce]'s own unnumbered
/// `reduce-repl`") -- confirmed directly by this file's own probe run, so
/// `maple_eval`'s raw output (one plain line per *displayed* statement)
/// is already directly comparable to the compiled side's `console.log`
/// output with no prefix-stripping step needed.
fn ground_truth(source: &str) -> String {
    let raw = maple_eval(source)
        .unwrap_or_else(|e| panic!("maple-runtime eval failed for {source:?}: {e}"));
    raw.trim_end_matches('\n').to_string()
}

/// Wrap every top-level statement's `expr` in the shared `"print"`
/// builtin — see this file's own module doc comment's "A harness-only
/// 'make it observable' step" section for the full rationale. Runs AFTER
/// `semantic_ir::validate` in [`compiled`] below, so validation itself
/// still exercises exactly what `maple_to_semantic_ir::compile_source`
/// actually shipped, unmodified. Mirrors `reduce-to-semantic-ir/tests/
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
/// `maple_to_semantic_ir::compile_source`, `semantic_ir::validate`,
/// [`wrap_top_level_in_print`], `semantic_ir_to_javascript::compile`, and
/// an actual `node` process. Mirrors `reduce-to-semantic-ir/tests/
/// oracle.rs`'s own `compiled` exactly, down to the
/// `OpenOptions::create_new(true)` temp-file handling (that file's own
/// doc comment explains why: `create_new` fails instead of silently
/// following an existing symlink planted at the shared, predictable
/// system temp path -- a deliberate security mitigation, kept exactly
/// as-is here, not weakened to `.create(true)`).
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
    path.push(format!("maple_sir_oracle_{name}_{}.js", std::process::id()));
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
fn oracle_corpus_matches_native_maple_runtime() {
    if !node_available() {
        eprintln!("skipping oracle_corpus_matches_native_maple_runtime: `node` not available");
        return;
    }
    for case in CORPUS {
        let gt = ground_truth(case.source);
        assert_eq!(
            gt, case.expected,
            "{}: maple-runtime itself disagrees with this corpus entry's own `expected` -- the \
             program or `expected` is wrong, fix the corpus rather than this assertion",
            case.name
        );

        match case.known_bug {
            None => {
                let got = compiled(case.name, case.source);
                assert_eq!(
                    got, case.expected,
                    "{}: maple-to-semantic-ir -> semantic-ir-to-javascript -> node disagrees with \
                     the maple-runtime ground truth ({gt:?}) -- see this file's module doc for the \
                     three documented, already-excluded findings (no held-form/calculus evaluation; \
                     no per-language SIR23 display convention, including the Maple-specific True/\
                     False case mismatch; Set's display-only gap) before assuming this is a new one",
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
