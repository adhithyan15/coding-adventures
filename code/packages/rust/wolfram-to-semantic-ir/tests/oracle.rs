//! Oracle/golden tests (HML01 §7): the SAME Wolfram source, run through
//! **two independent implementations**, and diffed:
//!
//!   (a) `wolfram-runtime` (`coding-adventures-wolfram-runtime`) — the
//!       sibling native runtime crate, which lowers to `symbolic-ir` and
//!       evaluates via `symbolic-vm`'s shared handler table (through
//!       `WolframBackend`, a decorator over `SymbolicBackend`) — the
//!       ground truth.
//!   (b) `wolfram_to_semantic_ir::compile_source` → `semantic_ir::Module`
//!       → `semantic_ir_to_javascript::compile` → an actual `node`
//!       process.
//!
//! Wolfram was one of the very first math-language frontends in this
//! rollout (predating the oracle-testing convention itself), so — unlike
//! every sibling `-to-semantic-ir` crate — it never got one. This file is
//! that missing harness, modeled directly on
//! [`maple-to-semantic-ir`'s own `tests/oracle.rs`](../../maple-to-semantic-ir/tests/oracle.rs)
//! (itself the sibling of `reduce-to-semantic-ir`'s/`derive-to-semantic-ir`'s):
//! same overall shape (`node_available` skip-not-fail guard, a
//! `Case`/`CORPUS`, a `ground_truth`/`compiled` pair, one looping
//! `#[test]`, a `known_bug` field for a disagreement rooted in a shared
//! crate rather than this frontend's own lowering) — Wolfram is the CAS
//! sibling closest to Maple: both lower straight to SIR23's symbolic/
//! pattern-matching vocabulary (`SymApply`/`SymPatternBlank`/
//! `SymPatternNamed`/`SymRule`/`SymReplaceAll`) with no host-language
//! binding at lowering time (`lower.rs`'s own "Everything is data"
//! module doc section), and both runtimes evaluate through the exact
//! same shared `symbolic-ir` + `symbolic-vm` stack Macsyma/Derive/Reduce
//! also drive.
//!
//! ## A harness-only "make it observable" step: wrapping every top-level
//! statement in `print`
//!
//! Exactly like `maple-to-semantic-ir/tests/oracle.rs`'s own identical
//! section (read that module doc first — this crate's harness is a
//! direct retarget of Maple's/Reduce's/Derive's): `wolfram_to_semantic_ir::
//! compile_source`'s lowering does **not** wrap a bare top-level
//! expression statement in the shared `"print"` builtin — confirmed
//! directly by this crate's own `tests/e2e_node.rs` module doc comment
//! ("nothing is ever printed... there is no `disp`-equivalent output to
//! assert on"). [`wrap_top_level_in_print`] below is the harness-only fix,
//! byte-for-byte the same helper `maple-to-semantic-ir`'s/
//! `reduce-to-semantic-ir`'s own oracle files use.
//!
//! ## Ground truth via the structured `Output` API, not string-prefix
//! stripping
//!
//! `wolfram-runtime`'s own `WolframSession::feed` renders each displayed
//! result as `"Out[n]= «value»\n"` (the numbered-worksheet convention —
//! see `WolframSession::feed`'s own doc comment). Rather than regex- or
//! string-stripping that prefix (fragile: a value that itself happens to
//! contain the substring `"Out["` would be a foot-gun), [`ground_truth`]
//! below uses the lower-level, already-structured
//! [`WolframSession::eval_to_outputs`] and joins each [`Output::text`]
//! (the bare rendered value, no prefix at all) with `"\n"` — directly
//! comparable to the compiled side's `console.log`-per-`print`-call
//! output, with no parsing step in between.
//!
//! ## Findings (confirmed by direct inspection of `semantic-ir-to-
//! javascript`'s `runtime.rs`/`emit.rs`, not assumed from this frontend's
//! own scope notes, which predate all four now-shipped SIR23 addendum
//! items)
//!
//! ### Finding one — the SIR23 JS backend now folds arithmetic,
//! comparisons, logic, elementary functions, calculus, AND executes the
//! three held forms (`Assign`/`Define`/`If`) plus user-function dispatch
//!
//! All four addendum items are shipped as of this PR (`runtime.rs`'s own
//! "Scope: items 1-4 of 4 ... all four now shipped" comment) — a much
//! more complete surface than `maple-to-semantic-ir/tests/oracle.rs` saw
//! when IT was written (that file's own module doc explicitly says item 2
//! "not yet landed", forcing nearly every one of its cases to
//! `known_bug: Some`). Concretely: `Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg`/
//! `Inv`/`Abs` fold numerically (with the same identity-law fallbacks —
//! `x+0->x`, `x*1->x`, `x^1->x`, `x^0->1` — `wolfram-runtime`'s own unit
//! tests already pin); the six comparisons fold to the `True`/`False`
//! **symbol**; `And`/`Or`/`Not` fold n-ary; `Sin`/`Cos`/`Sqrt` fold their
//! exact-value special cases (`Sin(0)->0`, `Cos(0)->1`, `Sqrt(4)->2`);
//! `Assign`/`Define`/`If` are real, environment-backed held forms, and a
//! `Define`-registered function dispatches by positional substitution.
//!
//! ### Finding two — Wolfram needs NO `True`/`False` case-bridging (unlike
//! Maple)
//!
//! `maple-to-semantic-ir`'s own "finding two" was that Maple's native
//! surface spells booleans lowercase (`true`/`false`), while the shared
//! JS backend's comparison/logic handlers always fold to the
//! **capitalized** `True`/`False` symbol (Wolfram's own convention) — a
//! genuine case mismatch for Maple, on every comparison/logic case.
//! Wolfram has no such gap at all: `wolfram-runtime::printer` renders the
//! `True`/`False` **symbol** completely generically (`IRNode::Symbol(s) =>
//! s.clone()`, `printer.rs`), so the native side ALREADY prints
//! capitalized `True`/`False`, byte-identical to the compiled side's
//! hardcoded spelling. Confirmed directly against this crate's own
//! `wolfram-runtime` probe run (every comparison/logic case below is
//! `known_bug: None`, a strictly larger `known_bug: None` set for this
//! sub-surface than Maple's own oracle file has).
//!
//! ### Finding three — GENUINELY NEW: `SetDelayed`/`Define` never
//! registers a callable function on the compiled side, for ANY arity
//! (not a display gap — a real arg-SHAPE mismatch between this frontend's
//! lowering and the shared JS handler)
//!
//! `wolfram-runtime`'s own lowering (`wolfram-runtime/src/lower.rs::
//! lower_assignment`, the `SETDELAYED` branch) does NOT lower `f[x_] :=
//! body` as a bare 2-argument `Define(f[x_], body)`: it explicitly
//! reduces the LHS `Apply` into the native `symbolic-vm::define_handler`'s
//! expected 3-argument record shape, `Define(f, List(x), body)` — via its
//! own `param_binding_symbol` helper, which strips each parameter's
//! pattern wrapper (`Pattern(x, Blank())` → bare `x`) so the VM's
//! `apply_user_function` can zip parameters by plain symbol name.
//! `wolfram-to-semantic-ir`'s own lowering (`src/lower.rs::
//! lower_assignment`) does NOT do this: per its own "Everything is data"
//! design (full fidelity — no lowering-time destructuring of the LHS at
//! all, so e.g. a parameter's type constraint `x_Integer` is preserved
//! rather than silently dropped), it always emits a plain 2-argument
//! `Define(lhs, rhs)` where `lhs` is the WHOLE, unsplit `f[x_]` apply
//! term — confirmed directly against this crate's own pinned
//! `tests/test_lower.rs::setdelayed_lowers_to_define_apply` test, which
//! asserts exactly this 2-arg shape. But `semantic-ir-to-javascript`'s
//! shared `defineHandler` (`runtime.rs`, ported byte-for-byte from
//! `symbolic-vm::handlers::define_handler`, which every SIR23 CAS
//! frontend's ground truth shares) requires EXACTLY 3 args — `if
//! (args.length !== 3) { return applyTerm(head, args); }` — so THIS
//! frontend's 2-arg `Define` call is always left completely unevaluated
//! on the compiled side (no callable ever gets registered in `symEnv`,
//! regardless of how many parameters `f` declares), while the identical
//! source evaluates correctly end-to-end natively. Not a bug in the JS
//! backend (which faithfully implements the shared, 3-arg contract every
//! OTHER frontend's lowering already honors) and not something this test
//! file changes: fixing it would mean changing this frontend's own
//! documented, deliberately-full-fidelity `Define` lowering (and its
//! pinned unit test) — out of scope for an oracle-test-only PR. Recorded
//! here as a `known_bug`, exactly like Maple's own held-form gaps were
//! before item 2 landed.
//!
//! ### Finding four — GENUINELY NEW: `SymReplaceAll` (`/.`/`//.`)
//! performs substitution ONLY on the compiled side; it never re-folds
//! the substituted result through `evalTerm`
//!
//! Confirmed directly against `emit.rs`: a `SymReplaceAll` node's own
//! codegen arm emits exactly `__Sir.Symbolic.unwrap(__Sir.Symbolic.
//! replaceAll(expr, rules))` (or `replaceRepeated`) — no `evalTerm` call
//! anywhere in it — and `emit_stmt`'s `is_sym23_root_shape`/
//! `pick_print_of_sym23_root` gate (which decides whether a top-level
//! statement, or this harness's own `print(...)` wrapper, gets an
//! `evalTerm` wrap) recognizes only `SymApply`/`SymSymbol`/`SymRational`
//! — **not** `SymReplaceAll`. So when a rule's substituted right-hand
//! side is itself a compound arithmetic expression needing further
//! folding (e.g. `h[3] /. h[n_] :> n + 1` substitutes `n -> 3` into `n +
//! 1`, producing the STILL-UNFOLDED term `Add(3, 1)`), nothing in the
//! compiled pipeline ever reduces it to `4`. Native `wolfram-runtime`, by
//! contrast, ALWAYS re-evaluates the whole statement through the VM after
//! the substitution pre-pass (`apply_replace_all` then `vm.eval(prepared)`
//! in `wolfram-runtime/src/lib.rs::eval_source`), so it folds to `4`
//! correctly. This is invisible whenever the substituted RHS is *already*
//! atomic post-substitution (a literal, or a bare captured symbol/value —
//! see `replace_all_with_a_literal_rule`/`replace_all_captures_and_
//! returns_the_bound_value`/`replace_repeated_chases_a_rule_chain_to_a_
//! fixed_point` below, all `known_bug: None`), which is why this gap
//! wasn't caught by `tests/sir23_symbolic.rs`'s own hand-built SIR23
//! tests either (every one of THOSE rules' RHS's happen to be a bare
//! pattern-capture reference, never a compound arithmetic expression) —
//! genuinely discovered while building this corpus, not re-derived from
//! an existing finding.
//!
//! ### Finding five (shared with Maple/every other SIR23 frontend, not
//! re-derived): no `SIR_DISPLAY_WOLFRAM` bracket/infix convention
//!
//! Exactly like Maple's own "finding one's second half": the sole SIR23
//! stringifier, `Symbolic.toDisplayString`, renders every compound term
//! generically as `head(args, ...)` — no infix `+`/`*`/`^`, no `{...}`
//! list-bracket convention, no `f[...]` square-bracket application
//! convention (Wolfram's own, distinct from Maple's/Reduce's `f(...)`-free
//! generic fallback only by coincidence of surface, not by any
//! Wolfram-specific display flag — grepping `runtime.rs` for
//! `SIR_DISPLAY_` finds that mechanism wired up only for Ruby/APL/J/
//! Derive/Q, never Wolfram). So any case whose fully-reduced result is
//! anything other than a plain literal, bare symbol, or the `True`/
//! `False` symbol (all atoms, unaffected by this gap) needs `known_bug`
//! for the bracket/infix mismatch alone, even when the underlying
//! evaluation is completely correct on both sides (`List`'s own
//! elementwise fold, `Neg`/`Sin` on a free symbol).
//!
//! ## Corpus
//!
//! Chosen to exercise Wolfram's own distinctive, already-supported
//! surface (MA04): literal arithmetic precedence, right-associative `^`
//! (opposite of IDL's left-assoc), unary minus binding LOOSER than `^`
//! (`-2^2 = -4`, not `4`); exact-integer vs. genuine-rational division;
//! the `x+0`/`x*1`/`x^1`/`x^0` identity-law simplifications pulled
//! straight from `wolfram-runtime`'s own `symbolic_evaluation` unit test;
//! every comparison (`==`/`!=`/`<`/`>`/`<=`/`>=`) plus a 3-term `&&` chain
//! (the n-ary fold) and `||`/`!`; the elementary-function exact-value
//! folds `Sin`/`Cos`/`Sqrt`; `=` (`Assign`) binding and reading back
//! across statements, including the self-referential-assign loop guard
//! (`x = x`); `:=` (`Define`) and a call (finding three's own
//! `known_bug`); `{...}` list-literal elementwise evaluation (finding
//! five's own `known_bug`); and `/.`/`:>`/`//.` — a literal rule, a
//! pattern-capture rule (both fully substitution-resolved, no arithmetic
//! refold needed, hence `known_bug: None` despite finding four), a
//! `RuleDelayed` rule whose RHS DOES need an arithmetic refold (finding
//! four's own `known_bug`), and a `//.` fixed-point chase through two
//! rules to convergence.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use coding_adventures_wolfram_runtime::WolframSession;
use semantic_ir::{EffectSet, Expr, Module, Stmt};
use wolfram_to_semantic_ir::compile_source;

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
    /// with `reason` naming which documented finding (module doc,
    /// findings three/four/five) applies here.
    known_bug: Option<&'static str>,
}

const CORPUS: &[Case] = &[
    // --- Arithmetic: precedence, right-associative `^`, unary minus
    // binding LOOSER than `^`, exact-integer vs. genuine-rational
    // division. Finding one: Add/Sub/Mul/Div/Pow/Neg folding. ---
    Case {
        name: "literal_arithmetic_precedence",
        source: "1 + 2*3\n",
        expected: "7",
        known_bug: None,
    },
    Case {
        name: "parens_override_precedence",
        source: "(1 + 2)*3\n",
        expected: "9",
        known_bug: None,
    },
    Case {
        name: "power_is_right_associative",
        // 2^3^2 == 2^(3^2) == 2^9 == 512, NOT (2^3)^2 == 64 -- the
        // OPPOSITE of IDL's left-associative `^` (see
        // `idl-to-semantic-ir/tests/oracle.rs`'s own identical-looking
        // but oppositely-associating case).
        source: "2^3^2\n",
        expected: "512",
        known_bug: None,
    },
    Case {
        name: "unary_minus_binds_looser_than_power",
        // -2^2 -> Neg(Pow(2, 2)) = Neg(4) = -4, NOT (-2)^2 = 4 --
        // confirmed against `wolfram-runtime`'s own grammar precedence
        // (`unary = (MINUS|PLUS) unary | power`).
        source: "-2^2\n",
        expected: "-4",
        known_bug: None,
    },
    Case {
        name: "exact_integer_division_folds_to_an_integer",
        source: "10 / 2\n",
        expected: "5",
        known_bug: None,
    },
    Case {
        name: "inexact_division_folds_to_a_rational",
        source: "1 / 3\n",
        expected: "1/3",
        known_bug: None,
    },
    Case {
        name: "negative_integer_literal",
        // Neg's numeric fold produces the plain integer term -5 directly
        // (never a compound Neg(5) term needing a prefix-minus display
        // rule), so no display-convention work is needed either.
        source: "-5\n",
        expected: "-5",
        known_bug: None,
    },
    Case {
        name: "negation_of_a_free_symbol",
        source: "-x\n",
        expected: "-x",
        known_bug: Some(
            "Finding five (module doc): compiles to a bare Neg(x) term; there is nothing to fold \
             (x is free, and there is no free-symbol identity law for Neg), but the display-\
             convention gap alone means this prints \"Neg(x)\", never the native prefix surface \"-x\".",
        ),
    },
    // --- Free-symbol identity-law simplifications, pulled straight from
    // `wolfram-runtime`'s own `symbolic_evaluation` unit test. ---
    Case {
        name: "additive_identity_simplifies_a_free_symbol",
        source: "x + 0\n",
        expected: "x",
        known_bug: None, // Finding one: addHandler's identity-law fallback.
    },
    Case {
        name: "multiplicative_identity_simplifies_a_free_symbol",
        source: "x*1\n",
        expected: "x",
        known_bug: None, // Finding one: mulHandler's identity-law fallback.
    },
    Case {
        name: "power_identity_exponent_one",
        source: "x^1\n",
        expected: "x",
        known_bug: None, // Finding one: powHandler's x^1 -> x fallback.
    },
    Case {
        name: "power_identity_exponent_zero",
        source: "x^0\n",
        expected: "1",
        known_bug: None, // Finding one: powHandler's x^0 -> 1 fallback.
    },
    // --- Comparisons: fold to the True/False SYMBOL on both sides, with
    // NO case-bridging needed for Wolfram (finding two) -- every one of
    // these is `known_bug: None`, unlike Maple's own equivalent cases. ---
    Case {
        name: "less_than_is_true",
        source: "1 < 2\n",
        expected: "True",
        known_bug: None,
    },
    Case {
        name: "greater_than_is_false",
        source: "3 > 5\n",
        expected: "False",
        known_bug: None,
    },
    Case {
        name: "equality_of_identical_free_symbols",
        // a == a folds to True via the eq_based structural-equality
        // fallback (`comparisonHandler`'s own `termEquals` branch) even
        // though `a` is a free, unbound symbol on both sides.
        source: "a == a\n",
        expected: "True",
        known_bug: None,
    },
    Case {
        name: "not_equal_is_true",
        source: "1 != 2\n",
        expected: "True",
        known_bug: None,
    },
    Case {
        name: "less_equal_boundary_is_true",
        source: "3 <= 3\n",
        expected: "True",
        known_bug: None,
    },
    // --- Logic: &&/||/! (n-ary And/Or fold, finding one). ---
    Case {
        name: "three_term_and_chain_folds_n_ary",
        source: "1 < 2 && 2 < 3 && 3 < 4\n",
        expected: "True",
        known_bug: None,
    },
    Case {
        name: "or_short_circuits_to_true",
        source: "1 > 2 || 3 < 4\n",
        expected: "True",
        known_bug: None,
    },
    Case {
        name: "not_negates_a_true_comparison",
        source: "!(1 < 2)\n",
        expected: "False",
        known_bug: None,
    },
    // --- Elementary functions: exact-value identity folds (finding one).
    Case {
        name: "sin_of_zero_is_exact_zero",
        source: "Sin[0]\n",
        expected: "0",
        known_bug: None,
    },
    Case {
        name: "cos_of_zero_is_exact_one",
        source: "Cos[0]\n",
        expected: "1",
        known_bug: None,
    },
    Case {
        name: "sqrt_of_a_perfect_square_is_exact",
        source: "Sqrt[4]\n",
        expected: "2",
        known_bug: None,
    },
    Case {
        name: "sin_of_a_free_symbol_stays_symbolic",
        source: "Sin[x]\n",
        expected: "Sin[x]",
        known_bug: Some(
            "Finding five (module doc): Sin(x) has nothing to fold (x is free), but the display-\
             convention gap means this prints \"Sin(x)\", never the native square-bracket surface \
             \"Sin[x]\".",
        ),
    },
    // --- Assign (`=`): a real, environment-backed held form on the
    // compiled side too (finding one, SIR23 addendum item 2) -- binds
    // and reads back across statements, including the self-referential
    // loop guard. ---
    Case {
        name: "assign_binds_and_reads_back_in_a_later_statement",
        source: "x = 5\nx + 1\n",
        expected: "5\n6",
        known_bug: None,
    },
    Case {
        name: "self_referential_assign_does_not_infinite_loop",
        // x = x -- the RHS is evaluated BEFORE the binding takes effect
        // (x is still free at that point, so it stays the bare symbol
        // x), then the second statement's lookup hits `evalSymbol`'s own
        // self-loop guard (`termEquals(bound, term)`) instead of
        // recursing forever.
        source: "x = x\nx\n",
        expected: "x\nx",
        known_bug: None,
    },
    // --- Define (`:=`) + call: finding three's own known_bug -- the
    // compiled side never registers a callable, for ANY arity. ---
    Case {
        name: "function_definition_and_call",
        // Ground truth: `f[x_] := x^2` echoes the bare name "f" (native
        // symbolic-vm's own define_handler convention, matching the JS
        // port's identical convention where IT does get a well-shaped
        // Define call); `f[5]` then dispatches to 25.
        source: "f[x_] := x^2\nf[5]\n",
        expected: "f\n25",
        known_bug: Some(
            "Finding three (module doc): wolfram-runtime's own lowering reduces `f[x_] := body`'s \
             LHS into the 3-arg Define(f, List(x), body) record symbolic-vm's define_handler (and \
             its JS port) require; this frontend's own lowering (test_lower.rs::setdelayed_lowers_\
             to_define_apply, a pinned test) deliberately keeps the LHS as one unsplit f[x_] apply \
             term instead, emitting a 2-arg Define(f[x_], body) call. The JS backend's defineHandler \
             requires exactly 3 args, so this 2-arg call is always left completely unevaluated \
             (never binds f in symEnv, regardless of parameter count) -- f[5] then never dispatches, \
             staying an unevaluated f(5) application rather than folding to 25.",
        ),
    },
    // --- Lists: elementwise evaluation is correct (finding one's
    // argument-evaluation fallthrough), but the bracket display
    // convention is missing (finding five) -- known_bug for display only.
    Case {
        name: "list_literal_evaluates_elementwise",
        source: "{1+1, 2*3, 2^3}\n",
        expected: "{2, 6, 8}",
        known_bug: Some(
            "Finding five (module doc): List(Add(1,1), Mul(2,3), Pow(2,3)) folds its elements \
             correctly (List has no HANDLERS entry, but evalApply's applicative-order argument \
             evaluation folds each element for free, exactly as it does for every other unhandled \
             head) -- this is NOT an evaluation gap. But the compiled side's generic \
             Symbolic.toDisplayString prints \"List(2, 6, 8)\", never Wolfram's own curly-brace \
             surface \"{2, 6, 8}\".",
        ),
    },
    // --- Replacement: `/.`/`:>`/`//.`. Finding four: SymReplaceAll never
    // re-folds a substituted RHS through evalTerm -- these first three
    // cases are `known_bug: None` anyway because their substituted RHS is
    // ALREADY atomic (no fold needed); the fourth explicitly exercises
    // the gap. ---
    Case {
        name: "replace_all_with_a_literal_rule",
        // x -> 9: a plain structural-equality match (x is a bare symbol
        // pattern, not a Blank), substituting the already-atomic literal
        // 9 -- no arithmetic refold needed, so finding four never bites.
        source: "x /. x -> 9\n",
        expected: "9",
        known_bug: None,
    },
    Case {
        name: "replace_all_captures_and_returns_the_bound_value",
        // f[y] /. f[t_] -> t: t_ captures y (an unconstrained Blank), and
        // the RHS is the bare pattern-reference t itself -- substitution
        // alone yields the already-atomic bound value y, no further fold
        // needed.
        source: "f[y] /. f[t_] -> t\n",
        expected: "y",
        known_bug: None,
    },
    Case {
        name: "rule_delayed_rhs_needs_arithmetic_refold",
        // h[3] /. h[n_] :> n + 1: n_ captures 3, but the RHS "n + 1"
        // substitutes to the STILL-UNFOLDED term Add(3, 1) -- exactly
        // finding four's gap. Ground truth folds this all the way to 4
        // (wolfram-runtime always re-evaluates the whole statement after
        // the substitution pre-pass); the compiled side never does.
        source: "h[3] /. h[n_] :> n + 1\n",
        expected: "4",
        known_bug: Some(
            "Finding four (module doc): SymReplaceAll's compiled-side substitution (matchPattern + \
             substituteTerm) is never followed by an evalTerm re-evaluation pass -- neither \
             SymReplaceAll's own codegen nor the print-wrapping is_sym23_root_shape gate (which \
             recognises only SymApply/SymSymbol/SymRational) wraps it in one. So the substituted \
             RHS \"n + 1\" with n bound to 3 becomes the literal term Add(3, 1), never re-folded to \
             4, while native wolfram-runtime always re-evaluates the whole statement via vm.eval \
             after its own substitution pre-pass.",
        ),
    },
    Case {
        name: "replace_repeated_chases_a_rule_chain_to_a_fixed_point",
        // a //. {a -> b, b -> c}: each step's RHS is already an atomic
        // bare symbol (b, then c) -- pure substitution alone reaches the
        // fixed point c with no arithmetic refold ever needed, so finding
        // four never bites here either.
        source: "a //. {a -> b, b -> c}\n",
        expected: "c",
        known_bug: None,
    },
];

/// Ground truth: run `source` through `wolfram-runtime`'s own
/// [`WolframSession::eval_to_outputs`], joining each displayed
/// [`coding_adventures_wolfram_runtime::Output::text`] with `"\n"` -- see
/// this file's module doc comment's "Ground truth via the structured
/// `Output` API" section for why this is used instead of stripping
/// `WolframSession::feed`'s `"Out[n]= "` string prefix.
fn ground_truth(source: &str) -> String {
    let outputs = WolframSession::new()
        .eval_to_outputs(source)
        .unwrap_or_else(|e| panic!("wolfram-runtime eval failed for {source:?}: {e}"));
    outputs
        .into_iter()
        .map(|o| o.text)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Wrap every top-level statement's `expr` in the shared `"print"`
/// builtin — see this file's own module doc comment's "A harness-only
/// 'make it observable' step" section for the full rationale. Runs AFTER
/// `semantic_ir::validate` in [`compiled`] below, so validation itself
/// still exercises exactly what `wolfram_to_semantic_ir::compile_source`
/// actually shipped, unmodified. Mirrors `maple-to-semantic-ir/tests/
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
/// `wolfram_to_semantic_ir::compile_source`, `semantic_ir::validate`,
/// [`wrap_top_level_in_print`], `semantic_ir_to_javascript::compile`, and
/// an actual `node` process. Mirrors `maple-to-semantic-ir/tests/
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
        "wolfram_sir_oracle_{name}_{}.js",
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
fn oracle_corpus_matches_native_wolfram_runtime() {
    if !node_available() {
        eprintln!("skipping oracle_corpus_matches_native_wolfram_runtime: `node` not available");
        return;
    }

    let mut failures: Vec<String> = Vec::new();

    for case in CORPUS {
        let gt = ground_truth(case.source);
        if gt != case.expected {
            failures.push(format!(
                "{}: wolfram-runtime itself disagrees with this corpus entry's own `expected` (got \
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
                        "{}: wolfram-to-semantic-ir -> semantic-ir-to-javascript -> node disagrees \
                         with the wolfram-runtime ground truth (got {got:?}, expected {:?}) -- see \
                         this file's module doc for the five documented findings (arithmetic/\
                         comparison/logic/elementary-function/held-form support; no True/False case \
                         mismatch; Define's 2-arg/3-arg shape mismatch; SymReplaceAll's missing \
                         refold; the generic display convention) before assuming this is a new one",
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
