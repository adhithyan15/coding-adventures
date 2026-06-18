//! End-to-end integration tests for the Wolfram runtime.
//!
//! These exercise the *whole* pipeline — parse → lower → evaluate (via the shared
//! `symbolic-vm`) → pretty-print — through the public [`WolframSession`] /
//! [`eval`] surface, mirroring how a REPL or embedder uses the crate. The unit
//! tests inside `src/` pin the individual stages; these assert the observable
//! string-in / string-out contract a user sees.

use coding_adventures_wolfram_runtime::{eval, WolframSession};

/// A representative `1 + 2*3 → 7` end-to-end (the canonical "arithmetic works"
/// check from the W-4 brief).
#[test]
fn arithmetic_precedence_end_to_end() {
    assert_eq!(eval("1 + 2*3\n").unwrap(), "Out[1]= 7\n");
    assert_eq!(eval("(1 + 2)*3\n").unwrap(), "Out[1]= 9\n");
    assert_eq!(eval("2^10\n").unwrap(), "Out[1]= 1024\n");
}

/// The explicit head-applications evaluate identically to their infix forms —
/// the head-name bridge (Plus→Add, Times→Mul, Power→Pow) is the crux of W-4.
#[test]
fn head_applications_match_infix() {
    assert_eq!(eval("Plus[1, 2, 3]\n").unwrap(), "Out[1]= 6\n");
    assert_eq!(eval("Times[2, 3, 4]\n").unwrap(), "Out[1]= 24\n");
    assert_eq!(eval("Power[2, 10]\n").unwrap(), "Out[1]= 1024\n");
    assert_eq!(eval("Subtract[10, 3]\n").unwrap(), "Out[1]= 7\n");
}

/// Symbols stay symbolic; algebraic identities fold (the `SymbolicBackend`).
#[test]
fn symbolic_evaluation() {
    assert_eq!(eval("x + 0\n").unwrap(), "Out[1]= x\n");
    assert_eq!(eval("x*1\n").unwrap(), "Out[1]= x\n");
    assert_eq!(eval("x^1\n").unwrap(), "Out[1]= x\n");
    assert_eq!(eval("x^0\n").unwrap(), "Out[1]= 1\n");
}

/// List literals evaluate element-wise and round-trip to brace notation.
#[test]
fn lists() {
    assert_eq!(eval("{1, 2, 3}\n").unwrap(), "Out[1]= {1, 2, 3}\n");
    assert_eq!(eval("{1 + 1, 2*2, 3^2}\n").unwrap(), "Out[1]= {2, 4, 9}\n");
    assert_eq!(eval("{}\n").unwrap(), "Out[1]= {}\n");
    assert_eq!(
        eval("{1, {2, 3}, 4}\n").unwrap(),
        "Out[1]= {1, {2, 3}, 4}\n"
    );
}

/// Built-in elementary functions reach the shared handlers.
#[test]
fn elementary_functions() {
    assert_eq!(eval("Sin[0]\n").unwrap(), "Out[1]= 0\n");
    assert_eq!(eval("Cos[0]\n").unwrap(), "Out[1]= 1\n");
    // An unknown head passes through unevaluated (Mathematica semantics).
    assert_eq!(eval("g[1, 2]\n").unwrap(), "Out[1]= g[1, 2]\n");
}

/// Assignment binds in the session env and persists across `feed` calls.
#[test]
fn stateful_assignment() {
    let mut s = WolframSession::new();
    assert_eq!(s.feed("a = 10\n").unwrap(), "Out[1]= 10\n");
    assert_eq!(s.feed("b = a + 5\n").unwrap(), "Out[2]= 15\n");
    assert_eq!(s.feed("a*b\n").unwrap(), "Out[3]= 150\n");
}

/// A user-defined function via `:=`, then applied.
#[test]
fn user_functions() {
    let mut s = WolframSession::new();
    s.feed("cube[x_] := x^3;\n").unwrap();
    assert_eq!(s.feed("cube[2]\n").unwrap(), "Out[2]= 8\n");
    assert_eq!(s.feed("cube[5]\n").unwrap(), "Out[3]= 125\n");
}

/// `/.` replacement with single rules, list rules, and pattern rules.
#[test]
fn replace_all() {
    assert_eq!(eval("x /. x -> 9\n").unwrap(), "Out[1]= 9\n");
    assert_eq!(
        eval("{a, b} /. {a -> 1, b -> 2}\n").unwrap(),
        "Out[1]= {1, 2}\n"
    );
    // A pattern rule that captures and reuses the bound name on the RHS.
    assert_eq!(eval("h[5] /. h[n_] -> n + 1\n").unwrap(), "Out[1]= 6\n");
}

/// Comparisons and logic fold when fully numeric, stay symbolic otherwise.
#[test]
fn comparisons_and_logic() {
    // `1 < 2` folds to True; an open comparison stays symbolic.
    let out = eval("1 < 2\n").unwrap();
    assert!(out.contains("True"), "got {out:?}");
    assert_eq!(eval("a == a\n").unwrap(), "Out[1]= True\n");
}

/// Multi-statement feed numbers `Out` in order; `;` suppresses display.
#[test]
fn output_numbering_and_suppression() {
    let mut s = WolframSession::new();
    assert_eq!(s.feed("1\n2\n").unwrap(), "Out[1]= 1\nOut[2]= 2\n");
    // A suppressed statement consumes an Out index but prints nothing.
    assert_eq!(s.feed("3;\n").unwrap(), "");
    assert_eq!(s.feed("4\n").unwrap(), "Out[4]= 4\n");
}

/// Errors are returned cleanly (never a panic) and the session survives.
#[test]
fn errors_are_clean_and_recoverable() {
    let mut s = WolframSession::new();
    assert!(s.feed("1 +\n").is_err());
    assert!(s.feed("f[x\n").is_err()); // unclosed bracket
    assert_eq!(s.feed("2 + 2\n").unwrap(), "Out[1]= 4\n");
}

/// A short program exercising several features at once.
#[test]
fn a_small_program() {
    let mut s = WolframSession::new();
    s.feed("area[r_] := Pi*r^2;\n").unwrap();
    s.feed("nums = {1, 2, 3};\n").unwrap();
    let out = s.feed("Power[2, 5] + Times[3, 4]\n").unwrap();
    // 32 + 12 = 44
    assert_eq!(out, "Out[3]= 44\n");
}

// ===========================================================================
// W-5 — list / functional / control / numeric built-ins, end-to-end.
// Each case is one of the acceptance examples from the W-5 brief.
// ===========================================================================

/// `Length`, `First`, `Last` on list literals.
#[test]
fn w5_length_first_last() {
    assert_eq!(eval("Length[{1, 2, 3}]\n").unwrap(), "Out[1]= 3\n");
    assert_eq!(eval("First[{9, 8}]\n").unwrap(), "Out[1]= 9\n");
    assert_eq!(eval("Last[{9, 8, 7}]\n").unwrap(), "Out[1]= 7\n");
    // Length of an empty list is 0.
    assert_eq!(eval("Length[{}]\n").unwrap(), "Out[1]= 0\n");
}

/// `First`/`Last` of an empty list are left unevaluated (no panic).
#[test]
fn w5_first_of_empty_is_unevaluated() {
    assert_eq!(eval("First[{}]\n").unwrap(), "Out[1]= First[{}]\n");
}

/// `Part` — 1-based, with negatives and the `0` (head) index.
#[test]
fn w5_part() {
    assert_eq!(eval("Part[{a, b, c}, 2]\n").unwrap(), "Out[1]= b\n");
    assert_eq!(eval("Part[{a, b, c}, -1]\n").unwrap(), "Out[1]= c\n");
    // Out of range stays unevaluated.
    assert_eq!(
        eval("Part[{a, b, c}, 9]\n").unwrap(),
        "Out[1]= Part[{a, b, c}, 9]\n"
    );
}

/// `Append` builds a new list.
#[test]
fn w5_append() {
    assert_eq!(eval("Append[{1, 2}, 3]\n").unwrap(), "Out[1]= {1, 2, 3}\n");
}

/// `Range` in its one-, two-, and three-argument forms.
#[test]
fn w5_range() {
    assert_eq!(eval("Range[3]\n").unwrap(), "Out[1]= {1, 2, 3}\n");
    assert_eq!(eval("Range[2, 5]\n").unwrap(), "Out[1]= {2, 3, 4, 5}\n");
    assert_eq!(eval("Range[1, 7, 2]\n").unwrap(), "Out[1]= {1, 3, 5, 7}\n");
}

/// `Range[10^9]`-style giant span is refused (left unevaluated), never
/// allocated — the W-5 DoS cap.
#[test]
fn w5_range_oversize_is_unevaluated_not_oom() {
    // 100_000_000 is well above MAX_RANGE_LENGTH; this must NOT hang/OOM.
    let out = eval("Range[100000000]\n").unwrap();
    assert_eq!(out, "Out[1]= Range[100000000]\n");
}

/// `Map` applies a function element-wise and re-evaluates the results.
#[test]
fn w5_map() {
    // f is unbound, so the results stay symbolic f[1], f[2].
    assert_eq!(eval("Map[f, {1, 2}]\n").unwrap(), "Out[1]= {f[1], f[2]}\n");
    // A built-in folds: Map[Sin, {0}] → {0}.
    assert_eq!(eval("Map[Sin, {0}]\n").unwrap(), "Out[1]= {0}\n");
}

/// `Apply` swaps the list head and re-evaluates: `Apply[Plus, {…}]` sums via the
/// Plus→Add bridge.
#[test]
fn w5_apply() {
    assert_eq!(eval("Apply[Plus, {1, 2, 3}]\n").unwrap(), "Out[1]= 6\n");
    assert_eq!(eval("Apply[Times, {2, 3, 4}]\n").unwrap(), "Out[1]= 24\n");
    // Apply with an unbound head stays symbolic as the application.
    assert_eq!(eval("Apply[g, {a, b}]\n").unwrap(), "Out[1]= g[a, b]\n");
}

/// `If` — the held control head selects a branch; comparisons drive the test.
#[test]
fn w5_if() {
    assert_eq!(eval("If[1 > 0, a, b]\n").unwrap(), "Out[1]= a\n");
    assert_eq!(eval("If[1 < 0, a, b]\n").unwrap(), "Out[1]= b\n");
    // A non-boolean condition leaves the If unevaluated.
    assert_eq!(eval("If[x, a, b]\n").unwrap(), "Out[1]= If[x, a, b]\n");
}

/// `N` coerces exact numbers to floats and maps over a list.
#[test]
fn w5_numeric_n() {
    assert_eq!(eval("N[1/2]\n").unwrap(), "Out[1]= 0.5\n");
    assert_eq!(eval("N[3]\n").unwrap(), "Out[1]= 3.0\n");
    assert_eq!(eval("N[{1, 1/4}]\n").unwrap(), "Out[1]= {1.0, 0.25}\n");
}

/// Built-ins compose: the VM evaluates inner heads before the outer one.
#[test]
fn w5_builtins_compose() {
    // Length[Append[Range[3], 9]] = Length[{1,2,3,9}] = 4
    assert_eq!(
        eval("Length[Append[Range[3], 9]]\n").unwrap(),
        "Out[1]= 4\n"
    );
    // First[Map[f, Range[2]]] = First[{f[1], f[2]}] = f[1]
    assert_eq!(eval("First[Map[f, Range[2]]]\n").unwrap(), "Out[1]= f[1]\n");
    // Apply[Plus, Range[4]] = 1+2+3+4 = 10
    assert_eq!(eval("Apply[Plus, Range[4]]\n").unwrap(), "Out[1]= 10\n");
}

// ===========================================================================
// W-6 — operator sugar /@, @@, [[ ]], end-to-end.
// Each sugar form must evaluate IDENTICALLY to its W-5 head form (MA04 §9).
// ===========================================================================

/// `f /@ x` ≡ `Map[f, x]` — same output string, exactly.
#[test]
fn w6_map_sugar_equals_map_head() {
    assert_eq!(
        eval("f /@ {1, 2}\n").unwrap(),
        eval("Map[f, {1, 2}]\n").unwrap()
    );
    assert_eq!(eval("f /@ {1, 2}\n").unwrap(), "Out[1]= {f[1], f[2]}\n");
    // A built-in folds through the sugar just as through the head form.
    assert_eq!(eval("Sin /@ {0}\n").unwrap(), "Out[1]= {0}\n");
}

/// `f @@ x` ≡ `Apply[f, x]`; `Plus @@ {1, 2, 3}` is `6`.
#[test]
fn w6_apply_sugar_equals_apply_head() {
    assert_eq!(
        eval("Plus @@ {1, 2, 3}\n").unwrap(),
        eval("Apply[Plus, {1, 2, 3}]\n").unwrap()
    );
    assert_eq!(eval("Plus @@ {1, 2, 3}\n").unwrap(), "Out[1]= 6\n");
    assert_eq!(eval("Times @@ {2, 3, 4}\n").unwrap(), "Out[1]= 24\n");
    // Unbound head stays symbolic as the application, like the head form.
    assert_eq!(eval("g @@ {a, b}\n").unwrap(), "Out[1]= g[a, b]\n");
}

/// `x[[i]]` ≡ `Part[x, i]`; `{a, b, c}[[2]]` is `b`.
#[test]
fn w6_part_sugar_equals_part_head() {
    assert_eq!(
        eval("{a, b, c}[[2]]\n").unwrap(),
        eval("Part[{a, b, c}, 2]\n").unwrap()
    );
    assert_eq!(eval("{a, b, c}[[2]]\n").unwrap(), "Out[1]= b\n");
    // Negative indexing and out-of-range carry over from Part unchanged.
    assert_eq!(eval("{a, b, c}[[-1]]\n").unwrap(), "Out[1]= c\n");
    assert_eq!(
        eval("{a, b, c}[[9]]\n").unwrap(),
        "Out[1]= Part[{a, b, c}, 9]\n"
    );
}

/// Nested / chained `[[ ]]` indexes a nested list: `{{1,2},{3,4}}[[1]][[2]]` = 2.
#[test]
fn w6_part_sugar_nests() {
    assert_eq!(eval("{{1, 2}, {3, 4}}[[1]][[2]]\n").unwrap(), "Out[1]= 2\n");
    // The multi-index spelling is identical: m[[1, 2]] == m[[1]][[2]].
    assert_eq!(
        eval("{{1, 2}, {3, 4}}[[1, 2]]\n").unwrap(),
        eval("{{1, 2}, {3, 4}}[[1]][[2]]\n").unwrap()
    );
}

/// Sugar interleaves with ordinary application without disturbing it.
#[test]
fn w6_sugar_interleaves_with_application() {
    // First[{a, b}][[…]] is nonsense, but f[…][[…]] is fine:
    // Range[3][[2]] = Part[{1,2,3}, 2] = 2.
    assert_eq!(eval("Range[3][[2]]\n").unwrap(), "Out[1]= 2\n");
    // Apply sugar feeding a head form: Plus @@ Range[4] = 10.
    assert_eq!(eval("Plus @@ Range[4]\n").unwrap(), "Out[1]= 10\n");
    // Map sugar then Part sugar: (f /@ {1,2})[[1]] = f[1].
    assert_eq!(eval("(f /@ {1, 2})[[1]]\n").unwrap(), "Out[1]= f[1]\n");
}

/// W-4/W-5 behaviour is unchanged by the W-6 grammar growth — the existing
/// forms still parse and evaluate exactly as before (regression guard).
#[test]
fn w6_does_not_disturb_existing_forms() {
    assert_eq!(eval("1 + 2*3\n").unwrap(), "Out[1]= 7\n");
    assert_eq!(eval("f[g[x]]\n").unwrap(), "Out[1]= f[g[x]]\n"); // nested apply
    assert_eq!(eval("x /. x -> 9\n").unwrap(), "Out[1]= 9\n");
    assert_eq!(eval("Map[f, {1, 2}]\n").unwrap(), "Out[1]= {f[1], f[2]}\n");
    assert_eq!(eval("Part[{a, b, c}, 2]\n").unwrap(), "Out[1]= b\n");
}


// ---------------------------------------------------------------------------
// W-7 — iteration constructs (Table, Do, Sum, Product)
// ---------------------------------------------------------------------------

/// `Table` builds a list of the body evaluated with the index bound over a
/// range — both the `{i, imax}` and `{i, imin, imax}` spec forms.
#[test]
fn w7_table_two_and_three_bound_forms() {
    // Table[i^2, {i, 3}] → {1, 4, 9} (index ranges 1..=3).
    assert_eq!(eval("Table[i^2, {i, 3}]\n").unwrap(), "Out[1]= {1, 4, 9}\n");
    // Table[i, {i, 2, 4}] → {2, 3, 4} (explicit lower bound).
    assert_eq!(eval("Table[i, {i, 2, 4}]\n").unwrap(), "Out[1]= {2, 3, 4}\n");
    // Stepped: Table[i, {i, 1, 9, 2}] → {1, 3, 5, 7, 9}.
    assert_eq!(
        eval("Table[i, {i, 1, 9, 2}]\n").unwrap(),
        "Out[1]= {1, 3, 5, 7, 9}\n"
    );
}

/// The index is *local*: the body sees `i` bound to each value, and the symbol
/// `i` does not leak into the session afterward (still a free symbol).
#[test]
fn w7_table_index_is_local() {
    let mut s = WolframSession::new();
    assert_eq!(s.feed("Table[i, {i, 3}]\n").unwrap(), "Out[1]= {1, 2, 3}\n");
    // After the Table, `i` is still unbound (free), not 3 — no env leak.
    assert_eq!(s.feed("i\n").unwrap(), "Out[2]= i\n");
}

/// The iterator *bounds* are evaluated even though the head is held — a bound
/// may be an expression (`{i, 1+1}`) or reference a session binding.
#[test]
fn w7_iterator_bounds_are_evaluated() {
    // A computed bound: Table[i, {i, 1+1}] → {1, 2}.
    assert_eq!(eval("Table[i, {i, 1+1}]\n").unwrap(), "Out[1]= {1, 2}\n");
    // A bound that references a prior binding.
    let mut s = WolframSession::new();
    s.feed("n = 4\n").unwrap();
    assert_eq!(s.feed("Table[i, {i, n}]\n").unwrap(), "Out[2]= {1, 2, 3, 4}\n");
}

/// `Sum` folds `+` over the range; `Product` folds `×`. The canonical
/// acceptance values from the W-7 brief.
#[test]
fn w7_sum_and_product() {
    assert_eq!(eval("Sum[i, {i, 1, 10}]\n").unwrap(), "Out[1]= 55\n");
    assert_eq!(eval("Product[i, {i, 1, 4}]\n").unwrap(), "Out[1]= 24\n");
    // A non-trivial body: Sum[i^2, {i, 1, 3}] = 1 + 4 + 9 = 14.
    assert_eq!(eval("Sum[i^2, {i, 1, 3}]\n").unwrap(), "Out[1]= 14\n");
}

/// An empty range returns the fold identity: `Sum` → 0, `Product` → 1,
/// `Table` → `{}`. (A wrong-way / degenerate range iterates zero times.)
#[test]
fn w7_empty_range_returns_identity() {
    assert_eq!(eval("Sum[i, {i, 5, 1}]\n").unwrap(), "Out[1]= 0\n");
    assert_eq!(eval("Product[i, {i, 5, 1}]\n").unwrap(), "Out[1]= 1\n");
    assert_eq!(eval("Table[i, {i, 0}]\n").unwrap(), "Out[1]= {}\n");
}

/// `Do` evaluates the body once per index *for side effects* and returns
/// `Null`. We prove the body ran the right number of times by observing the
/// final value of a variable it assigns: after `Do[x = i, {i, 3}]`, `x` is 3.
#[test]
fn w7_do_runs_n_times_and_returns_null() {
    let mut s = WolframSession::new();
    assert_eq!(s.feed("Do[x = i, {i, 3}]\n").unwrap(), "Out[1]= Null\n");
    // The body ran for i = 1, 2, 3 — the last assignment leaves x = 3.
    assert_eq!(s.feed("x\n").unwrap(), "Out[2]= 3\n");
}

/// Nested `Table` — each level binds its own index cleanly, and the cap
/// composes (the inner build is itself bounded). `i*j` requires explicit `*`.
#[test]
fn w7_nested_table() {
    // i ∈ {1, 2}, j ∈ {1, 2}: rows are {i*1, i*2}.
    assert_eq!(
        eval("Table[Table[i*j, {j, 2}], {i, 2}]\n").unwrap(),
        "Out[1]= {{1, 2}, {2, 4}}\n"
    );
}

/// **DoS cap**: an over-large iterator is left unevaluated rather than hanging
/// or exhausting memory — exactly the `Range` `MAX_RANGE_LENGTH` behaviour. The
/// test returns promptly (no allocation of two-million elements).
#[test]
fn w7_oversize_iterator_is_capped_not_oom() {
    // 2,000,000 > MAX_RANGE_LENGTH (1,000,000): stays unevaluated.
    assert_eq!(
        eval("Table[0, {i, 2000000}]\n").unwrap(),
        "Out[1]= Table[0, {i, 2000000}]\n"
    );
    // `Do` is capped identically even though it allocates nothing — the cap
    // bounds wall-clock work, so this also returns immediately.
    assert_eq!(
        eval("Do[0, {i, 2000000}]\n").unwrap(),
        "Out[1]= Do[0, {i, 2000000}]\n"
    );
}

/// A span too wide for `i64` but valid `i64` *bounds* must not overflow: the
/// count is computed in `i128`, exceeds the cap, and the form stays
/// unevaluated (no panic, no wrap).
#[test]
fn w7_extreme_span_does_not_overflow() {
    let src = "Sum[1, {i, -9000000000000000000, 9000000000000000000}]\n";
    assert_eq!(
        eval(src).unwrap(),
        "Out[1]= Sum[1, {i, -9000000000000000000, 9000000000000000000}]\n"
    );
}

/// A malformed iterator spec leaves the whole form unevaluated (never a panic):
/// a missing bound (`{i}`), a zero step, or a non-integer bound.
#[test]
fn w7_malformed_spec_stays_unevaluated() {
    // No bound: {i} is not a valid iterator.
    assert_eq!(eval("Table[i, {i}]\n").unwrap(), "Out[1]= Table[i, {i}]\n");
    // Zero step never terminates — refused.
    assert_eq!(
        eval("Table[i, {i, 1, 5, 0}]\n").unwrap(),
        "Out[1]= Table[i, {i, 1, 5, 0}]\n"
    );
    // A non-integer bound: this subset only iterates over integer ranges.
    assert_eq!(
        eval("Table[i, {i, 1.5}]\n").unwrap(),
        "Out[1]= Table[i, {i, 1.5}]\n"
    );
}

/// A negative / descending stepped range works (reuses the `Range` span logic).
#[test]
fn w7_negative_and_descending_ranges() {
    assert_eq!(eval("Table[i, {i, -2, 2}]\n").unwrap(), "Out[1]= {-2, -1, 0, 1, 2}\n");
    assert_eq!(eval("Table[i, {i, 5, 1, -2}]\n").unwrap(), "Out[1]= {5, 3, 1}\n");
}

// ---------------------------------------------------------------------------
// W-8 — local scoping (With, Module, Block)
// ---------------------------------------------------------------------------

/// `With[{x = e}, body]` substitutes the evaluated value of `x` into `body`.
/// The canonical acceptance values from the W-8 brief.
#[test]
fn w8_with_single_and_multiple_locals() {
    // With[{x = 3}, x^2] → 9.
    assert_eq!(eval("With[{x = 3}, x^2]\n").unwrap(), "Out[1]= 9\n");
    // With[{a = 1, b = 2}, a + b] → 3 (parallel binding).
    assert_eq!(eval("With[{a = 1, b = 2}, a + b]\n").unwrap(), "Out[1]= 3\n");
}

/// `Module[{x, y = e}, body]` — initialised locals bind like `With`; the
/// acceptance value `Module[{a = 1, b = 2}, a + b]` → `3`.
#[test]
fn w8_module_initialised_locals() {
    assert_eq!(eval("Module[{a = 1, b = 2}, a + b]\n").unwrap(), "Out[1]= 3\n");
}

/// `Block[{x = e}, body]` — for a self-contained body it binds like `With`;
/// `Block[{x = 5}, x + 1]` → `6`.
#[test]
fn w8_block_binds_local() {
    assert_eq!(eval("Block[{x = 5}, x + 1]\n").unwrap(), "Out[1]= 6\n");
}

/// **Locals do not leak to the session.** After `With[{x = 3}, x]`, a bare `x`
/// is still the free symbol `x` (the session env was never written). Same for
/// `Module` and `Block`.
#[test]
fn w8_locals_do_not_leak() {
    let mut s = WolframSession::new();
    assert_eq!(s.feed("With[{x = 3}, x]\n").unwrap(), "Out[1]= 3\n");
    // `x` is still unbound (free), not 3 — no env leak.
    assert_eq!(s.feed("x\n").unwrap(), "Out[2]= x\n");

    let mut s = WolframSession::new();
    assert_eq!(s.feed("Module[{y = 7}, y]\n").unwrap(), "Out[1]= 7\n");
    assert_eq!(s.feed("y\n").unwrap(), "Out[2]= y\n");

    let mut s = WolframSession::new();
    assert_eq!(s.feed("Block[{z = 9}, z]\n").unwrap(), "Out[1]= 9\n");
    assert_eq!(s.feed("z\n").unwrap(), "Out[2]= z\n");
}

/// A local must not *clobber* a same-named global: a global `x` set before the
/// scope is unchanged by a `With`/`Module`/`Block` that binds its own `x`.
#[test]
fn w8_local_does_not_clobber_a_global() {
    let mut s = WolframSession::new();
    s.feed("x = 100\n").unwrap();
    // Inside the scope, the local `x` shadows the global.
    assert_eq!(s.feed("With[{x = 1}, x]\n").unwrap(), "Out[2]= 1\n");
    // The global `x` is untouched afterwards.
    assert_eq!(s.feed("x\n").unwrap(), "Out[3]= 100\n");
}

/// Nested scoping: each level binds its own local cleanly, and an inner body
/// sees both the inner and the (already-substituted) outer local.
#[test]
fn w8_nested_scopes() {
    // With[{x = 1}, With[{y = 2}, x + y]] → 3.
    assert_eq!(
        eval("With[{x = 1}, With[{y = 2}, x + y]]\n").unwrap(),
        "Out[1]= 3\n"
    );
    // Module nested inside With composes too.
    assert_eq!(
        eval("With[{a = 10}, Module[{b = 5}, a + b]]\n").unwrap(),
        "Out[1]= 15\n"
    );
}

/// A declaration may **refer to an outer binding** — the RHS is evaluated
/// against the surrounding scope before substitution.
#[test]
fn w8_decl_refers_to_outer_binding() {
    // The inner decl `y = x + 1` reads the outer local `x = 1` → y = 2.
    assert_eq!(
        eval("With[{x = 1}, With[{y = x + 1}, y]]\n").unwrap(),
        "Out[1]= 2\n"
    );
    // A decl reading a session binding.
    let mut s = WolframSession::new();
    s.feed("n = 4\n").unwrap();
    assert_eq!(s.feed("With[{m = n + 1}, m]\n").unwrap(), "Out[2]= 5\n");
}

/// `Module` allows an *uninitialised* local, which stays symbolic in the body
/// (it is α-renamed to a fresh gensym, so it does not resolve to any global of
/// the same name).
#[test]
fn w8_module_uninitialised_local_is_symbolic() {
    // Module[{u}, u + 1] → 1 + u$nnn (u stays a free symbol; `+` keeps it
    // symbolic). The base name survives in the gensym, so the output mentions u.
    let out = eval("Module[{u}, u + 1]\n").unwrap();
    assert!(out.contains('u'), "uninitialised local should stay symbolic: {out:?}");
    // Even with a global `u = 42` set, the uninitialised Module local shadows it:
    // the result is a fresh `u$nnn` symbol, NOT the global value 42.
    let mut s = WolframSession::new();
    s.feed("u = 42\n").unwrap();
    let out = s.feed("Module[{u}, u]\n").unwrap();
    assert!(
        out.contains("u$") && !out.contains("42"),
        "uninitialised local must shadow the global, got {out:?}"
    );
}

/// Malformed scoping forms are left unevaluated (never a panic): a non-list
/// declaration argument, a `With`/`Block` local with no value, wrong arity.
#[test]
fn w8_malformed_forms_stay_unevaluated() {
    // First argument not a list.
    assert_eq!(eval("With[x, x]\n").unwrap(), "Out[1]= With[x, x]\n");
    // With requires an initialised local; a bare `x` is rejected.
    assert_eq!(eval("With[{x}, x]\n").unwrap(), "Out[1]= With[{x}, x]\n");
    assert_eq!(eval("Block[{x}, x]\n").unwrap(), "Out[1]= Block[{x}, x]\n");
}

/// W-4..W-7 behaviour is unchanged by the W-8 handlers (regression guard).
#[test]
fn w8_does_not_disturb_existing_forms() {
    assert_eq!(eval("1 + 2*3\n").unwrap(), "Out[1]= 7\n");
    assert_eq!(eval("Table[i^2, {i, 3}]\n").unwrap(), "Out[1]= {1, 4, 9}\n");
    assert_eq!(eval("Sum[i, {i, 1, 10}]\n").unwrap(), "Out[1]= 55\n");
    assert_eq!(eval("Map[f, {1, 2}]\n").unwrap(), "Out[1]= {f[1], f[2]}\n");
}

// ---------------------------------------------------------------------------
// W-9 list-manipulation builtins — Sort, Reverse, Join, Flatten, Select, Count,
// Total (plus the EvenQ/OddQ predicates). Full parse → lower → eval → print.
// ---------------------------------------------------------------------------

/// `Sort` orders a numeric list ascending, and round-trips through the printer.
#[test]
fn w9_sort_orders_ascending() {
    assert_eq!(eval("Sort[{3, 1, 2}]\n").unwrap(), "Out[1]= {1, 2, 3}\n");
    assert_eq!(eval("Sort[{}]\n").unwrap(), "Out[1]= {}\n");
    // Numbers sort before symbols in the subset's canonical order.
    assert_eq!(
        eval("Sort[{c, 2, a, 1}]\n").unwrap(),
        "Out[1]= {1, 2, a, c}\n"
    );
}

/// `Reverse` reverses a list.
#[test]
fn w9_reverse_reverses() {
    assert_eq!(eval("Reverse[{1, 2, 3}]\n").unwrap(), "Out[1]= {3, 2, 1}\n");
    assert_eq!(eval("Reverse[{}]\n").unwrap(), "Out[1]= {}\n");
}

/// `Join` concatenates two or more lists.
#[test]
fn w9_join_concatenates() {
    assert_eq!(eval("Join[{1}, {2, 3}]\n").unwrap(), "Out[1]= {1, 2, 3}\n");
    assert_eq!(
        eval("Join[{1}, {2}, {3}]\n").unwrap(),
        "Out[1]= {1, 2, 3}\n"
    );
    // A non-list argument leaves the form unevaluated.
    assert_eq!(eval("Join[{1}, 2]\n").unwrap(), "Out[1]= Join[{1}, 2]\n");
}

/// `Flatten` flattens all levels by default; `Flatten[list, n]` only the top n.
#[test]
fn w9_flatten_full_and_depth_n() {
    assert_eq!(
        eval("Flatten[{{1, 2}, {3}}]\n").unwrap(),
        "Out[1]= {1, 2, 3}\n"
    );
    // All levels.
    assert_eq!(
        eval("Flatten[{1, {2, {3}}}]\n").unwrap(),
        "Out[1]= {1, 2, 3}\n"
    );
    // One level only — the inner {3} survives.
    assert_eq!(
        eval("Flatten[{1, {2, {3}}}, 1]\n").unwrap(),
        "Out[1]= {1, 2, {3}}\n"
    );
}

/// `EvenQ`/`OddQ` classify integers (and are False for non-integers).
#[test]
fn w9_even_q_and_odd_q() {
    assert_eq!(eval("EvenQ[4]\n").unwrap(), "Out[1]= True\n");
    assert_eq!(eval("OddQ[3]\n").unwrap(), "Out[1]= True\n");
    assert_eq!(eval("EvenQ[3]\n").unwrap(), "Out[1]= False\n");
    assert_eq!(eval("EvenQ[x]\n").unwrap(), "Out[1]= False\n");
}

/// `Select`/`Count` apply a predicate (here the built-in `EvenQ`) to each element.
#[test]
fn w9_select_and_count_with_a_predicate() {
    assert_eq!(
        eval("Select[{1, 2, 3, 4}, EvenQ]\n").unwrap(),
        "Out[1]= {2, 4}\n"
    );
    assert_eq!(eval("Count[{1, 2, 3, 4}, EvenQ]\n").unwrap(), "Out[1]= 2\n");
}

/// `Select`/`Count` also accept a *user-defined* predicate — the same
/// application path as `Map`/`Apply`, so a `SetDelayed` function works.
#[test]
fn w9_select_with_a_user_defined_predicate() {
    let mut s = WolframSession::new();
    // big[x_] := x > 2  — a comparison predicate returning True/False.
    s.feed("big[x_] := x > 2\n").unwrap();
    assert_eq!(
        s.feed("Select[{1, 2, 3, 4}, big]\n").unwrap(),
        "Out[2]= {3, 4}\n"
    );
    assert_eq!(
        s.feed("Count[{1, 2, 3, 4}, big]\n").unwrap(),
        "Out[3]= 2\n"
    );
}

/// `Total` sums a list onto the shared `Add` head.
#[test]
fn w9_total_sums() {
    assert_eq!(eval("Total[{1, 2, 3}]\n").unwrap(), "Out[1]= 6\n");
    assert_eq!(eval("Total[{}]\n").unwrap(), "Out[1]= 0\n");
    // Consistent with Sum over a range.
    assert_eq!(eval("Sum[i, {i, 1, 3}]\n").unwrap(), "Out[1]= 6\n");
}

/// W-4..W-8 behaviour is unchanged by the W-9 handlers (regression guard).
#[test]
fn w9_does_not_disturb_existing_forms() {
    assert_eq!(eval("1 + 2*3\n").unwrap(), "Out[1]= 7\n");
    assert_eq!(eval("Map[f, {1, 2}]\n").unwrap(), "Out[1]= {f[1], f[2]}\n");
    assert_eq!(eval("Range[3]\n").unwrap(), "Out[1]= {1, 2, 3}\n");
    assert_eq!(eval("With[{x = 3}, x^2]\n").unwrap(), "Out[1]= 9\n");
}

// ---------------------------------------------------------------------------
// W-10 functional-iteration combinators — Nest, NestList, Fold, FoldList
// ---------------------------------------------------------------------------

/// `Nest[f, x, n]` applies a *symbolic* `f` `n` times, building the literal nest.
#[test]
fn w10_nest_symbolic() {
    assert_eq!(eval("Nest[f, x, 3]\n").unwrap(), "Out[1]= f[f[f[x]]]\n");
    // Zero applications is the identity.
    assert_eq!(eval("Nest[f, x, 0]\n").unwrap(), "Out[1]= x\n");
}

/// `NestList[f, x, n]` collects the `n + 1` intermediates, seed included.
#[test]
fn w10_nest_list_symbolic() {
    assert_eq!(
        eval("NestList[f, x, 2]\n").unwrap(),
        "Out[1]= {x, f[x], f[f[x]]}\n"
    );
}

/// `Nest`/`NestList` over a *user-defined* `SetDelayed` function reduce at each
/// step — the same application path as `Map`. `g[a_] := a + 1` is the canonical
/// W-10 driver (pure-function `#`/`&` syntax is the planned W-11 item).
#[test]
fn w10_nest_list_with_a_user_function() {
    let mut s = WolframSession::new();
    s.feed("g[a_] := a + 1\n").unwrap();
    // NestList[g, 0, 3] → {0, 1, 2, 3}.
    assert_eq!(s.feed("NestList[g, 0, 3]\n").unwrap(), "Out[2]= {0, 1, 2, 3}\n");
    // Nest[g, 0, 3] → 3 (the last of those).
    assert_eq!(s.feed("Nest[g, 0, 3]\n").unwrap(), "Out[3]= 3\n");
}

/// `Fold[f, x0, list]` is a left fold; with `Plus` it totals.
#[test]
fn w10_fold_left_folds() {
    assert_eq!(eval("Fold[Plus, 0, {1, 2, 3}]\n").unwrap(), "Out[1]= 6\n");
    // Left-associative: ((10 - 1) - 2) - 3 = 4.
    assert_eq!(
        eval("Fold[Subtract, 10, {1, 2, 3}]\n").unwrap(),
        "Out[1]= 4\n"
    );
    // Empty list → the seed.
    assert_eq!(eval("Fold[Plus, 42, {}]\n").unwrap(), "Out[1]= 42\n");
}

/// `FoldList[f, x0, list]` collects the running accumulations, seed included.
#[test]
fn w10_fold_list_running_accumulations() {
    assert_eq!(
        eval("FoldList[Plus, 0, {1, 2, 3}]\n").unwrap(),
        "Out[1]= {0, 1, 3, 6}\n"
    );
    // Empty list → just the seed.
    assert_eq!(eval("FoldList[Plus, 7, {}]\n").unwrap(), "Out[1]= {7}\n");
}

/// Malformed / DoS forms are left unevaluated (echoed back), never a panic.
#[test]
fn w10_malformed_forms_stay_unevaluated() {
    // Negative count.
    assert_eq!(
        eval("Nest[f, x, -1]\n").unwrap(),
        "Out[1]= Nest[f, x, -1]\n"
    );
    // Non-list third argument to Fold.
    assert_eq!(
        eval("Fold[Plus, 0, x]\n").unwrap(),
        "Out[1]= Fold[Plus, 0, x]\n"
    );
    // An enormous count is refused before iterating (DoS cap) — a tiny input
    // (1000001 > MAX_LIST_LENGTH) cannot drive a million-plus evals.
    assert_eq!(
        eval("Nest[f, x, 1000001]\n").unwrap(),
        "Out[1]= Nest[f, x, 1000001]\n"
    );
}

/// W-4..W-9 behaviour is unchanged by the W-10 handlers (regression guard).
#[test]
fn w10_does_not_disturb_existing_forms() {
    assert_eq!(eval("1 + 2*3\n").unwrap(), "Out[1]= 7\n");
    assert_eq!(eval("Map[f, {1, 2}]\n").unwrap(), "Out[1]= {f[1], f[2]}\n");
    assert_eq!(eval("Total[{1, 2, 3}]\n").unwrap(), "Out[1]= 6\n");
    assert_eq!(eval("With[{x = 3}, x^2]\n").unwrap(), "Out[1]= 9\n");
}
