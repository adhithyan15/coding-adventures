//! Oracle/golden tests (HML01 §7): the SAME Macsyma source, run through
//! **two independent implementations**, and diffed:
//!
//!   (a) `macsyma-runtime` (`coding-adventures-macsyma-runtime`) — the
//!       sibling native runtime crate, which lowers to `symbolic-ir` and
//!       evaluates via `symbolic-vm`'s shared handler table (through
//!       `MacsymaBackend`, a decorator over the generic `Backend` trait) —
//!       the ground truth.
//!   (b) `macsyma_to_semantic_ir::compile_source` → `semantic_ir::Module`
//!       → `semantic_ir_to_javascript::compile` → an actual `node`
//!       process.
//!
//! Macsyma (and its thin-alias sibling Maxima) was one of the very first
//! math-language frontends in this rollout (predating the oracle-testing
//! convention itself), so — unlike every sibling `-to-semantic-ir` crate —
//! it never got one. This file is that missing harness, modeled directly
//! on [`wolfram-to-semantic-ir`'s own `tests/oracle.rs`](../../wolfram-to-semantic-ir/tests/oracle.rs)
//! (itself modeled on `maple-to-semantic-ir`'s): same overall shape
//! (`node_available` skip-not-fail guard, a `Case`/`CORPUS`, a
//! `ground_truth`/`compiled` pair, one looping `#[test]`, a `known_bug`
//! field for a disagreement rooted in a shared crate rather than this
//! frontend's own lowering) — Macsyma is the CAS sibling closest to
//! Wolfram: both lower straight to SIR23's symbolic vocabulary
//! (`SymApply`/`SymSymbol`) with no host-language binding at lowering
//! time (`lower.rs`'s own "Everything is data" module doc section), and
//! both runtimes evaluate through the exact same shared `symbolic-ir` +
//! `symbolic-vm` stack Wolfram/Derive/Reduce also drive.
//!
//! ## A harness-only "make it observable" step: wrapping every top-level
//! statement in `print`
//!
//! Exactly like `wolfram-to-semantic-ir/tests/oracle.rs`'s own identical
//! section (read that module doc first — this crate's harness is a direct
//! retarget of Wolfram's/Maple's/Reduce's/Derive's): `macsyma_to_semantic_ir::
//! compile_source`'s lowering does **not** wrap a bare top-level
//! expression statement in the shared `"print"` builtin — confirmed
//! directly by this crate's own `tests/e2e_node.rs` module doc comment
//! ("these tests prove instead that every one of these programs compiles
//! to JavaScript that `node` actually executes without throwing... there
//! is no `disp`-equivalent stdout to assert on"). [`wrap_top_level_in_print`]
//! below is the harness-only fix, byte-for-byte the same helper
//! `wolfram-to-semantic-ir`'s/`maple-to-semantic-ir`'s own oracle files use.
//!
//! ## Ground truth via `MacsymaSession::eval_source`'s structured
//! `EvalResult::output_text`, not a hand-rolled formatter
//!
//! `macsyma-runtime`'s own [`MacsymaSession::eval_source`] returns one
//! [`coding_adventures_macsyma_runtime::EvalResult`] per top-level
//! statement, each carrying an `output_text` field already rendered by
//! `cas_pretty_printer::pretty(&output, &MacsymaDialect)` (see
//! `macsyma-runtime/src/lib.rs::display_text_for`). [`ground_truth`] below
//! joins every statement's `output_text` with `"\n"` — directly comparable
//! to the compiled side's `console.log`-per-`print`-call output, no
//! parsing step in between. Unlike Wolfram's own `;`/no-`;` convention
//! (every Wolfram statement displays), Macsyma additionally distinguishes
//! `;` (display) from `$` (suppress) — but that distinction is a pure
//! *runtime/REPL* concept: `macsyma-to-semantic-ir`'s own lowering
//! (`lower.rs::lower_file`) discards the terminator entirely and lowers
//! every top-level statement to a plain `Stmt::ExprStmt` with no
//! display/suppress flag at all (confirmed directly: `lower_file` never
//! inspects `DISPLAY`/`SUPPRESS`; those constants exist only in
//! `macsyma-runtime`, applied by `MacsymaSession::eval_statement`'s own
//! `unwrap_display`, downstream of this frontend entirely). Since
//! [`wrap_top_level_in_print`] wraps *every* top-level statement
//! regardless of its original terminator, every corpus entry below uses
//! `;` throughout so [`ground_truth`] naturally includes every statement's
//! output — sidestepping the (harness-irrelevant) `$`-suppression question
//! rather than needing a second code path to filter it out.
//!
//! ## Findings (confirmed by direct inspection of `semantic-ir-to-
//! javascript`'s `runtime.rs`/`emit.rs`, and of `macsyma-to-semantic-ir`'s
//! own `lower.rs`, not assumed from either crate's scope notes)
//!
//! ### Finding one — the SIR23 JS backend folds arithmetic, comparisons,
//! logic, elementary functions, calculus, AND executes the three held
//! forms (`Assign`/`Define`/`If`) plus user-function dispatch — the exact
//! same shared surface `wolfram-to-semantic-ir/tests/oracle.rs`'s own
//! "finding one" documents (this file does not re-derive it; see that
//! file's module doc for the full four-item addendum list). Concretely:
//! `Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg`/`Inv`/`Abs` fold numerically (with
//! the same identity-law fallbacks — `x+0->x`, `x*1->x`, `x^1->x`,
//! `x^0->1` — `macsyma-runtime`'s own `leaves_symbolic_results_unevaluated_
//! when_needed` unit test already pins the `x+0` half); the six
//! comparisons fold to the `True`/`False` **symbol**; `And`/`Or`/`Not`
//! fold n-ary; `Sin`/`Cos`/`Sqrt` fold their exact-value special cases;
//! `Assign`/`Define`/`If` are real, environment-backed held forms.
//!
//! ### Finding two — Macsyma needs NO `True`/`False` case-bridging (same
//! conclusion as Wolfram's own "finding two", for an independent reason)
//!
//! Macsyma's surface keywords are lowercase (`true`/`false` — see the
//! grammar's `KEYWORD` productions), but BOTH `macsyma-to-semantic-ir::
//! lower::Lowerer::lower_token` (the `KEYWORD if token.value == "true"`
//! arm) AND the native `macsyma-compiler::Compiler::compile_token` (the
//! identical arm, confirmed by direct inspection —
//! `code/packages/rust/macsyma-compiler/src/lib.rs` lines 170-171)
//! canonicalize the surface keyword to the **capitalized** `Symbol("True")`/
//! `Symbol("False")` at lowering time, before either evaluator ever sees
//! it. So both the native pretty-printer (`MacsymaDialect::format_symbol`,
//! a pure pass-through for any name other than `"ImaginaryUnit"`) and the
//! compiled side's hardcoded `"True"`/`"False"` (`comparisonHandler`/
//! `andHandler`/`orHandler`/`notHandler`) already agree, byte-for-byte —
//! confirmed against this crate's own probe run (every comparison/logic
//! case below is `known_bug: None`).
//!
//! ### Finding three — GENUINELY NEW, and the OPPOSITE of Wolfram's own
//! "finding three": `f(x) := body` DOES register a callable function on
//! the compiled side, for every arity tested
//!
//! Wolfram's own oracle file documents a real gap: `wolfram-to-semantic-ir`
//! lowers `f[x_] := body` to a 2-argument `Define(f[x_], body)` call (the
//! WHOLE unsplit LHS as argument one), but the shared JS `defineHandler`
//! (ported byte-for-byte from `symbolic-vm::handlers::define_handler`)
//! requires EXACTLY 3 args, so Wolfram's `f` is never actually registered.
//! Macsyma's own lowering (`lower.rs::Lowerer::lower_assign`, the
//! `COLONEQ` branch) does the opposite: it explicitly splits `f(x) := body`
//! into the 3-argument record `Define(f, List(x), body)` — the function
//! name, an explicit `List` of bare parameter symbols, and the body, as
//! SEPARATE arguments — confirmed directly against this crate's own
//! pinned `tests/test_lower.rs::colon_eq_with_call_shaped_lhs_lowers_to_
//! 3arg_define` test, which asserts exactly this 3-arg shape. This is
//! EXACTLY the shape `defineHandler`/`applyUserFunction`
//! (`semantic-ir-to-javascript/src/runtime.rs`) require, so
//! `function_definition_and_call` below is `known_bug: None` — a genuine,
//! confirmed-by-running-it correctness win this frontend gets "for free"
//! from its own deliberately-more-decomposed `:=` lowering, not something
//! this PR changed.
//!
//! ### Finding four — Macsyma's grammar (v0.1.0) has NO pattern-matching
//! or rewrite-rule surface syntax at all, so Wolfram's own "finding four"
//! (the `SymReplaceAll` missing-refold gap) simply has no analogue here
//!
//! `lower.rs`'s own module doc comment discloses this scope boundary
//! explicitly: no `_`/blank, no `->`/`:>` rule arrow (the lexer tokenizes
//! `ARROW` but no parser rule ever consumes it), no `/.`/`//.` operators.
//! This crate therefore only ever constructs `Expr::SymSymbol`/
//! `Expr::SymApply` (plus literals) — never `SymPatternBlank`/
//! `SymPatternNamed`/`SymRule`/`SymReplaceAll` — so there is no
//! replacement-operator sub-surface for this corpus to exercise at all,
//! unlike Wolfram's own four `/.`/`:>`/`//.` cases.
//!
//! ### Finding five (shared with Wolfram/Maple/every other SIR23 frontend,
//! not re-derived) — no `SIR_DISPLAY_MACSYMA` infix/bracket convention,
//! so almost every NON-atomic result needs `known_bug`
//!
//! The sole SIR23 stringifier, `Symbolic.toDisplayString`, renders every
//! compound term generically as `head(args, ...)` — no infix `+`/`*`/`^`,
//! no `[...]` list-bracket convention (grepping `runtime.rs` for
//! `SIR_DISPLAY_` finds that mechanism wired up only for Ruby/APL/J/
//! Derive/Q, never Macsyma). Macsyma's OWN native pretty-printer
//! (`cas_pretty_printer::MacsymaDialect`), by contrast, is fully
//! infix/bracket-aware (`x^2 + 1`, `[1, 2, 3]`, `sin(x)` — see that
//! crate's own module doc comment's surface-sugar table). So any case
//! whose fully-reduced result is anything other than a plain literal,
//! bare symbol, or the `True`/`False` symbol (all atoms, unaffected by
//! this gap) needs `known_bug` for the bracket/infix mismatch alone, even
//! when the underlying evaluation is completely correct on both sides.
//!
//! #### A surprising sub-case: control-flow-as-data heads (`While`/
//! `ForEach`/`ForRange`/`Block`/`Return`) render IDENTICALLY on both
//! sides, DESPITE being non-atomic — confirmed, not assumed
//!
//! `lower.rs`'s own module doc comment explains these five synthetic
//! heads (Macsyma has no host-language control-flow statement vocabulary
//! at all — see `lower.rs`'s "Retargeting `macsyma-compiler`" section) are
//! plain `SymApply` data with NO registered handler on EITHER side: not
//! in `MacsymaBackend::new`'s handler table (confirmed: its `held` set is
//! `[Assign, Define, If, Kill, Ev, Assume, Forget, Is, Declare,
//! Properties, PropVars, Solve, Subst, Radcan]` — none of the five), and
//! not in the JS backend's `HANDLERS`/`HELD_HANDLERS` maps either. Both
//! `VM::eval_apply` and its JS port therefore evaluate each argument once
//! and rebuild an unevaluated `Apply` term via the generic "unknown head"
//! fallback (`on_unknown_head`/`evalApply`'s own final `applyTerm` line) —
//! identical policy on both sides. Critically, `cas_pretty_printer`'s
//! `MacsymaDialect::function_name` has no override for these synthetic
//! names either (`default_function_name`'s `other => return
//! other.to_string()` fallback leaves `"While"`/`"Return"` unchanged,
//! same capitalization), and the walker's function-call form
//! (`format_call`, `walker.rs`) joins args with `", "` inside `(...)` —
//! the SAME shape the JS `toDisplayString` generic `apply` arm produces.
//! So as long as none of the (evaluated) arguments is itself a `List`
//! (which native's walker DOES special-case — `[1, 2, 3]` — while the JS
//! side's generic `toDisplayString` does not, reopening finding five one
//! level down), a control-flow head's rendering matches verbatim on both
//! sides. `while_head_is_unevaluated_symbolic_data_matching_verbatim` and
//! `return_head_is_unevaluated_symbolic_data_matching_verbatim` below
//! exercise exactly this — both `known_bug: None`, genuinely discovered
//! while building this corpus (not re-derived from Wolfram's file, which
//! has no control-flow grammar to have discovered it with in the first
//! place).
//!
//! ## Maxima coverage — documented finding, not assumed
//!
//! `maxima-to-semantic-ir::src/lib.rs` is a pure re-export of this
//! crate's own `compile`/`compile_source` (`pub use macsyma_to_semantic_ir::
//! {compile, compile_source, MacsymaLowerError};`) — no shim function,
//! confirmed directly by reading the file: "there is no Maxima-specific
//! CST; the only tree ever built is the Macsyma one". And
//! `maxima-runtime::MaximaSession` is a thin façade wrapping this crate's
//! own sibling `macsyma-runtime::MacsymaSession` unchanged — `feed`
//! forwards straight to `MacsymaSession::eval_source` and formats each
//! result's *same* `output_text` (rendered by the *same*
//! `cas_pretty_printer::MacsymaDialect`) behind a `(%oN) text` REPL-echo
//! prefix; no different builtin-name table, no different startup banner,
//! no surface rewriting of any kind (contrast Octave, which needs a real
//! `octavify` source-rewriting shim over `matlab-runtime` for genuine
//! surface departures — `#` comments, `endif`/`endfor`, `!=`/`!`).
//! Concretely: for any source string, `macsyma_runtime::MacsymaSession::
//! eval_source(src)[i].output_text` and
//! `maxima_runtime::MaximaSession::feed(src)`'s per-statement echo body
//! are the SAME string, modulo only the `(%oN) ` REPL-echo prefix
//! `maxima-runtime` adds and macsyma-runtime's `EvalResult` does not (a
//! presentation-layer difference the SIR/JS pipeline never sees either
//! way). A separate Maxima oracle corpus would therefore re-run the exact
//! same evaluator against the exact same lowering and assert the exact
//! same strings — pure duplication, not additional coverage. This file's
//! corpus stands in for both languages; a future Maxima-specific surface
//! departure (a different builtin spelling, say) would be the trigger to
//! split it, not any currently-observed difference.
//!
//! ## Corpus
//!
//! Chosen to exercise Macsyma's own distinctive, already-supported
//! surface (the 24 grammar productions `lower.rs`'s module doc catalogs):
//! literal arithmetic precedence, right-associative `^`, unary minus
//! binding LOOSER than `^` (`-2^2 = -4`, not `4` — same convention as
//! Wolfram, opposite of IDL's), exact-integer vs. genuine-rational
//! division; the `x+0`/`x*1`/`x^1`/`x^0` identity-law simplifications;
//! every comparison (Macsyma's OWN spelling: `=`/`#`/`<`/`>`/`<=`/`>=` —
//! notably `=` is EQUALITY here, not assignment, which is `:`) plus a
//! 3-term `and` chain (the n-ary fold) and `or`/`not` (keyword spellings,
//! not `&&`/`||`/`!`); the elementary-function exact-value folds
//! `sin`/`cos`/`sqrt`; `:` (`Assign`) binding and reading back across
//! statements, including the self-referential-assign loop guard (`x : x`);
//! `:=` (`Define`) and a call (finding three's own genuinely-working
//! case); `if`/`then`/`else` (both branches, plus the no-`else`
//! `False`-fallback case — also genuinely working, since `If` is a proper
//! 3-arg held form on both sides); `[...]` list-literal elementwise
//! evaluation (finding five's own `known_bug`); and two control-flow-as-
//! data cases (finding five's own surprising sub-case, `known_bug: None`
//! despite being non-atomic).

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use coding_adventures_macsyma_runtime::MacsymaSession;
use macsyma_to_semantic_ir::compile_source;
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
/// identical on both the [`ground_truth`] and [`compiled`] sides.
struct Case {
    name: &'static str,
    source: &'static str,
    expected: &'static str,
    /// `None`: both `ground_truth` and `compiled` must equal `expected`.
    /// `Some(reason)`: only `ground_truth` is checked against `expected`;
    /// the `compiled`-side call is skipped entirely (not even invoked),
    /// with `reason` naming which documented finding (module doc) applies
    /// here.
    known_bug: Option<&'static str>,
}

const CORPUS: &[Case] = &[
    // --- Arithmetic: precedence, right-associative `^`, unary minus
    // binding LOOSER than `^`, exact-integer vs. genuine-rational
    // division. Finding one: Add/Sub/Mul/Div/Pow/Neg folding. ---
    Case {
        name: "literal_arithmetic_precedence",
        source: "1 + 2*3;\n",
        expected: "7",
        known_bug: None,
    },
    Case {
        name: "parens_override_precedence",
        source: "(1 + 2)*3;\n",
        expected: "9",
        known_bug: None,
    },
    Case {
        name: "power_is_right_associative",
        // 2^3^2 == 2^(3^2) == 2^9 == 512, NOT (2^3)^2 == 64 -- the SAME
        // right-associative convention Wolfram's own grammar has (see
        // `wolfram-to-semantic-ir/tests/oracle.rs`'s identically-named
        // case), confirmed here independently against
        // `MacsymaDialect::is_right_associative`'s own `head_name ==
        // "Pow"` rule and `lower.rs::lower_power`'s own doc comment.
        source: "2^3^2;\n",
        expected: "512",
        known_bug: None,
    },
    Case {
        name: "unary_minus_binds_looser_than_power",
        // -2^2 -> Neg(Pow(2, 2)) = Neg(4) = -4, NOT (-2)^2 = 4 --
        // confirmed against `cas_pretty_printer::dialect`'s own
        // precedence table (PREC_NEG = 55 < PREC_POW = 60) and
        // `macsyma-to-semantic-ir`'s grammar (`unary = (MINUS|PLUS)
        // unary | power`).
        source: "-2^2;\n",
        expected: "-4",
        known_bug: None,
    },
    Case {
        name: "exact_integer_division_folds_to_an_integer",
        source: "10 / 2;\n",
        expected: "5",
        known_bug: None,
    },
    Case {
        name: "inexact_division_folds_to_a_rational",
        source: "1 / 3;\n",
        expected: "1/3",
        known_bug: None,
    },
    Case {
        name: "negative_integer_literal",
        // Neg's numeric fold produces the plain integer term -5 directly
        // (never a compound Neg(5) term needing a prefix-minus display
        // rule), so no display-convention work is needed either.
        source: "-5;\n",
        expected: "-5",
        known_bug: None,
    },
    Case {
        name: "negation_of_a_free_symbol",
        source: "-x;\n",
        expected: "-x",
        known_bug: Some(
            "Finding five (module doc): compiles to a bare Neg(x) term; there is nothing to fold \
             (x is free, and there is no free-symbol identity law for Neg), but the display-\
             convention gap alone means this prints \"Neg(x)\", never the native prefix surface \"-x\".",
        ),
    },
    // --- Free-symbol identity-law simplifications (finding one). ---
    Case {
        name: "additive_identity_simplifies_a_free_symbol",
        source: "x + 0;\n",
        expected: "x",
        known_bug: None, // Finding one: addHandler's identity-law fallback.
    },
    Case {
        name: "multiplicative_identity_simplifies_a_free_symbol",
        source: "x*1;\n",
        expected: "x",
        known_bug: None, // Finding one: mulHandler's identity-law fallback.
    },
    Case {
        name: "power_identity_exponent_one",
        source: "x^1;\n",
        expected: "x",
        known_bug: None, // Finding one: powHandler's x^1 -> x fallback.
    },
    Case {
        name: "power_identity_exponent_zero",
        source: "x^0;\n",
        expected: "1",
        known_bug: None, // Finding one: powHandler's x^0 -> 1 fallback.
    },
    // --- Comparisons: Macsyma's OWN spelling (`=`/`#`/`<=`/`>=`, NOT
    // `==`/`!=`) -- `=` is equality here, `:` is assignment (tested
    // separately below). Fold to the True/False SYMBOL on both sides,
    // with NO case-bridging needed (finding two) -- every one of these is
    // `known_bug: None`. ---
    Case {
        name: "equality_of_identical_free_symbols",
        // a = a folds to True via the structural-equality fallback even
        // though `a` is a free, unbound symbol on both sides.
        source: "a = a;\n",
        expected: "True",
        known_bug: None,
    },
    Case {
        name: "not_equal_is_true",
        // Macsyma's `#` means "not equal" (an idiosyncrasy of the
        // language -- see `lower.rs::comparison_head`'s own doc comment).
        source: "1 # 2;\n",
        expected: "True",
        known_bug: None,
    },
    Case {
        name: "less_than_is_true",
        source: "1 < 2;\n",
        expected: "True",
        known_bug: None,
    },
    Case {
        name: "greater_than_is_false",
        source: "3 > 5;\n",
        expected: "False",
        known_bug: None,
    },
    Case {
        name: "less_equal_boundary_is_true",
        source: "3 <= 3;\n",
        expected: "True",
        known_bug: None,
    },
    Case {
        name: "greater_equal_is_false",
        source: "3 >= 4;\n",
        expected: "False",
        known_bug: None,
    },
    // --- Logic: and/or/not are KEYWORD spellings in Macsyma, not
    // `&&`/`||`/`!` (n-ary And/Or fold, finding one). ---
    Case {
        name: "three_term_and_chain_folds_n_ary",
        source: "1 < 2 and 2 < 3 and 3 < 4;\n",
        expected: "True",
        known_bug: None,
    },
    Case {
        name: "or_short_circuits_to_true",
        source: "1 > 2 or 3 < 4;\n",
        expected: "True",
        known_bug: None,
    },
    Case {
        name: "not_negates_a_true_comparison",
        source: "not (1 < 2);\n",
        expected: "False",
        known_bug: None,
    },
    // --- Elementary functions: exact-value identity folds (finding one).
    Case {
        name: "sin_of_zero_is_exact_zero",
        source: "sin(0);\n",
        expected: "0",
        known_bug: None,
    },
    Case {
        name: "cos_of_zero_is_exact_one",
        source: "cos(0);\n",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "sqrt_of_a_perfect_square_is_exact",
        source: "sqrt(4);\n",
        expected: "2",
        known_bug: None,
    },
    Case {
        name: "sin_of_a_free_symbol_stays_symbolic",
        source: "sin(x);\n",
        expected: "sin(x)",
        known_bug: Some(
            "Finding five (module doc): Sin(x) has nothing to fold (x is free), but the display-\
             convention gap means this prints \"Sin(x)\", never the native lowercase function-call \
             surface \"sin(x)\" (MacsymaDialect::function_name lowercases well-known builtin heads).",
        ),
    },
    // --- Assign (`:`): a real, environment-backed held form on the
    // compiled side too (finding one, SIR23 addendum item 2) -- binds
    // and reads back across statements, including the self-referential
    // loop guard. ---
    Case {
        name: "assign_binds_and_reads_back_in_a_later_statement",
        source: "x : 5;\nx + 1;\n",
        expected: "5\n6",
        known_bug: None,
    },
    Case {
        name: "self_referential_assign_does_not_infinite_loop",
        // x : x -- the RHS is evaluated BEFORE the binding takes effect
        // (x is still free at that point, so it stays the bare symbol
        // x), then the second statement's lookup hits `evalSymbol`'s own
        // self-loop guard (`termEquals(bound, term)`) instead of
        // recursing forever. The same shared `symbolic-vm`/its JS port
        // Wolfram's own identically-named case exercises.
        source: "x : x;\nx;\n",
        expected: "x\nx",
        known_bug: None,
    },
    // --- Define (`:=`) + call: finding three's own genuinely-WORKING
    // case (the opposite of Wolfram's own known_bug here) -- Macsyma's
    // lowering already produces the 3-arg Define(name, List(params),
    // body) shape the shared JS defineHandler requires. ---
    Case {
        name: "function_definition_and_call",
        // Ground truth: `f(x) := x^2` echoes the bare name "f"
        // (`define_handler`'s own convention -- matches the JS port's
        // identical convention); `f(5)` then dispatches to 25 on BOTH
        // sides (finding three).
        source: "f(x) := x^2;\nf(5);\n",
        expected: "f\n25",
        known_bug: None,
    },
    // --- If/then/else: `If` is a real, environment-backed held form on
    // BOTH sides too (Macsyma's `if_expr` always lowers to the exact
    // 2-or-3-arg shape `ifHandler`/the native `if_handler` expect -- see
    // `lower.rs::lower_if`'s own doc comment). ---
    Case {
        name: "if_then_else_evaluates_the_true_branch",
        source: "if 1 < 2 then 10 else 20;\n",
        expected: "10",
        known_bug: None,
    },
    Case {
        name: "if_then_else_evaluates_the_false_branch",
        source: "if 1 > 2 then 10 else 20;\n",
        expected: "20",
        known_bug: None,
    },
    Case {
        name: "if_with_no_else_and_false_condition_falls_back_to_false_symbol",
        // `if a then b` with no `else` lowers to `If(a, b, False)`
        // (`lower.rs::lower_if`'s own synthetic-False-fallback branch) --
        // a false condition with no else branch evaluates to the bare
        // `False` symbol on both sides.
        source: "if 1 > 2 then 10;\n",
        expected: "False",
        known_bug: None,
    },
    // --- Lists: elementwise evaluation is correct (finding one's
    // argument-evaluation fallthrough), but the bracket display
    // convention is missing (finding five) -- known_bug for display only.
    Case {
        name: "list_literal_evaluates_elementwise",
        source: "[1+1, 2*3, 2^3];\n",
        expected: "[2, 6, 8]",
        known_bug: Some(
            "Finding five (module doc): List(Add(1,1), Mul(2,3), Pow(2,3)) folds its elements \
             correctly (List has no HANDLERS entry, but evalApply's applicative-order argument \
             evaluation folds each element for free, exactly as it does for every other unhandled \
             head) -- this is NOT an evaluation gap. But the compiled side's generic \
             Symbolic.toDisplayString prints \"List(2, 6, 8)\", never Macsyma's own bracket surface \
             \"[2, 6, 8]\" (MacsymaDialect::list_brackets, applied by the native walker's own \
             List-special-case, step 3 of its 6-step dispatch).",
        ),
    },
    // --- Control-flow-as-data: While/ForEach/ForRange/Block/Return have
    // NO registered handler on EITHER side (finding five's own surprising
    // sub-case) -- both sides evaluate args once and rebuild an
    // unevaluated Apply term via the identical "unknown head" fallback,
    // and (as long as no argument is itself a List, which native's walker
    // DOES special-case) the generic function-call rendering matches
    // verbatim on both sides despite the result being non-atomic. ---
    Case {
        name: "while_head_is_unevaluated_symbolic_data_matching_verbatim",
        // While(1 < 2, 5): both args evaluate to atoms (True, 5) on both
        // sides; While itself is never executed as a loop (no handler),
        // so the rebuilt term is While(True, 5) -- rendered identically
        // by the native walker's generic function-call form and the
        // compiled side's generic toDisplayString, since neither has a
        // While-specific display rule.
        source: "while 1 < 2 do 5;\n",
        expected: "While(True, 5)",
        known_bug: None,
    },
    Case {
        name: "return_head_is_unevaluated_symbolic_data_matching_verbatim",
        source: "return(5);\n",
        expected: "Return(5)",
        known_bug: None,
    },
];

/// Ground truth: run `source` through `macsyma-runtime`'s own
/// [`MacsymaSession::eval_source`], joining each statement's
/// [`coding_adventures_macsyma_runtime::EvalResult::output_text`] with
/// `"\n"` -- see this file's module doc comment's "Ground truth via
/// `MacsymaSession::eval_source`'s structured `EvalResult::output_text`"
/// section for why this needs no hand-rolled formatter (the runtime's own
/// `cas_pretty_printer::pretty` call already did that work) and why every
/// corpus entry above uses `;` (display) rather than mixing in `$`
/// (suppress).
fn ground_truth(source: &str) -> String {
    let mut session = MacsymaSession::new();
    let results = session
        .eval_source(source)
        .unwrap_or_else(|e| panic!("macsyma-runtime eval failed for {source:?}: {e}"));
    results
        .into_iter()
        .map(|r| r.output_text)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Wrap every top-level statement's `expr` in the shared `"print"`
/// builtin — see this file's own module doc comment's "A harness-only
/// 'make it observable' step" section for the full rationale. Runs AFTER
/// `semantic_ir::validate` in [`compiled`] below, so validation itself
/// still exercises exactly what `macsyma_to_semantic_ir::compile_source`
/// actually shipped, unmodified. Mirrors `wolfram-to-semantic-ir/tests/
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
/// `macsyma_to_semantic_ir::compile_source`, `semantic_ir::validate`,
/// [`wrap_top_level_in_print`], `semantic_ir_to_javascript::compile`, and
/// an actual `node` process. Mirrors `wolfram-to-semantic-ir/tests/
/// oracle.rs`'s own `compiled` exactly, down to the
/// `OpenOptions::create_new(true)` temp-file handling (that file's own
/// doc comment explains why: `create_new` fails instead of silently
/// following an existing symlink planted at the shared, predictable
/// system temp path — a deliberate security mitigation, kept exactly
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
    path.push(format!(
        "macsyma_sir_oracle_{name}_{}.js",
        std::process::id()
    ));
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
fn oracle_corpus_matches_native_macsyma_runtime() {
    if !node_available() {
        eprintln!("skipping oracle_corpus_matches_native_macsyma_runtime: `node` not available");
        return;
    }

    let mut failures: Vec<String> = Vec::new();

    for case in CORPUS {
        let gt = ground_truth(case.source);
        if gt != case.expected {
            failures.push(format!(
                "{}: macsyma-runtime itself disagrees with this corpus entry's own `expected` (got \
                 {gt:?}, expected {:?}) -- the program or `expected` is wrong, fix the corpus \
                 rather than this assertion",
                case.name, case.expected
            ));
            continue;
        }

        match case.known_bug {
            None => {
                let got = compiled(case.name, case.source);
                if got != case.expected {
                    failures.push(format!(
                        "{}: macsyma-to-semantic-ir -> semantic-ir-to-javascript -> node disagrees \
                         with the macsyma-runtime ground truth (got {got:?}, expected {:?}) -- see \
                         this file's module doc for the five documented findings (arithmetic/\
                         comparison/logic/elementary-function/held-form support; no True/False case \
                         mismatch; Define's matching 3-arg shape; no pattern-matching surface at all; \
                         the generic display convention, including its control-flow-as-data \
                         exception) before assuming this is a new one",
                        case.name, case.expected
                    ));
                }
            }
            Some(reason) => {
                // KNOWN BUG: the compiled-side assertion is deliberately
                // skipped (not even invoked) for this entry -- see this
                // file's module doc comment for why, and `reason` for
                // exactly which documented finding applies here.
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
